//! The instance's model gateway (`cortex.instance.ModelGateway`), spoken to as an
//! OpenAI-compatible endpoint: `POST <base_url>/v1/chat/completions` (`<base_url>/chat/completions`
//! when `base_url` already ends in `/v1`), with the token of `token_env` as a bearer token, read
//! when the answer starts and never written anywhere.

use std::time::Instant;

use serde_json::{json, Value};

use super::CallError;

/// Where the model is: the gateway's URL, the variable holding its token, and the model's name.
#[derive(Clone, Debug)]
pub struct Route {
    pub base_url: String,
    pub token_env: String,
    pub model: String,
}

/// Token counts, as the gateway's `usage` reports them; 0 where it reports none.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    pub input: i64,
    pub output: i64,
    pub cached: i64,
}

impl Usage {
    pub fn add(&mut self, other: &Usage) {
        self.input = self.input.saturating_add(other.input);
        self.output = self.output.saturating_add(other.output);
        self.cached = self.cached.saturating_add(other.cached);
    }
}

/// One tool call the model asked for.
#[derive(Clone, Debug)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    /// The arguments as the model gave them, JSON text.
    pub arguments: String,
}

/// One model answer.
#[derive(Clone, Debug)]
pub struct Reply {
    /// The assistant message as the gateway gave it, to send back in the conversation.
    pub message: Value,
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Usage,
    /// Why the model stopped (`/choices/0/finish_reason`), when the gateway says.
    pub finish_reason: Option<String>,
}

impl Reply {
    /// The finish reason when the reply was cut off: by the model's token limit (`length`) or a
    /// content filter (`content_filter`). Its text is then no answer.
    pub fn cut_off(&self) -> Option<&str> {
        self.finish_reason
            .as_deref()
            .filter(|r| matches!(*r, "length" | "content_filter"))
    }
}

pub struct Gateway {
    client: reqwest::blocking::Client,
    url: String,
    token: String,
    model: String,
}

impl Gateway {
    /// The gateway of `route`, its token read now; a missing or empty token is refused, naming
    /// the variable.
    pub fn new(route: &Route) -> Result<Self, String> {
        let token = std::env::var(&route.token_env)
            .ok()
            .filter(|t| !t.is_empty())
            .ok_or_else(|| {
                format!(
                    "the gateway token variable {} is not set or empty",
                    route.token_env
                )
            })?;
        let base = route.base_url.trim_end_matches('/');
        let url = if base.ends_with("/v1") {
            format!("{base}/chat/completions")
        } else {
            format!("{base}/v1/chat/completions")
        };
        let client = reqwest::blocking::Client::builder()
            .build()
            .map_err(|e| format!("cannot build the gateway client: {e}"))?;
        Ok(Self {
            client,
            url,
            token,
            model: route.model.clone(),
        })
    }

    /// Asks the model for the next message of `messages`, offering `tools`, by `deadline`.
    pub fn chat(
        &self,
        messages: &[Value],
        tools: &[Value],
        deadline: Instant,
    ) -> Result<Reply, CallError> {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(CallError::Timeout);
        }
        let mut body = json!({"model": self.model, "messages": messages, "stream": false});
        if !tools.is_empty() {
            body["tools"] = Value::from(tools.to_vec());
        }
        let response = self
            .client
            .post(&self.url)
            .bearer_auth(&self.token)
            .header("content-type", "application/json")
            .body(body.to_string())
            .timeout(left)
            .send()
            .map_err(|e| failure(&e))?;
        let status = response.status();
        let text = response.text().map_err(|e| failure(&e))?;
        if !status.is_success() {
            let (shown, _) = super::cut(text.trim(), 300);
            return Err(CallError::Failed(format!(
                "the gateway answered {status}: {shown}"
            )));
        }
        parse(&text).map_err(CallError::Failed)
    }
}

fn failure(e: &reqwest::Error) -> CallError {
    if e.is_timeout() {
        CallError::Timeout
    } else {
        CallError::Failed(format!("cannot reach the gateway: {e}"))
    }
}

fn count(v: &Value) -> i64 {
    v.as_i64().filter(|n| *n >= 0).unwrap_or(0)
}

/// A chat completion's first choice and its usage.
pub fn parse(text: &str) -> Result<Reply, String> {
    let value: Value =
        serde_json::from_str(text).map_err(|e| format!("the gateway answered no JSON: {e}"))?;
    let message = value
        .pointer("/choices/0/message")
        .cloned()
        .ok_or("the gateway answered no message")?;
    let tool_calls = message
        .get("tool_calls")
        .and_then(Value::as_array)
        .map(|calls| {
            calls
                .iter()
                .map(|c| ToolCall {
                    id: c.get("id").and_then(Value::as_str).unwrap_or("").into(),
                    name: c
                        .pointer("/function/name")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .into(),
                    arguments: match c.pointer("/function/arguments") {
                        Some(Value::String(s)) => s.clone(),
                        Some(other) => other.to_string(),
                        None => "{}".into(),
                    },
                })
                .collect()
        })
        .unwrap_or_default();
    let usage = value.get("usage").cloned().unwrap_or(Value::Null);
    Ok(Reply {
        content: message
            .get("content")
            .and_then(Value::as_str)
            .map(str::to_string),
        tool_calls,
        usage: Usage {
            input: count(&usage["prompt_tokens"]),
            output: count(&usage["completion_tokens"]),
            cached: count(&usage["prompt_tokens_details"]["cached_tokens"]),
        },
        finish_reason: value
            .pointer("/choices/0/finish_reason")
            .and_then(Value::as_str)
            .map(str::to_string),
        message,
    })
}
