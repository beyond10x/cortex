//! `story:schema-proposals-on-approval`: `cortex schema <instance> --review` records every
//! proposal `pending` and applies none; `cortex schema approve` and `reject` decide each one once;
//! `cortex schema apply` applies exactly the approved ones, re-planned against the ontology at the
//! head, after a snapshot on a `sqlite` store, and asks no model. The PostgreSQL stage and its gate
//! are in `tests/store_backend.rs`.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{executable, World};
use serde_json::{json, Value};

/// An extraction stand-in: three products, each with its own `version`, all citing the first
/// evidence id of the batch.
fn extract_products(w: &World) {
    executable(
        &w.bin.join("claude"),
        r#"id=$(grep -oE 'evidence id [0-9a-f-]{36}' | head -1 | cut -d' ' -f3)
f() { printf '{"!Property":{"subject":{"node_type":"Product","aliases":["Product %s"]},"property":"version","value":{"value_kind":"String","value":"1.%s"},"evidence":["%s"]}}' "$1" "$1" "$id"; }
e() { printf '{"node_type":"Product","aliases":["Product %s"]}' "$1"; }
cat <<EOF
{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{"format":"ekr.extraction-document/1","ontology":{"node_types":[{"name":"Product","parents":[],"abstract_type":false,"properties":[{"name":"version","value":{"value_kind":"String"},"cardinality":"One","required":false}]}],"edge_types":[]},"entities":[$(e 1),$(e 2),$(e 3)],"facts":[$(f 1),$(f 2),$(f 3)]}}
EOF
"#,
    );
}

/// The proposing model's stand-in: counts its calls in `schema-calls.log` and answers
/// `proposals.json`, with `FACT0` the first fact id of its prompt.
fn proposer(w: &World) {
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
echo call >> "$R/schema-calls.log"
prompt=$(cat)
fact=$(printf '%s\n' "$prompt" | grep -oE '^=== Fact [0-9a-f-]{{36}}' | head -1 | cut -d' ' -f3)
proposals=$(sed -e "s/FACT0/$fact/g" "$R/proposals.json")
echo "{{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"total_cost_usd\":0.02,\"structured_output\":{{\"proposals\":$proposals}}}}"
"#,
            root = w.root.display()
        ),
    );
}

fn answer(w: &World, proposals: Value) {
    std::fs::write(w.root.join("proposals.json"), proposals.to_string()).unwrap();
}

fn calls(w: &World) -> usize {
    w.lines("schema-calls.log").len()
}

fn node_type(name: &str) -> Value {
    json!({"change": {"kind": "add_node_type", "value": {"name": name, "properties": [
            {"name": "country", "value_kind": "String", "cardinality": "One"}]}},
         "reason": format!("the evidence names a {name}"), "facts": ["FACT0"], "evidence": ["E1"]})
}

/// An instance `name` whose store holds three product facts, with the proposer in place of the
/// extraction stand-in.
fn instance(name: &str) -> World {
    let w = World::new();
    let path = w.spec(name, "conn_test");
    extract_products(&w);
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let (code, ran) = w.cortex(&["run", &format!("{name}/news")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    proposer(&w);
    w
}

fn ekr(w: &World, name: &str, args: &[&str]) -> Value {
    let dir = w.home.join("instances").join(name);
    let out = Command::new(common::ekr())
        .args(args)
        .env("EKR_HOST", dir.join("host.json"))
        .env("EKR_BACKEND", "sqlite")
        .env("EKR_STORE", dir.join("store.sqlite"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "ekr {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

fn type_names(ontology: &Value) -> Vec<String> {
    ["node_types", "edge_types"]
        .iter()
        .flat_map(|k| ontology[*k].as_array().cloned().unwrap_or_default())
        .filter_map(|t| t["name"].as_str().map(str::to_string))
        .collect()
}

fn run_dir(w: &World, name: &str, stamp: &str) -> PathBuf {
    w.home
        .join("instances")
        .join(name)
        .join("schema")
        .join(stamp)
}

fn lines(dir: &Path) -> Vec<Value> {
    std::fs::read_to_string(dir.join("proposals.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn statuses(dir: &Path) -> Vec<String> {
    lines(dir)
        .iter()
        .map(|l| l["status"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// `cortex schema <name> --review` with `proposals`: the run's stamp.
fn review(w: &World, name: &str, proposals: Value) -> String {
    answer(w, proposals);
    let (code, out) = w.cortex(&["schema", name, "--sample", "5", "--review"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("reviewed")),
        "{out}"
    );
    out["detail"]["stamp"].as_str().unwrap().to_string()
}

/// Acceptance 2: a review run applies nothing, so the schema version and the head stay, and records
/// both proposals `pending`.
#[test]
fn a_review_run_applies_nothing_and_records_every_proposal_pending() {
    let w = instance("rv");
    let head = w.head("rv");
    let ontology = ekr(&w, "rv", &["ontology"]);
    answer(&w, json!([node_type("Maker"), node_type("Vendor")]));
    let (code, out) = w.cortex(&["schema", "rv", "--sample", "5", "--review"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("reviewed")),
        "{out}"
    );
    let d = &out["detail"];
    assert_eq!(
        [&d["proposed"], &d["pending"], &d["applied"], &d["refused"]],
        [&json!(2), &json!(2), &json!(0), &json!(0)],
        "{out}"
    );
    assert_eq!(w.head("rv"), head, "a review run commits nothing");
    let now = ekr(&w, "rv", &["ontology"]);
    assert_eq!(
        now["version"], ontology["version"],
        "the schema version moved"
    );
    assert_eq!(now, ontology);
    let stamp = d["stamp"].as_str().unwrap();
    assert_eq!(statuses(&run_dir(&w, "rv", stamp)), ["pending", "pending"]);
}

/// Acceptance 3: `approve --only 1`, then `apply`, applies the first and leaves the second
/// `pending`, and neither verb asks the model.
#[test]
fn approving_the_first_and_applying_commits_only_it_without_asking_the_model() {
    let w = instance("ap");
    let stamp = review(&w, "ap", json!([node_type("Maker"), node_type("Vendor")]));
    let head = w.head("ap");
    let asked = calls(&w);

    let (code, out) = w.cortex(&[
        "schema", "approve", "ap", &stamp, "--only", "1", "--by", "ops",
    ]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("approved")),
        "{out}"
    );
    assert_eq!(
        (&out["detail"]["decided"], &out["detail"]["pending"]),
        (&json!(1), &json!(1)),
        "{out}"
    );
    let dir = run_dir(&w, "ap", &stamp);
    assert_eq!(statuses(&dir), ["approved", "pending"]);
    assert_eq!(lines(&dir)[0]["decided_by"], "ops");
    assert_eq!(w.head("ap"), head, "approving commits nothing");

    let (code, out) = w.cortex(&["schema", "apply", "ap", &stamp]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("applied")),
        "{out}"
    );
    assert_eq!(out["detail"]["applied"], 1, "{out}");
    assert_eq!(w.head("ap"), head + 1, "one schema transaction");
    assert_eq!(out["detail"]["revision"], head + 1, "{out}");
    let applied = lines(&dir);
    assert_eq!(statuses(&dir), ["applied", "pending"]);
    assert_eq!(applied[0]["revision"], head + 1, "{applied:?}");
    assert!(applied[0]["schema_version"].is_string(), "{applied:?}");
    assert_eq!(applied[1]["revision"], Value::Null, "{applied:?}");
    let names = type_names(&ekr(&w, "ap", &["ontology"]));
    assert!(names.contains(&"Maker".to_string()), "{names:?}");
    assert!(!names.contains(&"Vendor".to_string()), "{names:?}");
    assert_eq!(calls(&w), asked, "approve or apply asked the model");
}

/// Acceptance 5: on `sqlite`, `apply` keeps a snapshot from before it, which `cortex restore`
/// puts back to the ontology before the apply.
#[test]
fn a_sqlite_apply_leaves_a_snapshot_that_restore_puts_back() {
    let w = instance("sn");
    let stamp = review(&w, "sn", json!([node_type("Maker")]));
    let before = ekr(&w, "sn", &["ontology"]);
    let (code, out) = w.cortex(&["schema", "approve", "sn", &stamp]);
    assert_eq!(code, 0, "{out}");
    let (code, out) = w.cortex(&["schema", "apply", "sn", &stamp]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("applied")),
        "{out}"
    );
    let snapshot = out["detail"]["snapshot"]
        .as_str()
        .unwrap_or_else(|| panic!("no snapshot named: {out}"))
        .to_string();
    assert!(snapshot.ends_with(&format!("schema-{stamp}")), "{snapshot}");
    assert_ne!(ekr(&w, "sn", &["ontology"]), before);

    let (code, out) = w.cortex(&["restore", "sn", &snapshot]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("restored")),
        "{out}"
    );
    assert_eq!(ekr(&w, "sn", &["ontology"]), before);
}

/// Acceptance 6: an approved proposal whose type another run added meanwhile is `invalid` and
/// nothing is applied for it.
#[test]
fn an_approved_type_added_meanwhile_is_invalid_and_not_applied() {
    let w = instance("mw");
    let stamp = review(&w, "mw", json!([node_type("Maker")]));
    let (code, out) = w.cortex(&["schema", "approve", "mw", &stamp]);
    assert_eq!(code, 0, "{out}");
    // A run without `--review` adds the same type at once.
    let (code, out) = w.cortex(&["schema", "mw", "--sample", "5"]);
    assert_eq!(
        (code, out["detail"]["applied"].as_i64()),
        (0, Some(1)),
        "{out}"
    );
    let head = w.head("mw");

    let (code, out) = w.cortex(&["schema", "apply", "mw", &stamp]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("applied")),
        "{out}"
    );
    assert_eq!(
        (&out["detail"]["applied"], &out["detail"]["invalid"]),
        (&json!(0), &json!(1)),
        "{out}"
    );
    assert_eq!(w.head("mw"), head, "nothing was applied");
    let line = &lines(&run_dir(&w, "mw", &stamp))[0];
    assert_eq!(line["status"], "invalid", "{line}");
    assert!(line["note"].is_string(), "{line}");
}

/// Acceptance 7, and a decided proposal is not decided again: `reject` records `rejected` with
/// its reason; `approve` and `apply` on it refuse and change nothing.
#[test]
fn a_rejected_proposal_keeps_its_reason_and_is_never_approved_or_applied() {
    let w = instance("rj");
    let stamp = review(&w, "rj", json!([node_type("Maker")]));
    let dir = run_dir(&w, "rj", &stamp);
    let head = w.head("rj");

    let (code, out) = w.cortex(&[
        "schema",
        "reject",
        "rj",
        &stamp,
        "--only",
        "1",
        "--reason",
        "makers are organisations",
    ]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("rejected")),
        "{out}"
    );
    let line = &lines(&dir)[0];
    assert_eq!(line["status"], "rejected", "{line}");
    assert_eq!(
        line["decision_reason"], "makers are organisations",
        "{line}"
    );
    let written = std::fs::read_to_string(dir.join("proposals.jsonl")).unwrap();

    let (code, out) = w.cortex(&["schema", "approve", "rj", &stamp, "--only", "1"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("not-pending")),
        "{out}"
    );
    let (code, out) = w.cortex(&["schema", "reject", "rj", &stamp, "--reason", "again"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("not-pending")),
        "{out}"
    );
    let (code, out) = w.cortex(&["schema", "apply", "rj", &stamp]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("not-approved")),
        "{out}"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("proposals.jsonl")).unwrap(),
        written,
        "a refused decision rewrote the run"
    );
    assert_eq!(w.head("rj"), head);
}

/// A proposal approved once is not approved again, and an index the run does not have is refused
/// with nothing decided; an unknown stamp is `no-such-run`.
#[test]
fn a_decided_or_unknown_proposal_is_refused_and_nothing_is_decided() {
    let w = instance("dd");
    let stamp = review(&w, "dd", json!([node_type("Maker"), node_type("Vendor")]));
    let dir = run_dir(&w, "dd", &stamp);
    let (code, out) = w.cortex(&["schema", "approve", "dd", &stamp, "--only", "1"]);
    assert_eq!(code, 0, "{out}");
    for only in ["1", "3"] {
        let (code, out) = w.cortex(&[
            "schema", "approve", "dd", &stamp, "--only", "2", "--only", only,
        ]);
        assert_eq!(
            (code, out["outcome"].as_str()),
            (1, Some("not-pending")),
            "{out}"
        );
        assert_eq!(statuses(&dir), ["approved", "pending"]);
    }
    let (code, out) = w.cortex(&["schema", "apply", "dd", "19700101T000000Z"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("no-such-run")),
        "{out}"
    );
}
