//! Adversary cases for `story:parent-identity-prefix-unmasked`.
//!
//! The decided rule writes a prefix part in hex only when masking would change the identity, and
//! keeps the identities of every source masking does not touch. These cases drive
//! `structured::prefix` and `Source::ended` from a parsed spec.

use std::collections::{BTreeMap, BTreeSet};

use cortex_cli::ekr::Operation;
use cortex_cli::structured::{prefix, Source};
use cortex_model::instance as m;
use serde_json::json;

fn structured(input: &str) -> m::StructuredSource {
    let yaml = format!(
        r#"format: cortex.instance/1
name: adv
description: Test brain.
ekr: {{version: "0.0.32", bin: "ekr"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
  - name: people
    schedule: daily
    settings:
      kind: structured
      value:
        input:
{input}
        records: "$.people"
        mapping:
          node_type: Person
          id: "$.id"
          name: "$.name"
          aliases: []
          properties: [{{property: age, path: "$.age"}}]
          relations: []
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
serve: {{view_port: 18995}}
"#
    );
    let spec = cortex_cli::spec::parse(&yaml).expect("a valid spec");
    match spec.sources.into_iter().next().unwrap().settings {
        m::SourceSettings::Structured(st) => st,
        _ => unreachable!(),
    }
}

/// A `files` path that is not the last is followed by `,` in the identity, never by `:`, so
/// masking does not read it: `files:vault/secret,registry:*.json:<id>` is left as it is by
/// `mask_key`. Its identity on the base is that text, and the decided rule keeps it.
#[test]
fn adv_a_files_path_followed_by_a_comma_keeps_its_text() {
    let st = structured(
        "          from: files\n          value: {paths: [\"vault/secret\", \"registry\"], glob: \"*.json\"}",
    );
    let base = "files:vault/secret,registry:*.json";
    // Masking leaves the base identity of a long id alone, so the source works on the base.
    let key = format!("{base}:person-0002");
    assert_eq!(cortex_cli::mask::mask_key(&key).1, 0, "{key}");
    assert_eq!(prefix(&st), base);
}

/// On the base, record `P-1` of a source whose operation is `vault.secret` had identity
/// `dir:vault.secret:P-1` (an id too short for masking to read), so its evidence names
/// `record:dir:vault.secret:P-1`, and `ended` retracted its values once the record was gone.
/// After the prefix becomes hex, those assertions stay active for good.
#[test]
fn adv_a_value_applied_under_the_base_prefix_still_ends_when_its_record_is_gone() {
    let st = structured(
        "          from: connectors\n          value: {adapter: dir, connection: conn_test, operation: vault.secret, inputs: [{}]}",
    );
    let s = Source::new(&st, None);
    let snapshot = json!({"graph": {"graph": {
        "assertions": {"a-old": {"id": "a-old", "subject": {"Node": "n1"},
            "predicate": {"Property": "p-age"},
            "object": {"Value": {"value_kind": "String", "value": "36"}},
            "evidence": ["e-1"], "lifecycle": "Active",
            "valid_time": {"from": 1, "to": null}}},
        "edges": {},
        "nodes": {},
        "evidence": {"e-1": {"source": {"HumanStatement": {
            "identity": "record:dir:vault.secret:P-1"}}}},
    }}});
    let ontology = json!({"node_types": [{"properties": [{"id": "p-age", "name": "age"}]}],
        "edge_types": []});
    let ops = s.ended(
        &snapshot,
        &ontology,
        &BTreeMap::new(),
        &BTreeSet::new(),
        &[],
    );
    assert!(
        matches!(&ops[..], [Operation::Retract { assertion, .. }] if assertion == "a-old"),
        "the value of a record the source no longer lists is never ended: {ops:?}"
    );
}
