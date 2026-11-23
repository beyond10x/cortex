//! `story:supersession-refusals-after-history`, with the real `ekr` (`CORTEX_TEST_EKR`) and a
//! stand-in model, on synthetic records.
//!
//! A replacement with nothing active to replace is applied again by cortex, the values of one
//! subject and property in time order. EKR dates a fact from the earliest record it cites. A fact
//! that cites an early and a late record was ordered by its late record, after a value it is older
//! than, and EKR refused it in the second round with `invalid-supersession`, which the history step
//! of 0.8.0 did not see. Every fact now ends current or kept as history, with no refusal.

mod common;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

use common::{ekr, executable, World};
use serde_json::Value;

/// 2026-01-15, 2026-02-15, 2026-03-15 and 2026-04-15, 00:00 UTC, in milliseconds.
const JAN: i64 = 1_768_435_200_000;
const FEB: i64 = 1_771_113_600_000;
const MAR: i64 = 1_773_532_800_000;
const APR: i64 = 1_776_211_200_000;

/// A stand-in `claude` that reads its documents. A line `owner <Name> for <alias>[|<alias>]` is
/// a `!Property` fact `owner` of the Product named by those aliases, marked `replaces: true`; a
/// second line of the same name and aliases cites its record in that same fact. It records each
/// call in `claude-args.log`.
const CLAUDE: &str = r##"R=$(cd "$(dirname "$0")/.." && pwd)
printf '%s' "$*" | tr '\n' ' ' >> "$R/claude-args.log"; echo >> "$R/claude-args.log"
input=$(cat)
facts=$(printf '%s\n' "$input" | awk '
/evidence id / { id = $0; sub(/.*evidence id /, "", id); sub(/ .*/, "", id) }
/owner [A-Za-z]+ for / {
  line = $0; sub(/.*owner /, "", line)
  v = line; sub(/ for .*/, "", v)
  s = line; sub(/.* for /, "", s); sub(/[.].*$/, "", s)
  k = v "/" s
  if (!(k in ev)) { order[n++] = k; val[k] = v; subj[k] = s; ev[k] = "\"" id "\"" }
  else { ev[k] = ev[k] ",\"" id "\"" }
}
END {
  for (i = 0; i < n; i++) {
    k = order[i]; m = split(subj[k], al, "|"); a = ""
    for (j = 1; j <= m; j++) { a = a (j > 1 ? "," : "") "\"" al[j] "\"" }
    printf "%s{\"!Property\":{\"subject\":{\"node_type\":\"Product\",\"aliases\":[%s]},\"property\":\"owner\",\"value\":{\"value_kind\":\"String\",\"value\":\"%s\"},\"replaces\":true,\"evidence\":[%s]}}", (i ? "," : ""), a, val[k], ev[k]
  }
}
')
cat <<EOF
{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{"format":"ekr.extraction-document/1","ontology":{"node_types":[{"name":"Product","parents":[],"abstract_type":false,"properties":[{"name":"owner","value":{"value_kind":"String"},"cardinality":"One","required":false}]}],"edge_types":[]},"entities":[{"node_type":"Product","aliases":["Widget engine"]}],"facts":[$facts]}}
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
ekr: {{version: "0.0.34", bin: "{ekr}"}}
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
serve: {{view_port: 18995}}
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

/// One `owner` assertion as the test reads it: `(records, from, to, active)`, and its id and
/// lifecycle.
#[derive(Debug)]
struct Owner {
    id: String,
    records: Vec<String>,
    from: Option<i64>,
    to: Option<i64>,
    lifecycle: Value,
}

impl Owner {
    fn seen(&self) -> (Vec<&str>, Option<i64>, Option<i64>, bool) {
        (
            self.records.iter().map(String::as_str).collect(),
            self.from,
            self.to,
            self.lifecycle == "Active",
        )
    }

    fn superseded_by(&self, other: &Owner) -> bool {
        self.lifecycle.to_string().contains(&other.id)
    }
}

/// Every `owner` assertion, by value.
fn owners(graph: &Value) -> BTreeMap<String, Owner> {
    let records: BTreeMap<String, String> = graph["evidence"]
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
        .collect();
    graph["assertions"]
        .as_object()
        .expect("assertions")
        .iter()
        .filter_map(|(id, a)| {
            let value = a["object"]["Value"]["value"].as_str()?.to_string();
            let mut cited: Vec<String> = a["evidence"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| records[e.as_str().unwrap()].clone())
                .collect();
            cited.sort();
            Some((
                value,
                Owner {
                    id: id.clone(),
                    records: cited,
                    from: a["valid_time"]["from"].as_i64(),
                    to: a["valid_time"]["to"].as_i64(),
                    lifecycle: a["lifecycle"].clone(),
                },
            ))
        })
        .collect()
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

fn record(id: &str, time: &str, text: &str) -> String {
    format!(r#"{{"id":"{id}","time":"{time}","text":"{text}"}}"#)
}

/// Acceptance, the runbook consumer's shape: one batch, nothing stored yet. Alice's fact cites a
/// January and a March record, Bob's a February one. EKR dates Alice from January, so she goes
/// first as a plain value and Bob replaces her from February; before the fix Alice was ordered by
/// March, after Bob, and refused `invalid-supersession` with both her records.
#[test]
fn a_fact_citing_an_early_and_a_late_record_is_ordered_by_its_earliest() {
    let w = world();
    let a = record("a", "2026-01-15", "The owner Alice for Widget engine.");
    let b = record("b", "2026-02-15", "The owner Bob for Widget engine.");
    let c = record("c", "2026-03-15", "The owner Alice for Widget engine.");
    let ran = run_with(&w, &[&a, &b, &c]);
    assert!(refusals(&ran).is_empty(), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 3, "{ran}");
    assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");

    let owners = owners(&graph(&w));
    assert_eq!(owners.len(), 2, "{owners:?}");
    let (alice, bob) = (&owners["Alice"], &owners["Bob"]);
    assert_eq!(bob.seen(), (vec!["b"], Some(FEB), None, true));
    assert_eq!(alice.seen(), (vec!["a", "c"], Some(JAN), Some(FEB), false));
    assert!(alice.superseded_by(bob), "{owners:?}");
}

/// A second-round refusal the time order cannot prevent: two alias lists of one Product, which
/// EKR resolves to one node and cortex keeps apart. Their later values go in one round, Bob
/// (April) before Dave (March), and EKR refuses Dave's replacement of Bob with
/// `invalid-supersession`. That refusal is kept as history too: Dave holds from March until Bob
/// begins, Bob stays current, and nothing is refused.
#[test]
fn a_second_round_replacement_older_than_the_current_value_is_kept_as_history() {
    let w = world();
    let a = record("a", "2026-01-15", "The owner Carol for Widget engine|WE.");
    let b = record("b", "2026-02-15", "The owner Alice for Widget engine.");
    let c = record("c", "2026-03-15", "The owner Dave for Widget engine.");
    let d = record("d", "2026-04-15", "The owner Bob for Widget engine|WE.");
    let ran = run_with(&w, &[&a, &b, &c, &d]);
    assert!(refusals(&ran).is_empty(), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 4, "{ran}");
    assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");
    assert_eq!(ran["detail"]["facts_history"], 1, "{ran}");

    let owners = owners(&graph(&w));
    assert_eq!(owners.len(), 4, "{owners:?}");
    let (bob, dave) = (&owners["Bob"], &owners["Dave"]);
    assert_eq!(bob.seen(), (vec!["d"], Some(APR), None, true));
    assert_eq!(dave.seen(), (vec!["c"], Some(MAR), Some(APR), false));
    assert!(dave.superseded_by(bob), "{owners:?}");
    let current: Vec<&str> = owners
        .iter()
        .filter(|(_, o)| o.lifecycle == "Active")
        .map(|(v, _)| v.as_str())
        .collect();
    assert_eq!(current, ["Bob"], "{owners:?}");
}
