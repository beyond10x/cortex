//! Adversary pass 2 on unit cost of wave 20261030a (`story:replacement-applied-as-assertion`,
//! correction b6750cf2): the re-applied replacements go in rounds by the time of their evidence.
//! These cases put a plain value and a replacement of one property into one batch, the shape the
//! rounds do not look at: only the facts EKR rejected for nothing to replace are ordered, so a
//! plain value the first apply stored is not among them.

mod common;

use std::path::PathBuf;
use std::process::Command;

use common::{ekr, executable, World};
use serde_json::Value;

/// The stand-in `claude` of `tests/supersession.rs`: `owner is <Name>` is a plain `!Property` fact
/// `owner` of the Widget engine, `owner is now <Name>` the same fact marked `replaces: true`.
const CLAUDE: &str = r##"R=$(cd "$(dirname "$0")/.." && pwd)
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

fn run_with(w: &World, records: &[&str]) -> Value {
    std::fs::write(w.root.join("chat/export.jsonl"), records.join("\n") + "\n").unwrap();
    let (code, ran) = w.cortex(&["run", "owned/chat"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    ran
}

/// `(value, lifecycle)` of every string-valued assertion of the store: the `owner` values.
fn owners(w: &World) -> Vec<(String, String)> {
    let dir: PathBuf = w.home.join("instances/owned");
    let out = Command::new(ekr())
        .arg("snapshot")
        .env("EKR_HOST", dir.join("host.json"))
        .env("EKR_BACKEND", "sqlite")
        .env("EKR_STORE", dir.join("store.sqlite"))
        .output()
        .unwrap();
    assert!(out.status.success());
    let snap: Value = serde_json::from_slice(&out.stdout).unwrap();
    snap["graph"]["graph"]["assertions"]
        .as_object()
        .expect("assertions")
        .values()
        .filter_map(|a| {
            Some((
                a["object"]["Value"]["value"].as_str()?.to_string(),
                a["lifecycle"].to_string(),
            ))
        })
        .collect()
}

fn active(owners: &[(String, String)]) -> Vec<String> {
    owners
        .iter()
        .filter(|(_, l)| l == "\"Active\"")
        .map(|(v, _)| v.clone())
        .collect()
}

/// An empty store and one batch: a, dated 2026-01-02, says the owner is now Alice (a replacement);
/// b, dated 2026-01-03, says the owner is Bob (a plain value). EKR stores Bob and rejects Alice's
/// replacement for nothing to replace; cortex re-applies Alice without `replaces`. A one-cardinality
/// property must end with one active value, Bob's, the newer, as two runs in date order would
/// leave it. Release 0.6.1 dropped Alice and left Bob alone active.
#[test]
fn an_older_replacement_beside_a_newer_plain_value_in_one_batch_leaves_one_value_active() {
    let w = world();
    let ran = run_with(
        &w,
        &[
            r#"{"id":"a","time":"2026-01-02","text":"The owner is now Alice."}"#,
            r#"{"id":"b","time":"2026-01-03","text":"The owner is Bob."}"#,
        ],
    );
    let owners = owners(&w);
    assert_eq!(
        active(&owners),
        ["Bob"],
        "one active owner, Bob: {owners:?}; run: {ran}"
    );
}

/// The same with the newer document first in the export: b, dated 2026-01-03, says the owner is
/// now Bob; a, dated 2026-01-02, says the owner is Alice. One value must stay active, Bob's.
#[test]
fn a_newer_replacement_listed_before_an_older_plain_value_leaves_one_value_active() {
    let w = world();
    let ran = run_with(
        &w,
        &[
            r#"{"id":"b","time":"2026-01-03","text":"The owner is now Bob."}"#,
            r#"{"id":"a","time":"2026-01-02","text":"The owner is Alice."}"#,
        ],
    );
    let owners = owners(&w);
    assert_eq!(
        active(&owners),
        ["Bob"],
        "one active owner, Bob: {owners:?}; run: {ran}"
    );
}

/// Three replacements of one property in one batch, an empty store, listed out of date order:
/// the rounds must end with the newest, Carol, the one active value.
#[test]
fn three_replacements_out_of_order_in_one_batch_leave_the_newest_active() {
    let w = world();
    let ran = run_with(
        &w,
        &[
            r#"{"id":"c","time":"2026-01-04","text":"The owner is now Carol."}"#,
            r#"{"id":"a","time":"2026-01-02","text":"The owner is now Alice."}"#,
            r#"{"id":"b","time":"2026-01-03","text":"The owner is now Bob."}"#,
        ],
    );
    let owners = owners(&w);
    assert_eq!(
        active(&owners),
        ["Carol"],
        "one active owner, Carol: {owners:?}; run: {ran}"
    );
}
