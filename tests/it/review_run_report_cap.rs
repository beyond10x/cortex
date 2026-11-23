//! What the runbook refresh needed of a run's report and its run directory
//! (`story:run-report-cap-and-refusals`):
//!
//! - A run that `max_documents_per_run` capped reports `stopped: document-cap` and the number of
//!   wanted documents it left for the next run (`documents_left`); a run that read every wanted
//!   document reports `stopped: null` and `0` left.
//! - A batch directory keeps each part EKR refused (`refusals.json`: the part, the outcome EKR
//!   named and its reason) beside `prompt.txt` and `answer.json`, and under a redaction policy the
//!   file holds placeholders, not the values they stand for.
//!
//! The model is a stand-in and the records are synthetic; no test calls a real model.

use std::path::PathBuf;

use crate::common::{ekr, executable, World};
use serde_json::{json, Value};

/// A stand-in `claude` that copies each prompt to `prompt.cur` and answers `answer.tpl` with
/// `@ID@` replaced by the batch's first evidence id and `@SHOWN@` by the first placeholder the
/// prompt shows (`[Email-1]` and the like), or `none`.
const CLAUDE: &str = r#"R="@ROOT@"
cat > "$R/prompt.cur"
echo call >> "$R/calls.log"
id=$(grep -oE 'evidence id [0-9a-f-]{36}' "$R/prompt.cur" | head -n 1 | cut -d' ' -f3)
shown=$(grep -oE '\[[A-Za-z]+-[0-9]+\]' "$R/prompt.cur" | head -n 1)
[ -n "$shown" ] || shown=none
printf '{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":'
sed -e "s/@ID@/$id/g" -e "s/@SHOWN@/$shown/g" "$R/answer.tpl"
printf '}\n'
"#;

/// One `Runbook` with an `owner` (a `String`), named by `@SHOWN@`, and its owner: every fact
/// applies.
const ANSWER_OK: &str = r#"{"format":"ekr.extraction-document/1","ontology":{"node_types":[{"name":"Runbook","parents":[],"abstract_type":false,"properties":[{"name":"owner","value":{"value_kind":"String"},"cardinality":"One","required":false}]}],"edge_types":[]},"entities":[{"node_type":"Runbook","aliases":["@SHOWN@ runbook"]}],"facts":[{"!Property":{"subject":{"node_type":"Runbook","aliases":["@SHOWN@ runbook"]},"property":"owner","value":{"value_kind":"String","value":"platform team"},"evidence":["@ID@"]}}]}"#;

/// [`ANSWER_OK`] and a second fact EKR refuses alone (`facts[1]`): a property the ontology does
/// not declare, its value the placeholder the prompt shows.
const ANSWER_REFUSED: &str = r#"{"format":"ekr.extraction-document/1","ontology":{"node_types":[{"name":"Runbook","parents":[],"abstract_type":false,"properties":[{"name":"owner","value":{"value_kind":"String"},"cardinality":"One","required":false}]}],"edge_types":[]},"entities":[{"node_type":"Runbook","aliases":["@SHOWN@ runbook"]}],"facts":[{"!Property":{"subject":{"node_type":"Runbook","aliases":["@SHOWN@ runbook"]},"property":"owner","value":{"value_kind":"String","value":"platform team"},"evidence":["@ID@"]}},{"!Property":{"subject":{"node_type":"Runbook","aliases":["@SHOWN@ runbook"]},"property":"owner","value":{"value_kind":"Integer","value":7},"evidence":["@ID@"]}}]}"#;

/// A world whose instance `name` reads `records` (one synthetic text each) from a files source
/// capped at `cap` documents per run, with `redaction` as the spec's top-level redaction block,
/// and the model answering `answer`.
fn world(name: &str, records: &[&str], cap: usize, redaction: &str, answer: &str) -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(&w.bin.join("claude"), &CLAUDE.replace("@ROOT@", &root));
    std::fs::write(w.root.join("answer.tpl"), answer).unwrap();
    std::fs::create_dir_all(w.root.join("notes")).unwrap();
    let lines: String = records
        .iter()
        .enumerate()
        .map(|(i, text)| format!("{}\n", json!({"id": format!("note-{i}"), "text": text})))
        .collect();
    std::fs::write(w.root.join("notes/list.jsonl"), lines).unwrap();
    let source = format!(
        "  - name: notes\n    schedule: daily\n    settings:\n      kind: files\n      value:\n        \
         paths: [notes]\n        glob: \"*.jsonl\"\n        records: {{format: JsonLines, id: id, \
         text: [\"{{text}}\"], filters: []}}\n    policy: {{refresh_after_days: 0, change: \
         ContentHash, max_documents_per_run: {cap}, max_chars_per_document: 5000}}\n"
    );
    let path: PathBuf = w.root.join(format!("{name}.yaml"));
    std::fs::write(
        &path,
        format!(
            "format: cortex.instance/1\nname: {name}\ndescription: Test brain.\n\
             ekr: {{version: \"0.0.34\", bin: \"{ekr}\"}}\nseed: {{documents: []}}\n\
             model: {{model: claude-haiku-4-5-20251001, budget_usd: \"1\", timeout_s: 60}}\n\
             sources:\n{source}serve: {{view_port: 18994}}\n{redaction}",
            ekr = ekr().display()
        ),
    )
    .unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    w
}

const FIVE: [&str; 5] = [
    "the deploy runbook covers restarts of the billing workers.",
    "the failover runbook names the standby region.",
    "the rollback runbook keeps the last three releases.",
    "the paging runbook lists the escalation order.",
    "the backup runbook checks the nightly copies.",
];

/// The batch directories of the instance's runs, oldest run first.
fn batch_dirs(w: &World, name: &str) -> Vec<PathBuf> {
    let runs = w.home.join("instances").join(name).join("runs");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&runs)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    dirs.sort();
    dirs.iter()
        .flat_map(|run| {
            let mut batches: Vec<PathBuf> = std::fs::read_dir(run)
                .unwrap()
                .map(|e| e.unwrap().path())
                .filter(|p| p.is_dir())
                .collect();
            batches.sort();
            batches
        })
        .collect()
}

fn run(w: &World, source: &str) -> Value {
    let (code, ran) = w.cortex(&["run", source]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    ran["detail"].clone()
}

#[test]
fn a_capped_run_reports_document_cap_and_the_documents_left_for_the_next_run() {
    let w = world("capped", &FIVE, 2, "", ANSWER_OK);

    let first = run(&w, "capped/notes");
    assert_eq!(first["documents_applied"], 2, "{first}");
    assert_eq!(first["stopped"], "document-cap", "{first}");
    assert_eq!(first["documents_left"], 3, "{first}");

    let second = run(&w, "capped/notes");
    assert_eq!(second["documents_applied"], 2, "{second}");
    assert_eq!(second["stopped"], "document-cap", "{second}");
    assert_eq!(second["documents_left"], 1, "{second}");

    // The last wanted document fits under the cap: nothing is left, and nothing stopped the run.
    let third = run(&w, "capped/notes");
    assert_eq!(third["documents_applied"], 1, "{third}");
    assert_eq!(third["stopped"], Value::Null, "{third}");
    assert_eq!(third["documents_left"], 0, "{third}");

    // The log line says the same.
    let log = std::fs::read_to_string(w.home.join("instances/capped/cortex.log")).unwrap();
    let lines: Vec<Value> = log
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .filter(|l: &Value| l["source"] == "notes")
        .collect();
    assert_eq!(lines.len(), 3, "{log}");
    assert_eq!(lines[0]["stopped"], "document-cap", "{log}");
    assert_eq!(lines[0]["documents_left"], 3, "{log}");
    assert_eq!(lines[2]["stopped"], Value::Null, "{log}");
}

#[test]
fn a_run_that_reads_every_wanted_document_reports_stopped_null_and_none_left() {
    let w = world("whole", &FIVE, 10, "", ANSWER_OK);
    let ran = run(&w, "whole/notes");
    assert_eq!(ran["documents_applied"], 5, "{ran}");
    assert_eq!(ran["stopped"], Value::Null, "{ran}");
    assert_eq!(ran["documents_left"], 0, "{ran}");

    // A cap exactly at the wanted count leaves nothing either: the run is not capped.
    let w = world("exact", &FIVE, 5, "", ANSWER_OK);
    let ran = run(&w, "exact/notes");
    assert_eq!(ran["documents_applied"], 5, "{ran}");
    assert_eq!(ran["stopped"], Value::Null, "{ran}");
    assert_eq!(ran["documents_left"], 0, "{ran}");
}

/// The rows of a batch directory's `refusals.json`.
fn refusals(batch: &std::path::Path) -> Vec<Value> {
    let path = batch.join("refusals.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        let report = std::fs::read_to_string(batch.join("report.json")).unwrap_or_default();
        panic!("{}: {e}; report.json: {report}", path.display())
    });
    serde_json::from_str::<Value>(&text)
        .unwrap()
        .as_array()
        .cloned()
        .unwrap_or_default()
}

#[test]
fn a_part_ekr_refuses_leaves_its_part_outcome_and_reason_in_the_batch_directory() {
    let w = world("refused", &FIVE[..1], 10, "", ANSWER_REFUSED);
    let ran = run(&w, "refused/notes");
    assert_eq!(ran["parts_rejected"], 1, "{ran}");
    assert_eq!(ran["documents_applied"], 1, "{ran}");

    let batches = batch_dirs(&w, "refused");
    assert_eq!(batches.len(), 1, "{batches:?}");
    let batch = &batches[0];
    for name in [
        "prompt.txt",
        "answer.json",
        "extraction.yaml",
        "refusals.json",
    ] {
        assert!(batch.join(name).is_file(), "{name} missing in {batch:?}");
    }
    let rows = refusals(batch);
    assert_eq!(rows.len(), 1, "{rows:?}");
    let row = &rows[0];
    assert_eq!(row["part"], "facts[1]", "{row}");
    assert_eq!(row["document"], "extraction.yaml", "{row}");
    assert_eq!(row["outcome"], "extraction-value-mismatch", "{row}");
    let reason = row["reason"].as_str().unwrap_or_default();
    assert!(!reason.is_empty(), "{row}");
    // The run report names the same refusal.
    let rejected = ran["rejected"][0]["refusal"].as_str().unwrap_or_default();
    assert!(rejected.contains("extraction-value-mismatch"), "{ran}");
}

#[test]
fn a_batch_without_a_refused_part_keeps_no_refusal_file() {
    let w = world("clean", &FIVE[..1], 10, "", ANSWER_OK);
    let ran = run(&w, "clean/notes");
    assert_eq!(ran["parts_rejected"], 0, "{ran}");
    let batches = batch_dirs(&w, "clean");
    assert!(!batches[0].join("refusals.json").exists(), "{batches:?}");
}

/// A record whose only replaced value is an address: the prompt shows `[Email-1]`, and the model
/// writes it back as the start of the runbook's name.
const ADDRESS: &str = "the deploy runbook is kept by ops.lead@example.org for the billing workers.";

#[test]
fn under_a_redaction_policy_the_refusal_file_holds_placeholders_only() {
    let w = world(
        "masked",
        &[ADDRESS],
        10,
        "redaction:\n  classes: [Email]\n  rules: []\n",
        ANSWER_REFUSED,
    );
    let ran = run(&w, "masked/notes");
    assert_eq!(ran["parts_rejected"], 1, "{ran}");
    assert_eq!(ran["redacted"]["Email"], 1, "{ran}");

    let batches = batch_dirs(&w, "masked");
    let batch = &batches[0];
    let prompt = std::fs::read_to_string(batch.join("prompt.txt")).unwrap();
    assert!(prompt.contains("[Email-1]"), "{prompt}");
    // The store holds the address: the answer was restored before it was applied.
    let applied = std::fs::read_to_string(batch.join("extraction.yaml")).unwrap();
    assert!(
        applied.contains("ops.lead@example.org runbook"),
        "{applied}"
    );

    let rows = refusals(batch);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0]["part"], "facts[1]", "{rows:?}");
    assert_eq!(rows[0]["outcome"], "extraction-value-mismatch", "{rows:?}");
    let text = std::fs::read_to_string(batch.join("refusals.json")).unwrap();
    assert!(!text.contains("ops.lead"), "{text}");
    assert!(!text.contains("example.org"), "{text}");
    // Every file the batch keeps holds placeholders only, the refusal file included.
    for name in ["prompt.txt", "answer.json", "refusals.json"] {
        let held = std::fs::read_to_string(batch.join(name)).unwrap();
        assert!(!held.contains("ops.lead@example.org"), "{name}: {held}");
    }
}
