//! Adversary cases for `story:run-metrics` and `story:model-unavailable-outcome`: SIGTERM at the
//! phases a stop reaches (a fetch, a model call, the wait before a model call is asked again), a
//! stop that reaches the run's children as `systemctl stop` does (the whole unit's cgroup), a
//! second SIGTERM, mixed outcomes under `--record-failure`, the metric file as Prometheus text,
//! and what a model error may leave in a reason.
//!
//! Signals: a test sends SIGTERM only to the PID of the `cortex` child it spawned. To act as
//! `systemctl stop` does, a stand-in the run started reads that PID from a file the test wrote
//! and signals exactly it, then ends itself. Every wait is bounded; no test can hang.

mod common;

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use common::{executable, World};
use serde_json::{json, Value};

/// A run that is sent SIGTERM must end within this.
const STOP_BOUND: Duration = Duration::from_secs(10);

/// The world's spec of `t`, its model line given `model_extra`, with `tail` appended.
fn spec(w: &World, model_extra: &str, tail: &str) -> PathBuf {
    let path = w.spec("t", "conn_test");
    let mut text = std::fs::read_to_string(&path).unwrap();
    let from = "timeout_s: 60}";
    assert!(text.contains(from), "{text}");
    text = text.replacen(from, &format!("timeout_s: 60{model_extra}}}"), 1);
    text.push_str(tail);
    std::fs::write(&path, text).unwrap();
    path
}

fn create(w: &World, spec: &Path) {
    let (code, out) = w.cortex(&[
        "create",
        "--spec",
        spec.to_str().unwrap(),
        "--no-extract",
        "--no-units",
    ]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("created")),
        "{out}"
    );
}

/// Two documents of 700 characters: two batches at `batch_chars: 1000`.
fn long_pages(w: &World) {
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
}

/// `cortex run t/news --record-failure` with `claude` and `connectors`, started and not waited
/// for. Its PID is written to `cortex.pid` in the world's root.
fn start(w: &World, claude: &Path, connectors: &Path, retry_ms: &str) -> Child {
    let child = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["run", "t/news", "--record-failure"])
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", connectors)
        .env("CORTEX_CLAUDE", claude)
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .env("CORTEX_MODEL_RETRY_MS", retry_ms)
        .env_remove("EKR_STAGE")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    std::fs::write(w.root.join("cortex.pid"), child.id().to_string()).unwrap();
    child
}

/// Waits for `child` at most `bound`; kills it (that child only) and fails the test past that.
/// Its status, its first stdout line, its stderr and how long the wait took.
fn finish(mut child: Child, bound: Duration) -> (ExitStatus, Value, String, Duration) {
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() > bound {
            let _ = child.kill();
            let _ = child.wait();
            panic!("the run did not end within {bound:?}");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let mut stdout = String::new();
    let mut stderr = String::new();
    let _ = child.stdout.take().unwrap().read_to_string(&mut stdout);
    let _ = child.stderr.take().unwrap().read_to_string(&mut stderr);
    let first = stdout
        .lines()
        .next()
        .and_then(|l| serde_json::from_str(l).ok())
        .unwrap_or(Value::Null);
    (status, first, stderr, started.elapsed())
}

fn run_with(w: &World, claude: &Path) -> (ExitStatus, Value, String) {
    let child = start(w, claude, &w.bin.join("connectors"), "10");
    let (status, out, stderr, _) = finish(child, Duration::from_secs(60));
    (status, out, stderr)
}

/// Waits at most 30 s for `path` to exist.
fn wait_for(path: &Path) {
    let until = Instant::now() + Duration::from_secs(30);
    while !path.exists() {
        assert!(Instant::now() < until, "{} never appeared", path.display());
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// SIGTERM to the PID of the `cortex` child this test spawned, and to nothing else.
fn sigterm(child: &Child) {
    let sent = Command::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .status()
        .unwrap();
    assert!(sent.success());
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

/// The value of the sample `series` in the metric file, `None` when it has none.
fn sample(text: &str, series: &str) -> Option<f64> {
    text.lines().find_map(|l| {
        l.strip_prefix(series)?
            .strip_prefix(' ')?
            .trim()
            .parse()
            .ok()
    })
}

/// A `claude` that reads its prompt, then answers an error answer `result` that cost zero, as
/// `claude -p` answers a call that reached no model.
fn error_claude(w: &World, name: &str, result: &str) -> PathBuf {
    let path = w.bin.join(name);
    let answer = json!({"type": "result", "subtype": "success", "is_error": true,
        "result": result, "total_cost_usd": 0});
    executable(
        &path,
        &format!(
            "touch \"{root}/asked\"\ncat > /dev/null\nprintf '%s\\n' '{answer}'\nexit 1\n",
            root = w.root.display(),
            answer = answer.to_string().replace('\'', "'\\''"),
        ),
    );
    path
}

/// `connectors` that is the world's own, except that `operations invoke` first runs `before`.
fn connectors_with(w: &World, before: &str) -> PathBuf {
    let path = w.bin.join("connectors-adv");
    executable(
        &path,
        &format!(
            r#"R="{root}"
case "$*" in
  *"operations invoke"*) {before} ;;
esac
exec "$R/bin/connectors" "$@"
"#,
            root = w.root.display()
        ),
    );
    path
}

/// What a stand-in runs to act as `systemctl stop` does to a unit's cgroup: SIGTERM to the run
/// (the PID the test recorded), and the stand-in itself ends as a process sent SIGTERM does.
const STOPPED_WITH_THE_UNIT: &str = r#"i=0; while [ ! -s "$R/cortex.pid" ] && [ $i -lt 500 ]; do sleep 0.01; i=$((i+1)); done
kill -TERM "$(cat "$R/cortex.pid")""#;

// ---------------------------------------------------------------------------------------------
// SIGTERM

/// `systemctl stop` of cortex's own unit (`schedule.rs` writes `ExecStart=… run … --record-failure`,
/// default KillMode=control-group) sends SIGTERM to the run and to its model call at once. The
/// call dies of it before the run's 20 ms poll reads the flag: the run then ends
/// `extraction-failed`, exit 1, and is counted against the source, though it was stopped.
#[test]
#[ignore = "finding: a model call that dies of the same SIGTERM ends the run extraction-failed and counted, not terminated"]
fn a_stop_that_also_ends_the_model_call_ends_the_run_terminated_not_counted() {
    let w = World::new();
    create(&w, &spec(&w, "", "monitoring: {metrics: true}\n"));
    let claude = w.bin.join("claude-stopped");
    executable(
        &claude,
        &format!(
            "R=\"{}\"\ncat > /dev/null\n{STOPPED_WITH_THE_UNIT}\nexit 143\n",
            w.root.display()
        ),
    );
    let child = start(&w, &claude, &w.bin.join("connectors"), "10");
    let (status, out, stderr, took) = finish(child, Duration::from_secs(60));
    assert!(took < STOP_BOUND, "{took:?}");
    let row = news(&w);
    assert_eq!(
        (status.code(), out["outcome"].as_str()),
        (Some(8), Some("terminated")),
        "{status:?} {out} {stderr}"
    );
    assert_eq!(row["consecutive_failures"], 0, "stopped, not failed: {row}");
}

/// The same stop during the fetch: the `connectors` child dies of it, the fetch fails before the
/// run reaches its first safe point, and the run ends `fetch-failed`, counted. With one earlier
/// failure, the next such stop disables a healthy source.
#[test]
#[ignore = "finding: a fetch that dies of the same SIGTERM ends the run fetch-failed and counted, not terminated"]
fn a_stop_that_also_ends_the_fetch_ends_the_run_terminated_not_counted() {
    let w = World::new();
    create(&w, &spec(&w, "", "monitoring: {metrics: true}\n"));
    let connectors = connectors_with(
        &w,
        &format!(
            "{STOPPED_WITH_THE_UNIT}\nsleep 0.3\necho '{{\"ok\":false,\"error\":{{\"code\":\"failure\"}}}}' >&2\nexit 143"
        ),
    );
    let child = start(&w, &w.bin.join("claude"), &connectors, "10");
    let (status, out, stderr, took) = finish(child, Duration::from_secs(60));
    assert!(took < STOP_BOUND, "{took:?}");
    let row = news(&w);
    assert_eq!(
        (status.code(), out["outcome"].as_str()),
        (Some(8), Some("terminated")),
        "{status:?} {out} {stderr}"
    );
    assert_eq!(row["consecutive_failures"], 0, "stopped, not failed: {row}");
}

/// A model that cannot be reached answers an error that cost zero, and the run waits
/// `CORTEX_MODEL_RETRY_MS` (60 s by default) before asking again. That wait is no safe point: a
/// run sent SIGTERM in it sleeps it out first, past a Kubernetes grace period of 30 s, and is
/// then killed having recorded nothing.
#[test]
#[ignore = "finding: SIGTERM during the wait before a zero-cost error answer is asked again is not handled until the wait ends"]
fn a_run_sent_sigterm_while_it_waits_to_ask_again_ends_within_a_bounded_time() {
    let w = World::new();
    create(&w, &spec(&w, "", "monitoring: {metrics: true}\n"));
    let claude = error_claude(&w, "claude-down", "API Error: Connection error.");
    let child = start(&w, &claude, &w.bin.join("connectors"), "30000");
    wait_for(&w.root.join("asked"));
    std::thread::sleep(Duration::from_millis(500));
    sigterm(&child);
    let (status, out, stderr, took) = finish(child, Duration::from_secs(45));
    assert!(
        took < STOP_BOUND,
        "the run took {took:?} to end after SIGTERM: {status:?} {out} {stderr}"
    );
    assert_eq!(
        (status.code(), out["outcome"].as_str()),
        (Some(8), Some("terminated")),
        "{out}"
    );
}

/// A fetch is no safe point either: a run sent SIGTERM while its fetch is slow runs the fetch
/// out before it stops. The spec says so ("before a fetch result is used"); the story's reason
/// for the outcome (a Job's deadline, then SIGKILL after the grace period) is not met by it.
#[test]
#[ignore = "finding: SIGTERM during a slow fetch is not handled until the fetch ends"]
fn a_run_sent_sigterm_during_a_slow_fetch_ends_within_a_bounded_time() {
    let w = World::new();
    create(&w, &spec(&w, "", "monitoring: {metrics: true}\n"));
    let connectors = connectors_with(&w, "touch \"$R/fetching\"; sleep 20");
    let child = start(&w, &w.bin.join("claude"), &connectors, "10");
    wait_for(&w.root.join("fetching"));
    sigterm(&child);
    let (status, out, stderr, took) = finish(child, Duration::from_secs(45));
    assert!(
        took < STOP_BOUND,
        "the run took {took:?} to end after SIGTERM: {status:?} {out} {stderr}"
    );
    assert_eq!(status.code(), Some(8), "{out}");
}

/// A second SIGTERM ends the process at once with 143 and records nothing, even while the run is
/// still ending a model call that ignores SIGTERM (5 s grace).
#[test]
fn a_second_sigterm_exits_143_at_once_and_records_nothing() {
    let w = World::new();
    create(&w, &spec(&w, "", "monitoring: {metrics: true}\n"));
    let claude = w.bin.join("claude-stubborn");
    // Ignores SIGTERM (and so does its `sleep`); bounded at 8 s by itself.
    executable(
        &claude,
        &format!(
            "trap '' TERM\ntouch \"{}/started\"\nsleep 8\n",
            w.root.display()
        ),
    );
    let child = start(&w, &claude, &w.bin.join("connectors"), "10");
    wait_for(&w.root.join("started"));
    sigterm(&child);
    std::thread::sleep(Duration::from_millis(1000));
    sigterm(&child);
    let (status, out, stderr, took) = finish(child, Duration::from_secs(20));
    assert_eq!(status.code(), Some(143), "{status:?} {out} {stderr}");
    assert!(took < Duration::from_secs(2), "{took:?}");
    assert_eq!(
        sample(
            &metrics(&w),
            "cortex_source_runs_total{source=\"t/news\",outcome=\"terminated\"}"
        )
        .unwrap_or(0.0),
        0.0,
        "a second SIGTERM records nothing"
    );
    // Let the stand-in's own bound run out before the world is removed.
    std::thread::sleep(Duration::from_secs(8));
}

// ---------------------------------------------------------------------------------------------
// model-unavailable: false positives and negatives

/// A 4xx authentication error is the caller's fault, not an outage: `extraction-failed`, counted.
#[test]
fn an_authentication_error_is_extraction_failed_and_counted() {
    let w = World::new();
    create(&w, &spec(&w, "", "monitoring: {metrics: true}\n"));
    let claude = error_claude(
        &w,
        "claude-401",
        "API Error: 401 {\"type\":\"error\",\"error\":{\"type\":\"authentication_error\",\"message\":\"invalid x-api-key\"}}",
    );
    let (status, out, _) = run_with(&w, &claude);
    assert_eq!(
        (status.code(), out["outcome"].as_str()),
        (Some(1), Some("extraction-failed")),
        "{out}"
    );
    assert_eq!(news(&w)["consecutive_failures"], 1);
}

/// A fetch error whose text says `connection refused` (the source's API down) is the source's:
/// `fetch-failed`, exit 1, counted; never `model-unavailable`.
#[test]
fn a_fetch_error_that_says_connection_refused_stays_fetch_failed_and_counted() {
    let w = World::new();
    create(&w, &spec(&w, "", "monitoring: {metrics: true}\n"));
    let connectors = connectors_with(
        &w,
        "echo 'connect ECONNREFUSED 10.0.0.1:443: Connection refused; 502 Bad Gateway; model not loaded' >&2; exit 1",
    );
    let child = start(&w, &w.bin.join("claude"), &connectors, "10");
    let (status, out, _, _) = finish(child, Duration::from_secs(60));
    assert_eq!(
        (status.code(), out["outcome"].as_str()),
        (Some(1), Some("fetch-failed")),
        "{out}"
    );
    assert_eq!(news(&w)["consecutive_failures"], 1);
}

/// The API overloaded (`529 overloaded_error`) is the commonest model outage of the Anthropic API;
/// it is no fault of the source, yet the run ends `extraction-failed` and is counted, and the
/// second such run disables the source: what the story set out to prevent ("repeated model
/// outages never disable a healthy source"). The spec lists 502, 503 and 504 only.
#[test]
#[ignore = "finding: a 529 overloaded answer is extraction-failed and counted, so an API overload disables a healthy source"]
fn an_overloaded_api_is_model_unavailable() {
    let w = World::new();
    create(&w, &spec(&w, "", "monitoring: {metrics: true}\n"));
    let claude = error_claude(
        &w,
        "claude-529",
        "API Error: 529 {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"Overloaded\"}}",
    );
    let (status, out, _) = run_with(&w, &claude);
    assert_eq!(
        (status.code(), out["outcome"].as_str()),
        (Some(7), Some("model-unavailable")),
        "{out}"
    );
    assert_eq!(news(&w)["consecutive_failures"], 0);
}

/// A model error's text may hold a credential and run long: the reason, the printed line, the
/// status, `cortex.log`, stderr and the metric file hold it masked and bounded.
#[test]
fn a_model_unavailable_reason_is_masked_and_bounded() {
    let w = World::new();
    create(&w, &spec(&w, "", "monitoring: {metrics: true}\n"));
    // Assembled at runtime so no credential-shaped literal sits in the source.
    let token = format!("sk-{}", "Ab9".repeat(20));
    let claude = w.bin.join("claude-refused");
    executable(
        &claude,
        &format!(
            "cat > /dev/null\nprintf 'connect ECONNREFUSED 127.0.0.1:4000 auth %s %s\\n' '{token}' \"$(head -c 4000 /dev/zero | tr '\\0' x)\" >&2\nexit 1\n"
        ),
    );
    let (status, out, stderr) = run_with(&w, &claude);
    assert_eq!(
        (status.code(), out["outcome"].as_str()),
        (Some(7), Some("model-unavailable")),
        "{out}"
    );
    assert_eq!(out["detail"]["cause"], "refused", "{out}");
    let reason = out["detail"]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("ECONNREFUSED"), "{reason}");
    assert!(reason.len() < 1500, "{} bytes", reason.len());
    let dir = w.home.join("instances/t");
    let status_file = std::fs::read_to_string(dir.join("status/news.json")).unwrap_or_default();
    let log = std::fs::read_to_string(dir.join("cortex.log")).unwrap_or_default();
    for (what, text) in [
        ("line", out.to_string()),
        ("stderr", stderr),
        ("status", status_file),
        ("cortex.log", log),
        ("metrics.prom", metrics(&w)),
    ] {
        assert!(
            !text.contains("Ab9Ab9Ab9"),
            "the credential in {what}: {text}"
        );
    }
}

/// A run that applied a batch before its model became unavailable spent tokens and applied a
/// document; `metrics.prom` counts neither (only runs that ended `ran`), so
/// `cortex_source_model_tokens_total` under-reports what the model cost and
/// `cortex_source_documents_total{kind="applied"}` what the store holds. The story's series are
/// "the model tokens it spent" and "the documents it read and applied".
#[test]
#[ignore = "finding: tokens spent and documents applied by a run that did not end ran are never counted in metrics.prom"]
fn a_partial_model_unavailable_run_counts_the_tokens_it_spent_and_documents_it_applied() {
    let w = World::new();
    long_pages(&w);
    create(
        &w,
        &spec(&w, ", batch_chars: 1000", "monitoring: {metrics: true}\n"),
    );
    let claude = w.bin.join("claude-then-down");
    executable(
        &claude,
        &format!(
            r#"R="{root}"
n=$(cat "$R/calls" 2>/dev/null || echo 0); n=$((n+1)); echo "$n" > "$R/calls"
if [ "$n" -ge 2 ]; then cat > /dev/null; echo '{{"type":"result","subtype":"success","is_error":true,"result":"API Error: Connection error.","total_cost_usd":0}}'; exit 1; fi
"$R/bin/claude" "$@" | sed 's/"is_error":false,/"is_error":false,"usage":{{"input_tokens":100,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":40}},/'
"#,
            root = w.root.display()
        ),
    );
    let head = w.head("t");
    let (status, out, _) = run_with(&w, &claude);
    assert_eq!(status.code(), Some(7), "{out}");
    assert!(w.head("t") > head, "the first batch stays applied");
    let text = metrics(&w);
    assert_eq!(
        sample(
            &text,
            "cortex_source_documents_total{source=\"t/news\",kind=\"applied\"}"
        ),
        Some(1.0),
        "{text}"
    );
    assert!(
        sample(
            &text,
            "cortex_source_model_tokens_total{source=\"t/news\",kind=\"input\"}"
        )
        .is_some_and(|n| n >= 100.0),
        "{text}"
    );
}

// ---------------------------------------------------------------------------------------------
// --record-failure across mixed outcomes, and the metric file as Prometheus text

/// fetch-failed (counted), then model-unavailable and terminated (neither counted, neither
/// disables after the one failure), then fetch-failed again (the second consecutive failure
/// disables). The metric file is then valid exposition: HELP and TYPE once per family, before
/// its samples, families contiguous, no duplicate series, every value a finite number of 0 or
/// more, every label value quoted and escaped.
#[test]
fn mixed_outcomes_count_only_source_failures_and_leave_a_valid_metric_file() {
    let w = World::new();
    create(&w, &spec(&w, "", "monitoring: {metrics: true}\n"));

    std::fs::write(w.root.join("fail"), "").unwrap();
    let (status, out, _) = run_with(&w, &w.bin.join("claude"));
    assert_eq!(status.code(), Some(1), "{out}");
    assert_eq!(news(&w)["consecutive_failures"], 1);
    std::fs::remove_file(w.root.join("fail")).unwrap();

    let (status, out, _) = run_with(&w, &w.bin.join("no-such-claude"));
    assert_eq!(
        (status.code(), out["outcome"].as_str()),
        (Some(7), Some("model-unavailable")),
        "{out}"
    );
    let row = news(&w);
    assert_eq!(
        (row["consecutive_failures"].as_i64(), row["state"].as_str()),
        (Some(1), Some("Enabled")),
        "{row}"
    );

    let hang = w.bin.join("claude-hang");
    executable(
        &hang,
        &format!("touch \"{}/hung\"\nexec sleep 30\n", w.root.display()),
    );
    let child = start(&w, &hang, &w.bin.join("connectors"), "10");
    wait_for(&w.root.join("hung"));
    sigterm(&child);
    let (status, out, stderr, took) = finish(child, Duration::from_secs(20));
    assert_eq!(status.code(), Some(8), "{out} {stderr}");
    assert!(took < STOP_BOUND, "{took:?}");
    let row = news(&w);
    assert_eq!(
        (row["consecutive_failures"].as_i64(), row["state"].as_str()),
        (Some(1), Some("Enabled")),
        "{row}"
    );

    std::fs::write(w.root.join("fail"), "").unwrap();
    let (status, out, _) = run_with(&w, &w.bin.join("claude"));
    assert_eq!(status.code(), Some(1), "{out}");
    assert_eq!(news(&w)["state"], "Disabled");

    let text = metrics(&w);
    for (outcome, want) in [
        ("fetch-failed", 2.0),
        ("model-unavailable", 1.0),
        ("terminated", 1.0),
        ("extraction-failed", 0.0),
    ] {
        assert_eq!(
            sample(
                &text,
                &format!("cortex_source_runs_total{{source=\"t/news\",outcome=\"{outcome}\"}}")
            ),
            Some(want),
            "{outcome}\n{text}"
        );
    }
    exposition(&text);
}

/// Holds `text` to the Prometheus text format, as far as cortex's series use it.
fn exposition(text: &str) {
    use std::collections::{HashMap, HashSet};
    assert!(text.ends_with('\n'), "no final newline");
    let mut help: HashSet<String> = HashSet::new();
    let mut types: HashMap<String, String> = HashMap::new();
    let mut done: HashSet<String> = HashSet::new();
    let mut current: Option<String> = None;
    let mut series: HashSet<String> = HashSet::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("# HELP ") {
            let name = rest.split(' ').next().unwrap().to_string();
            assert!(help.insert(name.clone()), "HELP twice for {name}");
            continue;
        }
        if let Some(rest) = line.strip_prefix("# TYPE ") {
            let mut parts = rest.split(' ');
            let name = parts.next().unwrap().to_string();
            let kind = parts.next().unwrap_or_default().to_string();
            assert!(
                ["counter", "gauge"].contains(&kind.as_str()),
                "type {kind} of {name}"
            );
            assert!(
                types.insert(name.clone(), kind).is_none(),
                "TYPE twice for {name}"
            );
            continue;
        }
        assert!(!line.starts_with('#') && !line.is_empty(), "{line:?}");
        let (head, value) = line.rsplit_once(' ').unwrap();
        let value: f64 = value
            .parse()
            .unwrap_or_else(|_| panic!("value of {line:?}"));
        assert!(value.is_finite() && value >= 0.0, "{line:?}");
        let (name, labels) = head.split_once('{').unwrap_or((head, "}"));
        assert!(
            types.contains_key(name),
            "a sample of {name} before its TYPE"
        );
        assert!(help.contains(name), "no HELP for {name}");
        if current.as_deref() != Some(name) {
            assert!(
                done.insert(name.to_string()),
                "{name}: its samples are not contiguous"
            );
            current = Some(name.to_string());
        }
        let labels = labels.strip_suffix('}').unwrap();
        // key="value" pairs, the value escaped.
        let mut rest = labels;
        while !rest.is_empty() {
            let (key, after) = rest.split_once("=\"").unwrap();
            assert!(
                key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "{key}"
            );
            let mut end = None;
            let mut escaped = false;
            for (i, c) in after.char_indices() {
                match (escaped, c) {
                    (true, _) => escaped = false,
                    (false, '\\') => escaped = true,
                    (false, '"') => {
                        end = Some(i);
                        break;
                    }
                    _ => {}
                }
            }
            let end = end.unwrap_or_else(|| panic!("an unclosed label in {line:?}"));
            rest = after[end + 1..]
                .strip_prefix(',')
                .unwrap_or(&after[end + 1..]);
        }
        assert!(series.insert(head.to_string()), "{head} twice");
    }
}
