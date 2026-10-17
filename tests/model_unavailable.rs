//! `story:model-unavailable-outcome`: a run whose model cannot be reached (its connection
//! refused, the gateway answering 502, 503 or 504, the model not loaded, `claude` not startable)
//! ends `model-unavailable`, exit 7, and `--record-failure` does not count it against the source.
//! An answer that is not valid still ends `extraction-failed`, exit 1, counted. A batch applied
//! before the model became unavailable stays applied.
//!
//! The stand-in `claude` reaches the gateway as the real one does, through `ANTHROPIC_BASE_URL`
//! (with `curl`), and words a failure as `claude -p` does: `API Error: Connection error.` or
//! `API Error: <status> <body>`. Every process and the stand-in gateway are bounded in time.

mod common;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use common::{executable, World};
use serde_json::{json, Value};

/// A `claude` that asks the gateway first: a refused connection, or an HTTP status of 500 or
/// more, is answered as `claude -p` answers it (an error that cost zero); otherwise it answers as
/// the world's stand-in does, with the usage a gateway's token budget counts. From its `from`-th
/// call on only; earlier calls are the stand-in's.
fn gateway_claude(w: &World, from: u32) -> PathBuf {
    let path = w.bin.join("claude-gw");
    executable(
        &path,
        &format!(
            r#"R="{root}"
n=$(cat "$R/gw-calls" 2>/dev/null || echo 0); n=$((n+1)); echo "$n" > "$R/gw-calls"
if [ "$n" -ge {from} ]; then
  body=$(curl -s -m 10 -o "$R/gw-body" -w '%{{http_code}}' -X POST "$ANTHROPIC_BASE_URL/v1/messages" -d '{{}}'); code=$?
  if [ "$code" -ne 0 ]; then
    echo '{{"type":"result","subtype":"success","is_error":true,"result":"API Error: Connection error.","total_cost_usd":0}}'
    exit 1
  fi
  if [ "$body" -ge 500 ]; then
    text=$(tr -d '"\n' < "$R/gw-body")
    printf '{{"type":"result","subtype":"success","is_error":true,"result":"API Error: %s %s","total_cost_usd":0}}\n' "$body" "$text"
    exit 1
  fi
fi
"$R/bin/claude" "$@" | sed 's/"is_error":false,/"is_error":false,"usage":{{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":10}},/'
"#,
            root = w.root.display()
        ),
    );
    path
}

/// A gateway on 127.0.0.1 that answers every request with `status` and `body`, for 60 s at most;
/// its base URL.
fn gateway(status: u16, body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    std::thread::spawn(move || {
        let until = Instant::now() + Duration::from_secs(60);
        while Instant::now() < until {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let _ = stream.set_nonblocking(false);
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                    let mut buf = [0u8; 4096];
                    let _ = stream.read(&mut buf);
                    let _ = write!(
                        stream,
                        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                }
                Err(_) => std::thread::sleep(Duration::from_millis(10)),
            }
        }
    });
    url
}

/// A base URL whose port refuses connections: bound, then closed.
fn refusing() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    url
}

/// The world's spec of `t` behind a gateway at `base_url`, with `model_extra` after it and
/// `monitoring.metrics`.
fn spec(w: &World, base_url: &str, model_extra: &str) -> PathBuf {
    let path = w.spec("t", "conn_test");
    let text = std::fs::read_to_string(&path).unwrap();
    let from = "timeout_s: 60}";
    assert!(text.contains(from), "{text}");
    let text = text.replacen(
        from,
        &format!(
            "timeout_s: 60, budget_tokens: 1000000, gateway: {{base_url: \"{base_url}\", token_env: GW_TOKEN}}{model_extra}}}"
        ),
        1,
    ) + "monitoring: {metrics: true}\n";
    std::fs::write(&path, text).unwrap();
    path
}

fn create(w: &World, spec: &Path) {
    let (code, out) = w.cortex_env(
        &[
            "create",
            "--spec",
            spec.to_str().unwrap(),
            "--no-extract",
            "--no-units",
        ],
        &[("GW_TOKEN", Path::new("gw-token-not-a-secret"))],
    );
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("created")),
        "{out}"
    );
}

/// `cortex run t/news --record-failure` with `claude`, within 60 s: its exit status and its first
/// line.
fn run(w: &World, claude: &Path) -> (Option<i32>, Value) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["run", "t/news", "--record-failure"])
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", claude)
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .env("CORTEX_MODEL_RETRY_MS", "10")
        .env("GW_TOKEN", "gw-token-not-a-secret")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() > Duration::from_secs(60) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("the run did not end within 60 s");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let mut stdout = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut stdout)
        .unwrap();
    let first = stdout
        .lines()
        .next()
        .and_then(|l| serde_json::from_str(l).ok())
        .unwrap_or(Value::Null);
    (status.code(), first)
}

fn news(w: &World) -> Value {
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .arg("sources")
        .env("CORTEX_HOME", &w.home)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .find(|row| row["source_id"] == "t/news")
        .expect("a row for t/news")
}

fn metrics(w: &World) -> String {
    std::fs::read_to_string(w.home.join("instances/t/metrics.prom")).unwrap_or_default()
}

#[test]
fn a_gateway_that_refuses_connections_ends_three_runs_model_unavailable_none_counted() {
    let w = World::new();
    create(&w, &spec(&w, &refusing(), ""));
    let claude = gateway_claude(&w, 1);
    let head = w.head("t");
    for _ in 0..3 {
        let (code, out) = run(&w, &claude);
        assert_eq!(
            (code, out["outcome"].as_str()),
            (Some(7), Some("model-unavailable")),
            "{out}"
        );
        assert_eq!(out["detail"]["cause"], "refused", "{out}");
        assert!(
            out["detail"]["reason"]
                .as_str()
                .is_some_and(|r| r.contains("Connection error")),
            "{out}"
        );
    }
    let row = news(&w);
    assert_eq!(row["consecutive_failures"], 0, "{row}");
    assert_eq!(row["state"], "Enabled", "{row}");
    assert_eq!(row["last_outcome"], "model-unavailable", "{row}");
    assert_eq!(w.head("t"), head, "nothing was applied");
    let text = metrics(&w);
    assert!(
        text.contains("cortex_source_runs_total{source=\"t/news\",outcome=\"model-unavailable\"} 3\n"),
        "{text}"
    );
    assert!(
        !text.contains("gw-token-not-a-secret"),
        "no credential in the metric file"
    );
}

#[test]
fn a_gateway_answering_502_503_or_504_ends_the_run_model_unavailable_gateway_down() {
    for status in [502, 503, 504] {
        let w = World::new();
        create(&w, &spec(&w, &gateway(status, "{\"error\":\"upstream\"}"), ""));
        let (code, out) = run(&w, &gateway_claude(&w, 1));
        assert_eq!(
            (code, out["outcome"].as_str()),
            (Some(7), Some("model-unavailable")),
            "{status}: {out}"
        );
        assert_eq!(out["detail"]["cause"], "gateway-down", "{status}: {out}");
        assert_eq!(news(&w)["consecutive_failures"], 0, "{status}");
    }
}

#[test]
fn a_gateway_whose_model_is_not_loaded_ends_the_run_model_unavailable_not_loaded() {
    let w = World::new();
    create(
        &w,
        &spec(
            &w,
            &gateway(503, "{\"error\":\"Model is currently loading\"}"),
            "",
        ),
    );
    let (code, out) = run(&w, &gateway_claude(&w, 1));
    assert_eq!(
        (code, out["outcome"].as_str()),
        (Some(7), Some("model-unavailable")),
        "{out}"
    );
    assert_eq!(out["detail"]["cause"], "not-loaded", "{out}");
}

#[test]
fn a_claude_that_cannot_be_started_ends_the_run_model_unavailable_refused() {
    let w = World::new();
    create(&w, &spec(&w, &refusing(), ""));
    let (code, out) = run(&w, &w.bin.join("no-such-claude"));
    assert_eq!(
        (code, out["outcome"].as_str()),
        (Some(7), Some("model-unavailable")),
        "{out}"
    );
    assert_eq!(out["detail"]["cause"], "refused", "{out}");
    assert_eq!(news(&w)["consecutive_failures"], 0);
}

#[test]
fn a_model_answering_invalid_output_still_ends_extraction_failed_and_is_counted() {
    let w = World::new();
    create(&w, &spec(&w, &refusing(), ""));
    let claude = w.bin.join("claude-invalid");
    executable(
        &claude,
        "cat > /dev/null\necho '{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"total_cost_usd\":0.01,\"result\":\"no document\"}'\n",
    );
    let (code, out) = run(&w, &claude);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (Some(1), Some("extraction-failed")),
        "{out}"
    );
    let row = news(&w);
    assert_eq!(row["consecutive_failures"], 1, "counted: {row}");
}

#[test]
fn a_batch_applied_before_the_model_became_unavailable_stays_applied() {
    let w = World::new();
    // Two documents of 700 characters: two batches at `batch_chars: 1000`.
    let body = |what: &str| {
        let mut text = format!("Example Labs develops the {what} engine. ");
        while text.len() < 700 {
            text.push_str("It is fast and small. ");
        }
        text
    };
    let pages = json!({
        "results": [
            {"url": "https://example.org/a", "title": "A", "description": "A.",
             "content": body("Widget"), "content_truncated": false, "published": null, "score": "0.9"},
            {"url": "https://example.org/b", "title": "B", "description": "B.",
             "content": body("Gadget"), "content_truncated": false, "published": null, "score": "0.5"},
        ],
        "complete": true, "truncation": [],
        "provenance": {"instance": "t", "profile": "tavily/2026-10", "received_at": "2026-10-05T00:00:00.000Z"},
    });
    std::fs::write(
        w.root.join("answer.json"),
        common::answer(&pages.to_string()),
    )
    .unwrap();
    create(&w, &spec(&w, &refusing(), ", batch_chars: 1000"));
    let head = w.head("t");

    // The first batch is answered; from the second call on the gateway refuses.
    let (code, out) = run(&w, &gateway_claude(&w, 2));
    assert_eq!(
        (code, out["outcome"].as_str()),
        (Some(7), Some("model-unavailable")),
        "{out}"
    );
    assert!(w.head("t") > head, "the first batch stays applied");
    assert_eq!(news(&w)["consecutive_failures"], 0);

    // The next run extracts only the document the unavailable model did not.
    let (code, out) = run(&w, &gateway_claude(&w, 1000));
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(out["detail"]["documents_applied"], 1, "{out}");
}
