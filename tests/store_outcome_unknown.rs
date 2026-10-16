//! `story:store-outcome-unknown-not-counted`: an EKR stage verb whose outcome is unknown (EKR
//! answers "publication outcome is unknown; resolve the exact retained occurrence", a fault with
//! exit status 1 and no refusal name) fails the run as `store-outcome-unknown`, exit 4, and is not
//! recorded as a failure of the source; the next run settles the stage. No database is needed: as
//! in `postgres_run_undo_adv.rs`, a stand-in `ekr` answers a `postgres` store from a SQLite file
//! through the real `ekr` (stages work on SQLite too).
//!
//! `story:apply-outcome-unknown-not-counted`: the same answer from `apply-extraction` is the same
//! outcome. In a stage the run leaves the stage, which the next run abandons; outside one it marks
//! nothing of the batch seen, so the next run extracts it again: nothing is lost, and a batch that
//! did land is written again, a second set of assertions and evidence, as a re-extraction writes.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{ekr, World};
use serde_json::Value;

/// What EKR 0.0.34 prints for `StoreError::UnknownCommit` (`crates/ekr/src/exit.rs`: a fault,
/// `ekr: <message>`, exit 1).
const UNKNOWN: &str = "ekr: publication outcome is unknown; resolve the exact retained occurrence";

/// The stand-in. Hooks are files under `<root>`:
/// - `unknown-begin`: `stage begin` begins the stage (EKR holds it `Begun`, as on the deployment
///   that found this), then answers the unknown outcome; fires on every begin while it exists.
/// - `unknown-after-publish`: `stage publish` publishes, then answers the unknown outcome; once.
/// - `unknown-before-publish`: `stage publish` publishes nothing and answers the unknown outcome;
///   once.
/// - `unknown-publish`: `stage publish` publishes nothing and answers the unknown outcome; on
///   every publish while it exists.
/// - `fail-begin`: `stage begin` fails with an ordinary fault; on every begin while it exists.
/// - `unknown-after-apply`: `apply-extraction` applies, then answers the unknown outcome; on every
///   apply while it exists (`story:apply-outcome-unknown-not-counted`).
/// - `unknown-before-apply`: `apply-extraction` applies nothing and answers the unknown outcome;
///   on every apply while it exists.
fn unknown_ekr(w: &World) -> PathBuf {
    let path = w.bin.join("ekr-pg");
    let body = r#"R="@R@"; EKR="@EKR@"; DB="$R/pg-as-sqlite.sqlite"; UNKNOWN="@UNKNOWN@"
printf '%s|%s\n' "${EKR_STAGE:--}" "$*" >> "$R/ekr-argv.log"
if [ "$1" = postgres-schema ]; then echo '{"format":"ekr.postgres-schema/1","ready":true}'; exit 0; fi
if [ "$EKR_BACKEND" = postgres ]; then
  if [ "$1" = head ] && [ ! -e "$DB" ]; then echo "the lineage has no seed" >&2; exit 1; fi
  EKR_BACKEND=sqlite; EKR_STORE="$DB"; export EKR_BACKEND EKR_STORE
fi
case "$1 $2" in
  "stage begin")
    if [ -e "$R/fail-begin" ]; then echo "ekr: the store is unavailable: injected" >&2; exit 1; fi
    if [ -e "$R/unknown-begin" ]; then "$EKR" "$@" > /dev/null; echo "$UNKNOWN" >&2; exit 1; fi ;;
  "stage publish")
    if [ -e "$R/unknown-publish" ]; then echo "$UNKNOWN" >&2; exit 1; fi
    if [ -e "$R/unknown-before-publish" ]; then rm "$R/unknown-before-publish"; echo "$UNKNOWN" >&2; exit 1; fi
    if [ -e "$R/unknown-after-publish" ]; then rm "$R/unknown-after-publish"; "$EKR" "$@" > /dev/null; echo "$UNKNOWN" >&2; exit 1; fi ;;
esac
if [ "$1" = apply-extraction ]; then
  if [ -e "$R/unknown-before-apply" ]; then echo "$UNKNOWN" >&2; exit 1; fi
  if [ -e "$R/unknown-after-apply" ]; then "$EKR" "$@" > /dev/null || exit 1; echo "$UNKNOWN" >&2; exit 1; fi
fi
exec "$EKR" "$@"
"#
    .replace("@R@", &w.root.display().to_string())
    .replace("@EKR@", &ekr().display().to_string())
    .replace("@UNKNOWN@", UNKNOWN);
    common::executable(&path, &body);
    path
}

/// A hook that appends its arguments, one call per line, to `hook.log`.
fn hook(w: &World) -> PathBuf {
    let path = w.bin.join("hook");
    common::executable(
        &path,
        &format!("echo \"$*\" >> \"{}/hook.log\"\n", w.root.display()),
    );
    path
}

/// A spec over `docs/` on a direct `postgres` store answered by `bin`, held to a gate (so its runs
/// are staged), with the metric file and `hook`.
fn spec(w: &World, bin: &Path, hook: &Path) -> PathBuf {
    spec_held(w, bin, hook, true)
}

/// [`spec`], held to its gate (`gated`, so its runs are staged) or to none (so they run in no
/// stage, as an ungated `postgres` store's do).
fn spec_held(w: &World, bin: &Path, hook: &Path, gated: bool) -> PathBuf {
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
    let gate = "gate:\n  checks:\n    - {measure: facts_refused, max: \"1\"}\n";
    assert!(text.contains(gate));
    let text = if gated { text } else { text.replace(gate, "") };
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

/// The run's own line: the first `cortex run` prints.
fn run_line(stdout: &str) -> Value {
    stdout
        .lines()
        .next()
        .and_then(|l| serde_json::from_str(l).ok())
        .unwrap_or(Value::Null)
}

/// `cortex run pg/docs --record-failure`, as the timers run it.
fn run(w: &World) -> (Option<i32>, Value, String) {
    let (code, out, err) = cortex(w, &["run", "pg/docs", "--record-failure"]);
    (code, run_line(&out), format!("{out}{err}"))
}

fn create(w: &World, spec: &Path) -> PathBuf {
    let (code, out, err) = cortex(
        w,
        &["create", "--spec", spec.to_str().unwrap(), "--no-extract"],
    );
    assert_eq!(code, Some(0), "{out}{err}");
    w.home.join("instances/pg")
}

/// `ekr <args>` on the SQLite file the stand-in answers the `postgres` store from.
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

/// The `sources` row of `pg/docs`.
fn docs_row(w: &World) -> Value {
    let (code, out, err) = cortex(w, &["sources"]);
    assert_eq!(code, Some(0), "{out}{err}");
    out.lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .find(|row| row["source_id"] == "pg/docs")
        .expect("a row for pg/docs")
}

fn status_file(dir: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(dir.join("status/docs.json")).unwrap()).unwrap()
}

fn runs_of(dir: &Path, outcome: &str) -> f64 {
    let text = std::fs::read_to_string(dir.join("metrics.prom")).unwrap_or_default();
    let series = format!("cortex_source_runs_total{{source=\"pg/docs\",outcome=\"{outcome}\"}} ");
    text.lines()
        .find_map(|l| l.strip_prefix(&series)?.trim().parse().ok())
        .unwrap_or(0.0)
}

fn stage_record(dir: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(dir.join("stage.json")).unwrap()).unwrap()
}

fn seen_a(dir: &Path) -> bool {
    let seen: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("state/docs.json")).unwrap())
            .unwrap_or(Value::Null);
    seen["documents"]
        .as_object()
        .is_some_and(|d| d.keys().any(|k| k.ends_with("/a.md")))
}

fn model_calls(w: &World) -> usize {
    std::fs::read_to_string(w.root.join("claude-calls.log"))
        .unwrap_or_default()
        .lines()
        .count()
}

/// Acceptance 2: `stage begin` answers the unknown outcome. Two runs with `--record-failure` each
/// exit 4 as `store-outcome-unknown`; the source stays enabled with `consecutive_failures` 0; its
/// status file, the metric file and the hook report the outcome. The stage EKR began is abandoned
/// by the next run, which runs.
#[test]
fn an_unknown_outcome_at_stage_begin_exits_4_twice_and_never_counts_against_the_source() {
    let w = World::new();
    let hook = hook(&w);
    let dir = create(&w, &spec(&w, &unknown_ekr(&w), &hook));

    std::fs::write(w.root.join("unknown-begin"), "").unwrap();
    for n in 1..=2 {
        let (code, line, all) = run(&w);
        assert_eq!(
            (code, line["outcome"].as_str()),
            (Some(4), Some("store-outcome-unknown")),
            "run {n}: {all}"
        );
        let reason = line["detail"]["reason"].as_str().unwrap_or_default();
        assert!(
            reason.contains("publication outcome is unknown"),
            "run {n}: {reason}"
        );
        let row = docs_row(&w);
        assert_eq!(row["state"], "Enabled", "run {n}: {row}");
        assert_eq!(row["consecutive_failures"], 0, "run {n}: {row}");
        assert_eq!(
            row["last_outcome"], "store-outcome-unknown",
            "run {n}: {row}"
        );
        let status = status_file(&dir);
        assert_eq!(status["last_outcome"], "store-outcome-unknown", "{status}");
        assert_eq!(status["runs"]["store-outcome-unknown"], n, "{status}");
        assert_eq!(runs_of(&dir, "store-outcome-unknown"), n as f64);
    }
    assert_eq!(
        w.lines("hook.log"),
        [
            "pg/docs store-outcome-unknown",
            "pg/docs store-outcome-unknown"
        ]
    );
    // Neither the timer was disabled nor a failure recorded.
    let systemctl = std::fs::read_to_string(w.root.join("systemctl.log")).unwrap_or_default();
    assert!(!systemctl.contains("disable"), "{systemctl}");

    std::fs::remove_file(w.root.join("unknown-begin")).unwrap();
    let (code, line, all) = run(&w);
    assert_eq!(
        (code, line["outcome"].as_str()),
        (Some(0), Some("ran")),
        "{all}"
    );
    assert!(
        stages_in(&w, &dir, "Begun").is_empty(),
        "a stage begun under an unknown outcome is never abandoned"
    );
}

/// Acceptance 3: `stage publish` commits, then answers the unknown outcome. The run exits 4,
/// `stage.json` stays with `publishing` set and `state/` is not put back; the next run settles the
/// publish at the same head and runs.
#[test]
fn an_unknown_outcome_after_a_publish_committed_keeps_the_stage_for_the_next_run() {
    let w = World::new();
    let hook = hook(&w);
    let dir = create(&w, &spec(&w, &unknown_ekr(&w), &hook));
    let before = head(&w, &dir);

    std::fs::write(w.root.join("unknown-after-publish"), "").unwrap();
    let (code, line, all) = run(&w);
    assert_eq!(
        (code, line["outcome"].as_str()),
        (Some(4), Some("store-outcome-unknown")),
        "{all}"
    );
    assert_eq!(stage_record(&dir)["publishing"], true, "{all}");
    assert!(seen_a(&dir), "state/ was put back: {all}");
    let published = head(&w, &dir);
    assert!(published > before, "the stand-in did not publish: {all}");
    assert!(
        stages_in(&w, &dir, "Abandoned").is_empty(),
        "the stage was abandoned"
    );
    assert_eq!(docs_row(&w)["consecutive_failures"], 0);

    let (code, line, all) = run(&w);
    assert_eq!(
        (code, line["outcome"].as_str()),
        (Some(0), Some("ran")),
        "{all}"
    );
    assert!(!dir.join("stage.json").exists(), "{all}");
    assert_eq!(head(&w, &dir), published, "{all}");
    assert_eq!(stages_in(&w, &dir, "Published").len(), 1);
    // a.md stayed seen, so the second run asked the model nothing.
    assert_eq!(model_calls(&w), 1);
}

/// `stage publish` answers the unknown outcome and did not publish: the run exits 4 with
/// `stage.json` kept; the next run publishes the stage at the head it recorded.
#[test]
fn an_unknown_outcome_before_a_publish_landed_is_published_by_the_next_run() {
    let w = World::new();
    let hook = hook(&w);
    let dir = create(&w, &spec(&w, &unknown_ekr(&w), &hook));
    let before = head(&w, &dir);

    std::fs::write(w.root.join("unknown-before-publish"), "").unwrap();
    let (code, line, all) = run(&w);
    assert_eq!(
        (code, line["outcome"].as_str()),
        (Some(4), Some("store-outcome-unknown")),
        "{all}"
    );
    let record = stage_record(&dir);
    assert_eq!(record["publishing"], true, "{record}");
    assert_eq!(head(&w, &dir), before);
    assert!(seen_a(&dir), "state/ was put back: {all}");

    let (code, line, all) = run(&w);
    assert_eq!(
        (code, line["outcome"].as_str()),
        (Some(0), Some("ran")),
        "{all}"
    );
    assert!(!dir.join("stage.json").exists());
    assert!(head(&w, &dir) > before, "{all}");
    let published = stages_in(&w, &dir, "Published");
    assert_eq!(published.len(), 1, "{published:?}");
    assert_eq!(published[0]["stage_id"], record["stage_id"]);
    assert_eq!(model_calls(&w), 1);
}

/// An unknown outcome while the next run settles a stage is the same outcome: not counted, and
/// `stage.json` stays for the run after it.
#[test]
fn an_unknown_outcome_while_settling_a_stage_is_the_same_outcome() {
    let w = World::new();
    let hook = hook(&w);
    let dir = create(&w, &spec(&w, &unknown_ekr(&w), &hook));
    let before = head(&w, &dir);

    std::fs::write(w.root.join("unknown-before-publish"), "").unwrap();
    let (code, _, all) = run(&w);
    assert_eq!(code, Some(4), "{all}");

    std::fs::write(w.root.join("unknown-publish"), "").unwrap();
    let (code, line, all) = run(&w);
    assert_eq!(
        (code, line["outcome"].as_str()),
        (Some(4), Some("store-outcome-unknown")),
        "{all}"
    );
    assert_eq!(stage_record(&dir)["publishing"], true, "{all}");
    let row = docs_row(&w);
    assert_eq!(row["consecutive_failures"], 0, "{row}");
    assert_eq!(row["state"], "Enabled", "{row}");

    std::fs::remove_file(w.root.join("unknown-publish")).unwrap();
    let (code, line, all) = run(&w);
    assert_eq!(
        (code, line["outcome"].as_str()),
        (Some(0), Some("ran")),
        "{all}"
    );
    assert!(head(&w, &dir) > before);
    assert!(!dir.join("stage.json").exists());
}

/// Any other failure of `stage begin` is still `apply-refused`, counted, and the second in a row
/// disables the source.
#[test]
fn any_other_stage_begin_failure_is_still_counted_and_the_second_disables_the_source() {
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
    let (code, _, all) = run(&w);
    assert_eq!(code, Some(1), "{all}");
    let row = docs_row(&w);
    assert_eq!(row["state"], "Disabled", "{row}");
    assert_eq!(
        w.lines("hook.log"),
        ["pg/docs apply-refused", "pg/docs disabled"]
    );
}

/// How many nodes, assertions and evidence records the store holds at its head.
fn held(w: &World, dir: &Path) -> (usize, usize, usize) {
    let graph = store_ekr(w, dir, &["snapshot"]);
    let count = |key: &str| {
        graph["graph"]["graph"][key]
            .as_object()
            .map_or(0, |m| m.len())
    };
    (count("nodes"), count("assertions"), count("evidence"))
}

/// What one run with no fault leaves in the store: the batch, held once.
fn held_once() -> (usize, usize, usize) {
    let w = World::new();
    let hook = hook(&w);
    let dir = create(&w, &spec_held(&w, &unknown_ekr(&w), &hook, false));
    let (code, line, all) = run(&w);
    assert_eq!(
        (code, line["outcome"].as_str()),
        (Some(0), Some("ran")),
        "{all}"
    );
    let held = held(&w, &dir);
    assert!(held.1 > 0, "the run applied nothing: {all}");
    held
}

/// Asserts the run exited 4 as `store-outcome-unknown`, naming EKR's answer, and that the source
/// stays enabled with `consecutive_failures` 0.
fn unknown_not_counted(w: &World, n: usize, (code, line, all): (Option<i32>, Value, String)) {
    assert_eq!(
        (code, line["outcome"].as_str()),
        (Some(4), Some("store-outcome-unknown")),
        "run {n}: {all}"
    );
    let reason = line["detail"]["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("publication outcome is unknown"),
        "run {n}: {reason}"
    );
    let row = docs_row(w);
    assert_eq!(row["state"], "Enabled", "run {n}: {row}");
    assert_eq!(row["consecutive_failures"], 0, "run {n}: {row}");
}

/// `story:apply-outcome-unknown-not-counted`, acceptance 2: on an ungated `postgres` store (no
/// stage), `apply-extraction` commits, then answers the unknown outcome. Two runs with
/// `--record-failure` exit 4 as `store-outcome-unknown` and leave `consecutive_failures` 0; neither
/// marks the batch seen, so the third run, with no fault, extracts it again and marks it seen:
/// nothing is lost. Each run whose apply landed wrote the batch's assertions and evidence, so the
/// store holds one set per such run, as re-extracting a document writes a second set
/// (`cortex adopt`'s help); its nodes are held once.
#[test]
fn an_unknown_apply_outcome_outside_a_stage_is_not_counted_and_the_batch_is_extracted_again() {
    let once = held_once();
    let w = World::new();
    let hook = hook(&w);
    let dir = create(&w, &spec_held(&w, &unknown_ekr(&w), &hook, false));

    std::fs::write(w.root.join("unknown-after-apply"), "").unwrap();
    for n in 1..=2 {
        unknown_not_counted(&w, n, run(&w));
        assert!(!seen_a(&dir), "run {n} marked the unknown batch seen");
        assert!(!dir.join("stage.json").exists(), "run {n}");
    }
    let sets = |k: usize| (once.0, k * once.1, k * once.2);
    assert_eq!(
        held(&w, &dir),
        sets(2),
        "the batch landed under the fault, once per run"
    );
    let systemctl = std::fs::read_to_string(w.root.join("systemctl.log")).unwrap_or_default();
    assert!(!systemctl.contains("disable"), "{systemctl}");
    assert_eq!(
        w.lines("hook.log"),
        [
            "pg/docs store-outcome-unknown",
            "pg/docs store-outcome-unknown"
        ]
    );

    std::fs::remove_file(w.root.join("unknown-after-apply")).unwrap();
    let (code, line, all) = run(&w);
    assert_eq!(
        (code, line["outcome"].as_str()),
        (Some(0), Some("ran")),
        "{all}"
    );
    assert_eq!(
        model_calls(&w),
        3,
        "the third run did not extract the batch again"
    );
    assert!(seen_a(&dir), "{all}");
    assert_eq!(
        held(&w, &dir),
        sets(3),
        "the third run's re-extraction is a further set: {all}"
    );
    assert_eq!(docs_row(&w)["consecutive_failures"], 0);
}

/// Acceptance 3: in a stage (a gated `postgres` run), `apply-extraction` commits, then answers the
/// unknown outcome. The run exits 4 and leaves its stage in `stage.json`, not publishing; the next
/// run abandons that stage, puts `state/` back, extracts the batch again and applies it once.
#[test]
fn an_unknown_apply_outcome_in_a_stage_leaves_the_stage_for_the_next_run() {
    let once = held_once();
    let w = World::new();
    let hook = hook(&w);
    let dir = create(&w, &spec_held(&w, &unknown_ekr(&w), &hook, true));
    let before = head(&w, &dir);

    std::fs::write(w.root.join("unknown-after-apply"), "").unwrap();
    unknown_not_counted(&w, 1, run(&w));
    let record = stage_record(&dir);
    // A record holds `publishing` only once a run set it; absent reads as not publishing
    // (`StageRecord::read`).
    assert_eq!(record["publishing"].as_bool(), None, "{record}");
    assert_eq!(head(&w, &dir), before, "the stage was published");
    assert!(stages_in(&w, &dir, "Abandoned").is_empty(), "abandoned now");
    assert!(!seen_a(&dir));

    std::fs::remove_file(w.root.join("unknown-after-apply")).unwrap();
    let (code, line, all) = run(&w);
    assert_eq!(
        (code, line["outcome"].as_str()),
        (Some(0), Some("ran")),
        "{all}"
    );
    let abandoned = stages_in(&w, &dir, "Abandoned");
    assert_eq!(abandoned.len(), 1, "{abandoned:?}");
    assert_eq!(abandoned[0]["stage_id"], record["stage_id"]);
    assert!(!dir.join("stage.json").exists(), "{all}");
    assert_eq!(model_calls(&w), 2);
    assert!(seen_a(&dir), "{all}");
    assert_eq!(held(&w, &dir), once, "{all}");
}

/// Acceptance 4: `apply-extraction` answers the unknown outcome before committing anything, in a
/// stage and outside one: the run exits 4 uncounted, and the next run applies the batch once.
#[test]
fn an_unknown_apply_outcome_before_committing_is_applied_once_by_the_next_run() {
    let once = held_once();
    for gated in [false, true] {
        let w = World::new();
        let hook = hook(&w);
        let dir = create(&w, &spec_held(&w, &unknown_ekr(&w), &hook, gated));
        let before = held(&w, &dir);

        std::fs::write(w.root.join("unknown-before-apply"), "").unwrap();
        unknown_not_counted(&w, 1, run(&w));
        assert_eq!(held(&w, &dir), before, "gated {gated}");
        assert!(!seen_a(&dir), "gated {gated}");
        assert_eq!(dir.join("stage.json").exists(), gated, "gated {gated}");

        std::fs::remove_file(w.root.join("unknown-before-apply")).unwrap();
        let (code, line, all) = run(&w);
        assert_eq!(
            (code, line["outcome"].as_str()),
            (Some(0), Some("ran")),
            "gated {gated}: {all}"
        );
        assert!(!dir.join("stage.json").exists(), "gated {gated}: {all}");
        assert_eq!(model_calls(&w), 2, "gated {gated}");
        assert_eq!(held(&w, &dir), once, "gated {gated}: {all}");
    }
}
