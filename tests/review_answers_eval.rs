//! `story:review-answers-eval`: findings 19 to 24 of the external review of cortex 0.7.2. An
//! answer, its citations and its evaluation report what happened: a refusal stays a refusal, a
//! citation names only what the reader returned, and spent usage is kept.
//!
//! The model is a stand-in gateway in this process (loopback only) that answers a scripted
//! sequence of Chat Completions and counts its requests; the MCP reader is a stand-in `ekr`
//! (`/bin/sh`) whose one tool answers the text `node n-1 is the Atlas project.`, as the
//! reviewer's reader did. No model is called and nothing leaves the machine. Finding 23 (the
//! judge's prompt) is held by the unit test in `src/quality.rs`, where the prompt is built.

mod common;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use common::executable;
use serde_json::{json, Value};

const TOKEN_ENV: &str = "REVIEW_ANSWERS_GATEWAY_TOKEN";
const HEADER: &str = "x-asker";
const NAME: &str = "answer-check";
const WAIT: Duration = Duration::from_secs(30);

/// A stand-in gateway: the n-th request is answered with the n-th scripted reply (the last again
/// once the script is spent).
struct Gateway {
    url: String,
    calls: Arc<AtomicUsize>,
}

impl Gateway {
    fn start(script: Vec<Value>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = calls.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { return };
                read_request(&stream);
                let n = counted.fetch_add(1, Ordering::SeqCst);
                let reply = script[n.min(script.len() - 1)].to_string();
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}",
                    reply.len()
                );
            }
        });
        Self { url, calls }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

fn read_request(stream: &TcpStream) {
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
}

fn usage(input: i64, output: i64) -> Value {
    json!({"prompt_tokens": input, "completion_tokens": output,
           "prompt_tokens_details": {"cached_tokens": 0}})
}

/// A completion asking for the tool `search`.
fn tool_reply(input: i64, output: i64) -> Value {
    json!({
        "object": "chat.completion",
        "choices": [{"index": 0, "finish_reason": "tool_calls", "message": {
            "role": "assistant", "content": null,
            "tool_calls": [{"id": "call-1", "type": "function",
                "function": {"name": "search", "arguments": "{}"}}]
        }}],
        "usage": usage(input, output),
    })
}

/// A completion answering `text`.
fn text_reply(text: &str, input: i64, output: i64) -> Value {
    json!({
        "object": "chat.completion",
        "choices": [{"index": 0, "finish_reason": "stop",
            "message": {"role": "assistant", "content": text}}],
        "usage": usage(input, output),
    })
}

/// A Chat Completions refusal: no content, a non-empty `refusal`.
fn refusal_reply(text: &str, input: i64, output: i64) -> Value {
    json!({
        "object": "chat.completion",
        "choices": [{"index": 0, "finish_reason": "stop",
            "message": {"role": "assistant", "content": null, "refusal": text}}],
        "usage": usage(input, output),
    })
}

/// A temporary root with the stand-in `ekr`, the spec of the instance `answer-check` and its home.
struct World {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    spec: PathBuf,
    listen: String,
}

/// A loopback port nobody listens on now.
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

impl World {
    /// A world whose spec names `gateway` and a `max_steps` of `max_steps`, and `ask.listen` on a
    /// fixed loopback port, as an operator configures it.
    fn new(gateway: &Gateway, max_steps: i64) -> Self {
        let tmp = tempfile::Builder::new()
            .prefix("cortex-review-answers")
            .tempdir_in(
                std::env::var_os("CARGO_TARGET_TMPDIR")
                    .map(PathBuf::from)
                    .unwrap_or_else(std::env::temp_dir),
            )
            .unwrap();
        let root = tmp.path().to_path_buf();
        std::fs::create_dir_all(root.join("bin")).unwrap();
        std::fs::write(
            root.join("tool-result.txt"),
            "node n-1 is the Atlas project.",
        )
        .unwrap();
        stand_in_ekr(&root);
        let listen = format!("127.0.0.1:{}", free_port());
        let spec = root.join("answer-check.yaml");
        std::fs::write(
            &spec,
            format!(
                r#"format: cortex.instance/1
name: {NAME}
description: Local fixture.
ekr: {{version: "0.0.34", bin: "{ekr}"}}
seed: {{documents: []}}
model:
  model: test-model
  budget_tokens: 100000
  timeout_s: 30
  gateway: {{base_url: "{url}", token_env: {TOKEN_ENV}}}
sources: []
serve: {{view_port: 18997}}
ask:
  listen: "{listen}"
  identity_header: {HEADER}
  limits: {{max_steps: {max_steps}, max_result_bytes: 8192, max_seconds: 20, per_identity_per_minute: 24}}
  link_templates: {{node: "https://brain.example/n/{{id}}"}}
"#,
                ekr = root.join("bin/ekr").display(),
                url = gateway.url,
            ),
        )
        .unwrap();
        Self {
            _tmp: tmp,
            root,
            spec,
            listen,
        }
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    fn instance_dir(&self) -> PathBuf {
        self.home().join("instances").join(NAME)
    }

    /// `cortex <args>`: its exit code and the JSON of its last line.
    fn cortex(&self, args: &[&str]) -> (i32, Value) {
        let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
            .args(args)
            .env("CORTEX_HOME", self.home())
            .env("CORTEX_CLAUDE", "/bin/false")
            .env("CORTEX_CONNECTORS", "/bin/false")
            .env("CORTEX_SYSTEMCTL", "/bin/false")
            .env("CORTEX_UNIT_DIR", self.root.join("units"))
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        let last = stdout.lines().last().unwrap_or("null");
        (
            out.status.code().unwrap_or(-1),
            serde_json::from_str(last).unwrap_or_else(
                |_| json!({"stdout": stdout, "stderr": String::from_utf8_lossy(&out.stderr)}),
            ),
        )
    }

    /// Registers the instance, so `eval run` knows it.
    fn create(&self) {
        let spec = self.spec.to_str().unwrap();
        let (code, out) = self.cortex(&["create", "--spec", spec, "--no-units", "--no-extract"]);
        assert_eq!(
            (code, out["outcome"].as_str()),
            (0, Some("created")),
            "{out}"
        );
    }

    /// The answer records, oldest first.
    fn records(&self) -> Vec<Row> {
        let db = rusqlite::Connection::open(self.instance_dir().join("ask.sqlite")).unwrap();
        let mut statement = db
            .prepare(
                "SELECT outcome, answer, reason, steps, input_tokens, output_tokens FROM answers \
                 ORDER BY at_ms, rowid",
            )
            .unwrap();
        let rows = statement
            .query_map([], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            })
            .unwrap();
        rows.map(Result::unwrap).collect()
    }
}

/// One answer record: `(outcome, answer, reason, steps, input_tokens, output_tokens)`.
type Row = (String, Option<String>, Option<String>, i64, i64, i64);

/// A stand-in `ekr`: `example`, `seed` and `head` as `cortex create` needs them, and `mcp` with one
/// tool, `search`, answering the text of `<root>/tool-result.txt`.
fn stand_in_ekr(root: &Path) {
    executable(
        &root.join("bin/ekr"),
        &format!(
            r#"R="{root}"
case "$1" in
  example) echo '{{}}' ;;
  seed) [ -n "$EKR_STORE" ] && : > "$EKR_STORE"; echo '{{"committed_at":1}}' ;;
  head) echo '{{"revision":1}}' ;;
  mcp)
    while IFS= read -r line; do
      id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9][0-9]*\).*/\1/p')
      case "$line" in
        *'"method":"initialize"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"protocolVersion":"2025-06-18","capabilities":{{"tools":{{}}}},"serverInfo":{{"name":"ekr","version":"0"}}}}}}\n' "$id" ;;
        *'"method":"tools/list"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"tools":[{{"name":"search","description":"Locate a project.","inputSchema":{{"type":"object","properties":{{}}}}}}]}}}}\n' "$id" ;;
        *'"method":"tools/call"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"content":[{{"type":"text","text":"%s"}}]}}}}\n' "$id" "$(cat "$R/tool-result.txt")" ;;
      esac
    done ;;
  *) echo "unexpected stand-in verb: $1" >&2; exit 3 ;;
esac
"#,
            root = root.display()
        ),
    );
}

/// A running `cortex ask serve` over the world's spec.
struct Service {
    child: Child,
    addr: String,
}

impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn serve(world: &World) -> Service {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["ask", "serve", "--spec"])
        .arg(&world.spec)
        .env("CORTEX_HOME", world.home())
        .env(TOKEN_ENV, "stand-in-token")
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let _ = BufReader::new(stdout).read_line(&mut line);
        let _ = tx.send(line);
    });
    let line = rx.recv_timeout(WAIT).unwrap_or_default();
    let ready: Value = serde_json::from_str(line.trim()).unwrap_or_else(|e| {
        let _ = child.kill();
        panic!("`cortex ask serve` printed no ready line ({e}): {line:?}")
    });
    let addr = ready["listening"].as_str().expect("listening").to_string();
    assert_eq!(
        addr, world.listen,
        "the service listens where its spec says"
    );
    Service { child, addr }
}

impl Service {
    /// Asks `question`; every event of the stream as `(name, value)`, each decoded with the
    /// generated wire type of `cortex.ask.AskEvent`.
    fn ask(&self, question: &str) -> Vec<(String, Value)> {
        let text = reqwest::blocking::Client::builder()
            .timeout(WAIT)
            .build()
            .unwrap()
            .post(format!("http://{}/ask", self.addr))
            .header(HEADER, "alice")
            .body(json!({"question": question}).to_string())
            .send()
            .unwrap()
            .text()
            .unwrap();
        text.split("\n\n")
            .filter(|b| !b.trim().is_empty())
            .map(|block| {
                let data: String = block
                    .lines()
                    .filter_map(|l| l.strip_prefix("data:"))
                    .map(|d| d.strip_prefix(' ').unwrap_or(d))
                    .collect();
                let decoded: cortex_spec_types::CortexAskAskEvent = serde_json::from_str(&data)
                    .unwrap_or_else(|e| panic!("not a cortex.ask.AskEvent ({e}): {data}"));
                let wire = serde_json::to_value(decoded).unwrap();
                (
                    wire["event"].as_str().unwrap().to_string(),
                    wire["value"].clone(),
                )
            })
            .collect()
    }
}

fn names(events: &[(String, Value)]) -> Vec<&str> {
    events.iter().map(|(n, _)| n.as_str()).collect()
}

/// Finding 19: with `cortex ask serve` running at the address its spec's `ask.listen` names,
/// `eval run --runner ask` asks it, scores its `done` events, and records a refused case as a
/// non-answer.
#[test]
fn eval_run_with_the_ask_runner_asks_the_instances_running_service() {
    let gateway = Gateway::start(vec![
        tool_reply(11, 2),
        text_reply("Atlas [node:n-1].", 11, 2),
        refusal_reply("I cannot help with that.", 11, 2),
    ]);
    let world = World::new(&gateway, 4);
    world.create();
    let _service = serve(&world);
    let set = world.root.join("cases.json");
    std::fs::write(
        &set,
        json!({"cases": [
            {"format": "cortex.eval-case/1", "id": "m1", "question": "Project identifier",
             "expect_facts": ["Atlas"], "expect_nodes": ["n-1"]},
            {"format": "cortex.eval-case/1", "id": "m2", "question": "Something refused",
             "expect_facts": ["cannot"], "expect_nodes": []},
        ]})
        .to_string(),
    )
    .unwrap();
    let before = gateway.calls();
    let (code, out) = world.cortex(&[
        "eval",
        "run",
        NAME,
        "--set",
        set.to_str().unwrap(),
        "--runner",
        "ask",
    ]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("evaluated")),
        "{out}"
    );
    assert_eq!(gateway.calls() - before, 3, "the eval asked the service");
    let detail = &out["detail"];
    assert_eq!(detail["cases"], 2, "{out}");
    assert_eq!(detail["passed"], 1, "{out}");
    // The `done` event's steps; an `error` event carries none (the record keeps them).
    assert_eq!(detail["model_calls"], 2, "{out}");
    assert_eq!(detail["input_tokens"], 22, "{out}");
    assert!(detail["cost_usd"].is_null(), "{out}");
    let dir = PathBuf::from(detail["dir"].as_str().unwrap());
    let results: Vec<Value> = std::fs::read_to_string(dir.join("results.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(results[0]["passed"], true, "{}", results[0]);
    assert_eq!(results[0]["cited_nodes"], json!(["n-1"]));
    assert!(results[0].get("outcome").is_none(), "{}", results[0]);
    // The refusal text holds the expected word, and is still no answer.
    assert_eq!(results[1]["passed"], false, "{}", results[1]);
    assert_eq!(results[1]["outcome"], "refused", "{}", results[1]);
    assert_eq!(results[1]["missing_facts"], json!(["cannot"]));
}

/// Finding 20: a Chat Completions refusal (content absent, `refusal` non-empty) ends the answer
/// with an `error` event whose reason is `refused`, streams no text, and is recorded as `refused`
/// with its text and the usage of its model call; it is never `answered`.
#[test]
fn a_chat_completions_refusal_is_a_refused_outcome_kept_with_its_text() {
    let gateway = Gateway::start(vec![refusal_reply("I can't help with that.", 11, 2)]);
    let world = World::new(&gateway, 4);
    let service = serve(&world);
    let events = service.ask("Native refusal");
    let (name, last) = events.last().unwrap();
    assert_eq!(name, "error", "{:?}", names(&events));
    assert_eq!(last["reason"], "refused", "{last}");
    assert!(
        last["message"]
            .as_str()
            .unwrap()
            .contains("I can't help with that."),
        "{last}"
    );
    assert!(!names(&events).contains(&"delta"), "{:?}", names(&events));
    assert!(!names(&events).contains(&"done"), "{:?}", names(&events));
    assert_eq!(
        world.records(),
        vec![(
            "refused".to_string(),
            Some("I can't help with that.".to_string()),
            Some("refused".to_string()),
            1,
            11,
            2
        )]
    );
}

/// Finding 21: the reader returned only `n-1`. A marker naming `n-2` is an unsupported
/// reference: not linked, not in `done.nodes`, listed in `done.unsupported`; `n-1` is linked.
#[test]
fn a_citation_of_a_node_the_reader_never_returned_is_marked_and_not_linked() {
    let gateway = Gateway::start(vec![
        tool_reply(11, 2),
        text_reply("Atlas [node:n-2], see [node:n-1].", 11, 2),
    ]);
    let world = World::new(&gateway, 4);
    let service = serve(&world);
    let events = service.ask("Other node citation");
    let (name, done) = events.last().unwrap();
    assert_eq!(name, "done", "{:?}", names(&events));
    assert_eq!(done["nodes"], json!(["n-1"]), "{done}");
    assert_eq!(done["unsupported"], json!(["node:n-2"]), "{done}");
    let answer = done["answer"].as_str().unwrap();
    assert!(!answer.contains("brain.example/n/n-2"), "{answer}");
    assert!(answer.contains("[unsupported node:n-2]"), "{answer}");
    assert!(
        answer.contains("[n-1](https://brain.example/n/n-1)"),
        "{answer}"
    );
    // A model that names a node before any tool returned anything cites nothing verified.
    let gateway = Gateway::start(vec![text_reply("Atlas [node:n-1].", 11, 2)]);
    let world = World::new(&gateway, 4);
    let service = serve(&world);
    let events = service.ask("No tool call");
    let (_, done) = events.last().unwrap();
    assert_eq!(done["nodes"], json!([]), "{done}");
    assert_eq!(done["unsupported"], json!(["node:n-1"]), "{done}");
}

/// Finding 22: a 0.7.2 `results.jsonl` holds a refused case (its reason in `answer`, holding the
/// expected word) and an ordinary failed answer. A replay keeps both failed, and the refused one
/// keeps its non-answer outcome, in this replay and in a replay of it.
#[test]
fn replay_keeps_a_refused_case_failed() {
    let gateway = Gateway::start(vec![text_reply("unused", 1, 1)]);
    let world = World::new(&gateway, 4);
    world.create();
    let common =
        json!({"format": "cortex.eval-case/1", "expect_facts": ["model"], "expect_nodes": []});
    let case = |id: &str, question: &str| {
        let mut c = common.clone();
        c["id"] = json!(id);
        c["question"] = json!(question);
        c
    };
    let set = world.root.join("replay-set.json");
    std::fs::write(
        &set,
        json!({"cases": [case("refused", "What is the default model?"),
                         case("ordinary", "What is the second default model?")]})
        .to_string(),
    )
    .unwrap();
    let original = "20260102T030405Z";
    let prior = world.instance_dir().join("eval").join(original);
    std::fs::create_dir_all(&prior).unwrap();
    let reason = "refused: the question was refused by policy: the instance's redaction policy \
                  still detects Email in it, and it is not sent to the model.";
    let rows = [
        json!({"format": "cortex.eval-result/1", "id": "refused",
               "question": "What is the default model?", "answer": reason, "cited_nodes": [],
               "passed": false, "missing_facts": [], "missing_nodes": []}),
        json!({"format": "cortex.eval-result/1", "id": "ordinary",
               "question": "What is the second default model?", "answer": "Unknown.",
               "cited_nodes": [], "passed": false, "missing_facts": ["model"],
               "missing_nodes": []}),
    ];
    let lines: String = rows.iter().map(|r| format!("{r}\n")).collect();
    std::fs::write(prior.join("results.jsonl"), lines).unwrap();
    std::fs::write(
        prior.join("summary.json"),
        json!({"format": "cortex.eval-run/1", "stamp": original, "instance_name": NAME,
               "runner": "mcp", "cases": 2, "passed": 0})
        .to_string(),
    )
    .unwrap();
    let replay = |from: &str| {
        let (code, out) = world.cortex(&[
            "eval",
            "run",
            NAME,
            "--set",
            set.to_str().unwrap(),
            "--runner",
            "replay",
            "--from",
            from,
        ]);
        assert_eq!(
            (code, out["outcome"].as_str()),
            (0, Some("evaluated")),
            "{out}"
        );
        let dir = PathBuf::from(out["detail"]["dir"].as_str().unwrap());
        let results: Vec<Value> = std::fs::read_to_string(dir.join("results.jsonl"))
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        (out, dir, results)
    };
    let (out, dir, results) = replay(original);
    assert_eq!(out["detail"]["passed"], 0, "{out}");
    assert_eq!(out["detail"]["model_calls"], 0, "{out}");
    assert_eq!(results[0]["passed"], false, "{}", results[0]);
    assert_eq!(results[0]["outcome"], "refused", "{}", results[0]);
    assert_eq!(results[0]["missing_facts"], json!(["model"]));
    assert_eq!(results[1]["passed"], false, "{}", results[1]);
    assert!(results[1].get("outcome").is_none(), "{}", results[1]);
    // A replay of the replay reads the outcome it wrote.
    let stamp = dir.file_name().unwrap().to_str().unwrap().to_string();
    let (_, _, again) = replay(&stamp);
    assert_eq!(again, results);
    assert_eq!(gateway.calls(), 0, "a replay asks no model");
}

/// Finding 24: an answer ended by the step limit after two model calls keeps their steps and
/// tokens in its record, as does a model failure after one call.
#[test]
fn a_limited_or_failed_answer_keeps_the_usage_of_the_model_calls_it_made() {
    let gateway = Gateway::start(vec![tool_reply(11, 2)]);
    let world = World::new(&gateway, 2);
    let service = serve(&world);
    let events = service.ask("Stop after two steps");
    let (name, last) = events.last().unwrap();
    assert_eq!(
        (name.as_str(), &last["reason"]),
        ("error", &json!("step-limit"))
    );
    assert_eq!(gateway.calls(), 2);
    assert_eq!(
        world.records(),
        vec![(
            "limited".to_string(),
            None,
            Some("step-limit".to_string()),
            2,
            22,
            4
        )]
    );

    // The second model call answers no message: `model-failed`, with the first call's usage.
    let gateway = Gateway::start(vec![
        tool_reply(11, 2),
        json!({"object": "chat.completion"}),
    ]);
    let world = World::new(&gateway, 4);
    let service = serve(&world);
    let events = service.ask("Fail on the second call");
    let (_, last) = events.last().unwrap();
    assert_eq!(last["reason"], "model-failed", "{last}");
    assert_eq!(
        world.records(),
        vec![(
            "failed".to_string(),
            None,
            Some("model-failed".to_string()),
            1,
            11,
            2
        )]
    );
}
