//! The gaps the runbook consumer found running cortex 0.8.0
//! (`story:placeholder-restore-and-retry-error`):
//!
//! - A model writes a placeholder back without the brackets it was shown (`Name-1` for
//!   `[Name-1]`) in a value or an alias. It is restored there as a bracketed one is; a bare
//!   placeholder the batch's mapping does not hold stays as written and is counted as unrestored.
//! - A model call that ends on `error_max_structured_output_retries` answers no `result` text.
//!   The batch's `claude/error.txt` and the run's reason name the subtype, the number of turns and
//!   the last validation message, masked.
//!
//! Finding 4 (`RareName` and the names the store holds) is `story:rare-name-keeps-store-types`, a
//! draft: exempting every store name breaks the rule that a known person stays hidden.
//!
//! The model is a stand-in; no test calls a real model. Every name here is invented.

mod common;

use std::path::PathBuf;

use common::{executable, World};
use serde_json::json;

/// A stand-in `claude` that copies each prompt to `prompt.log` and answers, citing the batch's
/// first evidence id, a `Runbook` entity with an `owner` property whose value is the content of
/// the file `value`, and a `Person` entity whose alias is the content of the file `alias`. When
/// the file `error.json` exists it answers that instead, exiting 1 as claude does.
const CLAUDE: &str = r#"R="@ROOT@"
echo call >> "$R/claude-calls.log"
cat > "$R/prompt.cur"
cat "$R/prompt.cur" >> "$R/prompt.log"
echo "=== end of prompt ===" >> "$R/prompt.log"
if [ -f "$R/error.json" ]; then cat "$R/error.json"; exit 1; fi
id=$(grep -oE 'evidence id [0-9a-f-]{36}' "$R/prompt.cur" | head -n 1 | cut -d' ' -f3)
value=$(cat "$R/value")
alias=$(cat "$R/alias")
cat <<EOF
{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{"format":"ekr.extraction-document/1","ontology":{"node_types":[{"name":"Runbook","parents":[],"abstract_type":false,"properties":[{"name":"owner","value":{"value_kind":"String"},"cardinality":"One","required":false}]},{"name":"Person","parents":[],"abstract_type":false,"properties":[]}],"edge_types":[]},"entities":[{"node_type":"Runbook","aliases":["deploy runbook"]},{"node_type":"Person","aliases":["$alias"]}],"facts":[{"!Property":{"subject":{"node_type":"Runbook","aliases":["deploy runbook"]},"property":"owner","value":{"value_kind":"String","value":"$value"},"evidence":["$id"]}}]}}
EOF
"#;

const POLICY: &str = "policy: {refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}";

const RARE: &str = "redaction:\n  classes: [RareName]\n  rules: []\n  refuse_if_left: [RareName]\n";

/// A world whose instance `name` reads the records of `notes/list.jsonl` (each `(id, text)` of
/// `records`) through the real `ekr`, under `RareName`, with the stand-in `claude` answering
/// `value` and `alias`.
fn world(name: &str, records: &[(&str, &str)], value: &str, alias: &str) -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(&w.bin.join("claude"), &CLAUDE.replace("@ROOT@", &root));
    std::fs::write(w.root.join("value"), value).unwrap();
    std::fs::write(w.root.join("alias"), alias).unwrap();
    std::fs::create_dir_all(w.root.join("notes")).unwrap();
    write_records(&w, records);
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
             sources:\n{source}serve: {{view_port: 18994}}\n{RARE}",
            ekr = common::ekr().display()
        ),
    )
    .unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
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

/// The run directories of instance `name`, oldest first.
fn run_dirs(w: &World, name: &str) -> Vec<PathBuf> {
    let runs = w.home.join("instances").join(name).join("runs");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&runs)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    dirs.sort();
    dirs
}

/// The one batch directory of the instance's one run.
fn batch_dir(w: &World, name: &str) -> PathBuf {
    let dirs = run_dirs(w, name);
    assert_eq!(dirs.len(), 1, "{dirs:?}");
    dirs[0].join("batch-0")
}

const RECORD: &str =
    "the deploy runbook for billing covers restarts; ask Ottilie Brannagh before a failover.";

#[test]
fn bare_placeholders_in_a_value_and_an_alias_are_restored_and_none_is_counted() {
    let w = world(
        "bare",
        &[("deploy", RECORD)],
        "Name-1 Name-2 owns the failover",
        "Name-1 Name-2",
    );
    let (code, ran) = w.cortex(&["run", "bare/notes"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    assert_eq!(ran["detail"]["unrestored"], 0, "{ran}");

    let prompt = &prompts(&w)[0];
    assert!(prompt.contains("ask [Name-1] [Name-2] before"), "{prompt}");
    let batch = batch_dir(&w, "bare");
    let answer = std::fs::read_to_string(batch.join("answer.json")).unwrap();
    assert!(
        answer.contains("\"Name-1 Name-2 owns the failover\""),
        "{answer}"
    );
    let applied = std::fs::read_to_string(batch.join("extraction.yaml")).unwrap();
    assert!(
        applied.contains("Ottilie Brannagh owns the failover"),
        "{applied}"
    );
    assert!(applied.contains("- Ottilie Brannagh"), "{applied}");
    assert!(!applied.contains("Name-"), "{applied}");
}

#[test]
fn a_bare_placeholder_the_mapping_does_not_hold_stays_as_written_and_is_counted() {
    let w = world(
        "orphan",
        &[("deploy", RECORD)],
        "Name-1 Name-2 hands over to Name-99",
        "Name-1 Name-2",
    );
    let (code, ran) = w.cortex(&["run", "orphan/notes"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["unrestored"], 1, "{ran}");
    let applied = std::fs::read_to_string(batch_dir(&w, "orphan").join("extraction.yaml")).unwrap();
    assert!(
        applied.contains("Ottilie Brannagh hands over to Name-99"),
        "{applied}"
    );
}

#[test]
fn exhausted_structured_output_retries_leave_a_diagnosable_error_txt_and_reason() {
    let w = world("retries", &[("deploy", RECORD)], "x", "y");
    // Assembled at runtime so no credential-shaped literal sits in the source.
    let token = format!("sk-{}", "Qw3rTy".repeat(5));
    let error = json!({
        "type": "result",
        "subtype": "error_max_structured_output_retries",
        "is_error": true,
        "num_turns": 6,
        "total_cost_usd": 0.8,
        "errors": [
            "an earlier attempt",
            format!("/facts/0: must have required property 'evidence' (seen near {token})"),
        ],
    });
    std::fs::write(w.root.join("error.json"), error.to_string()).unwrap();
    let (code, ran) = w.cortex(&["run", "retries/notes"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("extraction-failed")),
        "{ran}"
    );
    assert_eq!(
        w.lines("claude-calls.log").len(),
        1,
        "an answer with a cost is not asked again"
    );
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    let said = std::fs::read_to_string(batch_dir(&w, "retries").join("claude/error.txt")).unwrap();
    for text in [reason, said.as_str()] {
        assert!(
            text.contains("error_max_structured_output_retries"),
            "{text}"
        );
        assert!(text.contains("6 turns"), "{text}");
        assert!(
            text.contains("/facts/0: must have required property 'evidence'"),
            "{text}"
        );
        assert!(text.contains("[masked:api-key]"), "{text}");
        assert!(!text.contains(&token), "{text}");
        assert!(!text.contains("an earlier attempt"), "{text}");
    }
}
