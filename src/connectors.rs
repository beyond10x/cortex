//! The `connectors` binary: which connections exist, and invoking an admitted operation. cortex
//! never sees a credential; Connectors signs every call.

use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

pub struct Connectors {
    pub bin: PathBuf,
}

/// Why an invocation answered nothing.
#[derive(Debug)]
pub enum InvokeError {
    /// Connectors refused at admission: no such connection, adapter or operation here.
    Missing(String),
    /// Connectors refused at admission because the connection's validation evidence lapsed.
    Lapsed(String),
    /// Anything else: an outage, a refused or failed provider call, an unreadable answer.
    Failed(String),
}

impl std::fmt::Display for InvokeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(m) => write!(f, "connection missing: {m}"),
            Self::Lapsed(m) => write!(f, "connection evidence lapsed: {m}"),
            Self::Failed(m) => write!(f, "connectors failed: {m}"),
        }
    }
}

fn envelope(stdout: &[u8], what: &str) -> Result<Value, InvokeError> {
    let doc: Value = serde_json::from_slice(stdout)
        .map_err(|e| InvokeError::Failed(format!("{what}: no JSON answer: {e}")))?;
    if doc["ok"] == Value::Bool(true) {
        return Ok(doc["result"].clone());
    }
    let data = &doc["error"]["data"];
    let code = data["code"].as_str().unwrap_or("unknown");
    let stage = data["stage"].as_str().unwrap_or("unknown");
    let message = format!("{what}: {code} at {stage}");
    if code == "not_granted" && stage == "admission" {
        return Err(InvokeError::Lapsed(message));
    }
    if code == "not_found" && stage == "admission" {
        Err(InvokeError::Missing(message))
    } else {
        Err(InvokeError::Failed(message))
    }
}

impl Connectors {
    fn json(&self, args: &[&str], what: &str) -> Result<Value, InvokeError> {
        let out = Command::new(&self.bin)
            .args(args)
            .args(["--output", "json"])
            .output()
            .map_err(|e| InvokeError::Failed(format!("cannot run connectors: {e}")))?;
        // A refusal's JSON envelope goes to stderr, with nothing on stdout.
        let answer = if out.stdout.iter().all(u8::is_ascii_whitespace) {
            &out.stderr
        } else {
            &out.stdout
        };
        envelope(answer, what)
    }

    /// The state of `connection` among `adapter`'s connections, or `None` when it is not listed.
    pub fn connection_state(
        &self,
        adapter: &str,
        connection: &str,
    ) -> Result<Option<String>, InvokeError> {
        let listed = self.json(
            &["connections", "list", "--adapter", adapter],
            "connections list",
        )?;
        Ok(listed["connections"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|c| c["connection"].as_str() == Some(connection))
            .map(|c| c["state"].as_str().unwrap_or("unknown").to_string()))
    }

    /// Renews a connection whose validation evidence lapsed (`pending`), so a read is admitted.
    /// Connectors keeps evidence for a short lifetime (60 s for the shipped HTTP adapters), and
    /// refuses reads with `not_granted` after it; a scheduled run therefore revalidates first.
    pub fn ensure_ready(&self, adapter: &str, connection: &str) -> Result<(), InvokeError> {
        self.revalidate(adapter, connection, false)
    }

    /// Revalidates `connection`; unless `force`, only when it is not `ready`.
    fn revalidate(&self, adapter: &str, connection: &str, force: bool) -> Result<(), InvokeError> {
        let listed = self.json(
            &["connections", "list", "--adapter", adapter],
            "connections list",
        )?;
        let Some(found) = listed["connections"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|c| c["connection"].as_str() == Some(connection))
        else {
            return Err(InvokeError::Missing(format!(
                "{adapter}: no connection {connection}"
            )));
        };
        if !force && found["state"].as_str() == Some("ready") {
            return Ok(());
        }
        let revision = found["revision"].as_str().unwrap_or_default().to_string();
        self.json(
            &[
                "connections",
                "revalidate",
                "--adapter",
                adapter,
                "--connection",
                connection,
                "--expected-revision",
                &revision,
            ],
            "connections revalidate",
        )
        .map(|_| ())
    }

    /// Invokes `operation` once and answers the adapter's own result. The CLI wraps it as
    /// `{adapter, operation, revision, result}`, with `result` a JSON document in a string.
    pub fn invoke(
        &self,
        adapter: &str,
        connection: &str,
        operation: &str,
        input: &Value,
    ) -> Result<Value, InvokeError> {
        self.ensure_ready(adapter, connection)?;
        // Evidence listed as ready can lapse before the read is admitted; renew once and retry.
        let answer = match self.invoke_once(adapter, connection, operation, input) {
            Err(InvokeError::Lapsed(_)) => {
                self.revalidate(adapter, connection, true)?;
                self.invoke_once(adapter, connection, operation, input)?
            }
            other => other?,
        };
        match answer.get("result") {
            Some(Value::String(text)) => serde_json::from_str(text).map_err(|e| {
                InvokeError::Failed(format!("{adapter}.{operation}: result is not JSON: {e}"))
            }),
            Some(inner) if inner.is_object() => Ok(inner.clone()),
            _ => Ok(answer),
        }
    }

    fn invoke_once(
        &self,
        adapter: &str,
        connection: &str,
        operation: &str,
        input: &Value,
    ) -> Result<Value, InvokeError> {
        let described = self.json(
            &[
                "operations",
                "describe",
                "--adapter",
                adapter,
                "--operation",
                operation,
            ],
            "operations describe",
        )?;
        let schema = described["schema"]
            .as_str()
            .ok_or_else(|| InvokeError::Failed(format!("{adapter}.{operation}: no schema")))?
            .to_string();
        let revision = described["revision"]
            .as_str()
            .ok_or_else(|| InvokeError::Failed(format!("{adapter}.{operation}: no revision")))?
            .to_string();
        let input = serde_json::to_string(input).expect("plain JSON");
        self.json(
            &[
                "operations",
                "invoke",
                "--adapter",
                adapter,
                "--connection",
                connection,
                "--operation",
                operation,
                "--schema",
                &schema,
                "--revision",
                &revision,
                "--input-json",
                &input,
            ],
            &format!("{adapter}.{operation}"),
        )
    }
}

/// A provider's answer body: the catalog engine wraps an HTTP answer as `{status, body}`.
pub fn body(result: &Value) -> &Value {
    match result.get("body") {
        Some(body) if body.is_object() || body.is_array() => body,
        _ => result,
    }
}
