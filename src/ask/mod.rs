//! The answer service of an instance (`cortex.ask`): `cortex ask serve` answers `POST /ask` as a
//! stream of `cortex.ask.AskEvent`s, from a tool loop over the EKR MCP reader against the
//! instance's model gateway, within the spec's `ask.limits`.
//!
//! The loop ([`answer`]) is synchronous: it runs on a blocking thread of the server
//! ([`serve`]) and hands each event to the stream as it happens. Nothing here is on the path of
//! `cortex run`.
//!
//! Under the instance's `redaction` policy one answer is one redaction batch: the question and
//! each tool result are redacted before the model is given them ([`shown`]), a result still
//! holding a `refuse_if_left` class is replaced by a withheld tool error, and the answer is
//! restored as it is streamed. [`proxy`] does the same for the eval `mcp` runner's `claude`.

pub mod cite;
pub mod limit;
pub mod mcp;
pub mod model;
pub mod proxy;
pub mod records;
pub mod serve;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use cortex_model::ask as ma;
use cortex_model::instance as m;
use serde_json::{json, Value};

use crate::home::Home;
use crate::instance::Layout;
use crate::redact::Batch;

/// The event contract `cortex.ask.AskStatus.format` names.
pub const EVENTS_FORMAT: &str = "cortex.ask-events/1";

/// Why the records of `spec` have no place, naming the field, never a value: a `postgres` store
/// without `ask.records`, a `sqlite` store with one, or `ask.records` whose schema is no lowercase
/// identifier, is reserved or is the store's own (the `schema` of its `ekr.postgres/1` file,
/// `public` when it names none), or whose URL is no PostgreSQL URL or carries a password.
fn records_refusal(spec: &m::InstanceSpec, ask: &ma::AskSpec) -> Option<String> {
    use records::postgres::{schema_refusal, url_refusal};
    let store = match &spec.store {
        Some(m::StoreSpec::Postgres(p)) => p,
        _ => {
            return ask.records.as_ref().map(|_| {
                "ask.records: names a PostgreSQL schema, and the instance's store is not \
                 `postgres`; the records of a `sqlite` store live in ask.sqlite in the instance \
                 directory"
                    .to_string()
            })
        }
    };
    let Some(r) = &ask.records else {
        return Some(
            "ask.records: an instance on a `postgres` store keeps its answer records in a \
             PostgreSQL schema of their own, and the spec names none"
                .into(),
        );
    };
    if let Some(refused) = schema_refusal(&r.schema).or_else(|| url_refusal(&r.url)) {
        return Some(refused);
    }
    let config: Option<Value> = std::fs::read(crate::ekr::expand(&store.config))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    let Some(config) = config else {
        return Some(
            "ask.records.schema: cannot be held apart from the store's schema: the \
             ekr.postgres/1 file store.value.config names cannot be read as JSON"
                .into(),
        );
    };
    let ekr_schema = config["schema"].as_str().unwrap_or("public");
    (ekr_schema == r.schema).then(|| {
        "ask.records.schema: is the store's own schema (the schema of the ekr.postgres/1 file \
         store.value.config names); the answer records live in a schema of their own"
            .into()
    })
}

/// Why a spec cannot be served, naming the field: no `ask` section, no model gateway, a limit of
/// 0 or less (a retention or slow threshold included), an identity header that is no header name,
/// an empty listen address, or records with no place ([`records_refusal`]).
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
    for (field, value) in [
        ("ask.retention_days", ask.retention_days),
        ("ask.slow_after_ms", ask.slow_after_ms),
    ] {
        if let Some(value) = value.filter(|v| *v <= 0) {
            return Err(format!("{field}: {value} is not above 0"));
        }
    }
    if let Some(refused) = records_refusal(spec, ask) {
        return Err(refused);
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
        // `mcp::Reader::start` also removes every ANTHROPIC_* and CLAUDE_* variable.
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
    /// The instance's `redaction` policy, which [`Settings::of`] checked compiles.
    pub redaction: Option<m::RedactionPolicy>,
}

impl Settings {
    /// The settings of a spec [`check`] accepted.
    pub fn of(home: &Home, spec: &m::InstanceSpec) -> Result<Self, String> {
        let ask = check(spec)?.clone();
        crate::redact::compile(spec.redaction.as_ref())?;
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
            redaction: spec.redaction.clone(),
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

/// The tool error the model is given for a result in which the redaction policy still detects
/// a class its `refuse_if_left` names (`left`): it names the classes, never what was found.
pub fn withheld(left: &BTreeSet<&str>) -> String {
    format!(
        "The tool result was withheld by policy: the instance's redaction policy still detects {} \
         in it.",
        left.iter().copied().collect::<Vec<_>>().join(", ")
    )
}

/// Why a question in which the redaction policy still detects a class its `refuse_if_left` names
/// is refused: it names the classes, never what was found.
pub fn refused(left: &BTreeSet<&str>) -> String {
    format!(
        "the question was refused by policy: the instance's redaction policy still detects {} in \
         it, and it is not sent to the model.",
        left.iter().copied().collect::<Vec<_>>().join(", ")
    )
}

/// The reason `question` is refused under the settings' redaction policy, when a class its
/// `refuse_if_left` names is still detected in it as the model would be shown it.
pub fn question_refused(settings: &Settings, question: &str) -> Option<String> {
    let redactor = crate::redact::compile(settings.redaction.as_ref()).ok()??;
    let mut batch = redactor.batch();
    batch.reserve(question);
    let shown = batch.replace(question);
    let left = batch.left(&shown);
    (!left.is_empty()).then(|| refused(&left))
}

/// A tool result's text as the model is shown it, whether it was cut and whether it was
/// withheld. Without a policy (`batch` is `None`) it is `text` cut to `max` bytes. With one it is
/// redacted with the answer's batch, then cut; a cut text in which a `refuse_if_left` class is
/// still detected is replaced by the [`withheld`] tool error.
pub fn shown(batch: Option<&mut Batch<'_>>, text: &str, max: usize) -> (String, bool, bool) {
    let Some(batch) = batch else {
        let (shown, truncated) = cut(text, max);
        return (shown.to_string(), truncated, false);
    };
    // A withheld result is never shown, so none of its values may stay in the mapping: the
    // batch is put back as it was.
    let before = batch.clone();
    batch.reserve(text);
    let redacted = batch.replace(text);
    let (shown, truncated) = cut(&redacted, max);
    let left = batch.left(shown);
    if left.is_empty() {
        (shown.to_string(), truncated, false)
    } else {
        let message = withheld(&left);
        *batch = before;
        (message, false, true)
    }
}

/// `text` with every placeholder of `batch` that has a value restored; `text` itself without a
/// policy.
fn restored(batch: Option<&Batch<'_>>, text: &str) -> String {
    match batch {
        None => text.to_string(),
        Some(batch) => {
            let mut restoring = batch.restoring();
            let mut out = restoring.push(text);
            out.push_str(&restoring.finish().0);
            out
        }
    }
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
    // A limit past ten years is no limit in practice; the clamp keeps the sum from overflowing.
    let seconds = (limits.max_seconds as u64).min(10 * 365 * 24 * 3600);
    let deadline = Instant::now() + Duration::from_secs(seconds);
    let max_bytes = limits.max_result_bytes as usize;
    let gone = || model_failed("the request was closed before the answer".into());
    let status = |step: i64, message: &str| {
        ma::AskEvent::Status(ma::AskStatus {
            format: EVENTS_FORMAT.into(),
            step,
            message: message.into(),
        })
    };
    // One question is one redaction batch: one placeholder mapping, in memory, for this answer.
    let redactor = match crate::redact::compile(settings.redaction.as_ref()) {
        Ok(r) => r,
        Err(reason) => return model_failed(reason),
    };
    let mut batch = redactor.as_ref().map(|r| r.batch());
    let question = match batch.as_mut() {
        Some(batch) => {
            batch.reserve(question);
            batch.replace(question)
        }
        None => question.to_string(),
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
            // Each delta is restored as it is sent, holding back a tail that may be a split
            // placeholder; the answer is what the deltas carried.
            let mut restoring = batch.as_ref().map(|b| b.restoring());
            let mut pieces: Vec<String> = cited
                .answer
                .split_inclusive('\n')
                .map(|piece| match restoring.as_mut() {
                    Some(r) => r.push(piece),
                    None => piece.to_string(),
                })
                .collect();
            let mut unrestored = 0;
            if let Some(r) = restoring {
                let (rest, n) = r.finish();
                pieces.push(rest);
                unrestored = n;
            }
            let mut answer = String::new();
            for piece in pieces.into_iter().filter(|p| !p.is_empty()) {
                answer.push_str(&piece);
                if !emit(ma::AskEvent::Delta(ma::AskDelta { text: piece })) {
                    return gone();
                }
            }
            return ma::AskOutcome::Answered {
                answered: ma::Answered {
                    identity: identity.into(),
                    done: ma::AskDone {
                        answer_id: answer_id.into(),
                        answer,
                        nodes: cited.nodes,
                        evidence: cited.evidence,
                        input_tokens: usage.input,
                        output_tokens: usage.output,
                        cached_tokens: usage.cached,
                        context_tokens: context,
                        steps: step,
                        unrestored: unrestored as i64,
                    },
                },
            };
        }
        // Text before a tool call is the model thinking aloud, not the answer.
        if let Some(text) = reply.content.as_deref().filter(|t| !t.trim().is_empty()) {
            let text = restored(batch.as_ref(), text);
            if !emit(ma::AskEvent::Delta(ma::AskDelta { text }))
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
            let (shown, truncated, withheld) = shown(batch.as_mut(), &text, max_bytes);
            if !emit(ma::AskEvent::Tool(ma::AskTool {
                step,
                name: call.name.clone(),
                arguments: call.arguments.clone(),
                result_bytes: shown.len() as i64,
                truncated,
                failed,
                withheld,
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
