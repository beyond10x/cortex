//! A changed value supersedes the one a newer document replaces: the model marks a `!Property`
//! fact `replaces: true`, cortex passes it to `ekr apply-extraction`, and EKR supersedes every
//! active assertion of that subject and property from the new one's valid time on
//! (`story:extraction-supersedes`, EKR 0.0.31).
//!
//! Facts are dated by their document's time, so documents can arrive out of order. A replacement
//! older than the value it would supersede is rejected by EKR (`invalid-supersession`), and one
//! with nothing active to replace is rejected too (`replacement-without-active-assertion`). cortex
//! lists either under the run's `rejected`, with the document it cites, and the run goes on.

mod common;

use std::path::PathBuf;
use std::process::Command;

use common::{ekr, executable, World};
use serde_json::Value;

/// 2026-01-02T00:00:00Z and 2026-01-03T00:00:00Z, in milliseconds.
const JAN_2: i64 = 1_767_312_000_000;
const JAN_3: i64 = 1_767_398_400_000;

/// A stand-in `claude` that reads its documents: `owner is <Name>` is a `!Property` fact `owner` of
/// the Widget engine, `owner is now <Name>` the same fact marked `replaces: true`. Every document
/// also gives one `DEVELOPS` relation, so a run shows what applied beside a rejected fact. It
/// records its arguments in `claude-args.log`, one line per call.
const CLAUDE: &str = r##"R=$(cd "$(dirname "$0")/.." && pwd)
printf '%s' "$*" | tr '\n' ' ' >> "$R/claude-args.log"; echo >> "$R/claude-args.log"
facts=$(awk '
BEGIN { p = "{\"node_type\":\"Product\",\"aliases\":[\"Widget engine\"]}";
        o = "{\"node_type\":\"Organization\",\"aliases\":[\"Example Labs\"]}" }
/evidence id / { id = $0; sub(/.*evidence id /, "", id); sub(/ .*/, "", id);
  printf "%s{\"!Relation\":{\"subject\":%s,\"relation\":\"DEVELOPS\",\"object\":%s,\"evidence\":[\"%s\"]}}", sep, o, p, id; sep = "," }
/owner is / { v = $0; r = "";
  if (v ~ /owner is now /) { sub(/.*owner is now /, "", v); r = ",\"replaces\":true" } else { sub(/.*owner is /, "", v) }
  sub(/[^A-Za-z].*/, "", v);
  printf ",{\"!Property\":{\"subject\":%s,\"property\":\"owner\",\"value\":{\"value_kind\":\"String\",\"value\":\"%s\"}%s,\"evidence\":[\"%s\"]}}", p, v, r, id }
')
cat <<EOF
{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{"format":"ekr.extraction-document/1","ontology":{"node_types":[{"name":"Organization","parents":[],"abstract_type":false,"properties":[]},{"name":"Product","parents":[],"abstract_type":false,"properties":[{"name":"owner","value":{"value_kind":"String"},"cardinality":"One","required":false}]}],"edge_types":[{"name":"DEVELOPS","source_types":["Organization"],"target_types":["Product"],"cardinality":"Many","properties":[]}]},"entities":[{"node_type":"Organization","aliases":["Example Labs"]},{"node_type":"Product","aliases":["Widget engine"]}],"facts":[$facts]}}
EOF
"##;

/// An instance `owned` with a `files` source over `chat/*.jsonl`, each line one record with id
/// `id` and time `time`, and the reading stand-in `claude`.
fn world() -> World {
    let w = World::new();
    executable(&w.bin.join("claude"), CLAUDE);
    std::fs::create_dir_all(w.root.join("chat")).unwrap();
    let path = w.root.join("owned.yaml");
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: owned
description: Test brain.
ekr: {{version: "0.0.31", bin: "{ekr}"}}
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
serve: {{view_port: 18993}}
"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    w
}

/// Writes `records` as the export, then runs the source once: its report.
fn run_with(w: &World, records: &[&str]) -> Value {
    std::fs::write(w.root.join("chat/export.jsonl"), records.join("\n") + "\n").unwrap();
    let (code, ran) = w.cortex(&["run", "owned/chat"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    ran
}

fn snapshot(w: &World) -> Value {
    let dir: PathBuf = w.home.join("instances/owned");
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

/// The record id (after `#`) each evidence id of the store was issued for.
fn records(graph: &Value) -> std::collections::BTreeMap<String, String> {
    graph["evidence"]
        .as_object()
        .expect("evidence")
        .iter()
        .map(|(id, item)| {
            let identity = item["source"]
                .as_object()
                .into_iter()
                .flat_map(|s| s.values())
                .find_map(|s| s["identity"].as_str())
                .expect("an identity");
            let record = identity.rsplit('#').next().unwrap().to_string();
            (id.clone(), record)
        })
        .collect()
}

/// One `owner` assertion as the test reads it.
#[derive(Debug)]
struct Owner {
    id: String,
    value: String,
    records: Vec<String>,
    from: Option<i64>,
    to: Option<i64>,
    lifecycle: Value,
}

/// Every property assertion of the store whose value is a string: the `owner` assertions.
fn owners(graph: &Value) -> Vec<Owner> {
    let records = records(graph);
    graph["assertions"]
        .as_object()
        .expect("assertions")
        .iter()
        .filter_map(|(id, a)| {
            let value = a["object"]["Value"]["value"].as_str()?.to_string();
            Some(Owner {
                id: id.clone(),
                value,
                records: a["evidence"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|e| records[e.as_str().unwrap()].clone())
                    .collect(),
                from: a["valid_time"]["from"].as_i64(),
                to: a["valid_time"]["to"].as_i64(),
                lifecycle: a["lifecycle"].clone(),
            })
        })
        .collect()
}

fn active(o: &Owner) -> bool {
    o.lifecycle == "Active"
}

/// The record ids the `DEVELOPS` assertions cite: what applied beside an `owner` fact.
fn developed_by(graph: &Value) -> Vec<String> {
    let records = records(graph);
    let mut out: Vec<String> = graph["assertions"]
        .as_object()
        .expect("assertions")
        .values()
        .filter(|a| a["object"].get("Node").is_some())
        .flat_map(|a| a["evidence"].as_array().unwrap().clone())
        .map(|e| records[e.as_str().unwrap()].clone())
        .collect();
    out.sort();
    out
}

fn refusals(ran: &Value) -> Vec<(String, String)> {
    ran["detail"]["rejected"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| {
            (
                r["document"]
                    .as_str()
                    .unwrap()
                    .rsplit('#')
                    .next()
                    .unwrap()
                    .to_string(),
                r["refusal"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

/// The acceptance: two documents a day apart give one property two values; after both runs
/// exactly one assertion is active (the newer), and the older is superseded by it, from the newer
/// document's time on, the newer citing the newer document.
#[test]
fn a_newer_document_supersedes_the_value_it_replaces() {
    let w = world();
    let first = run_with(
        &w,
        &[r#"{"id":"a","time":"2026-01-02","text":"The owner is Alice."}"#],
    );
    assert_eq!(first["detail"]["parts_rejected"], 0, "{first}");
    let second = run_with(
        &w,
        &[
            r#"{"id":"a","time":"2026-01-02","text":"The owner is Alice."}"#,
            r#"{"id":"b","time":"2026-01-03","text":"The owner is now Bob."}"#,
        ],
    );
    assert_eq!(second["detail"]["documents_applied"], 1, "{second}");
    assert_eq!(second["detail"]["parts_rejected"], 0, "{second}");

    let snap = snapshot(&w);
    let graph = &snap["graph"]["graph"];
    let owners = owners(graph);
    assert_eq!(owners.len(), 2, "{owners:?}");
    let now: Vec<&Owner> = owners.iter().filter(|o| active(o)).collect();
    assert_eq!(now.len(), 1, "exactly one active owner: {owners:?}");
    let bob = now[0];
    assert_eq!(
        (bob.value.as_str(), &bob.records, bob.from, bob.to),
        ("Bob", &vec!["b".to_string()], Some(JAN_3), None),
        "{owners:?}"
    );
    let alice = owners
        .iter()
        .find(|o| !active(o))
        .expect("a superseded owner");
    assert_eq!(
        (alice.value.as_str(), &alice.records, alice.from, alice.to),
        ("Alice", &vec!["a".to_string()], Some(JAN_2), Some(JAN_3)),
        "{owners:?}"
    );
    assert!(
        alice.lifecycle.to_string().contains(&bob.id),
        "Alice's assertion is superseded by Bob's: {owners:?}"
    );

    // The answer schema the model is held to is EKR's own, `replaces` and all.
    let args = w.lines("claude-args.log");
    assert_eq!(args.len(), 2);
    assert!(
        args.iter()
            .all(|a| a.contains(r#""replaces":{"type":"boolean"}"#)),
        "{args:?}"
    );
}

/// Out of order: a replacement dated before the value it would supersede is rejected by EKR as
/// `invalid-supersession`. The run lists it with its document and goes on: the newer value stays
/// the one active, and the rest of the older document applies.
#[test]
fn a_replacement_older_than_the_value_it_replaces_is_listed_and_the_run_goes_on() {
    let w = world();
    run_with(
        &w,
        &[r#"{"id":"b","time":"2026-01-03","text":"The owner is Bob."}"#],
    );
    let ran = run_with(
        &w,
        &[
            r#"{"id":"b","time":"2026-01-03","text":"The owner is Bob."}"#,
            r#"{"id":"a","time":"2026-01-02","text":"The owner is now Alice."}"#,
        ],
    );
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    assert_eq!(ran["detail"]["parts_rejected"], 1, "{ran}");
    let refusals = refusals(&ran);
    assert_eq!(refusals.len(), 1, "{ran}");
    assert_eq!(refusals[0].0, "a", "{ran}");
    assert!(refusals[0].1.contains("invalid-supersession"), "{ran}");

    let snap = snapshot(&w);
    let graph = &snap["graph"]["graph"];
    let owners = owners(graph);
    assert_eq!(owners.len(), 1, "{owners:?}");
    assert!(active(&owners[0]) && owners[0].value == "Bob", "{owners:?}");
    assert_eq!(developed_by(graph), ["a", "b"]);
}

/// A replacement with nothing active to replace is rejected by EKR as
/// `replacement-without-active-assertion`, for that fact alone: the run lists it and the rest of
/// the document applies.
#[test]
fn a_replacement_with_nothing_to_replace_is_listed_and_the_run_goes_on() {
    let w = world();
    let ran = run_with(
        &w,
        &[r#"{"id":"a","time":"2026-01-02","text":"The owner is now Alice."}"#],
    );
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    assert_eq!(ran["detail"]["parts_rejected"], 1, "{ran}");
    let refusals = refusals(&ran);
    assert_eq!(refusals.len(), 1, "{ran}");
    assert_eq!(refusals[0].0, "a", "{ran}");
    assert!(
        refusals[0]
            .1
            .contains("replacement-without-active-assertion"),
        "{ran}"
    );

    let snap = snapshot(&w);
    let graph = &snap["graph"]["graph"];
    assert!(owners(graph).is_empty());
    assert_eq!(developed_by(graph), ["a"]);
}
