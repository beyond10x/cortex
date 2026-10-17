//! Adversary cases for `story:schema-proposals-on-approval` on a `sqlite` store: an EKR commit
//! that took effect but lost its answer, a gate that fails, an apply run twice, and the edges of
//! `approve` and `reject` (an index out of range, a duplicate, a blank reason, a run file a crash
//! left behind). The EKR the instance runs is a stand-in in front of the real one that, once the
//! file `lose-commit` holds `<n>`, lets the `<n>`-th `commit` take effect and then fails as if its
//! answer never arrived.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{executable, World};
use serde_json::{json, Value};

/// The extraction stand-in: three products, each citing the first evidence id of its batch.
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

/// The proposing model's stand-in: counts its calls and answers `proposals.json`.
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

/// The EKR stand-in in front of the real one; see the module comment.
fn losing_ekr(w: &World) -> PathBuf {
    let path = w.bin.join("ekr-losing");
    executable(
        &path,
        &format!(
            r#"R="{root}"; EKR="{ekr}"
if [ "$1" = commit ] && [ -e "$R/lose-commit" ]; then
  n=$(cat "$R/commits" 2>/dev/null || echo 0); n=$((n+1)); echo "$n" > "$R/commits"
  if [ "$n" = "$(cat "$R/lose-commit")" ]; then
    "$EKR" "$@" > /dev/null 2>&1
    echo "connection reset by peer" >&2
    exit 1
  fi
fi
exec "$EKR" "$@"
"#,
            root = w.root.display(),
            ekr = common::ekr().display()
        ),
    );
    path
}

fn node_type(name: &str) -> Value {
    json!({"change": {"kind": "add_node_type", "value": {"name": name, "properties": [
            {"name": "country", "value_kind": "String", "cardinality": "One"}]}},
         "reason": format!("the evidence names a {name}"), "facts": ["FACT0"], "evidence": ["E1"]})
}

/// An instance `name` on the losing EKR, whose store holds three product facts; answers the
/// spec's path.
fn instance(w: &World, name: &str) -> PathBuf {
    let path = w.spec(name, "conn_test");
    let bin = losing_ekr(w);
    let text = std::fs::read_to_string(&path).unwrap().replace(
        &format!("bin: \"{}\"", common::ekr().display()),
        &format!("bin: \"{}\"", bin.display()),
    );
    assert!(text.contains("ekr-losing"), "{text}");
    std::fs::write(&path, text).unwrap();
    extract_products(w);
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let (code, ran) = w.cortex(&["run", &format!("{name}/news")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    proposer(w);
    path
}

fn ontology_names(w: &World, name: &str) -> Vec<String> {
    let dir = w.home.join("instances").join(name);
    let out = Command::new(common::ekr())
        .arg("ontology")
        .env("EKR_HOST", dir.join("host.json"))
        .env("EKR_BACKEND", "sqlite")
        .env("EKR_STORE", dir.join("store.sqlite"))
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    v["node_types"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
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

fn statuses(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join("proposals.jsonl"))
        .unwrap()
        .lines()
        .map(|l| {
            serde_json::from_str::<Value>(l).unwrap()["status"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        })
        .collect()
}

fn review(w: &World, name: &str, proposals: Value) -> String {
    std::fs::write(w.root.join("proposals.json"), proposals.to_string()).unwrap();
    let (code, out) = w.cortex(&["schema", name, "--sample", "5", "--review"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("reviewed")),
        "{out}"
    );
    out["detail"]["stamp"].as_str().unwrap().to_string()
}

fn approve_all(w: &World, name: &str, stamp: &str) {
    let (code, out) = w.cortex(&["schema", "approve", name, stamp]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("approved")),
        "{out}"
    );
}

/// The first and only schema transaction of an apply commits in EKR, and EKR's answer is lost.
/// `apply-failed` says "Nothing was published ... on `sqlite` the snapshot taken before was
/// restored; every approved proposal is still `approved`", so the store's head and ontology must
/// be where they were before the apply.
#[test]
fn a_first_commit_that_took_effect_but_lost_its_answer_is_undone_on_sqlite() {
    let w = World::new();
    instance(&w, "lc");
    let stamp = review(&w, "lc", json!([node_type("Maker")]));
    approve_all(&w, "lc", &stamp);
    let head = w.head("lc");
    std::fs::write(w.root.join("lose-commit"), "1").unwrap();

    let (code, out) = w.cortex(&["schema", "apply", "lc", &stamp]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("apply-failed")),
        "{out}"
    );
    assert_eq!(statuses(&run_dir(&w, "lc", &stamp)), ["approved"]);
    assert_eq!(
        w.head("lc"),
        head,
        "apply-failed left the store's head moved, with its proposal still approved: {out}"
    );
    assert!(
        !ontology_names(&w, "lc").contains(&"Maker".to_string()),
        "apply-failed left Maker in the ontology: {out}"
    );
}

/// The second commit of an apply takes effect and loses its answer, after the first committed:
/// the snapshot is restored, both stay `approved`, and a retry applies each exactly once.
#[test]
fn a_lost_second_commit_restores_the_snapshot_and_a_retry_applies_each_once() {
    let w = World::new();
    instance(&w, "l2");
    let stamp = review(&w, "l2", json!([node_type("Maker"), node_type("Vendor")]));
    approve_all(&w, "l2", &stamp);
    let head = w.head("l2");
    std::fs::write(w.root.join("lose-commit"), "2").unwrap();

    let (code, out) = w.cortex(&["schema", "apply", "l2", &stamp]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("apply-failed")),
        "{out}"
    );
    let dir = run_dir(&w, "l2", &stamp);
    assert_eq!(statuses(&dir), ["approved", "approved"]);
    assert_eq!(w.head("l2"), head, "{out}");
    let names = ontology_names(&w, "l2");
    assert!(!names.contains(&"Maker".to_string()), "{names:?}");
    assert!(!names.contains(&"Vendor".to_string()), "{names:?}");

    std::fs::remove_file(w.root.join("lose-commit")).unwrap();
    let (code, out) = w.cortex(&["schema", "apply", "l2", &stamp]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("applied")),
        "{out}"
    );
    assert_eq!(out["detail"]["applied"], 2, "{out}");
    assert_eq!(w.head("l2"), head + 2, "{out}");
    assert_eq!(statuses(&dir), ["applied", "applied"]);
}

/// On `sqlite` a gate that fails restores the snapshot: head and ontology as before, every
/// approved proposal still `approved`.
#[test]
fn a_sqlite_apply_that_fails_its_gate_restores_the_store() {
    let w = World::new();
    let spec = instance(&w, "gf");
    let stamp = review(&w, "gf", json!([node_type("Maker")]));
    approve_all(&w, "gf", &stamp);
    let head = w.head("gf");
    let text = std::fs::read_to_string(&spec).unwrap();
    std::fs::write(
        &spec,
        format!("{text}gate:\n  checks:\n    - {{measure: assertions.active, max: \"0\"}}\n"),
    )
    .unwrap();
    let (code, out) = w.cortex(&["update", "gf", "--spec", spec.to_str().unwrap()]);
    assert_eq!(code, 0, "{out}");

    let (code, out) = w.cortex(&["schema", "apply", "gf", &stamp]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("gate-failed")),
        "{out}"
    );
    assert_eq!(w.head("gf"), head, "{out}");
    assert!(!ontology_names(&w, "gf").contains(&"Maker".to_string()));
    assert_eq!(statuses(&run_dir(&w, "gf", &stamp)), ["approved"]);
}

/// An apply run again applies nothing a second time; two pending proposals of one type are
/// applied once and the second is `invalid`.
#[test]
fn apply_after_apply_applies_nothing_twice() {
    let w = World::new();
    instance(&w, "aa");
    let stamp = review(&w, "aa", json!([node_type("Maker"), node_type("Maker")]));
    approve_all(&w, "aa", &stamp);
    let head = w.head("aa");
    let (code, out) = w.cortex(&["schema", "apply", "aa", &stamp]);
    assert_eq!(
        (code, &out["detail"]["applied"], &out["detail"]["invalid"]),
        (0, &json!(1), &json!(1)),
        "{out}"
    );
    assert_eq!(w.head("aa"), head + 1);
    let dir = run_dir(&w, "aa", &stamp);
    let written = std::fs::read_to_string(dir.join("proposals.jsonl")).unwrap();
    let (code, out) = w.cortex(&["schema", "apply", "aa", &stamp]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("not-approved")),
        "{out}"
    );
    assert_eq!(w.head("aa"), head + 1);
    assert_eq!(
        std::fs::read_to_string(dir.join("proposals.jsonl")).unwrap(),
        written
    );
}

/// `--only` 0 and a negative index are refused with nothing decided, a stamp that leaves the run
/// directory is no run, a duplicate index decides once, and a blank reason is refused.
#[test]
fn edges_of_only_stamp_and_reason() {
    let w = World::new();
    instance(&w, "ed");
    let stamp = review(&w, "ed", json!([node_type("Maker"), node_type("Vendor")]));
    let dir = run_dir(&w, "ed", &stamp);
    let written = std::fs::read_to_string(dir.join("proposals.jsonl")).unwrap();
    for only in ["--only=0", "--only=-1", "--only=3"] {
        let (code, out) = w.cortex(&["schema", "approve", "ed", &stamp, only]);
        assert_eq!(
            (code, out["outcome"].as_str()),
            (1, Some("not-pending")),
            "{only}: {out}"
        );
    }
    let (code, out) = w.cortex(&["schema", "reject", "ed", &stamp, "--reason", "  "]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("not-pending")),
        "{out}"
    );
    for stamp in ["..", "../..", "a/b", ""] {
        let (code, out) = w.cortex(&["schema", "approve", "ed", stamp]);
        assert_eq!(
            (code, out["outcome"].as_str()),
            (1, Some("no-such-run")),
            "{stamp:?}: {out}"
        );
    }
    assert_eq!(
        std::fs::read_to_string(dir.join("proposals.jsonl")).unwrap(),
        written
    );
    let (code, out) = w.cortex(&[
        "schema", "approve", "ed", &stamp, "--only", "2", "--only", "2",
    ]);
    assert_eq!(
        (code, &out["detail"]["decided"], &out["detail"]["pending"]),
        (0, &json!(1), &json!(1)),
        "{out}"
    );
    assert_eq!(statuses(&dir), ["pending", "approved"]);
}

/// A crash left a partial `.proposals.jsonl.next` beside the run: the next decision is not
/// affected by it. A run file cut short mid-line is `no-such-run` and is not rewritten.
#[test]
fn a_partial_write_left_by_a_crash_is_neither_read_nor_decided() {
    let w = World::new();
    instance(&w, "pw");
    let stamp = review(&w, "pw", json!([node_type("Maker"), node_type("Vendor")]));
    let dir = run_dir(&w, "pw", &stamp);
    std::fs::write(dir.join(".proposals.jsonl.next"), "{\"format\":").unwrap();
    let (code, out) = w.cortex(&["schema", "approve", "pw", &stamp, "--only", "1"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(statuses(&dir), ["approved", "pending"]);

    let full = std::fs::read_to_string(dir.join("proposals.jsonl")).unwrap();
    let cut = &full[..full.len() - 20];
    std::fs::write(dir.join("proposals.jsonl"), cut).unwrap();
    let (code, out) = w.cortex(&["schema", "approve", "pw", &stamp, "--only", "2"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("no-such-run")),
        "{out}"
    );
    let (code, out) = w.cortex(&["schema", "apply", "pw", &stamp]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("no-such-run")),
        "{out}"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("proposals.jsonl")).unwrap(),
        cut
    );
}
