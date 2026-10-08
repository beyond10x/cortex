//! Adversary pass 2 on `story:parent-identity-prefix-unmasked`: the greedy hex choice of
//! `structured::prefix` and the identities `Source::earlier` makes a source own.

use cortex_cli::structured::{prefix, Held, Source};
use cortex_model::instance::{SourceSettings, StructuredSource};
use serde_json::json;
use sha2::{Digest, Sha256};

const CHILD: &str = "            children:
              - operation: tags.list
                input: {project: \"{id}\"}
                records: \"$.tags\"
                parent: IN_PROJECT
                mapping:
                  node_type: Tag
                  id: \"$.name\"
                  name: \"$.name\"
                  aliases: []
                  properties: [{property: commit, path: \"$.commit\"}]
                  relations: []
";

fn source(name: &str, input: &str) -> String {
    format!(
        r#"  - name: {name}
    schedule: daily
    settings:
      kind: structured
      value:
        input:
{input}        records: "$.people"
        mapping:
          node_type: Person
          id: "$.id"
          name: "$.name"
          aliases: []
          properties: [{{property: email, path: "$.email"}}]
          relations: []
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
"#
    )
}

fn connectors(adapter: &str, operation: &str, children: bool) -> String {
    format!(
        "          from: connectors\n          value:\n            adapter: \"{adapter}\"\n            \
         connection: conn_test\n            operation: \"{operation}\"\n            inputs: [{{}}]\n{}",
        if children { CHILD } else { "" }
    )
}

fn files(path: &str, glob: &str) -> String {
    format!("          from: files\n          value: {{paths: [\"{path}\"], glob: \"{glob}\"}}\n")
}

fn spec(sources: &[String]) -> Vec<StructuredSource> {
    let yaml = format!(
        r#"format: cortex.instance/1
name: c
description: Test brain.
ekr: {{version: "0.0.32", bin: "ekr"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
{}serve: {{view_port: 18995}}
"#,
        sources.concat()
    );
    let spec = cortex_cli::spec::parse(&yaml).expect("a valid spec");
    spec.sources
        .into_iter()
        .map(|s| match s.settings {
            SourceSettings::Structured(st) => st,
            _ => unreachable!(),
        })
        .collect()
}

fn digest(raw: &str) -> String {
    cortex_cli::state::hex(&Sha256::digest(raw.as_bytes()))[..16].to_string()
}

/// Two parts each ending in a credential's name: hexing either alone leaves the other read with
/// what follows it, so no single round lowers the count and the loop stops with the prefix as
/// it was. The acceptance names a path and a glob, an adapter and an operation, each alone.
#[test]
fn a_prefix_with_two_parts_masking_reads_ends_with_nothing_masked() {
    let mut masked = Vec::new();
    for (what, input) in [
        ("files path and glob", files("vault/secret", "*.secret")),
        (
            "adapter and operation",
            connectors("vault-secret", "vault.secret", false),
        ),
    ] {
        let st = &spec(&[source("people", &input)])[0];
        let p = prefix(st);
        let s = Source::new(st, None);
        for raw in ["P-1", "person-0002"] {
            let id = s.identity(raw);
            if cortex_cli::mask::mask_key(&id).1 > 0 {
                masked.push(format!("{what}: prefix {p:?}, identity {id:?}"));
            }
        }
    }
    assert!(
        masked.is_empty(),
        "identities masking changes, so the seen state drops them and the record is applied on \
         every run: {masked:#?}"
    );
}

/// The first run after the prefix turned hex: a record the run read but did not apply (beyond
/// `max_documents_per_run`, held back, rejected) is in `unsettled` under its new identity, and
/// keeps its values. Its values in the store cite its identity under the base prefix, which
/// `earlier` makes the source's, and nothing maps that to the unsettled identity.
#[test]
fn an_unsettled_record_keeps_the_values_applied_under_its_base_identity() {
    let st = &spec(&[source("people", &connectors("dir", "vault.secret", false))])[0];
    let s = Source::new(st, None);
    let old = format!("dir:vault.secret:{}", digest("person-0002"));
    let snapshot = json!({"graph": {"graph": {
        "assertions": {"a-old": {"id": "a-old", "subject": {"Node": "n1"},
            "predicate": {"Property": "p-email"},
            "object": {"Value": {"value_kind": "String", "value": "g@example.org"}},
            "evidence": ["e-1"], "lifecycle": "Active",
            "valid_time": {"from": 1, "to": null}}},
        "edges": {}, "nodes": {},
        "evidence": {"e-1": {"source": {"HumanStatement": {"identity": format!("record:{old}")}}}},
    }}});
    let ontology = json!({"node_types": [{"properties": [{"id": "p-email", "name": "email"}]}],
        "edge_types": []});
    let unsettled = std::collections::BTreeSet::from([s.identity("person-0002")]);
    let ops = s.ended(&snapshot, &ontology, &Default::default(), &unsettled, &[]);
    assert!(
        ops.is_empty(),
        "a record the run did not apply loses the value it holds under {old}: {ops:?}"
    );
}

/// The same for a child call that failed for its parent on that run: `failed_child` holds the
/// new child prefix only, so the child values applied under the base prefix are ended though
/// the run never read them.
#[test]
fn a_failed_child_call_keeps_the_child_values_applied_under_the_base_prefix() {
    let st = &spec(&[source("people", &connectors("dir", "vault.secret", true))])[0];
    let s = Source::new(st, None);
    let (_, held) = s.failed_child("P-1", "tags.list");
    let held = Held {
        prefix: held.expect("a held prefix"),
        relation: None,
    };
    let old = "dir:vault.secret/tags.list:P-1:v1";
    let snapshot = json!({"graph": {"graph": {
        "assertions": {"a-old": {"id": "a-old", "subject": {"Node": "n1"},
            "predicate": {"Property": "p-commit"},
            "object": {"Value": {"value_kind": "String", "value": "c1"}},
            "evidence": ["e-1"], "lifecycle": "Active",
            "valid_time": {"from": 1, "to": null}}},
        "edges": {}, "nodes": {},
        "evidence": {"e-1": {"source": {"HumanStatement": {"identity": format!("record:{old}")}}}},
    }}});
    let ontology = json!({"node_types": [{"properties": [{"id": "p-commit", "name": "commit"}]}],
        "edge_types": []});
    let ops = s.ended(
        &snapshot,
        &ontology,
        &Default::default(),
        &Default::default(),
        &[held],
    );
    assert!(
        ops.is_empty(),
        "a child value of a parent whose child call failed is ended: {ops:?}"
    );
}

/// `earlier` holds the base child prefix and `owns` accepts anything under it followed by `:`,
/// looser than `child_of`, which needs a parent part. Another source whose operation is
/// `vault.secret/tags.list` has today's identities `dir:vault.secret/tags.list:<id>`, which
/// the first source now claims (on the base it did not: its prefix was followed by `/`, and
/// `child_of` found no parent part).
#[test]
fn earlier_does_not_claim_another_sources_current_identities() {
    let sts = spec(&[
        source("people", &connectors("dir", "vault.secret", true)),
        source("tags", &connectors("dir", "vault.secret/tags.list", false)),
    ]);
    let (a, b) = (Source::new(&sts[0], None), Source::new(&sts[1], None));
    let theirs = b.identity("P-1");
    assert_eq!(theirs, "dir:vault.secret/tags.list:P-1");
    let snapshot = json!({"graph": {"graph": {
        "assertions": {"b-1": {"id": "b-1", "subject": {"Node": "n1"},
            "predicate": {"Property": "p-email"},
            "object": {"Value": {"value_kind": "String", "value": "x@example.org"}},
            "evidence": ["e-1"], "lifecycle": "Active",
            "valid_time": {"from": 1, "to": null}}},
        "edges": {}, "nodes": {},
        "evidence": {"e-1": {"source": {"HumanStatement": {"identity": format!("record:{theirs}")}}}},
    }}});
    let ontology = json!({"node_types": [{"properties": [{"id": "p-email", "name": "email"}]}],
        "edge_types": []});
    let ops = a.ended(
        &snapshot,
        &ontology,
        &Default::default(),
        &Default::default(),
        &[],
    );
    assert!(
        ops.is_empty(),
        "source people ends a value of source tags: {ops:?}"
    );
}
