//! `story:apply-out-of-order-and-isolated`, with the real `ekr` (`CORTEX_TEST_EKR`) and a
//! stand-in model, on synthetic records.
//!
//! 1. A record read after a newer one, whose fact replaces a value, is older than the value the
//!    store holds: EKR refuses that replacement (`invalid-supersession`). cortex keeps the older
//!    value as history of the current one, closed where the current one begins, and refuses
//!    nothing.
//! 2. One record whose extraction redeclares a type the store holds otherwise
//!    (`extraction-type-conflict`) is refused alone, named in the run's `rejected` and not marked
//!    seen; the rest of its batch applies.

use std::path::PathBuf;
use std::process::Command;

use crate::common::{ekr, executable, World};
use serde_json::Value;

/// 2026-01-15, 2026-02-15 and 2026-03-15, 00:00 UTC, in milliseconds.
const JAN: i64 = 1_768_435_200_000;
const FEB: i64 = 1_771_113_600_000;
const MAR: i64 = 1_773_532_800_000;

/// A stand-in `claude` that reads its documents. `owner is <Name>` is a `!Property` fact `owner`
/// of the Widget engine, `owner is now <Name>` the same fact marked `replaces: true`; every
/// document gives one `DEVELOPS` relation; a line with ` owns ` gives an `OWNS` relation, and
/// the answer then declares `OWNS` with cardinality `Many`, or `One` when a document says
/// `owns it alone`. It records each call in `claude-args.log`.
const CLAUDE: &str = r##"R=$(cd "$(dirname "$0")/.." && pwd)
printf '%s' "$*" | tr '\n' ' ' >> "$R/claude-args.log"; echo >> "$R/claude-args.log"
input=$(cat)
facts=$(printf '%s\n' "$input" | awk '
BEGIN { p = "{\"node_type\":\"Product\",\"aliases\":[\"Widget engine\"]}";
        o = "{\"node_type\":\"Organization\",\"aliases\":[\"Example Labs\"]}" }
/evidence id / { id = $0; sub(/.*evidence id /, "", id); sub(/ .*/, "", id);
  printf "%s{\"!Relation\":{\"subject\":%s,\"relation\":\"DEVELOPS\",\"object\":%s,\"evidence\":[\"%s\"]}}", sep, o, p, id; sep = "," }
/ owns / { printf ",{\"!Relation\":{\"subject\":%s,\"relation\":\"OWNS\",\"object\":%s,\"evidence\":[\"%s\"]}}", o, p, id }
/owner is / { v = $0; r = "";
  if (v ~ /owner is now /) { sub(/.*owner is now /, "", v); r = ",\"replaces\":true" } else { sub(/.*owner is /, "", v) }
  sub(/[^A-Za-z].*/, "", v);
  printf ",{\"!Property\":{\"subject\":%s,\"property\":\"owner\",\"value\":{\"value_kind\":\"String\",\"value\":\"%s\"}%s,\"evidence\":[\"%s\"]}}", p, v, r, id }
')
owns=""
case "$input" in
  *" owns "*)
    card=Many
    case "$input" in *"owns it alone"*) card=One ;; esac
    owns=",{\"name\":\"OWNS\",\"source_types\":[\"Organization\"],\"target_types\":[\"Product\"],\"cardinality\":\"$card\",\"properties\":[]}" ;;
esac
cat <<EOF
{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{"format":"ekr.extraction-document/1","ontology":{"node_types":[{"name":"Organization","parents":[],"abstract_type":false,"properties":[]},{"name":"Product","parents":[],"abstract_type":false,"properties":[{"name":"owner","value":{"value_kind":"String"},"cardinality":"One","required":false}]}],"edge_types":[{"name":"DEVELOPS","source_types":["Organization"],"target_types":["Product"],"cardinality":"Many","properties":[]}$owns]},"entities":[{"node_type":"Organization","aliases":["Example Labs"]},{"node_type":"Product","aliases":["Widget engine"]}],"facts":[$facts]}}
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
ekr: {{version: "0.0.43", bin: "{ekr}"}}
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

fn graph(w: &World) -> Value {
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
    let snap: Value = serde_json::from_slice(&out.stdout).unwrap();
    snap["graph"]["graph"].clone()
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
            (id.clone(), identity.rsplit('#').next().unwrap().to_string())
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

impl Owner {
    fn active(&self) -> bool {
        self.lifecycle == "Active"
    }

    /// `(value, records, from, to, active)`.
    fn seen(&self) -> (&str, Vec<&str>, Option<i64>, Option<i64>, bool) {
        (
            self.value.as_str(),
            self.records.iter().map(String::as_str).collect(),
            self.from,
            self.to,
            self.active(),
        )
    }

    fn superseded_by(&self, other: &Owner) -> bool {
        self.lifecycle.to_string().contains(&other.id)
    }
}

/// Every `owner` assertion, by value.
fn owners(graph: &Value) -> std::collections::BTreeMap<String, Owner> {
    let records = records(graph);
    graph["assertions"]
        .as_object()
        .expect("assertions")
        .iter()
        .filter_map(|(id, a)| {
            let value = a["object"]["Value"]["value"].as_str()?.to_string();
            Some((
                value.clone(),
                Owner {
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
                },
            ))
        })
        .collect()
}

/// The record ids the relation assertions cite: what applied of each record.
fn related(graph: &Value) -> Vec<String> {
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
    out.dedup();
    out
}

/// `(record id, refusal)` of each entry of the run's `rejected`.
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

fn last_log_line(w: &World) -> Value {
    let text = std::fs::read_to_string(w.home.join("instances/owned/cortex.log")).unwrap();
    serde_json::from_str(text.lines().last().expect("a log line")).unwrap()
}

const BOB_FEB: &str = r#"{"id":"b","time":"2026-02-15","text":"The owner is now Bob."}"#;
const ALICE_JAN: &str = r#"{"id":"a","time":"2026-01-15","text":"The owner is now Alice."}"#;

/// Acceptance, first half: a record dated January applied after one dated February leaves
/// February current and January as its history, closed where February begins, with zero
/// refusals.
#[test]
fn a_january_record_applied_after_february_becomes_its_history() {
    let w = world();
    let first = run_with(&w, &[BOB_FEB]);
    assert_eq!(first["detail"]["facts_history"], 0, "{first}");

    let ran = run_with(&w, &[BOB_FEB, ALICE_JAN]);
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");
    assert_eq!(ran["detail"]["facts_history"], 1, "{ran}");
    assert!(refusals(&ran).is_empty(), "{ran}");
    assert_eq!(last_log_line(&w)["facts_history"], 1);

    let graph = graph(&w);
    let owners = owners(&graph);
    assert_eq!(owners.len(), 2, "{owners:?}");
    let (bob, alice) = (&owners["Bob"], &owners["Alice"]);
    assert_eq!(bob.seen(), ("Bob", vec!["b"], Some(FEB), None, true));
    assert_eq!(
        alice.seen(),
        ("Alice", vec!["a"], Some(JAN), Some(FEB), false)
    );
    assert!(alice.superseded_by(bob), "{owners:?}");
    // The rest of the older record applied as well.
    assert_eq!(related(&graph), ["a", "b"]);

    // Read again unchanged, nothing is asked or changed.
    let again = run_with(&w, &[BOB_FEB, ALICE_JAN]);
    assert_eq!(again["detail"]["documents_new"], 0, "{again}");
}

/// Two older records in one batch: each becomes history, January closed where February begins
/// and February where the current March value begins, in one transaction.
#[test]
fn two_older_records_in_one_batch_close_one_into_the_other_and_into_the_current() {
    let w = world();
    run_with(
        &w,
        &[r#"{"id":"m","time":"2026-03-15","text":"The owner is now Mallory."}"#],
    );
    let ran = run_with(
        &w,
        &[
            r#"{"id":"m","time":"2026-03-15","text":"The owner is now Mallory."}"#,
            ALICE_JAN,
            r#"{"id":"c","time":"2026-02-15","text":"The owner is now Carol."}"#,
        ],
    );
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
    assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");
    assert_eq!(ran["detail"]["facts_history"], 2, "{ran}");
    assert!(refusals(&ran).is_empty(), "{ran}");

    let owners = owners(&graph(&w));
    let (m, c, a) = (&owners["Mallory"], &owners["Carol"], &owners["Alice"]);
    assert_eq!(m.seen(), ("Mallory", vec!["m"], Some(MAR), None, true));
    assert_eq!(c.seen(), ("Carol", vec!["c"], Some(FEB), Some(MAR), false));
    assert_eq!(a.seen(), ("Alice", vec!["a"], Some(JAN), Some(FEB), false));
    assert!(c.superseded_by(m) && a.superseded_by(c), "{owners:?}");
}

/// Acceptance, second half: in a batch where one record's extraction redeclares an edge type
/// the store holds with another cardinality, every other record applies, and that one is
/// reported refused with `extraction-type-conflict`, not marked seen, so a later run asks it
/// again.
#[test]
fn one_record_redeclaring_an_edge_type_is_refused_alone() {
    let w = world();
    let owns = r#"{"id":"o","time":"2026-01-15","text":"Example Labs owns Widget engine."}"#;
    let first = run_with(&w, &[owns]);
    assert_eq!(first["detail"]["documents_applied"], 1, "{first}");

    let alone =
        r#"{"id":"x","time":"2026-02-15","text":"Example Labs owns it alone: Widget engine."}"#;
    let plain = r#"{"id":"p","time":"2026-02-15","text":"The owner is Bob."}"#;
    let ran = run_with(&w, &[owns, alone, plain]);
    assert_eq!(ran["detail"]["documents_new"], 2, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    let refused = refusals(&ran);
    assert_eq!(refused.len(), 1, "{ran}");
    assert_eq!(refused[0].0, "x", "{ran}");
    assert_eq!(refused[0].1, "extraction-type-conflict: OWNS", "{ran}");

    let graph = graph(&w);
    assert_eq!(related(&graph), ["o", "p"], "x applied nothing");
    let owners = owners(&graph);
    assert_eq!(
        owners["Bob"].seen(),
        ("Bob", vec!["p"], Some(FEB), None, true)
    );

    // Not marked seen: the next run asks it again, and refuses it again on its own.
    let again = run_with(&w, &[owns, alone, plain]);
    assert_eq!(again["detail"]["documents_new"], 1, "{again}");
    assert_eq!(again["detail"]["documents_applied"], 0, "{again}");
    assert_eq!(refusals(&again).len(), 1, "{again}");
    assert_eq!(w.lines("claude-args.log").len(), 3);
}
