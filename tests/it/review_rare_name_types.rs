//! `redaction.keep_store_types` (`story:rare-name-keeps-store-types`, decision D7): the node
//! types whose store names `RareName` never takes.
//!
//! The runbook consumer's run of 0.8.0 lost the facts about a customer named once in a batch:
//! `RareName` hid the name, and the model could not tell the customer from anyone else. With the
//! customer's node type listed, its names in the store at the run's head are kept as written; a
//! person the store knows, of a type the list does not name, stays hidden as before
//! (`tests/redaction_gate.rs`
//! `a_person_the_store_already_knows_is_not_shown_to_the_model_by_name`). Without the list a run
//! is as before.
//!
//! The model is a stand-in; no test calls a real model. Every name here is invented.

use crate::common;

use std::path::PathBuf;

use crate::common::{executable, World};
use serde_json::json;

/// A stand-in `claude` that copies each prompt to `prompt.log` and answers the extraction
/// document in the file `answer.json`, with `@ID@` replaced by the batch's first evidence id.
const CLAUDE: &str = r#"R="@ROOT@"
echo call >> "$R/claude-calls.log"
cat > "$R/prompt.cur"
cat "$R/prompt.cur" >> "$R/prompt.log"
echo "=== end of prompt ===" >> "$R/prompt.log"
id=$(grep -oE 'evidence id [0-9a-f-]{36}' "$R/prompt.cur" | head -n 1 | cut -d' ' -f3)
doc=$(sed "s/@ID@/$id/g" "$R/answer.json")
printf '{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":%s}\n' "$doc"
"#;

const POLICY: &str = "policy: {refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}";

/// The record of the second run: the customer and the person, each named once.
const SAID_ONCE: &str = "Quillfeather renewed the contract; Rowan Pell signed it.";

/// An extraction document of an `Organization` (with a `status`) and a `Person`, naming the
/// organization `org` and the person `person`, with `facts`.
fn extraction(org: &str, person: &str, facts: serde_json::Value) -> String {
    json!({
        "format": "ekr.extraction-document/1",
        "ontology": {"node_types": [
            {"name": "Organization", "parents": [], "abstract_type": false, "properties": [
                {"name": "status", "value": {"value_kind": "String"}, "cardinality": "One",
                 "required": false}]},
            {"name": "Person", "parents": [], "abstract_type": false, "properties": []}],
            "edge_types": []},
        "entities": [
            {"node_type": "Organization", "aliases": [org]},
            {"node_type": "Person", "aliases": [person]}],
        "facts": facts,
    })
    .to_string()
}

/// A world whose instance `name` reads `notes/list.jsonl` under `RareName` (and
/// `refuse_if_left: [RareName]`) with `keep` as the policy's further lines, after a first run
/// that put the `Organization` "Quillfeather" and the `Person` "Rowan Pell" in the store. The
/// second record, [`SAID_ONCE`], is written for the next run, whose stand-in answers `answer`.
fn seeded(name: &str, keep: &str, answer: &str) -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(&w.bin.join("claude"), &CLAUDE.replace("@ROOT@", &root));
    std::fs::create_dir_all(w.root.join("notes")).unwrap();
    write_records(&w, &[("r1", "a quiet week.")]);
    std::fs::write(
        w.root.join("answer.json"),
        extraction("Quillfeather", "Rowan Pell", json!([])),
    )
    .unwrap();
    let source = format!(
        "  - name: notes\n    schedule: daily\n    settings:\n      kind: files\n      value:\n        \
         paths: [notes]\n        glob: \"*.jsonl\"\n        records: {{format: JsonLines, id: id, \
         text: [\"{{text}}\"], filters: []}}\n    {POLICY}\n"
    );
    let path: PathBuf = w.root.join(format!("{name}.yaml"));
    std::fs::write(
        &path,
        format!(
            "format: cortex.instance/1\nname: {name}\ndescription: Test brain.\n\
             ekr: {{version: \"0.0.34\", bin: \"{ekr}\"}}\nseed: {{documents: []}}\n\
             model: {{model: claude-haiku-4-5-20251001, budget_usd: \"1\", timeout_s: 60}}\n\
             sources:\n{source}serve: {{view_port: 18993}}\n\
             redaction:\n  classes: [RareName]\n  rules: []\n  refuse_if_left: [RareName]\n{keep}",
            ekr = common::ekr().display()
        ),
    )
    .unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let (code, ran) = w.cortex(&["run", &format!("{name}/notes")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");

    write_records(&w, &[("r1", "a quiet week."), ("r2", SAID_ONCE)]);
    std::fs::write(w.root.join("answer.json"), answer).unwrap();
    w
}

fn write_records(w: &World, records: &[(&str, &str)]) {
    let lines: String = records
        .iter()
        .map(|(id, text)| format!("{}\n", json!({"id": id, "text": text})))
        .collect();
    std::fs::write(w.root.join("notes/list.jsonl"), lines).unwrap();
}

fn prompts(w: &World) -> Vec<String> {
    std::fs::read_to_string(w.root.join("prompt.log"))
        .unwrap_or_default()
        .split("=== end of prompt ===\n")
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect()
}

/// The second run of instance `name`: its outcome and the prompt it sent.
fn second_run(w: &World, name: &str) -> (serde_json::Value, String) {
    let (code, ran) = w.cortex(&["run", &format!("{name}/notes")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let all = prompts(w);
    assert_eq!(all.len(), 2, "one prompt per run: {all:?}");
    (ran, all[1].clone())
}

/// The restored extraction the second run applied.
fn second_extraction(w: &World, name: &str) -> String {
    let runs = w.home.join("instances").join(name).join("runs");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&runs)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    dirs.sort();
    assert_eq!(dirs.len(), 2, "{dirs:?}");
    std::fs::read_to_string(dirs[1].join("batch-0/extraction.yaml")).unwrap()
}

/// The story's acceptance: with `keep_store_types: [Organization]`, the customer the store holds
/// is shown as written though the page names it once, its fact is extracted and applied, and the
/// person the store holds stays placeholders in the text and in the known entities.
#[test]
fn a_customer_of_a_listed_type_said_once_keeps_its_name_and_its_facts() {
    let fact = json!([{"!Property": {
        "subject": {"node_type": "Organization", "aliases": ["Quillfeather"]},
        "property": "status",
        "value": {"value_kind": "String", "value": "renewed by [Name-1] [Name-2]"},
        "evidence": ["@ID@"]}}]);
    let answer = extraction("Quillfeather", "[Name-1] [Name-2]", fact);
    let w = seeded("kept", "  keep_store_types: [Organization]\n", &answer);
    let (ran, prompt) = second_run(&w, "kept");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    assert_eq!(ran["detail"]["unrestored"], 0, "{ran}");
    assert!(
        prompt.contains("Quillfeather renewed the contract; [Name-1] [Name-2] signed it."),
        "{prompt}"
    );
    for hidden in ["Rowan", "Pell"] {
        assert!(
            !prompt.contains(hidden),
            "{hidden} reached the model: {prompt}"
        );
    }
    let applied = second_extraction(&w, "kept");
    assert!(applied.contains("renewed by Rowan Pell"), "{applied}");
    assert!(applied.contains("- Quillfeather"), "{applied}");
    assert!(applied.contains("- Rowan Pell"), "{applied}");
    assert!(!applied.contains("Name-"), "{applied}");
}

/// A listed type keeps only its own nodes' names: with `keep_store_types: [Service]`, the person
/// and the customer the store holds, each named once, are hidden as before, and the
/// `refuse_if_left` check passes the batch.
#[test]
fn a_person_of_an_unlisted_type_the_store_knows_stays_hidden() {
    let answer = extraction("[Name-1]", "[Name-2] [Name-3]", json!([]));
    let w = seeded("unlisted", "  keep_store_types: [Service]\n", &answer);
    let (_, prompt) = second_run(&w, "unlisted");
    assert!(
        prompt.contains("[Name-1] renewed the contract; [Name-2] [Name-3] signed it."),
        "{prompt}"
    );
    for hidden in ["Quillfeather", "Rowan", "Pell"] {
        assert!(
            !prompt.contains(hidden),
            "{hidden} reached the model: {prompt}"
        );
    }
    let applied = second_extraction(&w, "unlisted");
    assert!(applied.contains("- Rowan Pell"), "{applied}");
    assert!(applied.contains("- Quillfeather"), "{applied}");
}

/// Without the field a run is as in 0.8.0: every name said once is hidden. An empty list keeps
/// nothing either (`redact::tests::keep_store_types_holds_only_where_rare_name_is_replaced_or_checked`).
#[test]
fn without_a_list_rare_name_hides_every_name_said_once_as_before() {
    let answer = extraction("[Name-1]", "[Name-2] [Name-3]", json!([]));
    let w = seeded("absent", "", &answer);
    let (_, prompt) = second_run(&w, "absent");
    assert!(
        prompt.contains("[Name-1] renewed the contract; [Name-2] [Name-3] signed it."),
        "{prompt}"
    );
    for hidden in ["Quillfeather", "Rowan", "Pell"] {
        assert!(
            !prompt.contains(hidden),
            "{hidden} reached the model: {prompt}"
        );
    }
}
