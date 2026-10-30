//! Adversary cases for `story:postgres-run-undo`: a gated run on a `postgres` store runs in an
//! EKR stage. No database is needed: a stand-in `ekr` answers a `postgres` store from a SQLite
//! file through the real `ekr` (stages work on SQLite too), and can kill the `cortex` that started
//! it right after a stage verb, fail a staged `ontology`, or write the store's base beside a staged
//! `apply-extraction`.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{ekr, World};
use serde_json::Value;

/// The stand-in. Hooks are files under `<root>`: `kill-after-begin`, `kill-after-publish` (run
/// the verb, then `kill -9` the parent `cortex`), `kill-before-publish` (`kill -9` the parent
/// `cortex` instead of publishing), `fail-second-ontology` (a staged `ontology` fails
/// from its second call), `write-base-too` (a staged `apply-extraction` is applied to the base as
/// well, as another writer of the store would). Every hook fires once, but the ontology one.
fn staged_pg_ekr(w: &World) -> PathBuf {
    let path = w.bin.join("ekr-pg");
    let body = r#"R="@R@"; EKR="@EKR@"; DB="$R/pg-as-sqlite.sqlite"
printf '%s|%s\n' "${EKR_STAGE:--}" "$*" >> "$R/ekr-argv.log"
if [ "$1" = postgres-schema ]; then echo '{"format":"ekr.postgres-schema/1","ready":true}'; exit 0; fi
if [ "$EKR_BACKEND" = postgres ]; then
  if [ "$1" = head ] && [ ! -e "$DB" ]; then echo "the lineage has no seed" >&2; exit 1; fi
  EKR_BACKEND=sqlite; EKR_STORE="$DB"; export EKR_BACKEND EKR_STORE
fi
case "$1 $2" in
  "stage begin") if [ -e "$R/kill-after-begin" ]; then rm "$R/kill-after-begin"; "$EKR" "$@" > /dev/null; kill -9 $PPID; exit 0; fi ;;
  "stage publish") if [ -e "$R/kill-after-publish" ]; then rm "$R/kill-after-publish"; "$EKR" "$@" > /dev/null; kill -9 $PPID; exit 0; fi
    if [ -e "$R/kill-before-publish" ]; then rm "$R/kill-before-publish"; kill -9 $PPID; exit 1; fi ;;
esac
if [ "$1" = ontology ] && [ -n "$EKR_STAGE" ] && [ -e "$R/fail-second-ontology" ]; then
  n=$(cat "$R/ontology-count" 2>/dev/null || echo 0); n=$((n+1)); echo "$n" > "$R/ontology-count"
  if [ "$n" -ge 2 ]; then echo "ekr: injected ontology failure" >&2; exit 1; fi
fi
if [ "$1" = apply-extraction ] && [ -n "$EKR_STAGE" ] && [ -e "$R/write-base-too" ]; then
  rm "$R/write-base-too"
  "$EKR" "$@" || exit $?
  env -u EKR_STAGE "$EKR" "$@" > /dev/null || exit $?
  env -u EKR_STAGE "$EKR" head > "$R/base-head.json"
  exit 0
fi
exec "$EKR" "$@"
"#
    .replace("@R@", &w.root.display().to_string())
    .replace("@EKR@", &ekr().display().to_string());
    common::executable(&path, &format!("{}{body}", common::EKR_SESSION_SHIM));
    path
}

/// A spec over `docs/` on a direct `postgres` store answered by `bin`, held to `facts_refused
/// max <max>`, with `budget_usd` and `max_chars_per_document` as given.
fn spec(w: &World, bin: &Path, max: &str, budget: &str, max_chars: usize) -> PathBuf {
    let config = w.root.join("pg/app.json");
    let held = w.root.join("pg/password.txt");
    std::fs::create_dir_all(held.parent().unwrap()).unwrap();
    std::fs::write(&held, "adv-not-a-secret").unwrap();
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
    let text = format!(
        r#"format: cortex.instance/1
name: pg
description: Test brain.
ekr: {{version: "0.0.30", bin: "{bin}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "{budget}", timeout_s: 60}}
sources:
  - name: docs
    schedule: daily
    settings:
      kind: files
      value: {{paths: [docs], glob: "**/*.md"}}
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: {max_chars}}}
serve: {{view_port: 18996}}
store: {{backend: postgres, value: {{config: "{config}"}}}}
gate:
  checks:
    - {{measure: facts_refused, max: "{max}"}}
"#,
        bin = bin.display(),
        config = config.display(),
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

fn last_json(stdout: &str) -> Value {
    serde_json::from_str(stdout.lines().last().unwrap_or("null")).unwrap_or(Value::Null)
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

fn stages(w: &World, dir: &Path) -> Vec<Value> {
    store_ekr(w, dir, &["stage", "list"])
        .as_array()
        .cloned()
        .unwrap_or_default()
}

fn docs(w: &World, files: &[(&str, &str)]) {
    for (name, text) in files {
        std::fs::write(w.root.join("docs").join(name), text).unwrap();
    }
}

/// A run killed after `ekr stage begin` answered and before `stage.json` was written (or while
/// `stage begin` copied a large store, its answer never read) leaves a `Begun` stage, a copy of
/// the whole store, that nothing records. `ekr stage list` is how such a stage is found; the next
/// run should settle it as it settles one `stage.json` names.
#[test]
fn adv_a_stage_whose_record_was_never_written_is_settled_by_the_next_run() {
    let w = World::new();
    let bin = staged_pg_ekr(&w);
    let dir = create(&w, &spec(&w, &bin, "1", "1", 5000));
    docs(
        &w,
        &[("a.md", "Example Labs develops the Widget engine.\n")],
    );

    std::fs::write(w.root.join("kill-after-begin"), "").unwrap();
    let (code, out, err) = cortex(&w, &["run", "pg/docs"]);
    assert_eq!(code, None, "the run was not killed: {out}{err}");
    assert!(!dir.join("stage.json").exists());
    let begun: Vec<Value> = stages(&w, &dir)
        .into_iter()
        .filter(|s| s["state"] == "Begun")
        .collect();
    assert_eq!(begun.len(), 1, "{begun:?}");

    let (code, out, err) = cortex(&w, &["run", "pg/docs"]);
    assert_eq!(code, Some(0), "{out}{err}");
    let left: Vec<Value> = stages(&w, &dir)
        .into_iter()
        .filter(|s| s["state"] == "Begun")
        .collect();
    assert!(
        left.is_empty(),
        "a stage a killed run began is never abandoned; it keeps its copy of the store: {left:?}"
    );
}

/// A run killed after `ekr stage publish` and before it removed `stage.json` published its
/// stage: the next run keeps what it published and does not put `state/` back.
#[test]
fn adv_a_run_killed_after_its_publish_keeps_what_it_published() {
    let w = World::new();
    let bin = staged_pg_ekr(&w);
    let dir = create(&w, &spec(&w, &bin, "1", "1", 5000));
    docs(
        &w,
        &[("a.md", "Example Labs develops the Widget engine.\n")],
    );
    let before = head(&w, &dir);

    std::fs::write(w.root.join("kill-after-publish"), "").unwrap();
    let (code, out, err) = cortex(&w, &["run", "pg/docs"]);
    assert_eq!(code, None, "the run was not killed: {out}{err}");
    assert!(dir.join("stage.json").exists(), "nothing left to settle");
    let published = head(&w, &dir);
    assert!(published > before, "the killed run published nothing");

    let (code, out, err) = cortex(&w, &["run", "pg/docs"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert!(!dir.join("stage.json").exists());
    assert_eq!(head(&w, &dir), published);
    let seen: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("state/docs.json")).unwrap())
            .unwrap();
    assert!(
        seen["documents"]
            .as_object()
            .is_some_and(|d| d.keys().any(|k| k.ends_with("/a.md"))),
        "state/ was put back although the stage was published: {seen}"
    );
    // a.md is seen, so the second run asked the model nothing.
    let calls = std::fs::read_to_string(w.root.join("claude-calls.log")).unwrap_or_default();
    assert_eq!(calls.lines().count(), 1, "{calls}");
}

/// The store's head moves while the run's stage is open (another writer of the store): the
/// publish is refused, the run fails as `apply-refused` with nothing of it in the store, and
/// `state/` is back. A publish that named the head read at publish time instead of the head the
/// run began from would pass the Docker case and fail this one.
#[test]
fn adv_a_publish_refused_because_the_head_moved_leaves_nothing_of_the_run() {
    let w = World::new();
    let bin = staged_pg_ekr(&w);
    let dir = create(&w, &spec(&w, &bin, "1", "1", 5000));
    docs(
        &w,
        &[("a.md", "Example Labs develops the Widget engine.\n")],
    );
    let before = head(&w, &dir);

    std::fs::write(w.root.join("write-base-too"), "").unwrap();
    let (code, out, err) = cortex(&w, &["run", "pg/docs"]);
    assert_ne!(code, Some(0), "{out}{err}");
    let ran = last_json(&out);
    assert_eq!(ran["outcome"], "apply-refused", "{out}{err}");
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("stage-head-moved"), "{reason}");
    // The other writer's commits only: the head it left.
    let other: Value =
        serde_json::from_str(&std::fs::read_to_string(w.root.join("base-head.json")).unwrap())
            .unwrap();
    assert!(other["revision"].as_i64().unwrap() > before);
    assert_eq!(Some(head(&w, &dir)), other["revision"].as_i64(), "{reason}");
    assert!(!dir.join("stage.json").exists());
    let seen = std::fs::read_to_string(dir.join("state/docs.json")).unwrap_or_default();
    assert!(
        !seen.contains("a.md"),
        "state/ keeps a.md seen, so it is never fetched again: {seen}"
    );
    assert!(stages(&w, &dir).iter().all(|s| s["state"] != "Published"));
}

/// A run that fails part-way, before its gate, publishes what it applied (operating.md: "keeps
/// what it applied, as on sqlite"). Two batches; the second batch's staged `ontology` fails.
#[test]
fn adv_a_run_that_fails_part_way_publishes_what_it_applied() {
    let w = World::new();
    let bin = staged_pg_ekr(&w);
    let dir = create(&w, &spec(&w, &bin, "1", "1", 50_000));
    // Each document is cut to its evidence payload (16384 bytes): five make two batches of
    // `BATCH_CHARS` 60000.
    let long = "Example Labs develops the Widget engine. ".repeat(1000);
    let names = ["a.md", "b.md", "c.md", "d.md", "e.md"];
    let files: Vec<(&str, String)> = names.iter().map(|n| (*n, format!("{n}: {long}"))).collect();
    let files: Vec<(&str, &str)> = files.iter().map(|(n, t)| (*n, t.as_str())).collect();
    docs(&w, &files);
    let before = head(&w, &dir);

    std::fs::write(w.root.join("fail-second-ontology"), "").unwrap();
    let (code, out, err) = cortex(&w, &["run", "pg/docs"]);
    assert_ne!(code, Some(0), "{out}{err}");
    assert!(
        head(&w, &dir) > before,
        "the first batch was not published: {out}{err}"
    );
    let listed = stages(&w, &dir);
    assert_eq!(listed.len(), 1, "{listed:?}");
    assert_eq!(listed[0]["state"], "Published", "{listed:?}");
    assert!(!dir.join("stage.json").exists());
}

/// A run killed while it publishes its stage, after its gate passed and `stage.json` recorded the
/// publish, before EKR published: the next run publishes that stage at the head it recorded, and
/// keeps `state/`, so what the killed run applied is in the store and not fetched again.
#[test]
fn adv_d_a_publish_cut_short_by_a_kill_is_completed_by_the_next_run() {
    let w = World::new();
    let bin = staged_pg_ekr(&w);
    let dir = create(&w, &spec(&w, &bin, "1", "1", 5000));
    docs(
        &w,
        &[("a.md", "Example Labs develops the Widget engine.\n")],
    );
    let before = head(&w, &dir);

    std::fs::write(w.root.join("kill-before-publish"), "").unwrap();
    let (code, out, err) = cortex(&w, &["run", "pg/docs"]);
    assert_eq!(code, None, "the run was not killed: {out}{err}");
    let record: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("stage.json")).unwrap()).unwrap();
    assert_eq!(record["publishing"], true, "{record}");
    let stage = record["stage_id"].as_str().unwrap().to_string();
    assert_eq!(head(&w, &dir), before, "the killed run published");

    let (code, out, err) = cortex(&w, &["run", "pg/docs"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert!(!dir.join("stage.json").exists());
    let listed = stages(&w, &dir);
    let state = listed
        .iter()
        .find(|s| s["stage_id"] == stage.as_str())
        .map(|s| s["state"].clone());
    assert_eq!(state, Some(Value::from("Published")), "{listed:?}");
    assert!(head(&w, &dir) > before);
    // a.md stayed seen, so the second run asked the model nothing.
    let calls = std::fs::read_to_string(w.root.join("claude-calls.log")).unwrap_or_default();
    assert_eq!(calls.lines().count(), 1, "{calls}");
}

/// A run that fails part-way publishes what it applied; when that publish is refused because the
/// store's head moved, the refusal is in the run's `reason`, not only on stderr, nothing of the
/// run is in the store, and its stage is abandoned.
#[test]
fn adv_e_a_part_way_publish_refusal_is_in_the_reason() {
    let w = World::new();
    let bin = staged_pg_ekr(&w);
    let dir = create(&w, &spec(&w, &bin, "1", "1", 50_000));
    let long = "Example Labs develops the Widget engine. ".repeat(1000);
    let names = ["a.md", "b.md", "c.md", "d.md", "e.md"];
    let files: Vec<(&str, String)> = names.iter().map(|n| (*n, format!("{n}: {long}"))).collect();
    let files: Vec<(&str, &str)> = files.iter().map(|(n, t)| (*n, t.as_str())).collect();
    docs(&w, &files);

    std::fs::write(w.root.join("fail-second-ontology"), "").unwrap();
    std::fs::write(w.root.join("write-base-too"), "").unwrap();
    let (code, out, err) = cortex(&w, &["run", "pg/docs"]);
    assert_ne!(code, Some(0), "{out}{err}");
    let ran = last_json(&out);
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("stage-head-moved"), "{out}{err}");
    let other: Value =
        serde_json::from_str(&std::fs::read_to_string(w.root.join("base-head.json")).unwrap())
            .unwrap();
    assert_eq!(Some(head(&w, &dir)), other["revision"].as_i64(), "{reason}");
    assert!(!dir.join("stage.json").exists());
    let listed = stages(&w, &dir);
    assert!(
        listed.iter().all(|s| s["state"] == "Abandoned"),
        "{listed:?}"
    );
}
