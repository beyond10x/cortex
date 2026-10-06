//! A run that leaves the store worse than its own checks allow is undone automatically
//! (`story:run-gate`), end to end through the real binary and a real `ekr` (`tests/common`). A
//! missing `ekr` fails the test; it never skips.
//!
//! The stand-in model answers one fact per evidence id it was given and one fact citing a forged
//! id, so every batch it answers has exactly one refused fact.

mod common;

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use common::{ekr, executable, World};
use cortex_cli::gate;
use cortex_cli::run::{add_cost, Report};
use cortex_model::instance as m;
use cortex_model::primitives::Decimal;
use serde_json::{json, Value};

const SERVE: &str = "serve: {view_port: 18999}\n";

/// An instance `t` created from the common spec with `gate` appended.
fn create(w: &World, gate: &str) -> PathBuf {
    create_with(w, gate, |t| t)
}

/// An instance `t` created from the common spec with `extra` appended, after `edit` on its text.
fn create_with(w: &World, extra: &str, edit: impl Fn(String) -> String) -> PathBuf {
    let spec = w.spec("t", "conn_test");
    let text = std::fs::read_to_string(&spec).unwrap();
    assert!(text.contains(SERVE), "{text}");
    std::fs::write(&spec, format!("{}{extra}", edit(text))).unwrap();
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap()]);
    assert_eq!(
        (code, created["outcome"].as_str()),
        (0, Some("created")),
        "{created}"
    );
    w.home.join("instances/t")
}

/// Every JSON line `cortex` printed, and its exit code: `run --record-failure` prints the run's
/// answer and then the failure's.
fn cortex_lines(w: &World, args: &[&str]) -> (i32, Vec<Value>) {
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(args)
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines = stdout
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    assert!(
        out.status.code() != Some(2),
        "cortex {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code().unwrap_or(-1), lines)
}

fn log(dir: &Path) -> Vec<Value> {
    std::fs::read_to_string(dir.join("cortex.log"))
        .unwrap_or_default()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn snapshots(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir.join("snapshots"))
        .map(|e| {
            e.flatten()
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|n| !n.starts_with('.'))
                .filter_map(|n| n.strip_suffix(".sqlite").map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

#[test]
fn a_run_with_one_refused_fact_fails_a_max_0_gate_and_the_store_is_back_where_it_was() {
    let w = World::new();
    let dir = create(
        &w,
        "gate:\n  checks:\n    - {measure: facts_refused, max: \"0\"}\n",
    );
    let before = w.head("t");
    let state_before = std::fs::read_to_string(dir.join("state/news.json")).ok();

    let (code, lines) = cortex_lines(&w, &["run", "t/news", "--record-failure"]);
    let [ran, recorded] = &lines[..] else {
        panic!("the run's answer and the failure's: {lines:?}");
    };
    assert_ne!(code, 0, "{ran}");
    assert_eq!(ran["outcome"], "apply-refused", "{ran}");
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("facts_refused = 1"),
        "the run reports the gate failure with the measure and its value: {reason}"
    );
    assert_eq!(
        w.head("t"),
        before,
        "ekr head answers the revision from before the run"
    );
    // The failed run is counted for the source, as any failed run the timers record.
    assert_eq!(recorded["outcome"], "counted", "{recorded}");

    // `state/` is back too: no document of the undone run is recorded as seen, so the next run
    // asks for them again.
    let state_after = std::fs::read_to_string(dir.join("state/news.json")).ok();
    let seen = |state: &Option<String>| -> usize {
        state
            .as_deref()
            .and_then(|s| serde_json::from_str::<Value>(s).ok())
            .and_then(|v| v["documents"].as_object().map(|d| d.len()))
            .unwrap_or(0)
    };
    assert_eq!(seen(&state_after), seen(&state_before), "{state_after:?}");

    // cortex.log records the failure with the measure and its value, and what was restored.
    let last = log(&dir).pop().expect("a log line for the run");
    assert_eq!(last["source"], "news", "{last}");
    let failed = &last["gate"]["failed"];
    assert_eq!(failed[0]["measure"], "facts_refused", "{last}");
    assert_eq!(failed[0]["value"], 1, "{last}");
    let restored = last["gate"]["restored"].as_str().unwrap_or_default();
    assert!(
        snapshots(&dir).iter().any(|s| s == restored),
        "the log names the snapshot the run was undone to: {last}"
    );
}

#[test]
fn a_run_within_its_gate_is_kept() {
    let w = World::new();
    create(
        &w,
        "gate:\n  checks:\n    - {measure: facts_refused, max: \"1\"}\n    \
         - {measure: documents_applied, min: \"1\"}\n",
    );
    let before = w.head("t");
    let (code, ran) = w.cortex(&["run", "t/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["facts_refused"], 1, "{ran}");
    assert!(w.head("t") > before, "the run committed and stays: {ran}");
}

#[test]
fn a_measure_of_ekr_quality_is_read_by_its_path_after_the_apply() {
    let w = World::new();
    create(
        &w,
        "gate:\n  checks:\n    - {measure: assertions.active, max: \"0\"}\n",
    );
    let before = w.head("t");
    let (code, ran) = w.cortex(&["run", "t/news"]);
    assert_ne!(code, 0, "{ran}");
    assert_eq!(ran["outcome"], "apply-refused", "{ran}");
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("assertions.active = "), "{reason}");
    assert!(!reason.contains("assertions.active = 0 "), "{reason}");
    assert_eq!(w.head("t"), before, "the run was undone: {ran}");
}

fn seen_count(dir: &Path) -> usize {
    std::fs::read_to_string(dir.join("state/news.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v["documents"].as_object().map(|d| d.len()))
        .unwrap_or(0)
}

fn check(measure: &str, min: Option<&str>, max: Option<&str>) -> m::GateCheck {
    m::GateCheck {
        measure: measure.into(),
        min: min.map(|v| Decimal(v.into())),
        max: max.map(|v| Decimal(v.into())),
    }
}

fn no_quality() -> Result<Value, String> {
    panic!("ekr quality was asked for a run-report measure")
}

/// Whether process `pid` holds `path` open, read from `/proc/<pid>/fd`.
fn holds(pid: u32, path: &Path) -> bool {
    let path = std::fs::canonicalize(path).unwrap();
    std::fs::read_dir(format!("/proc/{pid}/fd"))
        .map(|fds| {
            fds.flatten()
                .any(|fd| std::fs::read_link(fd.path()).is_ok_and(|t| t == path))
        })
        .unwrap_or(false)
}

/// an agent reading the store through the MCP server `cortex mcp-line` configures
/// (`ekr mcp`, which holds the store open for as long as it serves) is the documented way to use
/// an instance. A run that fails its gate while one is attached must still be undone: the docs say
/// "On a `sqlite` store the run is undone first".
#[test]
fn a_failed_run_is_undone_while_an_mcp_reader_is_attached() {
    let w = World::new();
    let dir = create_with(
        &w,
        "gate:\n  checks:\n    - {measure: facts_refused, max: \"0\"}\n",
        |t| t,
    );
    let before = w.head("t");
    let seen_before = seen_count(&dir);

    // The MCP server exactly as `cortex mcp-line t` prints it, after `--`.
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["mcp-line", "t"])
        .env("CORTEX_HOME", &w.home)
        .output()
        .unwrap();
    let line = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let (_, server) = line.split_once(" -- ").expect("an MCP command line");
    let mut parts = server.split_whitespace();
    let mut mcp = Command::new(parts.next().unwrap())
        .args(parts)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let store = dir.join("store.sqlite");
    let deadline = Instant::now() + Duration::from_secs(10);
    while !holds(mcp.id(), &store) {
        assert!(Instant::now() < deadline, "ekr mcp never opened the store");
        std::thread::sleep(Duration::from_millis(50));
    }

    let (code, lines) = cortex_lines(&w, &["run", "t/news"]);
    let head = w.head("t");
    let seen_after = seen_count(&dir);
    drop(mcp.stdin.take().map(|mut s| s.flush()));
    let _ = mcp.kill();
    let _ = mcp.wait();

    let ran = lines.last().cloned().unwrap_or(Value::Null);
    assert_ne!(code, 0, "{ran}");
    assert_eq!(ran["outcome"], "apply-refused", "{ran}");
    assert_eq!(
        head, before,
        "a run that failed its gate was left in the store while an MCP reader was attached: {ran}"
    );
    assert_eq!(seen_after, seen_before, "state/ was not undone: {ran}");
}

/// with `snapshots: {keep: 1}`, the log's `gate.restored` names the snapshot the run
/// was undone to, and the docs and `tests/run_gate.rs` say it is one of the instance's snapshots.
#[test]
fn keep_1_the_snapshot_the_log_names_still_exists() {
    let w = World::new();
    let dir = create_with(
        &w,
        "snapshots: {keep: 1}\ngate:\n  checks:\n    - {measure: facts_refused, max: \"0\"}\n",
        |t| t,
    );
    let before = w.head("t");
    let (code, lines) = cortex_lines(&w, &["run", "t/news"]);
    let ran = lines.last().cloned().unwrap_or(Value::Null);
    assert_ne!(code, 0, "{ran}");
    assert_eq!(w.head("t"), before, "{ran}");
    let last = log(&dir).pop().expect("a log line for the run");
    let restored = last["gate"]["restored"].as_str().unwrap_or_default();
    assert!(!restored.is_empty(), "{last}");
    assert!(
        snapshots(&dir).iter().any(|s| s == restored),
        "the log names snapshot {restored:?}, the instance holds {:?}",
        snapshots(&dir)
    );
}

/// three batches, the third pushes `facts_refused` over the gate. Everything the
/// earlier batches applied is undone as well, and state/ holds no document of the run.
#[test]
fn a_late_batch_failing_the_gate_undoes_every_batch() {
    let w = World::new();
    let results: Vec<Value> = (0..30)
        .map(|i| {
            json!({"url": format!("https://example.org/doc-{i}"), "title": format!("D{i}"),
                "description": "Example Labs ships Widget.",
                "content": format!("Example Labs develops the Widget engine {i}. {}", "word ".repeat(900)),
                "content_truncated": false, "published": null, "score": "0.5"})
        })
        .collect();
    let pages = json!({"results": results, "complete": true, "truncation": [],
        "provenance": {"instance": "t", "profile": "tavily/2026-10", "received_at": "2026-10-05T00:00:00.000Z"}});
    std::fs::write(
        w.root.join("answer.json"),
        common::answer(&pages.to_string()),
    )
    .unwrap();
    let dir = create_with(
        &w,
        "gate:\n  checks:\n    - {measure: facts_refused, max: \"2\"}\n",
        |t| t.replace("max_documents_per_run: 10", "max_documents_per_run: 30"),
    );
    let before = w.head("t");
    let (code, lines) = cortex_lines(&w, &["run", "t/news"]);
    let ran = lines.last().cloned().unwrap_or(Value::Null);
    assert!(
        w.lines("claude-calls.log").len() >= 3,
        "three batches: {}",
        w.lines("claude-calls.log").len()
    );
    assert_ne!(code, 0, "{ran}");
    assert!(
        ran["detail"]["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("facts_refused = 3"),
        "{ran}"
    );
    assert_eq!(w.head("t"), before, "{ran}");
    assert_eq!(seen_count(&dir), 0, "{ran}");
    let state: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("state/news.json")).unwrap())
            .unwrap();
    assert!(state["last_success_started_at"].is_null(), "{state}");
    assert!(state["pending_since"].is_i64(), "{state}");
}

/// costs are summed in binary floating point. Two answers costing 0.1 and 0.2 USD
/// cost 0.3 USD, and a decimal `max: "0.3"` — the spec carries decimals as strings so they
/// compare exactly — passes. `cortex run` prints that cost as `0.3000`.
#[test]
fn a_cost_equal_to_its_max_passes() {
    let cost = add_cost(add_cost(Some(0.0), Some(0.1)), Some(0.2));
    let report = Report {
        cost_usd: cost,
        ..Report::default()
    };
    let checks = m::RunGate {
        checks: vec![check("cost_usd", None, Some("0.3"))],
    };
    let failed = gate::evaluate(&checks, &report, no_quality);
    assert!(
        failed.is_empty(),
        "a run costing 0.3 USD failed max 0.3: {:?}",
        failed
            .iter()
            .map(gate::Failed::describe)
            .collect::<Vec<_>>()
    );
}

/// a measure equal to its `min`, or to its `max`, is within the gate.
#[test]
fn a_measure_equal_to_its_min_or_max_passes() {
    let report = Report {
        documents_applied: 1,
        facts_refused: 1,
        ..Report::default()
    };
    let checks = m::RunGate {
        checks: vec![
            check("documents_applied", Some("1"), None),
            check("facts_refused", None, Some("1")),
            check("documents_applied", Some("1"), Some("1")),
        ],
    };
    assert_eq!(gate::evaluate(&checks, &report, no_quality), vec![]);
    let below = m::RunGate {
        checks: vec![check("documents_applied", Some("2"), None)],
    };
    assert_eq!(gate::evaluate(&below, &report, no_quality).len(), 1);
}

// The structured path: a source of records, no model call.

const STRUCTURED_CONNECTORS: &str = r#"R="@ROOT@"
case "$*" in
  *"connections list"*) echo '{"ok":true,"result":{"connections":[{"adapter":"directory","connection":"conn_test","state":"ready","revision":"rev1"}]}}' ;;
  *"connections revalidate"*) echo '{"ok":true,"result":{}}' ;;
  *"operations describe"*) echo '{"ok":true,"result":{"schema":"s","revision":"r"}}' ;;
  *"--operation people.list"*)
    printf '{"ok":true,"result":{"adapter":"directory","operation":"people.list","revision":"r","result":{"people":%s}}}\n' "$(cat "$R/people.json")" ;;
  *) echo '{"ok":false}'; exit 2 ;;
esac
"#;

const PEOPLE: &str = r#"[
 {"id": "P-1", "name": "Ada Lovelace", "role": "Engineer"},
 {"id": "P-2", "name": "Grace Hopper", "role": "Admiral", "manager": "Ada Lovelace"},
 {"id": "P-3", "name": "Linus Example", "role": "Maintainer"}
]"#;

/// the gate holds a structured run as it holds a model run: three records applied
/// against `documents_applied max 1` are undone, store and state/.
#[test]
fn a_structured_run_over_its_gate_is_undone() {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(
        &w.bin.join("connectors"),
        &STRUCTURED_CONNECTORS.replace("@ROOT@", &root),
    );
    std::fs::write(w.root.join("people.json"), PEOPLE).unwrap();
    let path = w.root.join("t.yaml");
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: t
description: Test brain.
ekr: {{version: "0.0.30", bin: "{ekr}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
  - name: people
    schedule: daily
    settings:
      kind: structured
      value:
        input:
          from: connectors
          value: {{adapter: directory, connection: conn_test, operation: people.list, inputs: [{{}}]}}
        records: "$.people"
        mapping:
          node_type: Person
          id: "$.id"
          name: "$.name"
          aliases: ["$.handle"]
          properties:
            - {{property: role, path: "$.role"}}
          relations:
            - {{relation: REPORTS_TO, target_type: Person, target_name: "$.manager"}}
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
serve: {{view_port: 18995}}
gate:
  checks:
    - {{measure: documents_applied, max: "1"}}
"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let dir = w.home.join("instances/t");
    let before = w.head("t");
    let (code, lines) = cortex_lines(&w, &["run", "t/people"]);
    let ran = lines.last().cloned().unwrap_or(Value::Null);
    assert_ne!(code, 0, "{ran}");
    assert!(
        ran["detail"]["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("documents_applied = 3 (max 1)"),
        "{ran}"
    );
    assert_eq!(w.head("t"), before, "{ran}");
    let state: Value = std::fs::read_to_string(dir.join("state/people.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Value::Null);
    assert_eq!(
        state["documents"].as_object().map_or(0, |d| d.len()),
        0,
        "{state}"
    );
}
