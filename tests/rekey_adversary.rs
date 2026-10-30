//! Adversary cases for `story:update-refuses-rekey`: spec changes that change how a source keys
//! its documents but are not caught by `keyed_by`, and one harmless change that is refused.
//! Each case builds an instance whose source has run, so its `state/<source>.json` holds the keys
//! the source really produced, then updates the spec.

mod common;

use std::path::{Path, PathBuf};

use common::{ekr, executable, World};
use serde_json::{json, Value};

/// A stand-in `claude` that fails when called; the structured runs below never call it.
const CLAUDE: &str = r#"echo '{"type":"result","subtype":"error","is_error":true}'
exit 1
"#;

/// Writes `<dir>/people.json` under the world's root, two records with an `id` and a `code`.
fn registry(w: &World, dir: &str) {
    std::fs::create_dir_all(w.root.join(dir)).unwrap();
    std::fs::write(
        w.root.join(dir).join("people.json"),
        json!({"people": [
            {"id": "p1", "code": "A-1", "name": "Ada", "team": "core"},
            {"id": "p2", "code": "A-2", "name": "Bo", "team": "edge"},
        ]})
        .to_string(),
    )
    .unwrap();
}

/// The spec of instance `name` with one structured source `registry`, reading `<paths>/*.json`,
/// its records identified by `id`.
fn structured_spec(w: &World, name: &str, paths: &str, id: &str) -> PathBuf {
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
  - name: registry
    schedule: daily
    settings:
      kind: structured
      value:
        input:
          from: files
          value: {{paths: [{paths}], glob: "*.json"}}
        records: "$.people"
        mapping:
          node_type: Person
          id: "{id}"
          name: "$.name"
          aliases: []
          properties:
            - {{property: team, path: "$.team"}}
          relations: []
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
serve: {{view_port: 18993}}
"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    path
}

fn create(w: &World, spec: &Path) {
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
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

/// After an update that should have been refused was accepted: the next run, to show that every
/// record is extracted again under new keys.
fn rerun_evidence(w: &World, name: &str, source: &str) -> String {
    let before = seen_keys(w, name, source);
    let ran = run(w, &format!("{name}/{source}"));
    let after = seen_keys(w, name, source);
    format!(
        "documents_new after the update: {}; seen keys before {before:?}, after {after:?}",
        ran["detail"]["documents_new"]
    )
}

/// A structured source's record id path (`mapping.id`) keys its documents exactly as a
/// `connectors` source's `id` does (`<prefix>:<raw id>`, `src/sources.rs` `structured_records`),
/// the change C-04 is about. Changing it while the source has seen records must be refused.
#[test]
#[ignore = "adversary: keyed_by misses a structured source's mapping.id; the update is accepted and every record is re-extracted under new keys"]
fn a_structured_mapping_id_change_with_seen_records_is_refused() {
    let w = World::new();
    executable(&w.bin.join("claude"), CLAUDE);
    registry(&w, "registry");
    let spec = structured_spec(&w, "s", "registry", "$.id");
    create(&w, &spec);
    run(&w, "s/registry");
    assert_eq!(seen_keys(&w, "s", "registry").len(), 2);

    structured_spec(&w, "s", "registry", "$.code");
    let out = update(&w, "s", &spec);
    if out.1["outcome"] == "updated" {
        panic!(
            "mapping.id change accepted: {}; {}",
            out.1,
            rerun_evidence(&w, "s", "registry")
        );
    }
    assert_eq!(
        (out.0, out.1["outcome"].as_str()),
        (1, Some("rekey-refused")),
        "{}",
        out.1
    );
}

/// A structured `files` input's `paths` are part of every record's key
/// (`files:<paths>:<glob>:<raw id>`, `src/structured.rs` `prefix_parts`), and `commands.md` says
/// a structured source's input is a re-key field. Moving the files while the source has seen
/// records must be refused.
#[test]
#[ignore = "adversary: keyed_by compares only the structured input's kind; a files input's paths or glob re-key every record and the update is accepted"]
fn a_structured_files_paths_change_with_seen_records_is_refused() {
    let w = World::new();
    executable(&w.bin.join("claude"), CLAUDE);
    registry(&w, "registry");
    let spec = structured_spec(&w, "f", "registry", "$.id");
    create(&w, &spec);
    run(&w, "f/registry");
    assert_eq!(seen_keys(&w, "f", "registry").len(), 2);

    registry(&w, "people");
    structured_spec(&w, "f", "people", "$.id");
    let out = update(&w, "f", &spec);
    if out.1["outcome"] == "updated" {
        panic!(
            "paths change accepted: {}; {}",
            out.1,
            rerun_evidence(&w, "f", "registry")
        );
    }
    assert_eq!(
        (out.0, out.1["outcome"].as_str()),
        (1, Some("rekey-refused")),
        "{}",
        out.1
    );
}

/// The world's spec `t` with its one source `issues` a `connectors` source keyed by `id`.
fn connectors_spec(w: &World, id: &str) -> PathBuf {
    let path = w.spec("t", "conn_test");
    let text = std::fs::read_to_string(&path).unwrap();
    let start = text.find("sources:\n").unwrap();
    let end = text.find("serve:").unwrap();
    let source = format!(
        r#"sources:
  - name: issues
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
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
"#
    );
    std::fs::write(&path, format!("{}{source}{}", &text[..start], &text[end..])).unwrap();
    path
}

/// `id: key` and `id: "$.key"` read the same value (`src/sources.rs` `at` strips `$.`), so the
/// keys are the same and nothing is re-extracted; the update is harmless and should be accepted.
#[test]
#[ignore = "adversary: keyed_by compares the id path as text; an equivalent spelling ($.key for key) is refused as a re-key"]
fn an_equivalent_id_path_spelling_is_not_a_rekey() {
    let w = World::new();
    let spec = connectors_spec(&w, "key");
    create(&w, &spec);
    let dir = w.home.join("instances/t/state");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("issues.json"),
        json!({"format": "cortex.seen/1", "documents": {
            "tavily:issues.list:I-1": {"hash": "h", "applied_at": 1_790_000_000_000_i64}}})
        .to_string(),
    )
    .unwrap();

    connectors_spec(&w, "$.key");
    let out = update(&w, "t", &spec);
    assert_eq!(
        (out.0, out.1["outcome"].as_str()),
        (0, Some("updated")),
        "{}",
        out.1
    );
}

/// The spec of instance `name` with one `files` source `docs` reading `<paths>/**/*.md`.
fn files_spec(w: &World, name: &str, paths: &str) -> PathBuf {
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
  - name: docs
    schedule: daily
    settings:
      kind: files
      value: {{paths: [{paths}], glob: "**/*.md"}}
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
serve: {{view_port: 18992}}
"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    path
}

/// A `files` source keys each document by its path as `paths` reach it (`src/sources.rs`
/// `whole_file`), so `paths: [docs]` and `paths: [./docs]` name the same files under other keys.
#[test]
#[ignore = "adversary: a files source's paths key its documents and are not compared; the update is accepted and every file is extracted again"]
fn a_files_source_paths_change_with_seen_documents_is_refused() {
    let w = World::new();
    std::fs::create_dir_all(w.root.join("docs")).unwrap();
    std::fs::write(
        w.root.join("docs/a.md"),
        "Example Labs develops the Widget engine.\n",
    )
    .unwrap();
    let spec = files_spec(&w, "d", "docs");
    let (code, created) = w.cortex(&[
        "create",
        "--spec",
        spec.to_str().unwrap(),
        "--no-extract",
        "--no-units",
    ]);
    assert_eq!(code, 0, "{created}");
    run(&w, "d/docs");
    assert_eq!(seen_keys(&w, "d", "docs").len(), 1);

    files_spec(&w, "d", "./docs");
    let out = update(&w, "d", &spec);
    if out.1["outcome"] == "updated" {
        panic!(
            "paths change accepted: {}; {}",
            out.1,
            rerun_evidence(&w, "d", "docs")
        );
    }
    assert_eq!(
        (out.0, out.1["outcome"].as_str()),
        (1, Some("rekey-refused")),
        "{}",
        out.1
    );
}
