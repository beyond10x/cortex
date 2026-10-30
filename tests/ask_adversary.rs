//! Adversarial cases for `cortex ask serve` (`story:ask-service`): the HTTP surface, the stream's
//! one terminal event, the spec's limits, the reader child and its environment.
//!
//! The world is built like `tests/ask_service.rs`: a stand-in OpenAI-compatible gateway in this
//! process (here one thread per connection, with an optional delay) and a stand-in `ekr` whose
//! `mcp` speaks MCP over stdio. The stand-in records its pid in `<root>/ekr.pids` and its
//! environment in `<root>/ekr.env`.

mod common;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use common::executable;
use cortex_cli::ask::records::{self, Records};
use cortex_model::ask as ma;
use serde_json::{json, Value};

const TOKEN_ENV: &str = "ASK_ADV_GATEWAY_TOKEN";
const TOKEN: &str = "adversary-gateway-token-7f3a";
const HEADER: &str = "x-asker";

/// A stand-in gateway: each request on its own thread, answered after `delay` with the next
/// scripted reply (the last one again once the script is spent); each body is recorded.
struct Gateway {
    url: String,
    requests: Arc<Mutex<Vec<Value>>>,
}

impl Gateway {
    fn start(script: Vec<Value>, delay: Duration) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let seen = requests.clone();
        let script = Arc::new(script);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { return };
                let (seen, script) = (seen.clone(), script.clone());
                std::thread::spawn(move || {
                    let body = read_body(&stream);
                    let n = {
                        let mut seen = seen.lock().unwrap();
                        seen.push(serde_json::from_slice(&body).unwrap_or(Value::Null));
                        seen.len()
                    };
                    std::thread::sleep(delay);
                    let reply = script[(n - 1).min(script.len() - 1)].to_string();
                    let _ = write!(
                        stream,
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}",
                        reply.len()
                    );
                });
            }
        });
        Self { url, requests }
    }

    fn asked(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}

fn read_body(stream: &TcpStream) -> Vec<u8> {
    let mut reader = BufReader::new(stream);
    let mut length = 0;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
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
    let mut body = vec![0; length];
    let _ = reader.read_exact(&mut body);
    body
}

fn tool_reply(n: usize, arguments: &str) -> Value {
    json!({
        "id": format!("r{n}"),
        "object": "chat.completion",
        "choices": [{
            "index": 0,
            "finish_reason": "tool_calls",
            "message": {
                "role": "assistant",
                "content": "Let me look that up.",
                "tool_calls": [{
                    "id": format!("call-{n}"),
                    "type": "function",
                    "function": {"name": "search", "arguments": arguments}
                }]
            }
        }],
        "usage": {"prompt_tokens": 10, "completion_tokens": 1}
    })
}

fn text_reply(text: &str) -> Value {
    json!({
        "id": "final",
        "object": "chat.completion",
        "choices": [{
            "index": 0,
            "finish_reason": "stop",
            "message": {"role": "assistant", "content": text}
        }],
        "usage": {"prompt_tokens": 10, "completion_tokens": 1}
    })
}

const SEARCH: &str = "{\"query\":\"widget\"}";

/// How the stand-in `ekr mcp` behaves: shell run at its start, after it answered `tools/list`,
/// and for each `tools/call` (with `$id` set; the default answers `<root>/tool-result.txt`).
struct Reader {
    on_start: &'static str,
    after_list: &'static str,
    on_call: &'static str,
}

const ANSWER_CALL: &str = r#"printf '{"jsonrpc":"2.0","id":%s,"result":{"content":[{"type":"text","text":"%s"}]}}\n' "$id" "$(cat "$R/tool-result.txt")""#;

impl Default for Reader {
    fn default() -> Self {
        Self {
            on_start: ":",
            after_list: ":",
            on_call: ANSWER_CALL,
        }
    }
}

fn stand_in_ekr(root: &Path, reader: &Reader) -> PathBuf {
    let path = root.join("bin/ekr");
    let body = r#"R="@ROOT@"
[ "$1" = mcp ] || exit 2
echo $$ >> "$R/ekr.pids"
env > "$R/ekr.env"
@START@
while IFS= read -r line; do
  id=$(printf '%s' "$line" | head -c 200 | sed -n 's/.*"id":\([0-9][0-9]*\).*/\1/p')
  case "$line" in
    *'"method":"initialize"'*) printf '{"jsonrpc":"2.0","id":%s,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"ekr","version":"0"}}}\n' "$id" ;;
    *'"method":"tools/list"'*) printf '{"jsonrpc":"2.0","id":%s,"result":{"tools":[{"name":"search","description":"Search","inputSchema":{"type":"object"}}]}}\n' "$id"; @LIST@ ;;
    *'"method":"tools/call"'*) printf '%s\n' "$line" | head -c 200 >> "$R/mcp-calls.log"; @CALL@ ;;
  esac
done
"#
    .replace("@ROOT@", &root.display().to_string())
    .replace("@START@", reader.on_start)
    .replace("@LIST@", reader.after_list)
    .replace("@CALL@", reader.on_call);
    executable(&path, &body);
    path
}

struct Service {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    child: Child,
    addr: String,
}

impl Service {
    /// The pids of every reader this service started.
    fn reader_pids(&self) -> Vec<u32> {
        std::fs::read_to_string(self.root.join("ekr.pids"))
            .unwrap_or_default()
            .lines()
            .filter_map(|l| l.trim().parse().ok())
            .collect()
    }

    /// Kills, by its exact pid, each reader this test's service started that is still alive.
    fn kill_readers(&self) {
        for pid in self.reader_pids() {
            if alive(pid) {
                let _ = Command::new("kill")
                    .arg("-9")
                    .arg(pid.to_string())
                    .stderr(Stdio::null())
                    .status();
            }
        }
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.kill_readers();
    }
}

/// Whether `pid` is a live process (a zombie counts: nobody reaped it).
fn alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

struct Setup<'a> {
    limits: &'a str,
    tool_result: &'a str,
    reader: Reader,
    env: Vec<(&'a str, &'a str)>,
    /// Lines added under `ask:`, each indented by two spaces.
    ask_extra: &'a str,
}

impl Default for Setup<'_> {
    fn default() -> Self {
        Self {
            limits: LIMITS,
            tool_result: "a result",
            reader: Reader::default(),
            env: vec![],
            ask_extra: "",
        }
    }
}

const LIMITS: &str =
    "{max_steps: 4, max_result_bytes: 4096, max_seconds: 30, per_identity_per_minute: 10}";

/// Starts `cortex ask serve`; `Err` with the exit code and stderr when it does not start.
fn try_serve(gateway: &Gateway, setup: Setup<'_>) -> Result<Service, (Option<i32>, String)> {
    let tmp = tempfile::Builder::new()
        .prefix("cortex-ask-adv")
        .tempdir_in(
            std::env::var_os("CARGO_TARGET_TMPDIR")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir),
        )
        .unwrap();
    let root = tmp.path().to_path_buf();
    std::fs::create_dir_all(root.join("bin")).unwrap();
    std::fs::write(root.join("tool-result.txt"), setup.tool_result).unwrap();
    let ekr = stand_in_ekr(&root, &setup.reader);
    let spec = root.join("asker.yaml");
    std::fs::write(
        &spec,
        format!(
            r#"format: cortex.instance/1
name: asker
description: Test brain.
ekr: {{version: "0.0.34", bin: "{ekr}"}}
seed: {{documents: []}}
model:
  model: test-model
  budget_tokens: 100000
  timeout_s: 60
  gateway: {{base_url: "{url}", token_env: {TOKEN_ENV}}}
sources: []
serve: {{view_port: 18999}}
ask:
  listen: "127.0.0.1:0"
  identity_header: {HEADER}
  limits: {limits}
{extra}
"#,
            ekr = ekr.display(),
            url = gateway.url,
            limits = setup.limits,
            extra = setup.ask_extra,
        ),
    )
    .unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_cortex"));
    command
        .args(["ask", "serve", "--spec"])
        .arg(&spec)
        .env("CORTEX_HOME", root.join("home"))
        .env(TOKEN_ENV, TOKEN)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    for (name, value) in &setup.env {
        command.env(name, value);
    }
    let mut child = command.spawn().unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let Ok(ready) = serde_json::from_str::<Value>(line.trim()) else {
        let status = child.wait().unwrap();
        return Err((status.code(), line));
    };
    let addr = ready["listening"].as_str().unwrap().to_string();
    Ok(Service {
        _tmp: tmp,
        root,
        child,
        addr,
    })
}

fn serve(gateway: &Gateway, setup: Setup<'_>) -> Service {
    try_serve(gateway, setup).unwrap_or_else(|e| panic!("`cortex ask serve` did not start: {e:?}"))
}

struct Answer {
    status: u16,
    content_type: String,
    events: Vec<(String, Value)>,
}

impl Answer {
    fn names(&self) -> Vec<&str> {
        self.events.iter().map(|(n, _)| n.as_str()).collect()
    }

    /// Holds the stream contract: exactly one `done` or `error`, and it is the last event.
    fn terminal(&self) -> &(String, Value) {
        let names = self.names();
        let terminals = names
            .iter()
            .filter(|n| **n == "done" || **n == "error")
            .count();
        assert_eq!(
            terminals, 1,
            "not exactly one done or error (status {}): {names:?}",
            self.status
        );
        let last = self.events.last().unwrap();
        assert!(last.0 == "done" || last.0 == "error", "{names:?}");
        last
    }
}

/// Posts `body` as `identity`, reading the whole stream within `timeout`.
fn post(
    service: &Service,
    identity: Option<&str>,
    body: Vec<u8>,
    timeout: Duration,
) -> Result<Answer, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()
        .unwrap();
    let mut request = client
        .post(format!("http://{}/ask", service.addr))
        .header("content-type", "application/json")
        .body(body);
    if let Some(identity) = identity {
        request = request.header(HEADER, identity);
    }
    let response = request.send().map_err(|e| format!("send: {e}"))?;
    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let text = response.text().map_err(|e| format!("read: {e}"))?;
    let events = if content_type.starts_with("text/event-stream") {
        parse_stream(&text)
    } else {
        vec![]
    };
    Ok(Answer {
        status,
        content_type,
        events,
    })
}

fn ask(service: &Service, identity: &str, question: &str) -> Answer {
    post(
        service,
        Some(identity),
        json!({"question": question}).to_string().into_bytes(),
        Duration::from_secs(60),
    )
    .unwrap()
}

fn parse_stream(text: &str) -> Vec<(String, Value)> {
    let mut events = Vec::new();
    for block in text.split("\n\n").filter(|b| !b.trim().is_empty()) {
        let mut name = None;
        let mut data = String::new();
        for line in block.lines() {
            if let Some(n) = line.strip_prefix("event:") {
                name = Some(n.trim().to_string());
            } else if let Some(d) = line.strip_prefix("data:") {
                data.push_str(d.strip_prefix(' ').unwrap_or(d));
            }
        }
        let name = name.unwrap_or_else(|| panic!("an event without a name: {block:?}"));
        let decoded: cortex_spec_types::CortexAskAskEvent = serde_json::from_str(&data)
            .unwrap_or_else(|e| panic!("`{name}` is not a cortex.ask.AskEvent ({e}): {data}"));
        let wire = serde_json::to_value(&decoded).unwrap();
        assert_eq!(
            wire["event"],
            json!(name),
            "the SSE name and the tag differ"
        );
        events.push((name, wire["value"].clone()));
    }
    events
}

// ---------------------------------------------------------------------------------------------
// Spec limits

/// `max_seconds` is "an Integer above 0" and `ask::check` accepts any such value. At the Integer
/// maximum the service must either refuse the spec (exit 2, naming the field) or answer with
/// exactly one terminal event.
#[test]
fn max_seconds_at_the_integer_maximum_still_ends_an_answer_with_one_terminal_event() {
    let gateway = Gateway::start(vec![text_reply("Yes [node:n-1].")], Duration::ZERO);
    let limits = "{max_steps: 4, max_result_bytes: 4096, max_seconds: 9223372036854775807, \
                  per_identity_per_minute: 10}";
    let service = match try_serve(
        &gateway,
        Setup {
            limits,
            ..Setup::default()
        },
    ) {
        Ok(service) => service,
        Err((code, out)) => {
            assert_eq!(code, Some(2), "{out}");
            return;
        }
    };
    let answer = ask(&service, "alice", "Is there an answer?");
    assert_eq!(answer.status, 200);
    let (name, _) = answer.terminal();
    assert_eq!(name, "done", "{:?}", answer.events);
}

// ---------------------------------------------------------------------------------------------
// A reader or a model that hangs or dies

/// A reader that stops reading its stdin (stuck, or busy) while the model asks for a call whose
/// arguments exceed the pipe's buffer: `max_seconds` is "every model and tool call included", so
/// the answer must end with `time-limit` (or `reader-failed`) shortly after it, not hang.
#[test]
fn a_reader_that_stops_reading_cannot_hold_an_answer_past_max_seconds() {
    let arguments = json!({"query": "a".repeat(256 * 1024)}).to_string();
    let gateway = Gateway::start(
        vec![tool_reply(1, &arguments), text_reply("Never [node:n-1].")],
        Duration::ZERO,
    );
    let limits =
        "{max_steps: 4, max_result_bytes: 4096, max_seconds: 2, per_identity_per_minute: 10}";
    let service = serve(
        &gateway,
        Setup {
            limits,
            reader: Reader {
                after_list: "exec sleep 40",
                ..Reader::default()
            },
            ..Setup::default()
        },
    );
    let started = Instant::now();
    let answer = post(
        &service,
        Some("alice"),
        json!({"question": "Hang?"}).to_string().into_bytes(),
        Duration::from_secs(12),
    );
    let took = started.elapsed();
    service.kill_readers();
    let answer = answer.unwrap_or_else(|e| {
        panic!("no complete answer {took:?} into an answer of max_seconds 2: {e}")
    });
    let (name, error) = answer.terminal();
    assert_eq!(name, "error");
    assert!(
        error["reason"] == "time-limit" || error["reason"] == "reader-failed",
        "{error}"
    );
}

/// A model gateway that accepts the request and never answers ends the answer with `time-limit`
/// shortly after `max_seconds`.
#[test]
fn a_model_that_never_answers_ends_with_time_limit() {
    let gateway = Gateway::start(vec![text_reply("Late.")], Duration::from_secs(30));
    let limits =
        "{max_steps: 4, max_result_bytes: 4096, max_seconds: 2, per_identity_per_minute: 10}";
    let service = serve(
        &gateway,
        Setup {
            limits,
            ..Setup::default()
        },
    );
    let started = Instant::now();
    let answer = post(
        &service,
        Some("alice"),
        json!({"question": "Slow?"}).to_string().into_bytes(),
        Duration::from_secs(12),
    )
    .unwrap();
    assert!(started.elapsed() < Duration::from_secs(8));
    let (name, error) = answer.terminal();
    assert_eq!(name, "error");
    assert_eq!(error["reason"], "time-limit");
}

/// A reader that exits before the handshake ends the answer with `reader-failed`, and asks no
/// model.
#[test]
fn a_reader_that_exits_at_once_ends_with_reader_failed() {
    let gateway = Gateway::start(vec![text_reply("Yes.")], Duration::ZERO);
    let service = serve(
        &gateway,
        Setup {
            reader: Reader {
                on_start: "exit 3",
                ..Reader::default()
            },
            ..Setup::default()
        },
    );
    let answer = ask(&service, "alice", "Reader?");
    let (name, error) = answer.terminal();
    assert_eq!(name, "error");
    assert_eq!(error["reason"], "reader-failed");
    assert_eq!(gateway.asked(), 0);
}

/// A reader that dies on its first tool call ends the answer with `reader-failed`.
#[test]
fn a_reader_that_dies_on_a_call_ends_with_reader_failed() {
    let gateway = Gateway::start(
        vec![tool_reply(1, SEARCH), text_reply("Yes.")],
        Duration::ZERO,
    );
    let service = serve(
        &gateway,
        Setup {
            reader: Reader {
                on_call: "exit 1",
                ..Reader::default()
            },
            ..Setup::default()
        },
    );
    let answer = ask(&service, "alice", "Die?");
    let (name, error) = answer.terminal();
    assert_eq!(name, "error");
    assert_eq!(error["reason"], "reader-failed");
}

// ---------------------------------------------------------------------------------------------
// Children and their environment

/// Every reader an answer started is gone (killed and reaped) once its stream ended, for an
/// answer and for a step-limit error alike.
#[test]
fn every_reader_an_answer_started_is_gone_when_its_stream_ends() {
    let gateway = Gateway::start(
        vec![tool_reply(1, SEARCH), text_reply("Yes [node:n-1].")],
        Duration::ZERO,
    );
    let service = serve(&gateway, Setup::default());
    assert_eq!(ask(&service, "alice", "One?").terminal().0, "done");
    let looping = Gateway::start(vec![tool_reply(1, SEARCH)], Duration::ZERO);
    let looped = serve(
        &looping,
        Setup {
            limits: "{max_steps: 2, max_result_bytes: 4096, max_seconds: 30, \
                     per_identity_per_minute: 10}",
            ..Setup::default()
        },
    );
    assert_eq!(
        ask(&looped, "alice", "Loop?").terminal().1["reason"],
        "step-limit"
    );
    for service in [&service, &looped] {
        let pids = service.reader_pids();
        assert_eq!(pids.len(), 1, "{pids:?}");
        assert!(!alive(pids[0]), "reader {} outlived its answer", pids[0]);
    }
}

/// The reader inherits neither the gateway's token nor `ANTHROPIC_API_KEY`.
#[test]
fn the_reader_inherits_neither_the_gateway_token_nor_the_api_key() {
    let gateway = Gateway::start(vec![text_reply("Yes.")], Duration::ZERO);
    let service = serve(
        &gateway,
        Setup {
            env: vec![("ANTHROPIC_API_KEY", "sk-ant-adversary-key-91c2")],
            ..Setup::default()
        },
    );
    assert_eq!(ask(&service, "alice", "Env?").terminal().0, "done");
    let env = std::fs::read_to_string(service.root.join("ekr.env")).unwrap();
    assert!(!env.contains(TOKEN), "the gateway token reached the reader");
    assert!(
        !env.contains("sk-ant-adversary-key-91c2"),
        "the API key reached the reader"
    );
    assert!(!env.contains(TOKEN_ENV));
}

/// The reader reads a store; it needs no model credential of the operator's: no ANTHROPIC_* or
/// CLAUDE_* variable of the service's environment, nor the gateway's, reaches it.
#[test]
fn the_reader_inherits_no_other_model_credential() {
    let gateway = Gateway::start(vec![text_reply("Yes.")], Duration::ZERO);
    let service = serve(
        &gateway,
        Setup {
            env: vec![
                ("ANTHROPIC_AUTH_TOKEN", "adversary-auth-token-5d1e"),
                ("CLAUDE_CODE_OAUTH_TOKEN", "adversary-oauth-token-0b7a"),
            ],
            ..Setup::default()
        },
    );
    assert_eq!(ask(&service, "alice", "Env?").terminal().0, "done");
    let env = std::fs::read_to_string(service.root.join("ekr.env")).unwrap();
    assert!(
        !env.contains("adversary-auth-token-5d1e"),
        "ANTHROPIC_AUTH_TOKEN reached the reader"
    );
    assert!(
        !env.contains("adversary-oauth-token-0b7a"),
        "CLAUDE_CODE_OAUTH_TOKEN reached the reader"
    );
}

// ---------------------------------------------------------------------------------------------
// The stream

/// A request whose client goes away after the first event stops the tool loop and its reader: the
/// model is asked no more, and the reader is gone, long before the step limit.
#[test]
fn a_closed_request_stops_the_loop_and_its_reader() {
    let gateway = Gateway::start(vec![tool_reply(1, SEARCH)], Duration::from_millis(400));
    let service = serve(
        &gateway,
        Setup {
            limits: "{max_steps: 50, max_result_bytes: 4096, max_seconds: 60, \
                     per_identity_per_minute: 10}",
            ..Setup::default()
        },
    );
    let body = json!({"question": "Leave?"}).to_string();
    let mut stream = TcpStream::connect(&service.addr).unwrap();
    write!(
        stream,
        "POST /ask HTTP/1.1\r\nhost: x\r\n{HEADER}: dana\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let mut seen = Vec::new();
    let mut buf = [0; 4096];
    while !String::from_utf8_lossy(&seen).contains("event: status") {
        let n = stream.read(&mut buf).unwrap();
        assert!(n > 0, "the stream ended before its first event");
        seen.extend_from_slice(&buf[..n]);
    }
    drop(stream);
    std::thread::sleep(Duration::from_secs(6));
    let asked = gateway.asked();
    std::thread::sleep(Duration::from_secs(2));
    assert_eq!(
        gateway.asked(),
        asked,
        "the model is still asked after the client left"
    );
    assert!(
        asked < 10,
        "the model was asked {asked} times after the client left"
    );
    for pid in service.reader_pids() {
        assert!(!alive(pid), "reader {pid} outlived its closed request");
    }
}

/// The answer records `service` kept, read with the service's own store once it holds `n`
/// (`story:ask-outcome-truthful`).
fn kept(service: &Service, n: usize) -> (Records, Vec<ma::AnswerRecordData>) {
    let file = service.root.join("home/instances/asker/ask.sqlite");
    let started = Instant::now();
    loop {
        if file.exists() {
            let records = Records::open(&file).expect("ask.sqlite opens");
            let answers = records.answers().unwrap();
            if answers.len() >= n || started.elapsed() > Duration::from_secs(20) {
                assert_eq!(answers.len(), n, "{answers:?}");
                return (records, answers);
            }
        }
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "no ask.sqlite after 20 s"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// A reader who closes the stream mid-answer is recorded as `client-closed`, never as the
/// gateway failing (`model-failed`), with the tokens and steps of the model calls already made,
/// and opens no review item (`story:ask-outcome-truthful`, R1-01).
#[test]
fn a_reader_who_leaves_mid_answer_is_recorded_as_client_closed_with_its_tokens() {
    let gateway = Gateway::start(vec![tool_reply(1, SEARCH)], Duration::from_millis(300));
    let service = serve(
        &gateway,
        Setup {
            limits: "{max_steps: 50, max_result_bytes: 4096, max_seconds: 60, \
                     per_identity_per_minute: 10}",
            ..Setup::default()
        },
    );
    let body = json!({"question": "Leave?"}).to_string();
    let mut stream = TcpStream::connect(&service.addr).unwrap();
    write!(
        stream,
        "POST /ask HTTP/1.1\r\nhost: x\r\n{HEADER}: dana\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let mut seen = Vec::new();
    let mut buf = [0; 4096];
    // One whole step: its model call, then its `tool` event.
    while !String::from_utf8_lossy(&seen).contains("event: tool") {
        let n = stream.read(&mut buf).unwrap();
        assert!(n > 0, "the stream ended before its first tool event");
        seen.extend_from_slice(&buf[..n]);
    }
    drop(stream);
    let (store, answers) = kept(&service, 1);
    let record = &answers[0];
    assert_eq!(
        records::outcome_name(record.outcome),
        "client-closed",
        "{record:?}"
    );
    assert_eq!(record.reason.as_deref(), Some("client-closed"));
    assert_eq!(record.answer, None);
    assert!(record.steps >= 1, "{record:?}");
    assert_eq!(record.input_tokens, 10 * record.steps, "{record:?}");
    assert_eq!(record.output_tokens, record.steps, "{record:?}");
    assert_eq!(record.context_tokens, 10, "{record:?}");
    let reviews = store.reviews(&record.answer_id.0).unwrap();
    assert!(
        reviews.is_empty(),
        "a reader leaving is no review: {reviews:?}"
    );
}

/// A model reply cut off by its token limit (`finish_reason: "length"`) or by a content filter
/// (`"content_filter"`) is no answer: the stream ends with `error` `cut-off`, the record's outcome
/// is `cut-off` with the tokens spent, and each opens a review item (`story:ask-outcome-truthful`,
/// R1-02).
#[test]
fn a_cut_off_model_reply_is_recorded_as_cut_off_and_opens_a_review() {
    let finished = |text: &str, finish_reason: &str| {
        let mut reply = text_reply(text);
        reply["choices"][0]["finish_reason"] = json!(finish_reason);
        reply
    };
    let gateway = Gateway::start(
        vec![
            finished("The widget is made by", "length"),
            finished("", "content_filter"),
        ],
        Duration::ZERO,
    );
    let service = serve(&gateway, Setup::default());
    for question in ["Who makes it?", "And now?"] {
        let answer = ask(&service, "alice", question);
        assert_eq!(answer.status, 200);
        let (name, error) = answer.terminal();
        assert_eq!(name, "error", "{:?}", answer.events);
        assert_eq!(error["reason"], "cut-off", "{error}");
        assert!(
            !answer.names().contains(&"delta"),
            "a cut-off reply is not streamed as the answer: {:?}",
            answer.events
        );
    }
    let (store, answers) = kept(&service, 2);
    for record in &answers {
        assert_eq!(
            records::outcome_name(record.outcome),
            "cut-off",
            "{record:?}"
        );
        assert_eq!(record.reason.as_deref(), Some("cut-off"));
        assert_eq!(record.answer, None);
        assert_eq!(
            (record.input_tokens, record.output_tokens, record.steps),
            (10, 1, 1)
        );
        let causes: Vec<_> = store
            .reviews(&record.answer_id.0)
            .unwrap()
            .into_iter()
            .map(|r| records::cause_name(r.data.cause))
            .collect();
        assert_eq!(causes, ["failed"], "{record:?}");
    }
}

/// A cut inside a multi-byte character drops it whole: the model is shown valid UTF-8 of at most
/// `max_result_bytes`, and `result_bytes` is what it is shown.
#[test]
fn a_cut_inside_a_multibyte_character_keeps_whole_characters() {
    let gateway = Gateway::start(
        vec![tool_reply(1, SEARCH), text_reply("Done [node:n-1].")],
        Duration::ZERO,
    );
    let result = "é".repeat(100);
    let service = serve(
        &gateway,
        Setup {
            limits: "{max_steps: 4, max_result_bytes: 63, max_seconds: 30, \
                     per_identity_per_minute: 10}",
            tool_result: &result,
            ..Setup::default()
        },
    );
    let answer = ask(&service, "alice", "Accents?");
    assert_eq!(answer.terminal().0, "done");
    let (_, tool) = answer.events.iter().find(|(n, _)| n == "tool").unwrap();
    assert_eq!(tool["truncated"], true);
    assert_eq!(tool["result_bytes"], 62);
    let requests = gateway.requests.lock().unwrap().clone();
    let content = requests[1]["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["role"] == "tool")
        .unwrap()["content"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(content, "é".repeat(31));
}

/// The text a tool step showed is withdrawn by its `discard` before the step's `tool` event, and
/// the deltas after the last `discard` are exactly `done.answer`.
#[test]
fn the_thinking_text_is_discarded_before_its_tool_event() {
    let gateway = Gateway::start(
        vec![
            tool_reply(1, SEARCH),
            text_reply("Line one\nline two [node:n-1]."),
        ],
        Duration::ZERO,
    );
    let service = serve(&gateway, Setup::default());
    let answer = ask(&service, "alice", "Think?");
    let names = answer.names();
    let discard = names
        .iter()
        .position(|n| *n == "discard")
        .expect("a discard");
    let tool = names.iter().position(|n| *n == "tool").expect("a tool");
    assert!(discard < tool, "{names:?}");
    assert_eq!(names[discard - 1], "delta", "{names:?}");
    assert_eq!(answer.events[discard - 1].1["text"], "Let me look that up.");
    let streamed: String = answer.events[discard + 1..]
        .iter()
        .filter(|(n, _)| n == "delta")
        .map(|(_, v)| v["text"].as_str().unwrap().to_string())
        .collect();
    let (_, done) = answer.terminal();
    assert_eq!(streamed, done["answer"].as_str().unwrap());
}

// ---------------------------------------------------------------------------------------------
// The HTTP surface

/// Concurrent questions of one identity admit exactly `per_identity_per_minute`; each admitted one
/// ends with one `done`, each refused one is a 429 stream of one `error`.
#[test]
fn concurrent_questions_of_one_identity_admit_exactly_the_limit() {
    let gateway = Gateway::start(
        vec![text_reply("Yes [node:n-1].")],
        Duration::from_millis(300),
    );
    let service = Arc::new(serve(
        &gateway,
        Setup {
            limits: "{max_steps: 4, max_result_bytes: 4096, max_seconds: 30, \
                     per_identity_per_minute: 2}",
            ..Setup::default()
        },
    ));
    let handles: Vec<_> = (0..6)
        .map(|_| {
            let service = service.clone();
            std::thread::spawn(move || {
                let answer = ask(&service, "erin", "Together?");
                let terminal = answer.terminal().clone();
                (answer.status, terminal)
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let ok = results.iter().filter(|(s, _)| *s == 200).count();
    let limited = results.iter().filter(|(s, _)| *s == 429).count();
    assert_eq!((ok, limited), (2, 4), "{results:?}");
    for (status, (name, value)) in &results {
        match status {
            200 => assert_eq!(name, "done"),
            _ => assert_eq!(value["reason"], "rate-limit"),
        }
    }
}

/// A body that is not JSON is no question: it answers 400 and does not spend a question of the
/// identity's minute.
#[test]
fn a_malformed_body_does_not_spend_a_question_of_the_minute() {
    let gateway = Gateway::start(vec![text_reply("Yes [node:n-1].")], Duration::ZERO);
    let service = serve(
        &gateway,
        Setup {
            limits: "{max_steps: 4, max_result_bytes: 4096, max_seconds: 30, \
                     per_identity_per_minute: 1}",
            ..Setup::default()
        },
    );
    let bad = post(
        &service,
        Some("frank"),
        b"{not json".to_vec(),
        Duration::from_secs(30),
    )
    .unwrap();
    assert_eq!(bad.status, 400);
    assert_eq!(bad.terminal().1["reason"], "bad-request");
    let good = ask(&service, "frank", "Now?");
    assert_eq!(good.status, 200, "{:?}", good.events);
    assert_eq!(good.terminal().0, "done");
}

/// Every refusal is in the event contract: a body above the server's body limit answers a
/// `text/event-stream` holding one `error` event, like every other refusal before an answer.
#[test]
fn a_body_above_the_body_limit_is_refused_inside_the_event_contract() {
    let gateway = Gateway::start(vec![text_reply("Yes [node:n-1].")], Duration::ZERO);
    let service = serve(&gateway, Setup::default());
    let big = json!({"question": "q".repeat(3 * 1024 * 1024)}).to_string();
    let answer = post(
        &service,
        Some("gina"),
        big.into_bytes(),
        Duration::from_secs(60),
    )
    .unwrap();
    assert!(
        answer.content_type.starts_with("text/event-stream"),
        "status {} with content type {:?}",
        answer.status,
        answer.content_type
    );
    let (name, _) = answer.terminal();
    assert_eq!(
        name,
        if answer.status == 200 {
            "done"
        } else {
            "error"
        }
    );
}

// ---------------------------------------------------------------------------------------------
// Bounds of the service (`story:ask-service-bounds`)

/// With `ask.max_in_flight: 2`, a third question while two answers are in flight is refused at
/// once with 429 `busy`, asks no model and spends no question of the minute; once an answer
/// ends, a question is admitted again (R1-03).
#[test]
fn a_question_past_max_in_flight_is_refused_busy_at_once() {
    let gateway = Gateway::start(vec![text_reply("Yes [node:n-1].")], Duration::from_secs(2));
    let service = Arc::new(serve(
        &gateway,
        Setup {
            limits: "{max_steps: 4, max_result_bytes: 4096, max_seconds: 30, \
                     per_identity_per_minute: 1}",
            ask_extra: "  max_in_flight: 2",
            ..Setup::default()
        },
    ));
    let running: Vec<_> = ["ann", "ben"]
        .into_iter()
        .map(|identity| {
            let service = service.clone();
            std::thread::spawn(move || {
                let answer = ask(&service, identity, "Busy?");
                (answer.status, answer.terminal().clone())
            })
        })
        .collect();
    let started = Instant::now();
    while gateway.asked() < 2 {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "two answers did not start"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let asked = Instant::now();
    let third = ask(&service, "cid", "Me too?");
    assert!(
        asked.elapsed() < Duration::from_millis(1500),
        "the refusal waited {:?}",
        asked.elapsed()
    );
    assert_eq!(third.status, 429, "{:?}", third.events);
    assert_eq!(third.terminal().1["reason"], "busy", "{:?}", third.events);
    assert_eq!(gateway.asked(), 2, "a busy question asked the model");
    for handle in running {
        let (status, (name, _)) = handle.join().unwrap();
        assert_eq!((status, name.as_str()), (200, "done"));
    }
    // The busy refusal spent no question of `cid`'s minute (its limit is 1).
    let again = ask(&service, "cid", "Now?");
    assert_eq!(again.status, 200, "{:?}", again.events);
    assert_eq!(again.terminal().0, "done");
}

/// A chat completion that asks for `n` tool calls at once.
fn many_tool_reply(n: usize) -> Value {
    let calls: Vec<Value> = (1..=n)
        .map(|i| {
            json!({
                "id": format!("call-{i}"),
                "type": "function",
                "function": {"name": "search", "arguments": format!("{{\"query\":\"q{i}\"}}")}
            })
        })
        .collect();
    json!({
        "choices": [{"index": 0, "finish_reason": "tool_calls",
            "message": {"role": "assistant", "content": null, "tool_calls": calls}}],
        "usage": {"prompt_tokens": 10, "completion_tokens": 1}
    })
}

/// A step asking for more tool calls than `limits.max_tool_calls` makes only that many: the
/// reader is called that often, the stream says how many were not made, and the model is told
/// each one it asked for past the cap was not made (R1-06).
#[test]
fn a_step_past_max_tool_calls_is_cut_to_the_cap_and_says_so() {
    let gateway = Gateway::start(
        vec![many_tool_reply(5), text_reply("Done [node:n-1].")],
        Duration::ZERO,
    );
    let service = serve(
        &gateway,
        Setup {
            limits: "{max_steps: 4, max_result_bytes: 4096, max_seconds: 30, \
                     per_identity_per_minute: 10, max_tool_calls: 2}",
            ..Setup::default()
        },
    );
    let answer = ask(&service, "alice", "Many?");
    assert_eq!(answer.terminal().0, "done", "{:?}", answer.events);
    let tools = answer.events.iter().filter(|(n, _)| n == "tool").count();
    assert_eq!(tools, 2, "{:?}", answer.events);
    let calls = std::fs::read_to_string(service.root.join("mcp-calls.log")).unwrap_or_default();
    assert_eq!(calls.lines().count(), 2, "{calls}");
    let said = answer.events.iter().any(|(n, v)| {
        n == "status"
            && v["step"] == 1
            && v["message"]
                .as_str()
                .is_some_and(|m| m.contains("max_tool_calls") && m.contains('3'))
    });
    assert!(said, "no status says calls were cut: {:?}", answer.events);
    let requests = gateway.requests.lock().unwrap().clone();
    let results: Vec<String> = requests[1]["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| m["role"] == "tool")
        .map(|m| m["content"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        results.len(),
        5,
        "every call the model asked for is answered"
    );
    assert_eq!(
        results
            .iter()
            .filter(|r| r.contains("not made") && r.contains("max_tool_calls"))
            .count(),
        3,
        "{results:?}"
    );
}

/// A gateway answering a body above 4 MiB fails the answer with `model-failed`; the body is not
/// read past the cap (R1-06).
#[test]
fn a_gateway_body_above_four_mib_fails_the_answer() {
    let gateway = Gateway::start(
        vec![text_reply(&"x".repeat(5 * 1024 * 1024))],
        Duration::ZERO,
    );
    let service = serve(&gateway, Setup::default());
    let answer = ask(&service, "alice", "Huge?");
    let (name, error) = answer.terminal();
    assert_eq!(name, "error", "{}", answer.names().len());
    assert_eq!(error["reason"], "model-failed", "{error}");
}
