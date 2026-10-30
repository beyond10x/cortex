//! Adversarial cases for `story:one-store-read-per-run`: the run's index of store nodes
//! ([`known::Index`]) is grown from each applied document by [`known::Index::applied`], which says
//! it reads the named things "as the verb resolves them". The story's safety condition is that the
//! grown index gives batch k+1 the same `Store` hits a fresh `ekr snapshot` would. Each case
//! applies one document with the real `ekr` 0.0.34 to a store seeded with an `ekr-seed/2` file,
//! grows an index built before the apply, and compares its hits with an index of a fresh snapshot.
//!
//! A case marked `#[ignore = "adversary: ..."]` fails on the unit's head and names the finding.

mod common;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

use common::{ekr, executable, World};
use cortex_cli::known::Index;
use cortex_model::instance as m;
use serde_json::Value;

/// Types `Person` and `Organization`, the edge type `CEO_OF` from a person to an organization of
/// cardinality `One`, the person `Alice` and the organization whose canonical name is `Acme Corp`
/// and whose only alias is `acme` (a name is a property in EKR, not an identity).
const SEED: &str = "\
format: ekr-seed/2
ontology:
  version:
    id: 00000000-0000-4000-8000-000000000001
    number: 0
    parent: null
    created_at: 0
  node_types:
  - id: 00000000-0000-4000-8000-000000000201
    name: Person
    parents: []
    properties: {}
    abstract_type: false
    lifecycle: null
    operations: {}
  - id: 00000000-0000-4000-8000-000000000202
    name: Organization
    parents: []
    properties: {}
    abstract_type: false
    lifecycle: null
    operations: {}
  edge_types:
  - id: 00000000-0000-4000-8000-000000000203
    name: CEO_OF
    source_types:
    - 00000000-0000-4000-8000-000000000201
    target_types:
    - 00000000-0000-4000-8000-000000000202
    cardinality: One
    properties: {}
    inverse: null
    symmetric: false
    transitive: false
graph:
  format: ekr.graph-document/2
  graph:
    root:
      id: 00000000-0000-4000-8000-000000000002
      space: Canonical
      schema_version_id: 00000000-0000-4000-8000-000000000001
      parent: null
      created_at: 0
    revision: 0
    nodes:
      00000000-0000-4000-8000-000000000301:
        id: 00000000-0000-4000-8000-000000000301
        root_id: 00000000-0000-4000-8000-000000000002
        type_id: 00000000-0000-4000-8000-000000000201
        canonical_name: Alice
        aliases: [Alice]
        type_state: null
        properties: {}
      00000000-0000-4000-8000-000000000303:
        id: 00000000-0000-4000-8000-000000000303
        root_id: 00000000-0000-4000-8000-000000000002
        type_id: 00000000-0000-4000-8000-000000000202
        canonical_name: Acme Corp
        aliases: [acme]
        type_state: null
        properties: {}
    edges: {}
    assertions: {}
    evidence: {}
evidence_payloads: {}
";

/// A `claude` that is never asked: these cases call no model.
const CLAUDE: &str = "exit 1\n";

fn world() -> World {
    let w = World::new();
    executable(&w.bin.join("claude"), CLAUDE);
    std::fs::write(w.root.join("seed.yaml"), SEED).unwrap();
    let spec = w.root.join("k.yaml");
    std::fs::write(
        &spec,
        format!(
            "format: cortex.instance/1\nname: k\ndescription: Test brain.\n\
             ekr: {{version: \"0.0.34\", bin: \"{bin}\"}}\n\
             seed: {{ekr_seed: seed.yaml, documents: []}}\n\
             model: {{model: claude-haiku-4-5-20251001, budget_usd: \"1\", timeout_s: 60}}\n\
             sources: []\nserve: {{view_port: 18996}}\n",
            bin = ekr().display()
        ),
    )
    .unwrap();
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    w
}

fn instance(w: &World) -> PathBuf {
    w.home.join("instances").join("k")
}

/// `ekr <args>` on the instance's store, as JSON.
fn ekr_json(w: &World, args: &[&str]) -> Value {
    let dir = instance(w);
    let out = Command::new(ekr())
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

fn index(w: &World) -> Index {
    Index::from_snapshot(&ekr_json(w, &["snapshot"]), &ekr_json(w, &["ontology"]))
}

/// One evidence item, `e1`, citing `payload`, extracted by the instance's operator.
fn evidence(w: &World) -> String {
    let host = std::fs::read_to_string(instance(w).join("host.json")).unwrap();
    let operator = cortex_cli::ekr::operator(&host).unwrap();
    let payload = "Alice runs Beta Inc and Gamma Inc.";
    format!(
        "evidence:\n\
         - evidence:\n    id: 00000000-0000-4000-8000-0000000000e1\n    source: !HumanStatement\n      \
         identity: \"file:/notes/a.md\"\n    content_hash: {hash}\n    extracted_by: {operator}\n    \
         observed_at: 1773273600000\n    confidence: 8000\n  payload: {bytes:?}\n",
        hash = cortex_cli::evidence::content_hash(payload.as_bytes()),
        bytes = payload.as_bytes(),
    )
}

/// Applies `document` with `ekr apply-extraction`, grows `before` by it as the run does, and
/// answers the grown index's hits of `terms` beside a fresh snapshot's, each sorted, and the
/// report.
fn grown_and_fresh(
    w: &World,
    mut before: Index,
    document: &str,
    terms: &[&str],
) -> (Vec<String>, Vec<String>, Value) {
    let path = w.root.join("extraction.yaml");
    std::fs::write(&path, document).unwrap();
    let report = ekr_json(w, &["apply-extraction", path.to_str().unwrap()]);
    let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(document).unwrap();
    before.applied(&doc, &report);
    let terms: BTreeSet<String> = terms.iter().map(|t| cortex_cli::known::fold(t)).collect();
    let sorted = |hits: Vec<m::KnownEntity>| {
        let mut lines: Vec<String> = hits
            .iter()
            .map(|e| format!("{} {:?} {:?}", e.node_type, e.name, e.aliases))
            .collect();
        lines.sort();
        lines
    };
    (
        sorted(before.hits(&terms)),
        sorted(index(w).hits(&terms)),
        report,
    )
}

/// EKR resolves a named thing by its aliases only and never compares a canonical name
/// (`ekr-integrate` `resolve`, "`canonical_name` is never compared"), so an entity named
/// `Acme Corp` beside a store node of that canonical name but without that alias is a new node.
/// The grown index takes the store node's canonical name as one of its aliases, holds the entity
/// for that node and adds nothing: the next batch is not shown the node the apply created.
#[test]
#[ignore = "adversary: Index::applied treats a store node's canonical name as an alias, so a node EKR creates beside it is missing from the next batch's store hits"]
fn a_node_ekr_creates_beside_a_canonical_name_that_is_no_alias_is_in_the_grown_index() {
    let w = world();
    let before = index(&w);
    let document = format!(
        "format: ekr.extraction-document/1\nontology: {{node_types: [], edge_types: []}}\n\
         entities:\n- {{node_type: Organization, aliases: [Acme Corp]}}\nfacts: []\n{}",
        evidence(&w)
    );
    let (grown, fresh, report) = grown_and_fresh(&w, before, &document, &["Acme Corp"]);
    assert_eq!(
        grown, fresh,
        "the grown index's hits differ from a fresh snapshot's; report: {report}"
    );
}

/// A relation fact the kernel refuses at commit (here the second `CEO_OF` from one person, of
/// cardinality `One`) is reported as `facts[<i>]`; the nodes it names were created before its
/// commit was tried (`ekr-sdk` `extraction`, step 2 before step 3). The grown index drops every
/// named thing of a rejected `facts[<i>]`, so the organization created for it is not a store hit
/// of the next batch, although the store holds it.
#[test]
#[ignore = "adversary: Index::applied drops the subject and object of a fact rejected at commit, whose nodes EKR created before the commit, so the next batch is not shown them"]
fn the_object_of_a_fact_rejected_at_commit_is_in_the_grown_index() {
    let w = world();
    let before = index(&w);
    let document = format!(
        "format: ekr.extraction-document/1\nontology: {{node_types: [], edge_types: []}}\n\
         entities: []\nfacts:\n\
         - !Relation\n  subject: {{node_type: Person, aliases: [Alice]}}\n  relation: CEO_OF\n  \
         object: {{node_type: Organization, aliases: [Beta Inc]}}\n  \
         evidence: [00000000-0000-4000-8000-0000000000e1]\n\
         - !Relation\n  subject: {{node_type: Person, aliases: [Alice]}}\n  relation: CEO_OF\n  \
         object: {{node_type: Organization, aliases: [Gamma Inc]}}\n  \
         evidence: [00000000-0000-4000-8000-0000000000e1]\n{}",
        evidence(&w)
    );
    let (grown, fresh, report) = grown_and_fresh(&w, before, &document, &["Beta Inc", "Gamma Inc"]);
    assert_eq!(
        grown, fresh,
        "the grown index's hits differ from a fresh snapshot's; report: {report}"
    );
}
