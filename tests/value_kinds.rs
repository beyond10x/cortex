//! The extraction prompt names each existing property with its value kind
//! (`story:prompt-names-value-kinds`), so the model can answer a property in the kind the store
//! declares. A model that answers one in another kind anyway loses that fact alone: EKR 0.0.32
//! rejects the part (`extraction-value-mismatch`), and the run applies the batch's other facts and
//! goes on.

mod common;

use std::path::PathBuf;
use std::process::Command;

use common::{ekr, executable, World};
use serde_json::Value;

/// A seed schema declaring `Release.release_date` as `String`, as the `agent-tooling` example does.
const SCHEMA: &str = r#"format: ekr.extraction-document/1
ontology:
  node_types:
  - name: Organization
    parents: []
    abstract_type: false
    properties:
    - {name: website, value: {value_kind: String}, cardinality: One, required: false}
  - {name: Product, parents: [], abstract_type: false, properties: []}
  - name: Release
    parents: []
    abstract_type: false
    properties:
    - {name: version, value: {value_kind: String}, cardinality: One, required: false}
    - {name: release_date, value: {value_kind: String}, cardinality: One, required: false}
  edge_types:
  - {name: DEVELOPS, source_types: [Organization], target_types: [Product], cardinality: Many, properties: []}
entities: []
facts: []
evidence: []
"#;

/// A stand-in `claude` that records the prompt it was given in `prompts.log` and answers one
/// `DEVELOPS` relation per evidence id, then two properties of one release citing the first id:
/// `version` as a `String` and `release_date` as a `Timestamp` (`facts[3]`), as the model answered
/// on 2026-10-07.
const CLAUDE: &str = r##"R=$(cd "$(dirname "$0")/.." && pwd)
prompt=$(cat)
printf '%s\n' "$prompt" >> "$R/prompts.log"
ids=$(printf '%s\n' "$prompt" | grep -oE 'evidence id [0-9a-f-]{36}' | cut -d' ' -f3)
o='{"node_type":"Organization","aliases":["Example Labs"]}'
p='{"node_type":"Product","aliases":["Widget engine"]}'
r='{"node_type":"Release","aliases":["Widget engine 2.0"]}'
facts=""
for id in $ids; do
  facts="$facts{\"!Relation\":{\"subject\":$o,\"relation\":\"DEVELOPS\",\"object\":$p,\"evidence\":[\"$id\"]}},"
done
first=$(printf '%s\n' "$ids" | head -n 1)
facts="$facts{\"!Property\":{\"subject\":$r,\"property\":\"version\",\"value\":{\"value_kind\":\"String\",\"value\":\"2.0\"},\"evidence\":[\"$first\"]}},"
facts="$facts{\"!Property\":{\"subject\":$r,\"property\":\"release_date\",\"value\":{\"value_kind\":\"Timestamp\",\"value\":1789689600000},\"evidence\":[\"$first\"]}}"
cat <<EOF
{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{"format":"ekr.extraction-document/1","ontology":{"node_types":[],"edge_types":[]},"entities":[$o,$p,$r],"facts":[$facts]}}
EOF
"##;

/// An instance `t` over the world's two pages, seeded with [`SCHEMA`], and the recording stand-in.
fn world() -> World {
    let w = World::new();
    executable(&w.bin.join("claude"), CLAUDE);
    std::fs::write(w.root.join("schema.yaml"), SCHEMA).unwrap();
    let path = w.spec("t", "conn_test");
    let text = std::fs::read_to_string(&path).unwrap().replace(
        "seed: {documents: []}",
        "seed: {schema: schema.yaml, documents: []}",
    );
    std::fs::write(&path, text).unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(
        (code, created["outcome"].as_str()),
        (0, Some("created")),
        "{created}"
    );
    w
}

fn graph(w: &World) -> Value {
    let dir: PathBuf = w.home.join("instances/t");
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

/// The acceptance: the prompt of a run on an instance whose ontology declares
/// `Release.release_date` as `String` contains `release_date: String`; a model answering it as a
/// `Timestamp` loses that one fact, and the run applies the rest, counts one rejected part and
/// exits 0.
#[test]
fn the_prompt_names_release_date_as_a_string_and_a_timestamp_answer_loses_that_fact_alone() {
    let w = world();
    let (code, ran) = w.cortex(&["run", "t/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["parts_rejected"], 1, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
    let rejected = ran["detail"]["rejected"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(rejected.len(), 1, "{ran}");
    assert!(
        rejected[0]["refusal"]
            .as_str()
            .is_some_and(|r| r.contains("extraction-value-mismatch")),
        "{ran}"
    );

    // The batch's other facts are in the store: both relations and the release's `version`, and
    // no `release_date`.
    let graph = graph(&w);
    let assertions: Vec<&Value> = graph["assertions"]
        .as_object()
        .expect("assertions")
        .values()
        .collect();
    let relations = assertions
        .iter()
        .filter(|a| a["object"].get("Node").is_some())
        .count();
    assert_eq!(relations, 2, "{graph}");
    let values: Vec<&Value> = assertions
        .iter()
        .filter_map(|a| a["object"].get("Value"))
        .collect();
    assert!(values.iter().any(|v| v["value"] == "2.0"), "{graph}");
    assert!(
        !values.iter().any(|v| v["value"] == 1_789_689_600_000_i64),
        "{graph}"
    );

    let prompts = std::fs::read_to_string(w.root.join("prompts.log")).unwrap();
    assert!(prompts.contains("release_date: String"), "{prompts}");
}
