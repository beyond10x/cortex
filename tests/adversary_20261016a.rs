//! Adversary probes of `story:store-outcome-unknown-not-counted` (commit 2a42fab), on the paths
//! `tests/store_outcome_unknown.rs` leaves out: an unknown outcome between two counted failures,
//! recover's own `stage abandon` answering the unknown outcome after a killed run, and two runs in
//! a row whose publish answers it. The stand-in `ekr` answers a `postgres` store from a SQLite
//! file through the real `ekr`, as in `tests/store_outcome_unknown.rs`.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{ekr, World};
use serde_json::Value;

const UNKNOWN: &str = "ekr: publication outcome is unknown; resolve the exact retained occurrence";

/// The stand-in. Hooks are files under `<root>`:
/// - `unknown-begin`, `fail-begin`: on every `stage begin` while they exist, as in
///   `tests/store_outcome_unknown.rs`.
/// - `unknown-publish`: `stage publish` publishes nothing and answers the unknown outcome; on
///   every publish while it exists.
/// - `unknown-after-abandon`: `stage abandon` abandons, then answers the unknown outcome; once.
/// - `kill-after-apply`: a staged `apply-extraction` applies, then kills the `cortex` that started
///   it (a run killed part-way); once.
fn unknown_ekr(w: &World) -> PathBuf {
    let path = w.bin.join("ekr-pg");
    let body = r#"R="@R@"; EKR="@EKR@"; DB="$R/pg-as-sqlite.sqlite"; UNKNOWN="@UNKNOWN@"
printf '%s|%s\n' "${EKR_STAGE:--}" "$*" >> "$R/ekr-argv.log"
if [ "$1" = postgres-schema ]; then echo '{"format":"ekr.postgres-schema/1","ready":true}'; exit 0; fi
if [ "$EKR_BACKEND" = postgres ]; then
  if [ "$1" = head ] && [ ! -e "$DB" ]; then echo "the lineage has no seed" >&2; exit 1; fi
  EKR_BACKEND=sqlite; EKR_STORE="$DB"; export EKR_BACKEND EKR_STORE
fi
if [ "$1" = apply-extraction ] && [ -n "$EKR_STAGE" ] && [ -e "$R/kill-after-apply" ]; then
  rm "$R/kill-after-apply"; "$EKR" "$@" > /dev/null; kill -9 "$PPID"; exit 1
fi
case "$1 $2" in
  "stage begin")
    if [ -e "$R/fail-begin" ]; then echo "ekr: the store is unavailable: injected" >&2; exit 1; fi
    if [ -e "$R/unknown-begin" ]; then "$EKR" "$@" > /dev/null; echo "$UNKNOWN" >&2; exit 1; fi ;;
  "stage publish")
    if [ -e "$R/unknown-publish" ]; then echo "$UNKNOWN" >&2; exit 1; fi ;;
  "stage abandon")
    if [ -e "$R/unknown-after-abandon" ]; then rm "$R/unknown-after-abandon"; "$EKR" "$@" > /dev/null; echo "$UNKNOWN" >&2; exit 1; fi ;;
esac
exec "$EKR" "$@"
"#
    .replace("@R@", &w.root.display().to_string())
    .replace("@EKR@", &ekr().display().to_string())
    .replace("@UNKNOWN@", UNKNOWN);
    common::executable(&path, &body);
    path
}

fn hook(w: &World) -> PathBuf {
    let path = w.bin.join("hook");
    common::executable(
        &path,
        &format!("echo \"$*\" >> \"{}/hook.log\"\n", w.root.display()),
    );
    path
}

/// A spec over `docs/` on a direct `postgres` store answered by `bin`, held to a gate.
fn spec(w: &World, bin: &Path, hook: &Path) -> PathBuf {
    let config = w.root.join("pg/app.json");
    let held = w.root.join("pg/password.txt");
    std::fs::create_dir_all(held.parent().unwrap()).unwrap();
    std::fs::write(&held, "unknown-not-a-secret").unwrap();
    let document = serde_json::json!({
        "format": "ekr.postgres/1",
        "connection_file": "app.dsn",
        "password_file": held,
        "schema": "ekr",
        "database_connections": 100,
        "replicas": 1,
        "reserved_connections": 10,
    });
    std::fs::write(&config, document.to_string()).unwrap();
    std::fs::create_dir_all(w.root.join("docs")).unwrap();
    std::fs::write(
        w.root.join("docs/a.md"),
        "Example Labs develops the Widget engine.\n",
    )
    .unwrap();
    let text = format!(
        r#"format: cortex.instance/1
name: pg
description: Test brain.
ekr: {{version: "0.0.30", bin: "{bin}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
  - name: docs
    schedule: daily
    settings:
      kind: files
      value: {{paths: [docs], glob: "**/*.md"}}
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
serve: {{view_port: 18996}}
store: {{backend: postgres, value: {{config: "{config}"}}}}
gate:
  checks:
    - {{measure: facts_refused, max: "1"}}
monitoring:
  metrics: true
  hook: {{command: ["{hook}"], timeout_s: 10}}
"#,
        bin = bin.display(),
        config = config.display(),
        hook = hook.display(),
    );
    let path = w.root.join("pg.yaml");
    std::fs::write(&path, text).unwrap();
    path
}

fn cortex(w: &World, args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(args)
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .env_remove("EKR_STAGE")
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn run(w: &World) -> (Option<i32>, Value, String) {
    let (code, out, err) = cortex(w, &["run", "pg/docs", "--record-failure"]);
    let line = out
        .lines()
        .next()
        .and_then(|l| serde_json::from_str(l).ok())
        .unwrap_or(Value::Null);
    (code, line, format!("{out}{err}"))
}

fn create(w: &World, spec: &Path) -> PathBuf {
    let (code, out, err) = cortex(
        w,
        &["create", "--spec", spec.to_str().unwrap(), "--no-extract"],
    );
    assert_eq!(code, Some(0), "{out}{err}");
    w.home.join("instances/pg")
}

fn store_ekr(w: &World, dir: &Path, args: &[&str]) -> Value {
    let out = Command::new(ekr())
        .args(args)
        .env("EKR_HOST", dir.join("host.json"))
        .env("EKR_BACKEND", "sqlite")
        .env("EKR_STORE", w.root.join("pg-as-sqlite.sqlite"))
        .env_remove("EKR_STAGE")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "ekr {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

fn head(w: &World, dir: &Path) -> i64 {
    store_ekr(w, dir, &["head"])["revision"].as_i64().unwrap()
}

fn stages_in(w: &World, dir: &Path, state: &str) -> Vec<Value> {
    store_ekr(w, dir, &["stage", "list"])
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|s| s["state"] == state)
        .collect()
}

fn docs_row(w: &World) -> Value {
    let (code, out, err) = cortex(w, &["sources"]);
    assert_eq!(code, Some(0), "{out}{err}");
    out.lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .find(|row| row["source_id"] == "pg/docs")
        .expect("a row for pg/docs")
}

fn stage_record(dir: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(dir.join("stage.json")).unwrap()).unwrap()
}

fn model_calls(w: &World) -> usize {
    std::fs::read_to_string(w.root.join("claude-calls.log"))
        .unwrap_or_default()
        .lines()
        .count()
}

/// A counted failure, an unknown outcome, a counted failure: the unknown outcome neither counts
/// nor resets the count, so the second counted failure disables the source, and the hook hears
/// each run's outcome in order.
#[test]
fn an_unknown_outcome_between_two_counted_failures_neither_counts_nor_resets() {
    let w = World::new();
    let hook = hook(&w);
    create(&w, &spec(&w, &unknown_ekr(&w), &hook));

    std::fs::write(w.root.join("fail-begin"), "").unwrap();
    let (code, line, all) = run(&w);
    assert_eq!(
        (code, line["outcome"].as_str()),
        (Some(1), Some("apply-refused")),
        "{all}"
    );
    assert_eq!(docs_row(&w)["consecutive_failures"], 1);

    std::fs::remove_file(w.root.join("fail-begin")).unwrap();
    std::fs::write(w.root.join("unknown-begin"), "").unwrap();
    let (code, line, all) = run(&w);
    assert_eq!(
        (code, line["outcome"].as_str()),
        (Some(4), Some("store-outcome-unknown")),
        "{all}"
    );
    assert_eq!(
        out_lines(&all),
        1,
        "an unknown outcome prints no recorded-failure line: {all}"
    );
    let row = docs_row(&w);
    assert_eq!(row["consecutive_failures"], 1, "{row}");
    assert_eq!(row["state"], "Enabled", "{row}");

    std::fs::remove_file(w.root.join("unknown-begin")).unwrap();
    std::fs::write(w.root.join("fail-begin"), "").unwrap();
    let (code, line, all) = run(&w);
    assert_eq!(
        (code, line["outcome"].as_str()),
        (Some(1), Some("apply-refused")),
        "{all}"
    );
    assert_eq!(docs_row(&w)["state"], "Disabled");
    assert_eq!(
        w.lines("hook.log"),
        [
            "pg/docs apply-refused",
            "pg/docs store-outcome-unknown",
            "pg/docs disabled"
        ]
    );
}

/// The JSON lines a run printed on stdout (`all` is stdout then stderr).
fn out_lines(all: &str) -> usize {
    all.lines().take_while(|l| l.starts_with('{')).count()
}

/// A run killed part-way leaves `stage.json` with `publishing` false. The next run's recover
/// abandons the stage and EKR answers the unknown outcome after abandoning it: that run exits 4,
/// counts nothing and keeps `stage.json`. The run after abandons it again (EKR answers an abandon
/// of an abandoned stage with its original result), puts `state/` back and runs.
#[test]
fn recovers_abandon_answering_unknown_is_settled_by_the_run_after() {
    let w = World::new();
    let hook = hook(&w);
    let dir = create(&w, &spec(&w, &unknown_ekr(&w), &hook));
    let before = head(&w, &dir);

    std::fs::write(w.root.join("kill-after-apply"), "").unwrap();
    let (code, _, all) = run(&w);
    assert_eq!(code, None, "the run was not killed: {all}");
    let killed = stage_record(&dir);
    // `publishing` is written only once a publish began.
    assert_ne!(killed["publishing"], true, "{killed}");

    std::fs::write(w.root.join("unknown-after-abandon"), "").unwrap();
    let (code, line, all) = run(&w);
    assert_eq!(
        (code, line["outcome"].as_str()),
        (Some(4), Some("store-outcome-unknown")),
        "{all}"
    );
    assert_eq!(stage_record(&dir)["stage_id"], killed["stage_id"], "{all}");
    assert_eq!(docs_row(&w)["consecutive_failures"], 0);

    let (code, line, all) = run(&w);
    assert_eq!(
        (code, line["outcome"].as_str()),
        (Some(0), Some("ran")),
        "{all}"
    );
    assert!(!dir.join("stage.json").exists(), "{all}");
    let abandoned = stages_in(&w, &dir, "Abandoned");
    assert!(
        abandoned
            .iter()
            .any(|s| s["stage_id"] == killed["stage_id"]),
        "{abandoned:?}"
    );
    assert!(head(&w, &dir) > before, "{all}");
    // state/ was put back, so a.md was extracted again: once by the killed run, once now.
    assert_eq!(model_calls(&w), 2);
}

/// Two runs in a row whose publish answers the unknown outcome: the first keeps its stage in
/// `stage.json`; the second's recover answers it again and keeps the same stage, neither
/// overwriting the record nor beginning a stage of its own; the third publishes that stage.
#[test]
fn two_unknown_publishes_in_a_row_keep_the_first_stage() {
    let w = World::new();
    let hook = hook(&w);
    let dir = create(&w, &spec(&w, &unknown_ekr(&w), &hook));
    let before = head(&w, &dir);

    std::fs::write(w.root.join("unknown-publish"), "").unwrap();
    let (code, _, all) = run(&w);
    assert_eq!(code, Some(4), "{all}");
    let first = stage_record(&dir);
    assert_eq!(first["publishing"], true, "{first}");
    let begun = stages_in(&w, &dir, "Begun").len() + stages_in(&w, &dir, "Sealing").len();

    let (code, line, all) = run(&w);
    assert_eq!(
        (code, line["outcome"].as_str()),
        (Some(4), Some("store-outcome-unknown")),
        "{all}"
    );
    let second = stage_record(&dir);
    assert_eq!(second["stage_id"], first["stage_id"], "{second}");
    assert_eq!(second["head"], first["head"], "{second}");
    assert_eq!(second["publishing"], true, "{second}");
    assert_eq!(
        stages_in(&w, &dir, "Begun").len() + stages_in(&w, &dir, "Sealing").len(),
        begun,
        "the second run began a stage of its own"
    );
    assert_eq!(docs_row(&w)["consecutive_failures"], 0);

    std::fs::remove_file(w.root.join("unknown-publish")).unwrap();
    let (code, line, all) = run(&w);
    assert_eq!(
        (code, line["outcome"].as_str()),
        (Some(0), Some("ran")),
        "{all}"
    );
    assert!(!dir.join("stage.json").exists(), "{all}");
    assert!(head(&w, &dir) > before, "{all}");
    let published = stages_in(&w, &dir, "Published");
    assert_eq!(published.len(), 1, "{published:?}");
    assert_eq!(published[0]["stage_id"], first["stage_id"]);
    assert_eq!(model_calls(&w), 1);
}
