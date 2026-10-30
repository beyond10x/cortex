//! `story:schema-proposals-on-approval`: `cortex schema <instance> --review` records every
//! proposal `pending` and applies none; `cortex schema approve` and `reject` decide each one once;
//! `cortex schema apply` applies exactly the approved ones, re-planned against the ontology at the
//! head, after a snapshot on a `sqlite` store, and asks no model. The PostgreSQL stage and its gate
//! are in `tests/store_backend.rs`. `story:schema-widens-edge-type`: a `widen_edge_type` proposal
//! takes the same path, written as EKR's `!WidenEdgeType` with each end whole.

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

// `story:schema-widens-edge-type`: a proposal widens which node types an existing edge type
// connects, written as EKR's `!WidenEdgeType` with each end whole, and reviewed and applied like
// any other proposal.

/// An extraction stand-in over node types `Person`, `Team` and `Project` and the edge type
/// `works_on` from `Person` to `Project`: the fact `Team works_on Project`, and, while the file
/// `person` exists, `Person works_on Project`, each citing the batch's first evidence id.
fn extract_works_on(w: &World) {
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
id=$(grep -oE 'evidence id [0-9a-f-]{{36}}' | head -1 | cut -d' ' -f3)
r() {{ printf '{{"!Relation":{{"subject":{{"node_type":"%s","aliases":["%s"]}},"relation":"works_on","object":{{"node_type":"Project","aliases":["Atlas"]}},"evidence":["%s"]}}}}' "$1" "$2" "$id"; }}
facts=$(r Team Platform)
if [ -e "$R/person" ]; then facts="$(r Person Ada),$facts"; fi
n() {{ printf '{{"name":"%s","parents":[],"abstract_type":false,"properties":[]}}' "$1"; }}
e() {{ printf '{{"node_type":"%s","aliases":["%s"]}}' "$1" "$2"; }}
cat <<EOF
{{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{{"format":"ekr.extraction-document/1","ontology":{{"node_types":[$(n Person),$(n Team),$(n Project)],"edge_types":[{{"name":"works_on","source_types":["Person"],"target_types":["Project"],"cardinality":"Many","properties":[]}}]}},"entities":[$(e Person Ada),$(e Team Platform),$(e Project Atlas)],"facts":[$facts]}}}}
EOF
"#,
            root = w.root.display()
        ),
    );
}

/// The search answer: one page at `url`.
fn one_page(w: &World, url: &str) {
    let pages = json!({"results": [{"url": url, "title": "Atlas", "description": "Who works on Atlas.",
        "content": "Ada and the Platform team work on Atlas.", "content_truncated": false,
        "published": null, "score": "0.9"}], "complete": true, "truncation": [],
        "provenance": {"instance": "t", "profile": "tavily/2026-10",
                       "received_at": "2026-10-05T00:00:00.000Z"}});
    std::fs::write(
        w.root.join("answer.json"),
        common::answer(&pages.to_string()),
    )
    .unwrap();
}

/// A run of `name/news`: its detail.
fn run(w: &World, name: &str) -> Value {
    let (code, ran) = w.cortex(&["run", &format!("{name}/news")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    ran["detail"].clone()
}

/// An instance `name` whose store declares `Person`, `Team`, `Project` and `works_on` from
/// `Person` to `Project`, after one run that applied `Person works_on Project`; the run's detail,
/// and the proposer in place of the extraction stand-in.
fn works_on_instance(name: &str) -> (World, Value) {
    let w = World::new();
    let path = w.spec(name, "conn_test");
    extract_works_on(&w);
    one_page(&w, "https://example.org/atlas");
    std::fs::write(w.root.join("person"), "").unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let ran = run(&w, name);
    proposer(&w);
    (w, ran)
}

fn widen(edge: &str, sources: &[&str], targets: &[&str]) -> Value {
    json!({"change": {"kind": "widen_edge_type", "value": {"name": edge,
            "add_source_types": sources, "add_target_types": targets}},
         "reason": format!("the evidence joins other types by {edge}"), "facts": ["FACT0"],
         "evidence": ["E1"]})
}

/// The names of one end (`source_types` or `target_types`) of the edge type `edge`, as
/// `ekr ontology` lists them, sorted.
fn ends(ontology: &Value, edge: &str, end: &str) -> Vec<String> {
    let t = ontology["edge_types"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|t| t["name"] == edge)
        .unwrap_or_else(|| panic!("no edge type {edge}: {ontology}"));
    let mut names: Vec<String> = t[end]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|e| e["name"].as_str().or(e.as_str()).map(str::to_string))
        .collect();
    names.sort();
    names
}

/// Approves every proposal of `stamp` and applies them: `apply`'s detail.
fn approve_and_apply(w: &World, name: &str, stamp: &str) -> Value {
    let (code, out) = w.cortex(&["schema", "approve", name, stamp]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("approved")),
        "{out}"
    );
    let (code, out) = w.cortex(&["schema", "apply", name, stamp]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("applied")),
        "{out}"
    );
    out["detail"].clone()
}

/// Acceptance 2, 3 and 6: `Team works_on Project` is rejected by EKR (`extraction-relation-ends`,
/// counted in `parts_rejected`) before the widening; the widening is recorded `pending`, and after
/// approve and apply `ekr ontology` lists `works_on` from `Person` and `Team` to `Project`, the
/// written transaction carries the source end whole, and the same fact is applied.
#[test]
fn a_widening_is_reviewed_applied_whole_and_admits_the_fact_it_refused() {
    let (w, before) = works_on_instance("wd");
    assert_eq!(before["documents_applied"], 1, "{before}");
    assert_eq!(before["parts_rejected"], 1, "{before}");
    assert!(
        before.to_string().contains("extraction-relation-ends"),
        "EKR's refusal is named: {before}"
    );
    let ontology = ekr(&w, "wd", &["ontology"]);
    assert_eq!(ends(&ontology, "works_on", "source_types"), ["Person"]);

    let head = w.head("wd");
    let stamp = review(&w, "wd", json!([widen("works_on", &["Team"], &[])]));
    let dir = run_dir(&w, "wd", &stamp);
    assert_eq!(statuses(&dir), ["pending"]);
    assert_eq!(lines(&dir)[0]["change"]["kind"], "widen_edge_type");
    assert_eq!(w.head("wd"), head, "a review run commits nothing");

    let applied = approve_and_apply(&w, "wd", &stamp);
    assert_eq!(applied["applied"], 1, "{applied}");
    assert_eq!(w.head("wd"), head + 1, "one schema transaction");
    assert_eq!(statuses(&dir), ["applied"]);
    let ontology = ekr(&w, "wd", &["ontology"]);
    assert_eq!(
        ends(&ontology, "works_on", "source_types"),
        ["Person", "Team"]
    );
    assert_eq!(ends(&ontology, "works_on", "target_types"), ["Project"]);

    // The transaction names each end whole: the id of `Person` is still among its sources.
    let person = ontology["node_types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "Person")
        .and_then(|t| t["id"].as_str())
        .unwrap()
        .to_string();
    let written: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("schema-") && n.ends_with(".yaml"))
        })
        .map(|p| std::fs::read_to_string(p).unwrap())
        .collect();
    assert_eq!(written.len(), 1, "{written:?}");
    let text = &written[0];
    let at = text
        .find("!WidenEdgeType")
        .unwrap_or_else(|| panic!("no !WidenEdgeType in {text}"));
    let op = &text[at..];
    let sources = &op[op.find("source_types:").unwrap()..op.find("target_types:").unwrap()];
    assert!(sources.contains(&person), "Person {person} kept: {text}");

    // The same fact, on a new page, is applied now.
    extract_works_on(&w);
    std::fs::remove_file(w.root.join("person")).unwrap();
    one_page(&w, "https://example.org/atlas-team");
    let after = run(&w, "wd");
    assert_eq!(
        (&after["documents_applied"], &after["parts_rejected"]),
        (&json!(1), &json!(0)),
        "{after}"
    );
}

/// Acceptance 4 and 5: a widening naming an edge type or a node type the ontology lacks, or adding
/// nothing new to either end, is `invalid` with a note naming it, and nothing is submitted.
#[test]
fn a_widening_of_an_unknown_type_or_adding_nothing_is_invalid() {
    let (w, _) = works_on_instance("wi");
    let before = ekr(&w, "wi", &["ontology"]);
    let stamp = review(
        &w,
        "wi",
        json!([
            widen("leads", &["Team"], &[]),
            widen("works_on", &["Robot"], &[]),
            widen("works_on", &[], &["Galaxy"]),
            widen("works_on", &["Person"], &["Project"]),
            widen("works_on", &[], &[]),
        ]),
    );
    let head = w.head("wi");
    let applied = approve_and_apply(&w, "wi", &stamp);
    assert_eq!(
        (&applied["applied"], &applied["invalid"]),
        (&json!(0), &json!(5)),
        "{applied}"
    );
    assert_eq!(w.head("wi"), head, "nothing was submitted");
    assert_eq!(ekr(&w, "wi", &["ontology"]), before);
    let dir = run_dir(&w, "wi", &stamp);
    let notes: Vec<String> = lines(&dir)
        .iter()
        .map(|l| {
            assert_eq!(l["status"], "invalid", "{l}");
            l["note"].as_str().unwrap_or_default().to_string()
        })
        .collect();
    for (note, named) in notes.iter().zip(["leads", "Robot", "Galaxy"]) {
        assert!(note.contains(named), "{named} in {note}");
    }
    assert!(!std::fs::read_dir(&dir).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with("schema-")));
}

/// `Names::add` widens its copy: a second widening of the same round that adds what the first
/// added is `invalid`, so it is never submitted.
#[test]
fn a_second_widening_of_one_round_plans_against_the_first() {
    let (w, _) = works_on_instance("w2");
    answer(
        &w,
        json!([
            widen("works_on", &["Team"], &[]),
            widen("works_on", &["Team"], &[])
        ]),
    );
    let head = w.head("w2");
    let (code, out) = w.cortex(&["schema", "w2", "--sample", "5"]);
    assert_eq!(code, 0, "{out}");
    let d = &out["detail"];
    assert_eq!(
        (&d["applied"], &d["invalid"]),
        (&json!(1), &json!(1)),
        "{out}"
    );
    assert_eq!(w.head("w2"), head + 1, "one schema transaction");
}

/// A stand-in `ekr` in front of the real one: while `<root>/fail-head-after-commit` exists, an
/// `ekr head` after an `ekr commit` fails as an unavailable store does.
fn head_failing_ekr(w: &World) -> PathBuf {
    let path = w.bin.join("ekr-head");
    let body = r#"R="@R@"; EKR="@EKR@"
if [ "$1" = commit ] && [ -e "$R/fail-head-after-commit" ]; then
  "$EKR" "$@"; s=$?; [ "$s" = 0 ] && touch "$R/committed"; exit "$s"
fi
if [ "$1" = head ] && [ -e "$R/committed" ]; then
  echo "ekr: the store is unavailable: injected" >&2; exit 1
fi
exec "$EKR" "$@"
"#
    .replace("@R@", &w.root.display().to_string())
    .replace("@EKR@", &common::ekr().display().to_string());
    executable(&path, &body);
    path
}

/// [`instance`] on [`head_failing_ekr`].
fn instance_on_head_failing_ekr(name: &str) -> World {
    let w = World::new();
    let path = w.spec(name, "conn_test");
    let bin = head_failing_ekr(&w);
    let text = std::fs::read_to_string(&path).unwrap().replace(
        &common::ekr().display().to_string(),
        &bin.display().to_string(),
    );
    std::fs::write(&path, text).unwrap();
    extract_products(&w);
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let (code, ran) = w.cortex(&["run", &format!("{name}/news")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    proposer(&w);
    w
}

/// `story:schema-apply-outcome-unknown` acceptance 2: the store's head cannot be read after the
/// changes were committed. `apply-failed` says the changes are in the store and names the
/// revision they reached, not that every approved proposal is still approved, and the decisions
/// are written.
#[test]
fn a_head_that_cannot_be_read_after_the_apply_reports_what_was_published() {
    let w = instance_on_head_failing_ekr("hf");
    let stamp = review(&w, "hf", json!([node_type("Maker"), node_type("Vendor")]));
    let head = w.head("hf");
    let (code, out) = w.cortex(&["schema", "approve", "hf", &stamp]);
    assert_eq!(code, 0, "{out}");
    std::fs::write(w.root.join("fail-head-after-commit"), "").unwrap();

    let (code, out) = w.cortex(&["schema", "apply", "hf", &stamp]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("apply-failed")),
        "{out}"
    );
    let reason = out["detail"]["reason"].as_str().unwrap_or_default();
    assert!(!reason.contains("still approved"), "{out}");
    assert!(reason.contains("the changes are in the store"), "{out}");
    assert!(
        reason.contains(&format!("revision {}", head + 2)),
        "the revision is not named: {out}"
    );
    assert_eq!(w.head("hf"), head + 2, "both changes are committed");
    assert_eq!(statuses(&run_dir(&w, "hf", &stamp)), ["applied", "applied"]);
}

/// `story:schema-apply-outcome-unknown` acceptance 2: `proposals.jsonl` cannot be rewritten after
/// the changes were committed. `apply-failed` says the changes are in the store, names the
/// revision, and says the file still lists the proposals approved.
#[test]
fn decisions_that_cannot_be_written_after_the_apply_report_what_was_published() {
    let w = instance("wf");
    let stamp = review(&w, "wf", json!([node_type("Maker"), node_type("Vendor")]));
    let head = w.head("wf");
    let (code, out) = w.cortex(&["schema", "approve", "wf", &stamp]);
    assert_eq!(code, 0, "{out}");
    let dir = run_dir(&w, "wf", &stamp);
    // The rewrite goes through `.proposals.jsonl.next`; a directory there fails it.
    std::fs::create_dir(dir.join(".proposals.jsonl.next")).unwrap();

    let (code, out) = w.cortex(&["schema", "apply", "wf", &stamp]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("apply-failed")),
        "{out}"
    );
    let reason = out["detail"]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("the changes are in the store"), "{out}");
    assert!(
        reason.contains(&format!("revision {}", head + 2)),
        "the revision is not named: {out}"
    );
    assert!(
        reason.contains("still lists every approved proposal approved"),
        "{out}"
    );
    assert_eq!(w.head("wf"), head + 2, "both changes are committed");
    assert_eq!(statuses(&dir), ["approved", "approved"]);
    let names = type_names(&ekr(&w, "wf", &["ontology"]));
    assert!(names.contains(&"Maker".to_string()), "{names:?}");
    assert!(names.contains(&"Vendor".to_string()), "{names:?}");
}
