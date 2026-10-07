//! A fact is dated by when its source said it: a document's time is its evidence's `observed_at`,
//! and EKR takes a fact's `valid_time.from` from the earliest `observed_at` it cites
//! (`story:document-time-as-valid-time`).

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::{ekr, World};
use cortex_cli::sources::rfc3339;
use serde_json::Value;

/// 2026-01-02T00:00:00Z and 2026-03-04T00:00:00Z, in milliseconds.
const JAN_2: i64 = 1_767_312_000_000;
const MAR_4: i64 = 1_772_582_400_000;

/// A `files` source over `chat/*.jsonl`, each line one record with id `id` and time `time`.
fn spec(w: &World) -> PathBuf {
    let path = w.root.join("dated.yaml");
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: dated
description: Test brain.
ekr: {{version: "0.0.32", bin: "{ekr}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
  - name: chat
    schedule: daily
    settings:
      kind: files
      value:
        paths: [chat]
        glob: "*.jsonl"
        records: {{format: JsonLines, id: id, time: time, text: ["{{text}}"], filters: []}}
    policy: {{refresh_after_days: 7, change: ContentHash, max_documents_per_run: 50, max_chars_per_document: 5000}}
serve: {{view_port: 18994}}
"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    path
}

/// Creates the instance on `export` and runs its source once: the run report and the run's start
/// and end, in milliseconds.
fn run_on(export: &str) -> (World, Value, i64, i64) {
    let w = World::new();
    std::fs::create_dir_all(w.root.join("chat")).unwrap();
    std::fs::write(w.root.join("chat/export.jsonl"), export).unwrap();
    let path = spec(&w);
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let before = now_ms();
    let (code, ran) = w.cortex(&["run", "dated/chat"]);
    let after = now_ms();
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    (w, ran, before, after)
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

fn snapshot(w: &World) -> Value {
    let dir = w.home.join("instances/dated");
    let out = Command::new(ekr())
        .arg("snapshot")
        .env("EKR_HOST", dir.join("host.json"))
        .env("EKR_BACKEND", "sqlite")
        .env("EKR_STORE", dir.join("store.sqlite"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

/// For each record id (the part of its key after `#`), the `valid_time.from` of every assertion
/// citing that record's evidence, and the evidence's own `observed_at`.
fn dated(w: &World, root: &Path) -> BTreeMap<String, (Vec<i64>, i64)> {
    let snap = snapshot(w);
    let graph = &snap["graph"]["graph"];
    let mut by_record = BTreeMap::new();
    for (id, item) in graph["evidence"].as_object().expect("evidence") {
        let identity = item["source"]
            .as_object()
            .into_iter()
            .flat_map(|s| s.values())
            .find_map(|s| s["identity"].as_str())
            .expect("an identity");
        let prefix = format!("file:{}#", root.join("chat/export.jsonl").display());
        let record = identity
            .strip_prefix(&prefix)
            .unwrap_or_else(|| panic!("{identity} is not a record of the export"))
            .to_string();
        let froms: Vec<i64> = graph["assertions"]
            .as_object()
            .expect("assertions")
            .values()
            .filter(|a| {
                a["evidence"]
                    .as_array()
                    .unwrap()
                    .contains(&Value::from(id.as_str()))
            })
            .map(|a| a["valid_time"]["from"].as_i64().expect("a bounded from"))
            .collect();
        assert!(!froms.is_empty(), "no assertion cites {record}: {graph}");
        let observed = item["observed_at"].as_i64().expect("observed_at");
        by_record.insert(record, (froms, observed));
    }
    by_record
}

/// The acceptance: two records dated 2026-01-02 and 2026-03-04 (one a bare date, one RFC 3339 with
/// an offset) yield facts valid from those instants in `ekr snapshot`, and evidence observed then.
#[test]
fn a_fact_is_valid_from_the_time_its_record_carries() {
    assert_eq!(rfc3339(JAN_2), "2026-01-02T00:00:00Z");
    assert_eq!(rfc3339(MAR_4), "2026-03-04T00:00:00Z");
    let (w, ran, _, _) = run_on(
        r#"{"id":"a","time":"2026-01-02","text":"Example Labs develops the Widget engine."}
{"id":"b","time":"2026-03-04T01:00:00+01:00","text":"Example Labs still develops the Widget engine."}
"#,
    );
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
    assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");
    let dated = dated(&w, &w.root);
    assert_eq!(
        dated,
        BTreeMap::from([
            ("a".to_string(), (vec![JAN_2], JAN_2)),
            ("b".to_string(), (vec![MAR_4], MAR_4)),
        ])
    );
}

/// EKR 0.0.31 writes a `!Relation` fact's edge as well as its assertion, once per pair of nodes:
/// the two records' `DEVELOPS` facts are two assertions, each valid from its own record's time, on
/// one edge.
#[test]
fn two_facts_of_one_relation_are_two_assertions_on_one_edge() {
    let (w, _, _, _) = run_on(
        r#"{"id":"a","time":"2026-01-02","text":"Example Labs develops the Widget engine."}
{"id":"b","time":"2026-03-04","text":"Example Labs still develops the Widget engine."}
"#,
    );
    let snap = snapshot(&w);
    let graph = &snap["graph"]["graph"];
    assert_eq!(
        graph["edges"].as_object().map(|e| e.len()),
        Some(1),
        "{graph}"
    );
    assert_eq!(
        graph["assertions"].as_object().map(|a| a.len()),
        Some(2),
        "{graph}"
    );
}

/// A chat export's epoch-seconds time, fraction and all, is the instant; a time that does not
/// parse, or that lies in the future, falls back to the run's start.
#[test]
fn epoch_seconds_count_and_an_unusable_time_falls_back_to_the_run_start() {
    let (w, _, before, after) = run_on(
        r#"{"id":"epoch","time":"1767312000.250","text":"Example Labs develops the Widget engine."}
{"id":"vague","time":"next tuesday","text":"Example Labs also develops the Widget engine."}
{"id":"future","time":"2999-01-01T00:00:00Z","text":"Example Labs will develop the Widget engine."}
"#,
    );
    let dated = dated(&w, &w.root);
    assert_eq!(dated["epoch"], (vec![JAN_2 + 250], JAN_2 + 250));
    for record in ["vague", "future"] {
        let (froms, observed) = &dated[record];
        assert!(
            (before..=after).contains(observed) && froms == &vec![*observed],
            "{record}: {froms:?} / {observed} not the run start in {before}..={after}"
        );
    }
}
