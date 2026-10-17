//! The answer service of an instance (`cortex.ask`): `cortex ask serve` answers `POST /ask` as a
//! stream of `cortex.ask.AskEvent`s, from a tool loop over the EKR MCP reader against the
//! instance's model gateway, within the spec's `ask.limits`.
//!
//! The loop ([`answer`]) is synchronous: it runs on a blocking thread of the server
//! ([`serve`]) and hands each event to the stream as it happens. Nothing here is on the path of
//! `cortex run`.

pub mod cite;
pub mod limit;
pub mod mcp;
pub mod model;
pub mod records;
pub mod serve;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use cortex_model::ask as ma;
use cortex_model::instance as m;
use serde_json::{json, Value};

use crate::home::Home;
use crate::instance::Layout;

/// The event contract `cortex.ask.AskStatus.format` names.
pub const EVENTS_FORMAT: &str = "cortex.ask-events/1";

/// Why a spec cannot be served, naming the field: no `ask` section, no model gateway, a limit of
/// 0 or less, an identity header that is no header name, or an empty listen address.
pub fn check(spec: &m::InstanceSpec) -> Result<&ma::AskSpec, String> {
    let ask = spec
        .ask
        .as_ref()
        .ok_or("ask: the spec has no `ask` section to serve")?;
    if spec.model.gateway.is_none() {
        return Err(
            "model.gateway: the ask service speaks to the instance's model gateway, and \
             the spec names none"
                .into(),
        );
    }
    let limits = &ask.limits;
    for (field, value) in [
        ("ask.limits.max_steps", limits.max_steps),
        ("ask.limits.max_result_bytes", limits.max_result_bytes),
        ("ask.limits.max_seconds", limits.max_seconds),
        (
            "ask.limits.per_identity_per_minute",
            limits.per_identity_per_minute,
        ),
    ] {
        if value <= 0 {
            return Err(format!("{field}: {value} is not above 0"));
        }
    }
    let header = &ask.identity_header;
    if header.is_empty()
        || !header
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
    {
        return Err(format!(
            "ask.identity_header: {header:?} is not an HTTP header name"
        ));
    }
    if ask.listen.trim().is_empty() {
        return Err("ask.listen: empty".into());
    }
    Ok(ask)
}

/// How to start the instance's MCP reader, as `cortex mcp-line` prints it: `ekr mcp` under the
/// instance's host and store, or through `connectors connections launch` for a launched store.
#[derive(Clone, Debug)]
pub struct ReaderCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    /// Variables the reader must not inherit: the gateway's token among them.
    pub remove: Vec<String>,
}

impl ReaderCommand {
    /// The reader of the instance the spec names, in `home`.
    pub fn of(home: &Home, spec: &m::InstanceSpec) -> Result<Self, String> {
        let layout = Layout::new(home.instance_dir(&spec.name.0));
        let store = layout.store_handle(spec);
        let env = vec![
            ("EKR_HOST".into(), store.host.display().to_string()),
            ("EKR_BACKEND".into(), store.backend.name().to_string()),
            ("EKR_STORE".into(), store.store.display().to_string()),
        ];
        let mut remove = vec!["ANTHROPIC_API_KEY".to_string()];
        if let Some(g) = &spec.model.gateway {
            remove.push(g.token_env.clone());
        }
        let (program, args) = match &store.launch {
            Some(launch) => {
                let connection = launch.resolve(&launch.connection)?;
                (
                    launch.connectors.clone(),
                    vec![
                        "connections".into(),
                        "launch".into(),
                        "--adapter".into(),
                        launch.connection.adapter.clone(),
                        "--connection".into(),
                        connection,
                        "--consumer".into(),
                        crate::ekr::CONSUMER.into(),
                        "--args".into(),
                        json!(["mcp"]).to_string(),
                    ],
                )
            }
            None => (store.bin.clone(), vec!["mcp".into()]),
        };
        Ok(Self {
            program,
            args,
            env,
            remove,
        })
    }
}

/// Everything one answer needs, fixed when the service starts.
#[derive(Clone, Debug)]
pub struct Settings {
    pub ask: ma::AskSpec,
    pub model: model::Route,
    pub reader: ReaderCommand,
    /// The system prompt: how to use the tools and cite, then the spec's `guidance`.
    pub system: String,
}

impl Settings {
    /// The settings of a spec [`check`] accepted.
    pub fn of(home: &Home, spec: &m::InstanceSpec) -> Result<Self, String> {
        let ask = check(spec)?.clone();
        let gateway = spec.model.gateway.as_ref().expect("checked");
        let mut system = format!(
            "You answer questions over the knowledge brain `{name}`: {description}\n\
             Read its store with the tools you are given and answer only from what they return. \
             Cite each node you use as [node:<id>] and each evidence as [evidence:<id>], with the \
             ids the tools gave you. When the store does not hold the answer, say so.",
            name = spec.name.0,
            description = spec.description.trim(),
        );
        if let Some(guidance) = &ask.guidance {
            system.push_str("\n\n");
            system.push_str(guidance.trim());
        }
        Ok(Self {
            model: model::Route {
                base_url: gateway.base_url.clone(),
                token_env: gateway.token_env.clone(),
                model: spec.model.model.clone(),
            },
            reader: ReaderCommand::of(home, spec)?,
            system,
            ask,
        })
    }
}

/// A failure of a model or reader call: past the answer's deadline, or anything else.
#[derive(Debug)]
pub enum CallError {
    Timeout,
    Failed(String),
}

/// A tool result as the model is shown it: the text of its content, each text that is JSON
/// written compactly, joined by newlines; and whether the reader answered an error.
pub fn compact(result: &Value) -> (String, bool) {
    let failed = result
        .get("isError")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let parts: Vec<String> = match result.get("content").and_then(Value::as_array) {
        Some(content) => content
            .iter()
            .map(|c| match c.get("text").and_then(Value::as_str) {
                Some(text) => match serde_json::from_str::<Value>(text) {
                    Ok(v) if v.is_object() || v.is_array() => v.to_string(),
                    _ => text.trim().to_string(),
                },
                None => format!(
                    "[{} content]",
                    c.get("type").and_then(Value::as_str).unwrap_or("unknown")
                ),
            })
            .collect(),
        None => vec![result.to_string()],
    };
    (parts.join("\n"), failed)
}

/// `text` cut at a character boundary to at most `max` bytes, and whether it was cut.
pub fn cut(text: &str, max: usize) -> (&str, bool) {
    if text.len() <= max {
        return (text, false);
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (&text[..end], true)
}

/// The last event of an answer: `done` for an answer, `error` for any other outcome.
pub fn final_event(outcome: &ma::AskOutcome) -> ma::AskEvent {
    let error = |reason: &str, message: String, retry_after: Option<i64>| {
        ma::AskEvent::Error(ma::AskError {
            reason: reason.into(),
            message,
            retry_after,
        })
    };
    match outcome {
        ma::AskOutcome::Answered { answered } => ma::AskEvent::Done(answered.done.clone()),
        ma::AskOutcome::Unidentified { error: e } => error(
            "unidentified",
            format!("the request carries no `{}` header", e.header),
            None,
        ),
        ma::AskOutcome::RateLimited { error: e } => error(
            "rate-limit",
            format!(
                "this identity asked its questions of the minute; ask again in {} s",
                e.retry_after
            ),
            Some(e.retry_after),
        ),
        ma::AskOutcome::LimitReached { error: e } => error(
            &e.limit,
            match e.limit.as_str() {
                "step-limit" => "the model still asked for a tool at the step limit".into(),
                _ => "the answer took longer than the time limit".into(),
            },
            None,
        ),
        ma::AskOutcome::ModelFailed { error: e } => error("model-failed", e.reason.clone(), None),
        ma::AskOutcome::ReaderFailed { error: e } => error("reader-failed", e.reason.clone(), None),
    }
}

fn limit(limit: &str) -> ma::AskOutcome {
    ma::AskOutcome::LimitReached {
        error: ma::LimitReached {
            limit: limit.into(),
        },
    }
}

fn reader_failed(reason: String) -> ma::AskOutcome {
    ma::AskOutcome::ReaderFailed {
        error: ma::ReaderFailed { reason },
    }
}

fn model_failed(reason: String) -> ma::AskOutcome {
    ma::AskOutcome::ModelFailed {
        error: ma::ModelFailed { reason },
    }
}

/// Answers `question` for `identity`, under `answer_id`: the tool loop. Each event before the last is handed to
/// `emit` as it happens; `emit` answers false once nobody reads the stream, which ends the loop.
/// The outcome is the answer or why there is none; [`final_event`] makes it the last event.
pub fn answer(
    settings: &Settings,
    answer_id: &str,
    identity: &str,
    question: &str,
    emit: &mut dyn FnMut(ma::AskEvent) -> bool,
) -> ma::AskOutcome {
    let limits = &settings.ask.limits;
    let deadline = Instant::now() + Duration::from_secs(limits.max_seconds as u64);
    let max_bytes = limits.max_result_bytes as usize;
    let gone = || model_failed("the request was closed before the answer".into());
    let status = |step: i64, message: &str| {
        ma::AskEvent::Status(ma::AskStatus {
            format: EVENTS_FORMAT.into(),
            step,
            message: message.into(),
        })
    };
    if !emit(status(0, "starting the reader")) {
        return gone();
    }
    let mut reader = match mcp::Reader::start(&settings.reader, deadline) {
        Ok(r) => r,
        Err(CallError::Timeout) => return limit("time-limit"),
        Err(CallError::Failed(reason)) => return reader_failed(reason),
    };
    let tools: Vec<Value> = match reader.tools(deadline) {
        Ok(tools) => tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.get("name").cloned().unwrap_or(Value::Null),
                        "description": t.get("description").cloned().unwrap_or(json!("")),
                        "parameters": t.get("inputSchema").cloned()
                            .unwrap_or_else(|| json!({"type": "object"})),
                    }
                })
            })
            .collect(),
        Err(CallError::Timeout) => return limit("time-limit"),
        Err(CallError::Failed(reason)) => return reader_failed(reason),
    };
    let gateway = match model::Gateway::new(&settings.model) {
        Ok(g) => g,
        Err(reason) => return model_failed(reason),
    };
    let mut messages = vec![
        json!({"role": "system", "content": settings.system}),
        json!({"role": "user", "content": question}),
    ];
    let mut usage = model::Usage::default();
    for step in 1..=limits.max_steps {
        if Instant::now() >= deadline {
            return limit("time-limit");
        }
        if !emit(status(step, "asking the model")) {
            return gone();
        }
        let reply = match gateway.chat(&messages, &tools, deadline) {
            Ok(r) => r,
            Err(CallError::Timeout) => return limit("time-limit"),
            Err(CallError::Failed(reason)) => return model_failed(reason),
        };
        usage.add(&reply.usage);
        let context = reply.usage.input;
        if reply.tool_calls.is_empty() {
            let text = reply.content.unwrap_or_default();
            let cited = cite::cite(&text, settings.ask.link_templates.as_ref());
            for piece in cited.answer.split_inclusive('\n') {
                if !emit(ma::AskEvent::Delta(ma::AskDelta { text: piece.into() })) {
                    return gone();
                }
            }
            return ma::AskOutcome::Answered {
                answered: ma::Answered {
                    identity: identity.into(),
                    done: ma::AskDone {
                        answer_id: answer_id.into(),
                        answer: cited.answer,
                        nodes: cited.nodes,
                        evidence: cited.evidence,
                        input_tokens: usage.input,
                        output_tokens: usage.output,
                        cached_tokens: usage.cached,
                        context_tokens: context,
                        steps: step,
                    },
                },
            };
        }
        // Text before a tool call is the model thinking aloud, not the answer.
        if let Some(text) = reply.content.as_deref().filter(|t| !t.trim().is_empty()) {
            if !emit(ma::AskEvent::Delta(ma::AskDelta { text: text.into() }))
                || !emit(ma::AskEvent::Discard(ma::AskDiscard { step }))
            {
                return gone();
            }
        }
        if step == limits.max_steps {
            return limit("step-limit");
        }
        messages.push(reply.message.clone());
        for call in &reply.tool_calls {
            let (text, failed) = match serde_json::from_str::<Value>(&call.arguments) {
                Ok(arguments) => match reader.call(&call.name, arguments, deadline) {
                    Ok(result) => compact(&result),
                    Err(CallError::Timeout) => return limit("time-limit"),
                    Err(CallError::Failed(reason)) => return reader_failed(reason),
                },
                Err(e) => (format!("the arguments are not JSON: {e}"), true),
            };
            let (shown, truncated) = cut(&text, max_bytes);
            if !emit(ma::AskEvent::Tool(ma::AskTool {
                step,
                name: call.name.clone(),
                arguments: call.arguments.clone(),
                result_bytes: shown.len() as i64,
                truncated,
                failed,
            })) {
                return gone();
            }
            messages.push(json!({"role": "tool", "tool_call_id": call.id, "content": shown}));
        }
    }
    limit("step-limit")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cut_keeps_whole_characters() {
        assert_eq!(cut("abc", 3), ("abc", false));
        assert_eq!(cut("abcd", 3), ("abc", true));
        // `é` is two bytes: a cut inside it drops it whole.
        assert_eq!(cut("aé", 2), ("a", true));
    }

    #[test]
    fn a_json_result_is_written_compactly() {
        let result = json!({"content": [{"type": "text", "text": "{ \"a\" : [1, 2] }"}]});
        assert_eq!(compact(&result), ("{\"a\":[1,2]}".to_string(), false));
        let failed = json!({"isError": true, "content": [{"type": "text", "text": "no"}]});
        assert_eq!(compact(&failed), ("no".to_string(), true));
    }
}
