//! The gaps the runbook refresh probe found on cortex 0.7.2 (`story:runbook-probe-fixes`):
//!
//! - `RareName` leaves the store's ontology names alone: the prompt shows the node type, property
//!   and relation names as the store holds them, and a model that writes them back has its facts
//!   applied. A placeholder the model writes where a type, property or relation name goes, mangled
//!   or not, is restored before the answer is applied, or the batch fails naming it.
//! - A batch directory under `runs/` keeps the prompt as sent (`prompt.txt`) and the model's raw
//!   answer (`answer.json`) beside the extraction document, under a redaction policy too, and
//!   neither holds a value the policy replaced.
//! - `create --no-units` and `cortex list` name no viewer address for an instance without units.
//!
//! The model is a stand-in; no test calls a real model.

use crate::common;

use std::path::{Path, PathBuf};

use crate::common::{ekr, executable, World};
use serde_json::{json, Value};

/// A stand-in `claude` that copies each prompt to `prompt.log` and answers one entity and one
/// property fact citing the batch's first evidence id. Its node type is the first name on the
/// prompt's `Existing node types:` line with its brackets dropped, as a model writes a type name
/// as an identifier, or the content of the file `type` when it exists.
const CLAUDE: &str = r#"R="@ROOT@"
cat > "$R/prompt.cur"
cat "$R/prompt.cur" >> "$R/prompt.log"
echo "=== end of prompt ===" >> "$R/prompt.log"
id=$(grep -oE 'evidence id [0-9a-f-]{36}' "$R/prompt.cur" | head -n 1 | cut -d' ' -f3)
if [ -f "$R/type" ]; then ty=$(cat "$R/type"); else ty=$(sed -n 's/^Existing node types: \([^(;]*\).*/\1/p' "$R/prompt.cur" | tr -d '[]'); fi
cat <<EOF
{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{"format":"ekr.extraction-document/1","ontology":{"node_types":[{"name":"$ty","parents":[],"abstract_type":false,"properties":[{"name":"owner","value":{"value_kind":"String"},"cardinality":"One","required":false}]}],"edge_types":[]},"entities":[{"node_type":"$ty","aliases":["deploy runbook"]}],"facts":[{"!Property":{"subject":{"node_type":"$ty","aliases":["deploy runbook"]},"property":"owner","value":{"value_kind":"String","value":"platform team"},"evidence":["$id"]}}]}}
EOF
"#;

/// A stand-in `ekr`: the real one, except that `ekr ontology` answers `ontology.json`, as a store
/// an earlier run widened would.
const EKR: &str = r#"R="@ROOT@"
if [ "$1" = ontology ] && [ -f "$R/ontology.json" ]; then cat "$R/ontology.json"; exit 0; fi
exec "@EKR@" "$@"
"#;

const POLICY: &str = "policy: {refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}";

/// The record the source reads: two rare names, no other capitalised word, and none of the
/// ontology's names.
const RECORD: &str =
    "the deploy runbook for billing covers restarts; ask Quentin Marlow before a failover.";

/// A world whose instance `name` reads one record through the stand-in `ekr`, whose ontology
/// holds `Runbook(owner: String)` and `Service` and the relation `COVERS: Runbook -> Service`,
/// with `redaction` as the spec's top-level redaction block.
fn world(name: &str, redaction: &str) -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(&w.bin.join("claude"), &CLAUDE.replace("@ROOT@", &root));
    let standin = w.root.join("ekr-standin");
    executable(
        &standin,
        &[
            common::EKR_SESSION_SHIM,
            &EKR.replace("@ROOT@", &root)
                .replace("@EKR@", &ekr().display().to_string()),
        ]
        .concat(),
    );
    std::fs::write(
        w.root.join("ontology.json"),
        json!({
            "node_types": [
                {"id": "t1", "name": "Runbook", "parents": [], "abstract_type": false,
                 "properties": [{"name": "owner", "value_type": {"value_kind": "String"}}]},
                {"id": "t2", "name": "Service", "parents": [], "abstract_type": false,
                 "properties": []},
            ],
            "edge_types": [{"id": "e1", "name": "COVERS",
                "source_types": [{"name": "Runbook"}], "target_types": [{"name": "Service"}]}],
        })
        .to_string(),
    )
    .unwrap();
    std::fs::create_dir_all(w.root.join("notes")).unwrap();
    std::fs::write(
        w.root.join("notes/list.jsonl"),
        format!("{}\n", json!({"id": "deploy", "text": RECORD})),
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
             ekr: {{version: \"0.0.43\", bin: \"{ekr}\"}}\nseed: {{documents: []}}\n\
             model: {{model: claude-haiku-4-5-20251001, budget_usd: \"1\", timeout_s: 60}}\n\
             sources:\n{source}serve: {{view_port: 18995}}\n{redaction}",
            ekr = standin.display()
        ),
    )
    .unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    w
}

const RARE: &str = "redaction:\n  classes: [RareName]\n  rules: []\n  refuse_if_left: [RareName]\n";

fn prompts(w: &World) -> Vec<String> {
    std::fs::read_to_string(w.root.join("prompt.log"))
        .unwrap_or_default()
        .split("=== end of prompt ===\n")
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect()
}

/// The prompt line that starts with `head`.
fn line<'p>(prompt: &'p str, head: &str) -> &'p str {
    prompt
        .lines()
        .find(|l| l.starts_with(head))
        .unwrap_or_else(|| panic!("no line {head:?} in the prompt:\n{prompt}"))
}

/// The one batch directory of the instance's one run.
fn batch_dir(w: &World, name: &str) -> PathBuf {
    let runs = w.home.join("instances").join(name).join("runs");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&runs)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(dirs.len(), 1, "{dirs:?}");
    let batch = dirs.pop().unwrap().join("batch-0");
    assert!(batch.is_dir(), "{}", batch.display());
    batch
}

/// Every file under `dir`, by its path relative to `dir`.
fn files(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push(p.strip_prefix(dir).unwrap().display().to_string());
            }
        }
    }
    out.sort();
    out
}

#[test]
fn rare_name_leaves_the_ontology_names_and_a_model_echoing_them_has_its_facts_applied() {
    let w = world("rare", RARE);

    let (code, ran) = w.cortex(&["run", "rare/notes"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let detail = &ran["detail"];
    assert_eq!(detail["documents_applied"], 1, "{ran}");
    assert_eq!(detail["unrestored"], 0, "{ran}");
    assert_eq!(detail["parts_rejected"], 0, "{ran}");
    assert_eq!(detail["facts_refused"], 0, "{ran}");
    assert_eq!(detail["redacted"]["RareName"], 2, "{ran}");

    let prompts = prompts(&w);
    assert_eq!(prompts.len(), 1, "{prompts:?}");
    let prompt = &prompts[0];
    assert_eq!(
        line(prompt, "Existing node types: "),
        "Existing node types: Runbook(owner: String); Service"
    );
    assert_eq!(
        line(prompt, "Existing relations: "),
        "Existing relations: COVERS: Runbook -> Service"
    );
    // The record's rare names are still kept from the model.
    for name in ["Quentin", "Marlow"] {
        assert!(
            !prompt.contains(name),
            "{name} reached the model:\n{prompt}"
        );
    }

    let applied = std::fs::read_to_string(batch_dir(&w, "rare").join("extraction.yaml")).unwrap();
    assert!(applied.contains("node_type: Runbook"), "{applied}");
    assert!(!applied.contains("Name-"), "{applied}");
}

#[test]
fn a_batch_directory_keeps_the_prompt_as_sent_and_the_raw_answer_and_no_original() {
    let w = world("kept", RARE);
    let (code, ran) = w.cortex(&["run", "kept/notes"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");

    let batch = batch_dir(&w, "kept");
    let listed = files(&batch);
    for name in ["prompt.txt", "answer.json", "extraction.yaml"] {
        assert!(
            listed.iter().any(|f| f == name),
            "{name} missing: {listed:?}"
        );
    }
    // Nothing else of the batch is kept under a policy: no EKR report, no mapping.
    assert!(
        listed
            .iter()
            .all(
                |f| ["prompt.txt", "answer.json", "extraction.yaml"].contains(&f.as_str())
                    || f.starts_with("claude/")
            ),
        "{listed:?}"
    );

    let prompt = std::fs::read_to_string(batch.join("prompt.txt")).unwrap();
    let sent = std::fs::read_to_string(w.root.join("prompt.cur")).unwrap();
    assert_eq!(prompt, sent, "prompt.txt is the prompt as sent");
    assert!(prompt.contains("ask [Name-1] [Name-2] before"), "{prompt}");

    let answer: Value =
        serde_json::from_str(&std::fs::read_to_string(batch.join("answer.json")).unwrap()).unwrap();
    assert_eq!(answer["format"], "ekr.extraction-document/1", "{answer}");
    assert_eq!(answer["entities"][0]["node_type"], "Runbook", "{answer}");

    for file in &listed {
        let text = std::fs::read_to_string(batch.join(file)).unwrap();
        for name in ["Quentin", "Marlow"] {
            assert!(!text.contains(name), "{file} holds {name}:\n{text}");
        }
    }
}

#[test]
fn a_placeholder_written_as_a_type_name_is_restored_before_the_answer_is_applied() {
    // A known name that is also a type name is replaced in the prompt, as the policy says; the
    // model writes its placeholder back without brackets.
    let w = world(
        "known",
        "redaction:\n  classes: [RareName]\n  rules: []\n  known_names: [Runbook]\n",
    );
    let (code, ran) = w.cortex(&["run", "known/notes"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    assert_eq!(ran["detail"]["unrestored"], 0, "{ran}");

    let prompt = &prompts(&w)[0];
    assert!(
        line(prompt, "Existing node types: ").starts_with("Existing node types: [Name-"),
        "{prompt}"
    );
    let answer = std::fs::read_to_string(batch_dir(&w, "known").join("answer.json")).unwrap();
    assert!(answer.contains("\"node_type\": \"Name-"), "{answer}");
    let applied = std::fs::read_to_string(batch_dir(&w, "known").join("extraction.yaml")).unwrap();
    assert!(applied.contains("node_type: Runbook"), "{applied}");
    assert!(!applied.contains("Name-"), "{applied}");
}

#[test]
fn a_placeholder_with_no_value_in_a_type_position_fails_the_batch_naming_it() {
    let w = world("orphan", RARE);
    std::fs::write(w.root.join("type"), "Name-32").unwrap();
    let (code, ran) = w.cortex(&["run", "orphan/notes"]);
    assert_ne!(code, 0, "{ran}");
    assert_eq!(ran["outcome"], "extraction-failed", "{ran}");
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("Name-32") && reason.contains("node type"),
        "{ran}"
    );
}

#[test]
fn create_without_units_and_list_name_no_viewer_and_with_units_both_name_it() {
    let w = World::new();
    let spec = w.spec("bare", "conn_test");
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    assert_eq!(created["detail"]["view"], Value::Null, "{created}");
    let (code, listed) = w.cortex(&["list"]);
    assert_eq!(code, 0, "{listed}");
    assert_eq!(listed["name"], "bare", "{listed}");
    assert_eq!(listed["view"], Value::Null, "{listed}");

    let w = World::new();
    let spec = w.spec("served", "conn_test");
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap()]);
    assert_eq!(code, 0, "{created}");
    assert_eq!(
        created["detail"]["view"], "http://127.0.0.1:18999/",
        "{created}"
    );
    let (code, listed) = w.cortex(&["list"]);
    assert_eq!(code, 0, "{listed}");
    assert_eq!(listed["view"], "http://127.0.0.1:18999/", "{listed}");
}
