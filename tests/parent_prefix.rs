//! A structured parent's identity prefix never reads as a credential to masking
//! (`story:parent-identity-prefix-unmasked`). A record's identity is `<adapter>:<operation>:<id>`
//! or `files:<paths>:<glob>:<id>`; an operation, a path or a glob ending in a credential's name
//! (`vault.secret`) reads with the `:` and the id after it as an assigned value, so the seen state
//! dropped the key on load and the unchanged record was applied again on every run. Such a part
//! is written in hex; every other part keeps its text, so the identities of other sources do not
//! change. Each case drives the `cortex` binary against a real `ekr` with a stand-in
//! `connectors`.

mod common;

use std::path::PathBuf;

use common::{ekr, executable, World};
use serde_json::{json, Value};

/// A stand-in `connectors` with two ready connections, `conn_test` of adapter `dir` and
/// `conn_vault` of adapter `vault-secret`, whose every operation answers `people.json`.
const CONNECTORS: &str = r#"R="@ROOT@"
case "$*" in
  *"connections list"*) echo '{"ok":true,"result":{"connections":[{"adapter":"dir","connection":"conn_test","state":"ready","revision":"rev1"},{"adapter":"vault-secret","connection":"conn_vault","state":"ready","revision":"rev1"}]}}' ;;
  *"connections revalidate"*) echo '{"ok":true,"result":{}}' ;;
  *"operations describe"*) echo '{"ok":true,"result":{"schema":"s","revision":"r"}}' ;;
  *"--operation "*) printf '{"ok":true,"result":{"adapter":"dir","operation":"op","revision":"r","result":%s}}\n' "$(cat "$R/people.json")" ;;
  *) echo '{"ok":false}'; exit 2 ;;
esac
"#;

/// A stand-in `claude` that logs the call and fails: a structured source never calls it.
const CLAUDE: &str = r#"echo call >> "@ROOT@/claude-calls.log"
echo '{"type":"result","subtype":"error","is_error":true}'
exit 1
"#;

/// Three people: one id too short to read as a credential's value, two long enough.
fn people() -> Value {
    json!({"people": [
        {"id": "P-1", "name": "Ada Lovelace"},
        {"id": "person-0002", "name": "Grace Hopper"},
        {"id": "person-0003", "name": "Linus Example"},
    ]})
}

fn world() -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(
        &w.bin.join("connectors"),
        &CONNECTORS.replace("@ROOT@", &root),
    );
    executable(&w.bin.join("claude"), &CLAUDE.replace("@ROOT@", &root));
    std::fs::write(w.root.join("people.json"), people().to_string()).unwrap();
    w
}

/// Writes the people as `<dir>/<file>` under the world's root.
fn people_file(w: &World, dir: &str, file: &str) {
    let dir = w.root.join(dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(file), people().to_string()).unwrap();
}

/// Creates instance `name` with one structured source `people` whose `input` is the YAML given.
fn create(w: &World, name: &str, input: &str) {
    let path = w.root.join(format!("{name}.yaml"));
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: {name}
description: Test brain.
ekr: {{version: "0.0.32", bin: "{ekr}"}}
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
          properties: []
          relations: []
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
serve: {{view_port: 18995}}
"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
}

fn connectors_input(operation: &str) -> String {
    connection_input("dir", "conn_test", operation)
}

fn connection_input(adapter: &str, connection: &str, operation: &str) -> String {
    format!(
        "          from: connectors\n          value: {{adapter: {adapter}, connection: \
         {connection}, operation: {operation}, inputs: [{{}}]}}"
    )
}

fn files_input(path: &str, glob: &str) -> String {
    format!("          from: files\n          value: {{paths: [\"{path}\"], glob: \"{glob}\"}}")
}

fn run(w: &World, name: &str) -> Value {
    let (code, ran) = w.cortex(&["run", &format!("{name}/people")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert!(
        w.lines("claude-calls.log").is_empty(),
        "the model was called"
    );
    ran
}

fn state_path(w: &World, name: &str) -> PathBuf {
    w.home
        .join("instances")
        .join(name)
        .join("state/people.json")
}

/// The keys of the seen state of source `people`, as the next run loads them.
fn seen(w: &World, name: &str) -> Vec<String> {
    cortex_cli::state::SeenState::load(&state_path(w, name))
        .unwrap()
        .documents
        .keys()
        .cloned()
        .collect()
}

/// Runs instance `name` twice over unchanged records: the first run applies the three people,
/// the seen state keeps all three identities, none of which masking would change, and the second
/// run applies none. Answers the identities.
fn applied_once(w: &World, name: &str) -> Vec<String> {
    let first = run(w, name);
    assert_eq!(first["detail"]["documents_applied"], 3, "{first}");
    let keys = seen(w, name);
    assert_eq!(keys.len(), 3, "the seen state lost an identity: {keys:?}");
    for key in &keys {
        assert_eq!(
            cortex_cli::mask::mask_key(key).1,
            0,
            "an identity masking would change: {key}"
        );
    }
    let again = run(w, name);
    assert_eq!(
        again["detail"]["documents_applied"], 0,
        "nothing changed, so nothing is applied: {again}"
    );
    keys
}

#[test]
fn a_connectors_source_whose_operation_ends_in_a_credential_name_applies_unchanged_records_once() {
    let w = world();
    create(&w, "o", &connectors_input("vault.secret"));
    let keys = applied_once(&w, "o");
    let hex = "dir:%x7661756c742e736563726574:";
    assert!(keys.iter().all(|k| k.starts_with(hex)), "{keys:?}");
}

/// The adapter is followed by `:` and the operation, which masking reads as the assigned value.
#[test]
fn a_connectors_source_whose_adapter_ends_in_a_credential_name_applies_unchanged_records_once() {
    let w = world();
    create(
        &w,
        "a",
        &connection_input("vault-secret", "conn_vault", "people.list"),
    );
    let keys = applied_once(&w, "a");
    let hex = "%x7661756c742d736563726574:people.list:";
    assert!(keys.iter().all(|k| k.starts_with(hex)), "{keys:?}");
}

#[test]
fn a_files_source_whose_path_ends_in_a_credential_name_applies_unchanged_records_once() {
    let w = world();
    people_file(&w, "vault/secret", "people.json");
    create(&w, "p", &files_input("vault/secret", "*.json"));
    let keys = applied_once(&w, "p");
    let hex = "files:%x7661756c742f736563726574:*.json:";
    assert!(keys.iter().all(|k| k.starts_with(hex)), "{keys:?}");
}

#[test]
fn a_files_source_whose_glob_ends_in_a_credential_name_applies_unchanged_records_once() {
    let w = world();
    people_file(&w, "registry", "people.secret");
    create(&w, "g", &files_input("registry", "*.secret"));
    let keys = applied_once(&w, "g");
    let hex = "files:registry:%x2a2e736563726574:";
    assert!(keys.iter().all(|k| k.starts_with(hex)), "{keys:?}");
}

/// A source no part of whose prefix masking reads keeps the identities it had before: the
/// operation, the path and the glob as written, `:` and `%` in a path escaped.
#[test]
fn a_source_with_no_such_part_keeps_its_identities_byte_for_byte() {
    let w = world();
    create(&w, "c", &connectors_input("people.list"));
    assert_eq!(
        applied_once(&w, "c"),
        [
            "dir:people.list:P-1",
            "dir:people.list:person-0002",
            "dir:people.list:person-0003"
        ]
    );

    people_file(&w, "a:b%c", "people.json");
    create(&w, "f", &files_input("a:b%c", "*.json"));
    assert_eq!(
        applied_once(&w, "f"),
        [
            "files:a%3Ab%25c:*.json:P-1",
            "files:a%3Ab%25c:*.json:person-0002",
            "files:a%3Ab%25c:*.json:person-0003"
        ]
    );
}

/// A child record applied before its source's prefix was written in hex has an identity under
/// the base prefix (`dir:vault.secret/tags.list:<parent>:<id>`). It is still the source's, so
/// its values end once it is gone, as a parent record's do.
#[test]
fn a_child_value_applied_under_the_base_prefix_still_ends_when_its_record_is_gone() {
    let yaml = format!(
        r#"format: cortex.instance/1
name: c
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
            children:
              - operation: tags.list
                input: {{project: "{{id}}"}}
                records: "$.tags"
                parent: IN_PROJECT
                mapping:
                  node_type: Tag
                  id: "$.name"
                  name: "$.name"
                  aliases: []
                  properties: [{{property: commit, path: "$.commit"}}]
                  relations: []
        records: "$.people"
        mapping:
          node_type: Person
          id: "$.id"
          name: "$.name"
          aliases: []
          properties: []
          relations: []
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
serve: {{view_port: 18995}}
"#,
        input = "          from: connectors\n          value:\n            adapter: dir\n            \
                 connection: conn_test\n            operation: vault.secret\n            inputs: [{}]"
    );
    let spec = cortex_cli::spec::parse(&yaml).expect("a valid spec");
    let cortex_model::instance::SourceSettings::Structured(st) = &spec.sources[0].settings else {
        unreachable!()
    };
    let s = cortex_cli::structured::Source::new(st, None);
    let snapshot = json!({"graph": {"graph": {
        "assertions": {"a-old": {"id": "a-old", "subject": {"Node": "n1"},
            "predicate": {"Property": "p-commit"},
            "object": {"Value": {"value_kind": "String", "value": "c1"}},
            "evidence": ["e-1"], "lifecycle": "Active",
            "valid_time": {"from": 1, "to": null}}},
        "edges": {},
        "nodes": {},
        "evidence": {"e-1": {"source": {"HumanStatement": {
            "identity": "record:dir:vault.secret/tags.list:P-1:v1"}}}},
    }}});
    let ontology = json!({"node_types": [{"properties": [{"id": "p-commit", "name": "commit"}]}],
        "edge_types": []});
    let ops = s.ended(
        &snapshot,
        &ontology,
        &Default::default(),
        &Default::default(),
        &[],
    );
    assert!(
        matches!(&ops[..], [cortex_cli::ekr::Operation::Retract { assertion, .. }] if assertion == "a-old"),
        "the value of a child record the source no longer lists is never ended: {ops:?}"
    );
}
