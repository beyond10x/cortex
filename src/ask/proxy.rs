//! A redacting MCP server between `claude` and the store's MCP reader, for the eval `mcp` runner
//! under the instance's `redaction` policy.
//!
//! `claude` runs its own tool loop, so cortex cannot redact a tool result it never sees. Under a
//! policy the runner gives `claude` this server instead of `ekr mcp`: MCP over HTTP (the
//! streamable-HTTP transport, one JSON response per `POST`), bound to `127.0.0.1` on a free port
//! under a random path, for one case only. It answers `initialize` and `ping` itself and forwards
//! `tools/list` and `tools/call` to the reader ([`mcp::Reader`]). Each tool result is shown as
//! the ask service shows it ([`super::shown`]): its text redacted with the case's batch, or a
//! withheld tool error. The batch, the case's one placeholder mapping, stays in this process, so
//! the runner restores the answer with it; nothing of it is written anywhere.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::mcp::Reader;
use super::{compact, shown, CallError, ReaderCommand};
use crate::redact::{Batch, Redactor};

/// The most bytes of one request this server reads.
const MAX_REQUEST: usize = 1 << 20;

/// A tool result this server passes on whole: the reader's results are not cut.
const NO_CUT: usize = usize::MAX;

/// What `f` answered, and the batch the case's question and tool results were redacted with.
pub struct Served<'r, T> {
    pub answer: T,
    pub batch: Batch<'r>,
}

/// Starts the reader `command` and the server in front of it, redacts `question` with a new
/// batch of `redactor`, and calls `f` with the redacted question and the `--mcp-config` document
/// that names the server as `server`. The server and the reader stop when `f` returns.
pub fn serve<'r, T>(
    redactor: &'r Redactor,
    command: &ReaderCommand,
    server: &str,
    question: &str,
    deadline: Instant,
    f: impl FnOnce(&str, &Value) -> T,
) -> Result<Served<'r, T>, String> {
    let mut batch = redactor.batch();
    batch.reserve(question);
    let question = batch.replace(question);
    let reader = match Reader::start(command, deadline) {
        Ok(r) => r,
        Err(CallError::Timeout) => return Err("the MCP reader did not start in time".into()),
        Err(CallError::Failed(reason)) => return Err(format!("the MCP reader: {reason}")),
    };
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| format!("127.0.0.1: {e}"))?;
    let addr = listener.local_addr().map_err(|e| e.to_string())?;
    let path = format!("/{}", token()?);
    let config = json!({"mcpServers": {server: {
        "type": "http",
        "url": format!("http://{addr}{path}"),
    }}});
    let state = Mutex::new((reader, batch));
    let stop = AtomicBool::new(false);
    let answer = std::thread::scope(|s| {
        s.spawn(|| {
            for stream in listener.incoming() {
                if stop.load(Ordering::SeqCst) {
                    return;
                }
                if let Ok(stream) = stream {
                    handle(stream, &path, &state, deadline);
                }
            }
        });
        let answer = f(&question, &config);
        stop.store(true, Ordering::SeqCst);
        wake(addr);
        answer
    });
    let (reader, batch) = state.into_inner().unwrap_or_else(|e| e.into_inner());
    drop(reader);
    Ok(Served { answer, batch })
}

/// Unblocks the server's `accept` so it sees `stop`.
fn wake(addr: SocketAddr) {
    let _ = TcpStream::connect_timeout(&addr, Duration::from_secs(1));
}

/// 128 random bits, in hex: the server's path, so only who was given the configuration finds it.
fn token() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .map_err(|e| format!("/dev/urandom: {e}"))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// Answers one HTTP request on `stream`, then closes it.
fn handle(stream: TcpStream, path: &str, state: &Mutex<(Reader, Batch<'_>)>, deadline: Instant) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).unwrap_or(0) == 0 {
        return;
    }
    let mut length = 0;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                length = value.trim().parse().unwrap_or(0);
            }
        }
    }
    let mut parts = request_line.split_whitespace();
    let (method, target) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    let mut out = &stream;
    if target != path {
        return respond(&mut out, "404 Not Found", None);
    }
    if method != "POST" {
        return respond(&mut out, "405 Method Not Allowed", None);
    }
    if length > MAX_REQUEST {
        return respond(&mut out, "413 Payload Too Large", None);
    }
    let mut body = vec![0; length];
    if reader.read_exact(&mut body).is_err() {
        return;
    }
    let Ok(message) = serde_json::from_slice::<Value>(&body) else {
        return respond(&mut out, "400 Bad Request", None);
    };
    // A notification (no id) is accepted and needs no answer.
    let Some(id) = message.get("id").cloned() else {
        return respond(&mut out, "202 Accepted", None);
    };
    let method = message.get("method").and_then(Value::as_str).unwrap_or("");
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    let answer = match answer(method, &params, state, deadline) {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err((code, text)) => {
            json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": text}})
        }
    };
    respond(&mut out, "200 OK", Some(&answer));
}

/// The result of one request, or its JSON-RPC error.
fn answer(
    method: &str,
    params: &Value,
    state: &Mutex<(Reader, Batch<'_>)>,
    deadline: Instant,
) -> Result<Value, (i64, String)> {
    let failed = |e: CallError| match e {
        CallError::Timeout => (-32000, "the MCP reader did not answer in time".to_string()),
        CallError::Failed(reason) => (-32000, format!("the MCP reader: {reason}")),
    };
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": params.get("protocolVersion").cloned()
                .unwrap_or_else(|| json!(super::mcp::PROTOCOL)),
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "cortex-redacting-reader", "version": env!("CARGO_PKG_VERSION")},
        })),
        "ping" => Ok(json!({})),
        "tools/list" => {
            let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
            let tools = guard.0.tools(deadline).map_err(failed)?;
            Ok(json!({"tools": tools}))
        }
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
            let mut guard = state.lock().unwrap_or_else(|e| e.into_inner());
            let (reader, batch) = &mut *guard;
            let result = reader.call(name, arguments, deadline).map_err(failed)?;
            let (text, is_error) = compact(&result);
            let (text, _, withheld) = shown(Some(batch), &text, NO_CUT);
            Ok(json!({
                "content": [{"type": "text", "text": text}],
                "isError": is_error || withheld,
            }))
        }
        other => Err((-32601, format!("method not found: {other}"))),
    }
}

/// Writes one response with `body` as JSON, and asks the client to close the connection.
fn respond(out: &mut &TcpStream, status: &str, body: Option<&Value>) {
    let body = body.map(Value::to_string).unwrap_or_default();
    let kind = if body.is_empty() {
        ""
    } else {
        "content-type: application/json\r\n"
    };
    let _ = write!(
        out,
        "HTTP/1.1 {status}\r\n{kind}content-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = out.flush();
}
