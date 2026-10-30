//! The EKR MCP reader (`ekr mcp`), spoken to over its stdio: one JSON-RPC message per line. One
//! reader is started per answer and killed when the answer ends.
//!
//! The reader is not pooled across answers, on purpose: an answer that ends at a limit, fails or
//! loses its client may leave its reader mid-call, stuck or holding state, and only killing it
//! guarantees the next answer does not inherit it. `tests/ask_adversary.rs` holds that every
//! reader an answer started is gone when its stream ends, and that a closed request stops its
//! reader. The cost is one start (and one store open) per question, bounded by
//! `ask.max_in_flight` readers at once.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Instant;

use serde_json::{json, Value};

use super::{CallError, ReaderCommand};

/// The MCP protocol version cortex asks for.
pub(crate) const PROTOCOL: &str = "2025-06-18";

pub struct Reader {
    child: Child,
    /// The requests for the writer thread, which owns the child's stdin: a reader that stops
    /// reading blocks that thread, never the answer, whose deadline the wait for a response keeps.
    requests: Sender<String>,
    lines: Receiver<String>,
    next: u64,
}

impl Drop for Reader {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Reader {
    /// Starts the reader and completes the MCP handshake by `deadline`.
    pub fn start(command: &ReaderCommand, deadline: Instant) -> Result<Self, CallError> {
        let mut cmd = Command::new(&command.program);
        cmd.args(&command.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        // The reader reads a store and needs no model credential: no ANTHROPIC_* or CLAUDE_*
        // variable, nor the names the command lists (the gateway's token), reaches it.
        // Nor any `EKR_*` variable it inherits (such as `EKR_STAGE` or `EKR_FULL_REPLAY` from the
        // operator's shell): only the ones the command sets below, as for a store `ekr` a run
        // starts (`crate::ekr`).
        for (name, _) in std::env::vars_os() {
            let name = name.to_string_lossy();
            if name.starts_with("ANTHROPIC_")
                || name.starts_with("CLAUDE_")
                || name.starts_with("EKR_")
            {
                cmd.env_remove(name.as_ref());
            }
        }
        for name in &command.remove {
            cmd.env_remove(name);
        }
        for (name, value) in &command.env {
            cmd.env(name, value);
        }
        let mut child = cmd.spawn().map_err(|e| {
            CallError::Failed(format!(
                "cannot start the reader {}: {e}",
                command.program.display()
            ))
        })?;
        let stdin = child.stdin.take().expect("piped");
        let stdout = child.stdout.take().expect("piped");
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { return };
                if tx.send(line).is_err() {
                    return;
                }
            }
        });
        let (requests, queue) = mpsc::channel::<String>();
        std::thread::spawn(move || {
            let mut stdin = stdin;
            for line in queue {
                if writeln!(stdin, "{line}")
                    .and_then(|_| stdin.flush())
                    .is_err()
                {
                    return;
                }
            }
        });
        let mut reader = Self {
            child,
            requests,
            lines,
            next: 1,
        };
        reader
            .request(
                "initialize",
                json!({
                    "protocolVersion": PROTOCOL,
                    "capabilities": {},
                    "clientInfo": {"name": "cortex-ask", "version": env!("CARGO_PKG_VERSION")},
                }),
                deadline,
            )?
            .map_err(CallError::Failed)?;
        reader.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))?;
        Ok(reader)
    }

    /// The tools the reader offers, as `tools/list` answers them.
    pub fn tools(&mut self, deadline: Instant) -> Result<Vec<Value>, CallError> {
        let result = self
            .request("tools/list", json!({}), deadline)?
            .map_err(CallError::Failed)?;
        Ok(result
            .get("tools")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default())
    }

    /// Calls `name` with `arguments`: the `CallToolResult`, or for an error the reader answered,
    /// a result holding its message, marked `isError`.
    pub fn call(
        &mut self,
        name: &str,
        arguments: Value,
        deadline: Instant,
    ) -> Result<Value, CallError> {
        Ok(self
            .request(
                "tools/call",
                json!({"name": name, "arguments": arguments}),
                deadline,
            )?
            .unwrap_or_else(
                |text| json!({"isError": true, "content": [{"type": "text", "text": text}]}),
            ))
    }

    fn send(&mut self, message: &Value) -> Result<(), CallError> {
        self.requests
            .send(message.to_string())
            .map_err(|_| CallError::Failed("the reader stopped reading".into()))
    }

    /// Sends one request and waits for the response with its id, skipping anything else: its
    /// result, or the message of the error the reader answered.
    fn request(
        &mut self,
        method: &str,
        params: Value,
        deadline: Instant,
    ) -> Result<Result<Value, String>, CallError> {
        let id = self.next;
        self.next += 1;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))?;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(CallError::Timeout);
            }
            let line = match self.lines.recv_timeout(left) {
                Ok(line) => line,
                Err(RecvTimeoutError::Timeout) => return Err(CallError::Timeout),
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(CallError::Failed(format!(
                        "the reader exited before it answered `{method}`"
                    )))
                }
            };
            let Ok(message) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            if message.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            if let Some(error) = message.get("error") {
                let text = error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("no message");
                return Ok(Err(format!(
                    "the reader answered `{method}` with an error: {text}"
                )));
            }
            return Ok(Ok(message.get("result").cloned().unwrap_or(Value::Null)));
        }
    }
}
