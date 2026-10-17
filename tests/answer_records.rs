//! `story:answer-records-and-review`: every answer `cortex ask serve` gives is kept in `ask.sqlite`
//! in the instance directory (`cortex.ask.AnswerRecord`), a reader casts one changeable vote per
//! answer (`POST /ask/{id}/vote`, `cortex.ask.CastVote`), an answer voted down, failed or slow
//! opens a review item (`cortex.ask.ReviewItem`) that `GET /review` lists and
//! `POST /review/{id}/close` closes, and records past `retention_days` are purged with their votes
//! and review items when the service starts.
//!
//! The model is a stand-in gateway in this process that answers a scripted sequence of chat
//! completions, each after an optional delay; the MCP reader is a stand-in `ekr`. The records are
//! read back with `cortex_cli::ask::records::Records`, the service's own store.

mod common;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use common::executable;
use cortex_cli::ask::records::Records;
use cortex_model::ask as ma;
use serde_json::{json, Value};

const TOKEN_ENV: &str = "ASK_RECORDS_GATEWAY_TOKEN";
const HEADER: &str = "x-asker";
const LIMITS: &str =
    "{max_steps: 4, max_result_bytes: 4096, max_seconds: 30, per_identity_per_minute: 30}";

/// A stand-in model gateway: the n-th request is answered with the n-th scripted reply (the last
/// again once the script is spent), after that reply's delay.
fn gateway(script: Vec<(u64, Value)>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for (n, stream) in listener.incoming().enumerate() {
            let Ok(mut stream) = stream else { return };
            read_request(&stream);
            let (delay, reply) = script[n.min(script.len() - 1)].clone();
            std::thread::sleep(Duration::from_millis(delay));
            let reply = reply.to_string();
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}",
                reply.len()
            );
        }
    });
    url
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

/// A chat completion that asks for the tool `search`.
fn tool_reply(usage: (i64, i64, i64)) -> Value {
    json!({
        "choices": [{"index": 0, "finish_reason": "tool_calls", "message": {
            "role": "assistant", "content": "Looking.",
            "tool_calls": [{"id": "call-1", "type": "function",
                "function": {"name": "search", "arguments": "{\"query\":\"widget\"}"}}]}}],
        "usage": {"prompt_tokens": usage.0, "completion_tokens": usage.1,
            "prompt_tokens_details": {"cached_tokens": usage.2}}
    })
}

/// A chat completion that answers `text`.
fn text_reply(text: &str, usage: (i64, i64, i64)) -> Value {
    json!({
        "choices": [{"index": 0, "finish_reason": "stop",
            "message": {"role": "assistant", "content": text}}],
        "usage": {"prompt_tokens": usage.0, "completion_tokens": usage.1,
            "prompt_tokens_details": {"cached_tokens": usage.2}}
    })
}

/// A reply with no message: the model failed.
fn broken_reply() -> Value {
    json!({"choices": []})
}

/// A temporary root holding the stand-in `ekr`, the spec and the home.
struct World {
    _tmp: tempfile::TempDir,
    root: PathBuf,
}

impl World {
    fn new() -> Self {
        let tmp = tempfile::Builder::new()
            .prefix("cortex-records")
            .tempdir_in(
                std::env::var_os("CARGO_TARGET_TMPDIR")
                    .map(PathBuf::from)
                    .unwrap_or_else(std::env::temp_dir),
            )
            .unwrap();
        let root = tmp.path().to_path_buf();
        std::fs::create_dir_all(root.join("bin")).unwrap();
        stand_in_ekr(&root);
        Self { _tmp: tmp, root }
    }

    /// The spec of the instance `asker`, its gateway at `url`, with `extra` lines under `ask:` and
    /// `store` as its store section (none when empty).
    fn spec(&self, url: &str, extra: &str, store: &str) -> PathBuf {
        let spec = self.root.join("asker.yaml");
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
serve: {{view_port: 18998}}
{store}
ask:
  listen: "127.0.0.1:0"
  identity_header: {HEADER}
  limits: {LIMITS}
{extra}
"#,
                ekr = self.root.join("bin/ekr").display(),
            ),
        )
        .unwrap();
        spec
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    /// `ask.sqlite` of the instance `asker`.
    fn records_file(&self) -> PathBuf {
        self.home().join("instances/asker/ask.sqlite")
    }

    fn records(&self) -> Records {
        Records::open(&self.records_file()).expect("ask.sqlite opens")
    }
}

/// A stand-in `ekr` whose `mcp` answers one tool, `search`, with a fixed text.
fn stand_in_ekr(root: &Path) {
    executable(
        &root.join("bin/ekr"),
        r#"[ "$1" = mcp ] || exit 2
while IFS= read -r line; do
  id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9][0-9]*\).*/\1/p')
  case "$line" in
    *'"method":"initialize"'*) printf '{"jsonrpc":"2.0","id":%s,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"ekr","version":"0"}}}\n' "$id" ;;
    *'"method":"tools/list"'*) printf '{"jsonrpc":"2.0","id":%s,"result":{"tools":[{"name":"search","description":"Search","inputSchema":{"type":"object"}}]}}\n' "$id" ;;
    *'"method":"tools/call"'*) printf '{"jsonrpc":"2.0","id":%s,"result":{"content":[{"type":"text","text":"Widget engine, made by Example Labs"}]}}\n' "$id" ;;
  esac
done
"#,
    );
}

/// A running `cortex ask serve`, stopped on drop.
struct Service {
    child: Child,
    addr: String,
    client: reqwest::blocking::Client,
}

impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The longest any wait of these tests lasts: a ready line, a request, a process's exit.
const WAIT: Duration = Duration::from_secs(30);

/// Runs `command` to its exit, killing it after [`WAIT`]: its exit code (none when killed) and
/// its stdout and stderr together.
fn run_bounded(command: &mut Command) -> (Option<i32>, String) {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let drain = |mut r: Box<dyn Read + Send>| {
        std::thread::spawn(move || {
            let mut text = String::new();
            let _ = r.read_to_string(&mut text);
            text
        })
    };
    let out = drain(Box::new(child.stdout.take().unwrap()));
    let err = drain(Box::new(child.stderr.take().unwrap()));
    let start = std::time::Instant::now();
    let code = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status.code();
        }
        if start.elapsed() > WAIT {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let text = format!("{}{}", out.join().unwrap(), err.join().unwrap());
    (code, text)
}

fn serve(world: &World, spec: &Path) -> Service {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["ask", "serve", "--spec"])
        .arg(spec)
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
    let Ok(line) = rx.recv_timeout(WAIT) else {
        let _ = child.kill();
        let _ = child.wait();
        panic!("`cortex ask serve` printed no ready line within {WAIT:?}");
    };
    let ready: Value = serde_json::from_str(line.trim()).unwrap_or_else(|e| {
        let _ = child.kill();
        let _ = child.wait();
        panic!("`cortex ask serve` printed no ready line ({e}): {line:?}")
    });
    Service {
        child,
        addr: ready["listening"].as_str().expect("listening").to_string(),
        client: reqwest::blocking::Client::builder()
            .timeout(WAIT)
            .build()
            .unwrap(),
    }
}

impl Service {
    /// Asks `question` as `identity`; the last event of the stream, as `(name, value)`.
    fn ask(&self, identity: &str, question: &str) -> (String, Value) {
        let text = self
            .client
            .post(format!("http://{}/ask", self.addr))
            .header(HEADER, identity)
            .body(json!({"question": question}).to_string())
            .send()
            .unwrap()
            .text()
            .unwrap();
        let block = text
            .split("\n\n")
            .filter(|b| !b.trim().is_empty())
            .last()
            .expect("an event");
        let data: String = block
            .lines()
            .filter_map(|l| l.strip_prefix("data:"))
            .map(str::trim)
            .collect();
        let decoded: cortex_spec_types::CortexAskAskEvent =
            serde_json::from_str(&data).unwrap_or_else(|e| panic!("not an AskEvent ({e}): {data}"));
        let wire = serde_json::to_value(decoded).unwrap();
        (
            wire["event"].as_str().unwrap().to_string(),
            wire["value"].clone(),
        )
    }

    /// Asks and answers the `done` event's `answer_id`.
    fn answered(&self, identity: &str, question: &str) -> (String, Value) {
        let (name, done) = self.ask(identity, question);
        assert_eq!(name, "done", "{done}");
        let id = done["answer_id"]
            .as_str()
            .expect("an answer_id")
            .to_string();
        (id, done)
    }

    /// A request with the identity header when `identity` is given: the status and the JSON body.
    fn call(
        &self,
        method: &str,
        path: &str,
        identity: Option<&str>,
        body: Option<Value>,
    ) -> (u16, Value) {
        let url = format!("http://{}{path}", self.addr);
        let mut request = match method {
            "GET" => self.client.get(url),
            _ => self.client.post(url),
        };
        if let Some(identity) = identity {
            request = request.header(HEADER, identity);
        }
        if let Some(body) = body {
            request = request
                .header("content-type", "application/json")
                .body(body.to_string());
        }
        let response = request.send().unwrap();
        let status = response.status().as_u16();
        let text = response.text().unwrap();
        (status, serde_json::from_str(&text).unwrap_or(Value::Null))
    }

    fn vote(&self, id: &str, identity: &str, value: &str) -> (u16, Value) {
        self.call(
            "POST",
            &format!("/ask/{id}/vote"),
            Some(identity),
            Some(json!({"value": value, "comment": "a comment"})),
        )
    }

    /// The open review items, as `GET /review` lists them.
    fn reviews(&self) -> Vec<Value> {
        let (status, body) = self.call("GET", "/review", Some("reviewer"), None);
        assert_eq!(status, 200, "{body}");
        body["reviews"].as_array().expect("reviews").clone()
    }
}

fn causes_of(reviews: &[Value], answer_id: &str) -> Vec<String> {
    let mut causes: Vec<String> = reviews
        .iter()
        .filter(|r| r["answer_id"] == answer_id)
        .map(|r| r["cause"].as_str().unwrap().to_string())
        .collect();
    causes.sort();
    causes
}

/// After one answer, `ask.sqlite` holds one record of it, whose token counts are the `done`
/// event's, with its tool call, question, identity and outcome.
#[test]
fn one_answer_is_kept_as_one_record_with_the_token_counts_of_its_done_event() {
    let world = World::new();
    let url = gateway(vec![
        (0, tool_reply((100, 10, 40))),
        (0, text_reply("Example Labs [node:n-1].", (200, 20, 60))),
    ]);
    let service = serve(&world, &world.spec(&url, "", ""));
    let (id, done) = service.answered("alice", "Who makes the Widget engine?");
    let records = world.records().answers().unwrap();
    assert_eq!(records.len(), 1, "{records:?}");
    let record = &records[0];
    assert_eq!(record.answer_id.0, id);
    assert_eq!(record.instance_name.0, "asker");
    assert_eq!(record.identity, "alice");
    assert_eq!(record.question, "Who makes the Widget engine?");
    assert_eq!(record.outcome, ma::AnswerOutcome::Answered);
    assert_eq!(record.answer.as_deref(), done["answer"].as_str());
    assert_eq!(record.input_tokens, done["input_tokens"].as_i64().unwrap());
    assert_eq!(
        record.output_tokens,
        done["output_tokens"].as_i64().unwrap()
    );
    assert_eq!(
        record.cached_tokens,
        done["cached_tokens"].as_i64().unwrap()
    );
    assert_eq!(
        record.context_tokens,
        done["context_tokens"].as_i64().unwrap()
    );
    assert_eq!(record.steps, done["steps"].as_i64().unwrap());
    assert_eq!((record.input_tokens, record.output_tokens), (300, 30));
    assert_eq!(record.tool_calls.len(), 1, "{:?}", record.tool_calls);
    assert_eq!(record.tool_calls[0].name, "search");
    assert!(record.latency_ms >= 0);
    // UUIDv7: the version nibble is 7.
    assert_eq!(id.len(), 36, "{id}");
    assert_eq!(&id[14..15], "7", "{id}");
}

/// One identity votes `down` then `up` on one answer: one vote remains, `up`, and the second vote
/// opens no review item beside the one the first opened. A vote needs an identity (401) and an
/// answer that exists (404), and a value of `up` or `down` (400).
#[test]
fn a_vote_changed_from_down_to_up_stays_one_vote_and_opens_no_second_review() {
    let world = World::new();
    let url = gateway(vec![(0, text_reply("It is Example Labs.", (5, 2, 0)))]);
    let service = serve(&world, &world.spec(&url, "", ""));
    let (id, _) = service.answered("alice", "Who makes it?");

    let (status, body) = service.vote(&id, "bob", "down");
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["value"], "down");
    assert_eq!(causes_of(&service.reviews(), &id), ["voted-down"]);

    let (status, body) = service.vote(&id, "bob", "up");
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["value"], "up");
    let votes = world.records().votes(&id).unwrap();
    assert_eq!(votes.len(), 1, "{votes:?}");
    assert_eq!(votes[0].value, ma::VoteValue::Up);
    assert_eq!(votes[0].identity, "bob");
    assert_eq!(votes[0].comment.as_deref(), Some("a comment"));
    assert_eq!(causes_of(&service.reviews(), &id), ["voted-down"]);
    assert_eq!(world.records().reviews(&id).unwrap().len(), 1);

    // A second identity's down vote is a second vote, and still one item for the cause.
    assert_eq!(service.vote(&id, "carol", "down").0, 200);
    assert_eq!(world.records().votes(&id).unwrap().len(), 2);
    assert_eq!(world.records().reviews(&id).unwrap().len(), 1);

    let (status, _) = service.call(
        "POST",
        &format!("/ask/{id}/vote"),
        None,
        Some(json!({"value": "up"})),
    );
    assert_eq!(status, 401);
    let (status, body) = service.vote("0192f000-0000-7000-8000-000000000000", "bob", "up");
    assert_eq!(status, 404, "{body}");
    assert_eq!(body["error"], "no-such-answer");
    let (status, _) = service.vote(&id, "bob", "sideways");
    assert_eq!(status, 400);
    assert_eq!(world.records().votes(&id).unwrap().len(), 2);
}

/// A `down` vote, a failed answer and an answer slower than `slow_after_ms` each open one review
/// item with the matching cause; `GET /review` lists them; closing one removes it from the list,
/// and closing it again answers 409, an unknown one 404.
#[test]
fn a_down_vote_a_failure_and_a_slow_answer_each_open_one_review_with_its_cause() {
    let world = World::new();
    let url = gateway(vec![
        (0, text_reply("Fast answer.", (5, 2, 0))),
        (0, broken_reply()),
        (2500, text_reply("Slow answer.", (5, 2, 0))),
    ]);
    let service = serve(&world, &world.spec(&url, "  slow_after_ms: 1500", ""));
    let (voted, _) = service.answered("alice", "first?");
    let (name, error) = service.ask("alice", "second?");
    assert_eq!(name, "error", "{error}");
    let (slow, _) = service.answered("alice", "third?");
    assert_eq!(service.vote(&voted, "bob", "down").0, 200);

    let failed = world
        .records()
        .answers()
        .unwrap()
        .into_iter()
        .find(|r| r.question == "second?")
        .expect("the failed answer is recorded");
    assert_eq!(failed.outcome, ma::AnswerOutcome::Failed);
    assert_eq!(failed.answer, None);
    assert_eq!(failed.reason.as_deref(), Some("model-failed"));
    let failed = failed.answer_id.0;

    let reviews = service.reviews();
    assert_eq!(reviews.len(), 3, "{reviews:?}");
    assert_eq!(causes_of(&reviews, &voted), ["voted-down"]);
    assert_eq!(causes_of(&reviews, &failed), ["failed"]);
    assert_eq!(causes_of(&reviews, &slow), ["slow"]);

    let item = reviews
        .iter()
        .find(|r| r["answer_id"] == failed.as_str())
        .unwrap();
    let review_id = item["review_id"].as_str().unwrap();
    let path = format!("/review/{review_id}/close");
    let (status, body) = service.call(
        "POST",
        &path,
        Some("reviewer"),
        Some(json!({"reason": "the gateway was down"})),
    );
    assert_eq!(status, 200, "{body}");
    let reviews = service.reviews();
    assert_eq!(reviews.len(), 2, "{reviews:?}");
    assert!(causes_of(&reviews, &failed).is_empty());
    let closed = world.records().reviews(&failed).unwrap();
    assert_eq!(closed.len(), 1);
    assert_eq!(closed[0].state, ma::ReviewItemState::Closed);
    assert_eq!(
        closed[0].data.close_reason.as_deref(),
        Some("the gateway was down")
    );

    let (status, body) = service.call(
        "POST",
        &path,
        Some("reviewer"),
        Some(json!({"reason": "again"})),
    );
    assert_eq!(status, 409, "{body}");
    let (status, _) = service.call(
        "POST",
        "/review/nothing:failed/close",
        Some("reviewer"),
        Some(json!({"reason": "x"})),
    );
    assert_eq!(status, 404);
    let (status, _) = service.call("GET", "/review", None, None);
    assert_eq!(status, 401);
}

/// With `retention_days: 1`, a record dated two days ago is gone after the purge the service makes
/// when it starts, with its votes and review items; one dated today stays.
#[test]
fn a_record_past_retention_is_purged_with_its_votes_and_reviews_and_a_new_one_stays() {
    let world = World::new();
    let url = gateway(vec![(0, text_reply("An answer.", (5, 2, 0)))]);
    let spec = world.spec(&url, "  retention_days: 1", "");
    let (old, new) = {
        let service = serve(&world, &spec);
        let (old, _) = service.answered("alice", "old?");
        let (new, _) = service.answered("alice", "new?");
        for id in [&old, &new] {
            assert_eq!(service.vote(id, "bob", "down").0, 200);
        }
        assert_eq!(service.reviews().len(), 2);
        (old, new)
    };
    let two_days_ago = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
        - 2 * 86_400_000;
    let db = rusqlite::Connection::open(world.records_file()).unwrap();
    let changed = db
        .execute(
            "UPDATE answers SET at_ms = ?1 WHERE answer_id = ?2",
            rusqlite::params![two_days_ago, old],
        )
        .unwrap();
    assert_eq!(changed, 1);
    drop(db);

    let service = serve(&world, &spec);
    let records = world.records();
    let kept: Vec<String> = records
        .answers()
        .unwrap()
        .into_iter()
        .map(|r| r.answer_id.0)
        .collect();
    assert_eq!(kept, std::slice::from_ref(&new));
    assert!(records.votes(&old).unwrap().is_empty());
    assert!(records.reviews(&old).unwrap().is_empty());
    assert_eq!(records.votes(&new).unwrap().len(), 1);
    assert_eq!(causes_of(&service.reviews(), &new), ["voted-down"]);
    assert!(causes_of(&service.reviews(), &old).is_empty());
    let (status, _) = service.vote(&old, "bob", "up");
    assert_eq!(status, 404);
}

/// A `retention_days` or `slow_after_ms` of 0 or less is refused before anything binds: exit 2,
/// naming the field.
#[test]
fn a_retention_or_slow_threshold_of_zero_is_refused_naming_the_field() {
    let world = World::new();
    for (extra, field) in [
        ("  retention_days: 0", "ask.retention_days"),
        ("  slow_after_ms: -5", "ask.slow_after_ms"),
    ] {
        let spec = world.spec("http://127.0.0.1:9", extra, "");
        let (code, text) = run_bounded(
            Command::new(env!("CARGO_BIN_EXE_cortex"))
                .args(["ask", "serve", "--spec"])
                .arg(&spec)
                .env("CORTEX_HOME", world.home())
                .env(TOKEN_ENV, "t"),
        );
        assert_eq!(code, Some(2), "{text}");
        assert!(text.contains(field), "{text}");
    }
}

/// An instance on a PostgreSQL store keeps its records in a schema of their own, a backend not
/// built yet: until it is, `cortex ask serve` refuses the instance before anything binds (exit 2,
/// naming `store.backend`), and writes no `ask.sqlite` beside a store that is not SQLite.
#[test]
fn an_instance_on_a_postgres_store_is_refused_until_its_records_backend_exists() {
    let world = World::new();
    let config = world.root.join("ekr-postgres.json");
    std::fs::write(
        &config,
        json!({"format": "ekr.postgres/1", "connection_file": "pg.connection",
               "schema": "ekr", "database_connections": 10, "replicas": 1,
               "reserved_connections": 1})
        .to_string(),
    )
    .unwrap();
    let spec = world.spec(
        "http://127.0.0.1:9",
        "",
        &format!(
            "store: {{backend: postgres, value: {{config: \"{}\"}}}}",
            config.display()
        ),
    );
    let (code, text) = run_bounded(
        Command::new(env!("CARGO_BIN_EXE_cortex"))
            .args(["ask", "serve", "--spec"])
            .arg(&spec)
            .env("CORTEX_HOME", world.home())
            .env(TOKEN_ENV, "t"),
    );
    assert_eq!(code, Some(2), "{text}");
    assert!(text.contains("store.backend"), "{text}");
    assert!(text.contains("postgres"), "{text}");
    assert!(!text.contains("listening"), "{text}");
    assert!(!world.records_file().exists());
}
