//! `story:ask-service`: `cortex ask serve --spec <instance spec>` answers `POST /ask` with the event
//! stream `cortex.ask-events/1` (`cortex.ask.AskEvent`), from a tool loop over the EKR MCP reader
//! against the instance's model gateway, within the spec's limits.
//!
//! The model is a stand-in gateway in this process that answers a scripted sequence of
//! OpenAI-compatible chat completions and records every request; the MCP reader is a stand-in
//! `ekr` whose `mcp` answers one tool. Every event the service streams is decoded with the
//! generated wire type of `cortex.ask.AskEvent` (`generated/spec-types`), which refuses a missing
//! or unknown field: that is the event contract, held field by field.

mod common;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use common::executable;
use serde_json::{json, Value};

/// The variable the spec's gateway names for its token, and the token the stand-in expects.
const TOKEN_ENV: &str = "ASK_TEST_GATEWAY_TOKEN";
const TOKEN: &str = "stand-in-token";
const HEADER: &str = "x-asker";

/// A stand-in model gateway: answers each `POST /v1/chat/completions` with the next scripted
/// reply (the last one again once the script is spent) and records each request body.
struct Gateway {
    url: String,
    requests: Arc<Mutex<Vec<Value>>>,
    authorizations: Arc<Mutex<Vec<String>>>,
}

impl Gateway {
    fn start(script: Vec<Value>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let authorizations = Arc::new(Mutex::new(Vec::new()));
        let (seen, auth) = (requests.clone(), authorizations.clone());
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { return };
                let (body, authorization) = read_request(&stream);
                let n = {
                    let mut seen = seen.lock().unwrap();
                    seen.push(serde_json::from_slice(&body).unwrap_or(Value::Null));
                    seen.len()
                };
                auth.lock().unwrap().push(authorization);
                let reply = script[(n - 1).min(script.len() - 1)].to_string();
                let mut stream = stream;
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}",
                    reply.len()
                );
            }
        });
        Self {
            url,
            requests,
            authorizations,
        }
    }

    fn requests(&self) -> Vec<Value> {
        self.requests.lock().unwrap().clone()
    }
}

/// One HTTP/1.1 request's body and its `authorization` header.
fn read_request(stream: &TcpStream) -> (Vec<u8>, String) {
    let mut reader = BufReader::new(stream);
    let mut length = 0;
    let mut authorization = String::new();
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
            match name.trim().to_ascii_lowercase().as_str() {
                "content-length" => length = value.trim().parse().unwrap_or(0),
                "authorization" => authorization = value.trim().to_string(),
                _ => {}
            }
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).unwrap();
    (body, authorization)
}

/// A chat completion that asks for the tool `search` with `{"query": …}`.
fn tool_reply(n: usize, usage: (i64, i64, i64)) -> Value {
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
                    "function": {"name": "search", "arguments": "{\"query\":\"widget\"}"}
                }]
            }
        }],
        "usage": {
            "prompt_tokens": usage.0,
            "completion_tokens": usage.1,
            "prompt_tokens_details": {"cached_tokens": usage.2}
        }
    })
}

/// A chat completion that answers `text`.
fn text_reply(text: &str, usage: (i64, i64, i64)) -> Value {
    json!({
        "id": "final",
        "object": "chat.completion",
        "choices": [{
            "index": 0,
            "finish_reason": "stop",
            "message": {"role": "assistant", "content": text}
        }],
        "usage": {
            "prompt_tokens": usage.0,
            "completion_tokens": usage.1,
            "prompt_tokens_details": {"cached_tokens": usage.2}
        }
    })
}

/// A world for one service: a temporary root with the stand-in `ekr` and the spec file.
struct Service {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    child: Child,
    addr: String,
}

impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A stand-in `ekr` whose `mcp` speaks MCP over stdio: one tool, `search`, whose result is the
/// text of `<root>/tool-result.txt`. Each `tools/call` line is appended to `<root>/mcp-calls.log`.
fn stand_in_ekr(root: &Path) -> PathBuf {
    let path = root.join("bin/ekr");
    executable(
        &path,
        &format!(
            r#"R="{root}"
[ "$1" = mcp ] || exit 2
while IFS= read -r line; do
  id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9][0-9]*\).*/\1/p')
  case "$line" in
    *'"method":"initialize"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"protocolVersion":"2025-06-18","capabilities":{{"tools":{{}}}},"serverInfo":{{"name":"ekr","version":"0"}}}}}}\n' "$id" ;;
    *'"method":"tools/list"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"tools":[{{"name":"search","description":"Search the store","inputSchema":{{"type":"object","properties":{{"query":{{"type":"string"}}}}}}}}]}}}}\n' "$id" ;;
    *'"method":"tools/call"'*) printf '%s\n' "$line" >> "$R/mcp-calls.log"; printf '{{"jsonrpc":"2.0","id":%s,"result":{{"content":[{{"type":"text","text":"%s"}}]}}}}\n' "$id" "$(cat "$R/tool-result.txt")" ;;
  esac
done
"#,
            root = root.display()
        ),
    );
    path
}

/// Starts `cortex ask serve` over a spec whose `ask.limits` are `limits` (YAML flow text), with
/// the gateway at `gateway` and the stand-in reader's tool answering `tool_result`.
fn serve(gateway: &Gateway, limits: &str, tool_result: &str) -> Service {
    serve_with(gateway, limits, tool_result, "")
}

/// [`serve`] over a spec that ends with `extra` (top-level YAML, such as a `redaction` policy).
fn serve_with(gateway: &Gateway, limits: &str, tool_result: &str, extra: &str) -> Service {
    let tmp = tempfile::Builder::new()
        .prefix("cortex-ask")
        .tempdir_in(
            std::env::var_os("CARGO_TARGET_TMPDIR")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir),
        )
        .unwrap();
    let root = tmp.path().to_path_buf();
    std::fs::create_dir_all(root.join("bin")).unwrap();
    std::fs::write(root.join("tool-result.txt"), tool_result).unwrap();
    let ekr = stand_in_ekr(&root);
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
  link_templates: {{node: "https://brain.example/node/{{id}}"}}
{extra}"#,
            ekr = ekr.display(),
            url = gateway.url,
        ),
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["ask", "serve", "--spec"])
        .arg(&spec)
        .env("CORTEX_HOME", root.join("home"))
        .env(TOKEN_ENV, TOKEN)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let ready: Value = serde_json::from_str(line.trim())
        .unwrap_or_else(|e| panic!("`cortex ask serve` printed no ready line ({e}): {line:?}"));
    let addr = ready["listening"]
        .as_str()
        .unwrap_or_else(|| panic!("no `listening` in {ready}"))
        .to_string();
    Service {
        _tmp: tmp,
        root,
        child,
        addr,
    }
}

/// What one `POST /ask` answered: the status code, and each event as `(name, value)`.
struct Answer {
    status: u16,
    retry_after: Option<String>,
    events: Vec<(String, Value)>,
}

impl Answer {
    fn names(&self) -> Vec<&str> {
        self.events.iter().map(|(n, _)| n.as_str()).collect()
    }
    fn last(&self) -> &(String, Value) {
        self.events.last().expect("at least one event")
    }
}

/// Posts `question` as `identity` (no identity header when `None`) and reads the whole stream.
fn ask(service: &Service, identity: Option<&str>, question: &str) -> Answer {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .unwrap();
    let mut request = client
        .post(format!("http://{}/ask", service.addr))
        .header("content-type", "application/json")
        .body(json!({"question": question}).to_string());
    if let Some(identity) = identity {
        request = request.header(HEADER, identity);
    }
    let response = request.send().unwrap();
    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(
        content_type.starts_with("text/event-stream"),
        "content type {content_type:?}"
    );
    let retry_after = response
        .headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let text = response.text().unwrap();
    Answer {
        status,
        retry_after,
        events: parse_stream(&text),
    }
}

/// Each SSE event of `text`, its `event:` name held to its data's `event` tag and its data
/// decoded with the generated wire type of `cortex.ask.AskEvent`.
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
            serde_json::from_str::<Value>(&data).unwrap(),
            wire,
            "`{name}` carries what the contract does not state"
        );
        assert_eq!(
            wire["event"],
            json!(name),
            "the SSE name and the tag differ"
        );
        events.push((name, wire["value"].clone()));
    }
    events
}

const LIMITS: &str =
    "{max_steps: 4, max_result_bytes: 4096, max_seconds: 30, per_identity_per_minute: 10}";

/// With the model scripted to call one tool and then answer citing one node, the stream is
/// `status`, `tool`, at least one `delta`, then `done`, whose cited node ids are the scripted one
/// and whose token counts are the stand-in's.
#[test]
fn one_tool_call_then_an_answer_streams_status_tool_delta_and_done() {
    let gateway = Gateway::start(vec![
        tool_reply(1, (100, 10, 40)),
        text_reply(
            "Example Labs develops the Widget engine [node:n-42].",
            (200, 20, 60),
        ),
    ]);
    let service = serve(&gateway, LIMITS, "Widget engine, made by Example Labs");
    let answer = ask(&service, Some("alice"), "Who makes the Widget engine?");
    assert_eq!(answer.status, 200, "{:?}", answer.events);
    let names = answer.names();
    assert_eq!(names.first(), Some(&"status"), "{names:?}");
    let tool = names
        .iter()
        .position(|n| *n == "tool")
        .expect("a tool event");
    let delta = names
        .iter()
        .rposition(|n| *n == "delta")
        .expect("a delta event");
    assert!(tool < delta, "{names:?}");
    assert_eq!(names.last(), Some(&"done"), "{names:?}");
    assert!(!names.contains(&"error"), "{names:?}");
    let status = &answer.events[0].1;
    assert_eq!(status["format"], "cortex.ask-events/1");
    let (_, tool) = &answer.events[tool];
    assert_eq!(tool["name"], "search");
    assert_eq!(tool["truncated"], false);
    let (_, done) = answer.last();
    assert_eq!(done["nodes"], json!(["n-42"]));
    assert_eq!(done["evidence"], json!([]));
    assert_eq!(done["input_tokens"], 300);
    assert_eq!(done["output_tokens"], 30);
    assert_eq!(done["cached_tokens"], 100);
    assert_eq!(done["context_tokens"], 200);
    assert_eq!(done["steps"], 2);
    assert_eq!(
        done["answer"],
        "Example Labs develops the Widget engine [n-42](https://brain.example/node/n-42)."
    );
    // The answer the deltas carry since the last discard is the answer `done` carries.
    let after_discard = answer
        .names()
        .iter()
        .rposition(|n| *n == "discard")
        .map_or(0, |i| i + 1);
    let streamed: String = answer.events[after_discard..]
        .iter()
        .filter(|(n, _)| n == "delta")
        .map(|(_, v)| v["text"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(streamed, done["answer"].as_str().unwrap());
    // The reader's tool was called with the model's arguments, and its result reached the model.
    let calls = std::fs::read_to_string(service.root.join("mcp-calls.log")).unwrap();
    assert_eq!(calls.lines().count(), 1, "{calls}");
    assert!(calls.contains("\"name\":\"search\""), "{calls}");
    let requests = gateway.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0]["model"], "test-model");
    assert_eq!(requests[0]["tools"][0]["function"]["name"], "search");
    let tool_message = requests[1]["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["role"] == "tool")
        .expect("a tool message");
    assert_eq!(tool_message["tool_call_id"], "call-1");
    assert_eq!(
        tool_message["content"],
        "Widget engine, made by Example Labs"
    );
    assert!(gateway
        .authorizations
        .lock()
        .unwrap()
        .iter()
        .all(|a| a == &format!("Bearer {TOKEN}")));
}

/// A scripted loop longer than `max_steps` ends with an `error` naming the step limit, and no
/// `done`; the model is asked `max_steps` times, no more.
#[test]
fn a_loop_longer_than_max_steps_ends_with_an_error_naming_the_step_limit() {
    let gateway = Gateway::start(vec![
        tool_reply(1, (10, 1, 0)),
        tool_reply(2, (10, 1, 0)),
        tool_reply(3, (10, 1, 0)),
        text_reply("Too late [node:n-1].", (10, 1, 0)),
    ]);
    let limits =
        "{max_steps: 2, max_result_bytes: 4096, max_seconds: 30, per_identity_per_minute: 10}";
    let service = serve(&gateway, limits, "a result");
    let answer = ask(&service, Some("alice"), "Loop forever?");
    let names = answer.names();
    assert!(!names.contains(&"done"), "{names:?}");
    let (name, error) = answer.last();
    assert_eq!(name, "error", "{names:?}");
    assert_eq!(error["reason"], "step-limit");
    assert!(error.get("retry_after").is_none(), "{error}");
    assert_eq!(gateway.requests().len(), 2);
}

/// A tool result larger than `max_result_bytes` reaches the model cut to that many bytes, and its
/// `tool` event says so.
#[test]
fn a_tool_result_larger_than_max_result_bytes_reaches_the_model_cut_to_it() {
    let gateway = Gateway::start(vec![
        tool_reply(1, (10, 1, 0)),
        text_reply("Done [node:n-1].", (10, 1, 0)),
    ]);
    let limits =
        "{max_steps: 4, max_result_bytes: 64, max_seconds: 30, per_identity_per_minute: 10}";
    let big = "abcdefghij".repeat(500);
    let service = serve(&gateway, limits, &big);
    let answer = ask(&service, Some("alice"), "Big?");
    assert_eq!(answer.last().0, "done", "{:?}", answer.names());
    let (_, tool) = answer
        .events
        .iter()
        .find(|(n, _)| n == "tool")
        .expect("a tool event");
    assert_eq!(tool["truncated"], true);
    assert_eq!(tool["result_bytes"], 64);
    let requests = gateway.requests();
    let tool_message = requests[1]["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["role"] == "tool")
        .expect("a tool message")
        .clone();
    let content = tool_message["content"].as_str().unwrap();
    assert_eq!(content.len(), 64, "{content}");
    assert!(big.starts_with(content));
}

/// The `per_identity_per_minute`+1-th question of one identity within a minute answers 429 with
/// an `error` event carrying `retry_after`, and asks no model; another identity is still answered.
#[test]
fn the_question_past_the_per_identity_limit_answers_429_with_retry_after() {
    let gateway = Gateway::start(vec![text_reply("Yes [node:n-1].", (10, 1, 0))]);
    let limits =
        "{max_steps: 4, max_result_bytes: 4096, max_seconds: 30, per_identity_per_minute: 2}";
    let service = serve(&gateway, limits, "a result");
    for _ in 0..2 {
        let answer = ask(&service, Some("bob"), "Again?");
        assert_eq!(answer.status, 200);
        assert_eq!(answer.last().0, "done", "{:?}", answer.names());
    }
    let asked = gateway.requests().len();
    let answer = ask(&service, Some("bob"), "Again?");
    assert_eq!(answer.status, 429);
    assert_eq!(answer.names(), vec!["error"]);
    let (_, error) = answer.last();
    assert_eq!(error["reason"], "rate-limit");
    let retry = error["retry_after"].as_i64().expect("retry_after");
    assert!((1..=60).contains(&retry), "{error}");
    assert_eq!(answer.retry_after, Some(retry.to_string()));
    assert_eq!(
        gateway.requests().len(),
        asked,
        "a refused question asked the model"
    );
    let other = ask(&service, Some("carol"), "Me?");
    assert_eq!(other.status, 200);
    assert_eq!(other.last().0, "done");
}

/// A request without the identity header the spec names answers 401 and asks no model; neither
/// does an empty one.
#[test]
fn a_request_without_the_identity_header_answers_401_and_asks_no_model() {
    let gateway = Gateway::start(vec![text_reply("Yes [node:n-1].", (10, 1, 0))]);
    let service = serve(&gateway, LIMITS, "a result");
    for identity in [None, Some("")] {
        let answer = ask(&service, identity, "Who am I?");
        assert_eq!(answer.status, 401);
        assert_eq!(answer.names(), vec!["error"]);
        assert_eq!(answer.last().1["reason"], "unidentified");
    }
    assert!(gateway.requests().is_empty(), "the model was asked");
    assert!(!service.root.join("mcp-calls.log").exists());
}

/// A spec without an `ask` section, or with a limit of 0, does not start a service (exit 2,
/// naming the field).
#[test]
fn a_spec_without_ask_or_with_a_zero_limit_does_not_serve() {
    let gateway = Gateway::start(vec![]);
    let service = serve(&gateway, LIMITS, "");
    let spec = service.root.join("asker.yaml");
    let text = std::fs::read_to_string(&spec).unwrap();
    for (name, body, field) in [
        (
            "no-ask.yaml",
            text[..text.find("ask:\n").unwrap()].to_string(),
            "ask",
        ),
        (
            "zero.yaml",
            text.replace("max_steps: 4", "max_steps: 0"),
            "ask.limits.max_steps",
        ),
    ] {
        let path = service.root.join(name);
        std::fs::write(&path, body).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
            .args(["ask", "serve", "--spec"])
            .arg(&path)
            .env("CORTEX_HOME", service.root.join("home"))
            .env(TOKEN_ENV, TOKEN)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2), "{name}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains(field), "{name}: {stderr}");
    }
}

// `story:ask-redaction`: the instance's `redaction` policy holds wherever the model reads its
// store. One question is one redaction batch; the question and each tool result are redacted
// before the model is given them, and the answer is restored before the client sees it.

/// The document the stand-in reader's tool answers: a known name and an email address.
const PERSONAL: &str = "Zorblat leads the widget team, write to help@acme.example.";

/// A policy that hides email addresses and the known name `Zorblat`.
const POLICY: &str = "redaction:\n  classes: [Email]\n  rules: []\n  known_names: [Zorblat]\n";

/// The text of each `delta` after the last `discard`, joined: the answer as the client saw it.
fn streamed(answer: &Answer) -> String {
    let after_discard = answer
        .names()
        .iter()
        .rposition(|n| *n == "discard")
        .map_or(0, |i| i + 1);
    answer.events[after_discard..]
        .iter()
        .filter(|(n, _)| n == "delta")
        .map(|(_, v)| v["text"].as_str().unwrap().to_string())
        .collect()
}

/// The content of the first `tool` message of a request the model received.
fn tool_content(request: &Value) -> String {
    request["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["role"] == "tool")
        .expect("a tool message")["content"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn with_a_policy_the_model_sees_placeholders_and_the_client_the_restored_answer() {
    let gateway = Gateway::start(vec![
        tool_reply(1, (100, 10, 0)),
        text_reply(
            "[Name-1] leads the widget team.\nWrite to [Email-1], not [Email-7].",
            (200, 20, 0),
        ),
    ]);
    let service = serve_with(&gateway, LIMITS, PERSONAL, POLICY);
    let answer = ask(&service, Some("alice"), "What does Zorblat lead?");
    assert_eq!(answer.names().last(), Some(&"done"), "{:?}", answer.events);

    // The model received placeholders, never a value: in the question and in the tool result.
    let requests = gateway.requests();
    assert_eq!(requests.len(), 2, "{requests:?}");
    for request in &requests {
        let sent = request.to_string();
        assert!(!sent.contains("Zorblat"), "{sent}");
        assert!(!sent.contains("help@acme.example"), "{sent}");
    }
    let question = requests[0]["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["role"] == "user")
        .expect("the question")["content"]
        .clone();
    assert_eq!(question, "What does [Name-1] lead?");
    assert_eq!(
        tool_content(&requests[1]),
        "[Name-1] leads the widget team, write to [Email-1]."
    );

    // The client sees the values, in the deltas and in `done`; a placeholder with no value stays
    // as written and is counted.
    let (_, done) = answer.last();
    let restored = "Zorblat leads the widget team.\nWrite to help@acme.example, not [Email-7].";
    assert_eq!(done["answer"], restored, "{done}");
    assert_eq!(streamed(&answer), restored);
    assert_eq!(done["unrestored"], 1, "{done}");
    let tool = answer
        .events
        .iter()
        .find(|(n, _)| n == "tool")
        .expect("a tool event");
    assert_eq!(tool.1["withheld"], false, "{tool:?}");
}

#[test]
fn a_tool_result_still_holding_a_refused_class_is_withheld_and_the_answer_completes() {
    let gateway = Gateway::start(vec![
        tool_reply(1, (100, 10, 0)),
        text_reply("The store would not say.", (200, 20, 0)),
    ]);
    // The rule's replacement is itself an address the `Email` detector finds.
    let policy = "redaction:\n  classes: [Email]\n  rules:\n    - {name: Team, pattern: \"widget team\", replacement: \"ops@acme.example\"}\n  known_names: [Zorblat]\n  refuse_if_left: [Email]\n";
    let service = serve_with(&gateway, LIMITS, PERSONAL, policy);
    // The question avoids the rule's pattern: rewritten to an address, it would be refused itself.
    let answer = ask(&service, Some("alice"), "Who leads which team?");
    let names = answer.names();
    assert_eq!(names.last(), Some(&"done"), "{:?}", answer.events);
    assert!(!names.contains(&"error"), "{names:?}");
    let tool = answer
        .events
        .iter()
        .find(|(n, _)| n == "tool")
        .expect("a tool event");
    assert_eq!(tool.1["withheld"], true, "{tool:?}");
    let requests = gateway.requests();
    assert_eq!(requests.len(), 2, "{requests:?}");
    let content = tool_content(&requests[1]);
    assert!(content.contains("withheld by policy"), "{content}");
    for value in ["ops@acme.example", "help@acme.example", "Zorblat"] {
        assert!(!content.contains(value), "{content}");
    }
    assert_eq!(answer.last().1["answer"], "The store would not say.");
}

#[test]
fn without_a_policy_the_model_receives_the_tool_result_byte_for_byte() {
    let gateway = Gateway::start(vec![
        tool_reply(1, (100, 10, 0)),
        text_reply("[Name-1] is not a placeholder here.", (200, 20, 0)),
    ]);
    let service = serve(&gateway, LIMITS, PERSONAL);
    let answer = ask(&service, Some("alice"), "What does Zorblat lead?");
    let requests = gateway.requests();
    assert_eq!(requests.len(), 2, "{requests:?}");
    assert_eq!(tool_content(&requests[1]), PERSONAL);
    let question = requests[0]["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["role"] == "user")
        .expect("the question")["content"]
        .clone();
    assert_eq!(question, "What does Zorblat lead?");
    let (_, done) = answer.last();
    assert_eq!(done["answer"], "[Name-1] is not a placeholder here.");
    assert_eq!(done["unrestored"], 0, "{done}");
    let tool = answer
        .events
        .iter()
        .find(|(n, _)| n == "tool")
        .expect("a tool event");
    assert_eq!(tool.1["withheld"], false, "{tool:?}");
}

/// `refuse_if_left` names what the model must never be shown; the question reaches the model as
/// well as a tool result does. Today the question is redacted but never checked: with a class the
/// policy refuses and does not replace, the caller's question carries the original to the model.
#[test]
fn a_question_still_holding_a_refused_class_does_not_reach_the_model() {
    let gateway = Gateway::start(vec![text_reply("I cannot say.", (100, 10, 0))]);
    let policy = "redaction:\n  classes: []\n  rules: []\n  refuse_if_left: [Email]\n";
    let service = serve_with(&gateway, LIMITS, PERSONAL, policy);
    let _ = ask(&service, Some("alice"), "Who reads help@acme.example?");
    for request in gateway.requests() {
        let sent = request.to_string();
        assert!(!sent.contains("help@acme.example"), "{sent}");
    }
}
