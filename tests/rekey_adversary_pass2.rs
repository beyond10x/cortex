//! Adversary cases, pass 2, for `story:update-refuses-rekey` after its correction: inputs to a
//! seen key that `keyed_by` (`src/main.rs`) still does not compare, and a harmless change it
//! refuses. Each case builds an instance whose source has seen documents, then updates the spec.

mod common;

use std::path::{Path, PathBuf};

use common::{ekr, executable, World};
use serde_json::{json, Value};

/// A stand-in `claude` that fails when called; the structured runs below never call it.
const CLAUDE: &str = r#"echo '{"type":"result","subtype":"error","is_error":true}'
exit 1
"#;

const POLICY: &str = "policy: {refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}";

/// The spec of instance `name` with `sources` (already indented as list items) and `extra`
/// top-level lines after `serve`.
fn instance_spec(w: &World, name: &str, sources: &str, extra: &str) -> PathBuf {
    let path = w.root.join(format!("{name}.yaml"));
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: {name}
description: Test brain.
ekr: {{version: "0.0.34", bin: "{ekr}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
{sources}serve: {{view_port: 18994}}
{extra}"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    path
}

fn create(w: &World, spec: &Path) {
    let (code, created) = w.cortex(&[
        "create",
        "--spec",
        spec.to_str().unwrap(),
        "--no-extract",
        "--no-units",
    ]);
    assert_eq!(
        (code, created["outcome"].as_str()),
        (0, Some("created")),
        "{created}"
    );
}

fn update(w: &World, name: &str, spec: &Path) -> (i32, Value) {
    w.cortex(&[
        "update",
        name,
        "--spec",
        spec.to_str().unwrap(),
        "--no-units",
    ])
}

fn run(w: &World, id: &str) -> Value {
    let (code, ran) = w.cortex(&["run", id]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    ran
}

/// The keys `state/<source>.json` of instance `name` holds.
fn seen_keys(w: &World, name: &str, source: &str) -> Vec<String> {
    let path = w
        .home
        .join("instances")
        .join(name)
        .join("state")
        .join(format!("{source}.json"));
    let state: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    state["documents"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect()
}

/// Writes `state/<source>.json` of instance `name` holding one seen document under `key`.
fn seed_state(w: &World, name: &str, source: &str, key: &str) {
    let dir = w.home.join("instances").join(name).join("state");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(format!("{source}.json")),
        json!({"format": "cortex.seen/1", "documents": {
            key: {"hash": "h", "applied_at": 1_790_000_000_000_i64}}})
        .to_string(),
    )
    .unwrap();
}

/// After an update that should have been refused was accepted: the next run, to show what it
/// extracts again and under which keys.
fn rerun_evidence(w: &World, name: &str, source: &str) -> String {
    let before = seen_keys(w, name, source);
    let (code, ran) = w.cortex(&["run", &format!("{name}/{source}")]);
    let after = seen_keys(w, name, source);
    format!(
        "rerun exit {code}, outcome {}, documents_new {}; seen keys before {before:?}, after {after:?}",
        ran["outcome"], ran["detail"]["documents_new"]
    )
}

/// One structured source `registry` reading `registry/*.json`, records identified by `$.id`.
fn registry_source() -> String {
    format!(
        r#"  - name: registry
    schedule: daily
    settings:
      kind: structured
      value:
        input:
          from: files
          value: {{paths: [registry], glob: "*.json"}}
        records: "$.people"
        mapping:
          node_type: Person
          id: "$.id"
          name: "$.name"
          aliases: []
          properties:
            - {{property: team, path: "$.team"}}
          relations: []
    {POLICY}
"#
    )
}

/// A structured record's stored key is its identity (`structured::Source::identity`), which is
/// `<prefix>:<raw id>` unless the instance's redaction policy would change the id, and then
/// `<prefix>:<digest>`. So adding an irreversible rule that matches the ids re-keys every seen
/// record, though no source field changed. The correction says every change to a field that goes
/// into a seen key is refused; this one is accepted.
#[test]
#[ignore = "story:ignored-adversary-limits: a redaction rule that rewrites structured ids of a source with seen records is not refused"]
fn a_redaction_rule_that_rewrites_structured_ids_with_seen_records_is_refused() {
    let w = World::new();
    executable(&w.bin.join("claude"), CLAUDE);
    std::fs::create_dir_all(w.root.join("registry")).unwrap();
    std::fs::write(
        w.root.join("registry/people.json"),
        json!({"people": [
            {"id": "p1", "name": "Ada", "team": "core"},
            {"id": "p2", "name": "Bo", "team": "edge"},
        ]})
        .to_string(),
    )
    .unwrap();
    let spec = instance_spec(&w, "r", &registry_source(), "");
    create(&w, &spec);
    run(&w, "r/registry");
    assert_eq!(seen_keys(&w, "r", "registry").len(), 2);

    instance_spec(
        &w,
        "r",
        &registry_source(),
        "redaction:\n  classes: []\n  rules:\n    - {name: pid, pattern: \"p[0-9]\", replacement: \"P\"}\n",
    );
    let out = update(&w, "r", &spec);
    if out.1["outcome"] == "updated" {
        panic!(
            "redaction rule over the ids accepted: {}; {}",
            out.1,
            rerun_evidence(&w, "r", "registry")
        );
    }
    assert_eq!(
        (out.0, out.1["outcome"].as_str()),
        (1, Some("rekey-refused")),
        "{}",
        out.1
    );
}

/// One `files` source `logs` reading `logs/*.jsonl` with `records` in `format`.
fn logs_source(format: &str) -> String {
    logs_source_keyed(format, "id")
}

/// [`logs_source`] with its records' `id` path `id`, or with no `records` when `format` is empty.
fn logs_source_keyed(format: &str, id: &str) -> String {
    let records = if format.is_empty() {
        String::new()
    } else {
        format!(
            "\n        records: {{format: {format}, id: \"{id}\", text: [\"{{text}}\"], filters: []}}"
        )
    };
    format!(
        r#"  - name: logs
    schedule: daily
    settings:
      kind: files
      value:
        paths: [logs]
        glob: "*.jsonl"{records}
    {POLICY}
"#
    )
}

/// A `logs` instance `name` over one JSON Lines file, created from `source`, whose state holds
/// the file's whole-file key.
fn seen_logs(w: &World, name: &str, source: &str) -> PathBuf {
    std::fs::create_dir_all(w.root.join("logs")).unwrap();
    std::fs::write(
        w.root.join("logs/a.jsonl"),
        "{\"id\": \"1\", \"other\": \"x\", \"text\": \"Example Labs develops the Widget engine.\"}\n",
    )
    .unwrap();
    let spec = instance_spec(w, name, source, "");
    create(w, &spec);
    let key = w.root.join("logs/a.jsonl").display().to_string();
    seed_state(w, name, "logs", &key);
    spec
}

/// Under `WholeFile` a file is one document keyed by its path and `records.id` is not read, so
/// changing it keys no seen document another way.
#[test]
fn a_records_id_change_under_whole_file_is_not_a_rekey() {
    let w = World::new();
    let spec = seen_logs(&w, "i", &logs_source_keyed("WholeFile", "id"));
    instance_spec(&w, "i", &logs_source_keyed("WholeFile", "$.other"), "");
    let out = update(&w, "i", &spec);
    assert_eq!(
        (out.0, out.1["outcome"].as_str()),
        (0, Some("updated")),
        "{}",
        out.1
    );
}

/// No `records` and `records: {format: WholeFile}` read a file alike (`spec-file.md`), so moving
/// between them keys no seen document another way.
#[test]
fn whole_file_records_and_no_records_are_one_keying() {
    let w = World::new();
    let spec = seen_logs(&w, "n", &logs_source_keyed("", "id"));
    instance_spec(&w, "n", &logs_source_keyed("WholeFile", "id"), "");
    let out = update(&w, "n", &spec);
    assert_eq!(
        (out.0, out.1["outcome"].as_str()),
        (0, Some("updated")),
        "{}",
        out.1
    );
}

/// `records.format` decides a `files` document's key: `WholeFile` keys the file by its path
/// (`src/sources.rs` `file_records` returns `whole_file`), `JsonLines` keys each record
/// `<path>#<id>`. `keyed_by` compares only whether `records` is set and `records.id`, so moving
/// from `WholeFile` to `JsonLines` is accepted, while the same key change from no `records` to
/// `JsonLines` is refused.
#[test]
fn a_files_records_format_change_with_seen_documents_is_refused() {
    let w = World::new();
    std::fs::create_dir_all(w.root.join("logs")).unwrap();
    std::fs::write(
        w.root.join("logs/a.jsonl"),
        "{\"id\": \"1\", \"text\": \"Example Labs develops the Widget engine.\"}\n\
         {\"id\": \"2\", \"text\": \"Widget engine ships on Fridays.\"}\n",
    )
    .unwrap();
    let spec = instance_spec(&w, "l", &logs_source("WholeFile"), "");
    create(&w, &spec);
    let dir = w.root.join("logs/a.jsonl").display().to_string();
    seed_state(&w, "l", "logs", &dir);

    instance_spec(&w, "l", &logs_source("JsonLines"), "");
    let out = update(&w, "l", &spec);
    if out.1["outcome"] == "updated" {
        panic!(
            "records.format WholeFile -> JsonLines accepted: {}; {}",
            out.1,
            rerun_evidence(&w, "l", "logs")
        );
    }
    assert_eq!(
        (out.0, out.1["outcome"].as_str()),
        (1, Some("rekey-refused")),
        "{}",
        out.1
    );
}

/// One `files` source `docs` over `paths` (written as a YAML flow list body), whole files.
fn docs_source(paths: &str) -> String {
    format!(
        r#"  - name: docs
    schedule: daily
    settings:
      kind: files
      value: {{paths: [{paths}], glob: "**/*.md"}}
    {POLICY}
"#
    )
}

/// A `files` document's key is the file's path (`<base>/<root>/<file>`), so adding a root keys
/// no seen document another way: every seen key is still produced, and only the new root's files
/// are new. `keyed_by` compares the whole `paths` list as text, so the update is refused, and the
/// refusal's only way forward (remove `state/<source>.json`) re-extracts every seen document,
/// the cost the story exists to prevent.
#[test]
fn adding_a_root_to_a_files_source_with_seen_documents_is_not_a_rekey() {
    let w = World::new();
    for dir in ["docs", "notes"] {
        std::fs::create_dir_all(w.root.join(dir)).unwrap();
    }
    std::fs::write(w.root.join("docs/a.md"), "Example Labs develops Widget.\n").unwrap();
    std::fs::write(w.root.join("notes/b.md"), "Widget ships on Fridays.\n").unwrap();
    let spec = instance_spec(&w, "a", &docs_source("docs"), "");
    create(&w, &spec);
    let key = w.root.join("docs").join("a.md").display().to_string();
    seed_state(&w, "a", "docs", &key);

    instance_spec(&w, "a", &docs_source("docs, notes"), "");
    let out = update(&w, "a", &spec);
    assert_eq!(
        (out.0, out.1["outcome"].as_str()),
        (0, Some("updated")),
        "{}",
        out.1
    );
}

/// One `connectors` source `issues` keyed by `id`.
fn issues_source(id: &str) -> String {
    format!(
        r#"  - name: issues
    schedule: daily
    settings:
      kind: connectors
      value:
        adapter: tavily
        connection: conn_test
        operation: issues.list
        inputs: [{{}}]
        records: issues
        id: "{id}"
        text: [summary]
    {POLICY}
"#
    )
}

/// `sources::at` reads `$` as the whole record and `$.$` as its member named `$`: two different
/// values. `record_path` maps both to `$`, so the change compares equal and is accepted.
#[test]
#[ignore = "story:ignored-adversary-limits: the whole record id `$` and a member literally named `$` are one path"]
fn the_whole_record_id_and_a_member_named_dollar_are_two_paths() {
    let w = World::new();
    let spec = instance_spec(&w, "d", &issues_source("$"), "");
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    seed_state(&w, "d", "issues", "tavily:issues.list:{\"key\":\"I-1\"}");

    instance_spec(&w, "d", &issues_source("$.$"), "");
    let out = update(&w, "d", &spec);
    assert_eq!(
        (out.0, out.1["outcome"].as_str()),
        (1, Some("rekey-refused")),
        "{}",
        out.1
    );
}
