//! A run reads the store once, not twice per batch (`story:one-store-read-per-run`): the known
//! entities of every batch come from one `ekr snapshot` taken at the first batch, which the run
//! keeps and updates after each apply. A stand-in `ekr` in front of the real one logs every
//! command; a stand-in `claude` names, in each batch, the record of the next batch.
//!
//! `ekr apply-extraction` reports no canonical name of a node it created or resolved
//! (`ekr.integrate.ExtractionReport` holds `committed`, `rejected`, `ambiguous`, `held` and
//! `stopped`, EKR 0.0.34), so the run updates its index from the applied document's named things:
//! a new node is named by the least, in byte order, of its members' first aliases, as EKR names
//! one it creates.

mod common;

use std::path::PathBuf;

use common::{ekr, executable, World};

/// The stand-in `claude`: copies each prompt to `prompt.log` and answers one `Service` entity
/// named by the next record's id (`beta` for the batch of record `alpha`, `gamma` for `beta`'s),
/// and none for the last.
const CLAUDE: &str = r#"R="@ROOT@"
cat > "$R/prompt.cur"
cat "$R/prompt.cur" >> "$R/prompt.log"
echo "=== end of prompt ===" >> "$R/prompt.log"
N=""
if grep -q '#beta' "$R/prompt.cur"; then N=gamma; elif grep -q '#alpha' "$R/prompt.cur"; then N=beta; fi
E=""
if [ -n "$N" ]; then E="{\"node_type\":\"Service\",\"aliases\":[\"$N\"]}"; fi
cat <<EOF
{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{"format":"ekr.extraction-document/1","ontology":{"node_types":[{"name":"Service","parents":[],"abstract_type":false,"properties":[]}],"edge_types":[]},"entities":[$E],"facts":[]}}
EOF
"#;

/// The stand-in `ekr`: logs its arguments, one command per line, to `ekr-argv.log`, then runs the
/// real `ekr`.
const EKR: &str = r#"printf '%s\n' "$*" >> "@ROOT@/ekr-argv.log"
exec "@EKR@" "$@"
"#;

const POLICY: &str = "policy: {refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}";

/// A world with instance `name` on a `files` source `repos` of three records, `alpha`, `beta` and
/// `gamma`, each of about 800 characters, so a `batch_chars` of 1,000 makes three batches;
/// `redaction` is the spec's `redaction` block, if any.
fn world(name: &str, redaction: &str) -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(&w.bin.join("claude"), &CLAUDE.replace("@ROOT@", &root));
    let stand_in = w.bin.join("ekr");
    executable(
        &stand_in,
        &EKR.replace("@ROOT@", &root)
            .replace("@EKR@", &ekr().display().to_string()),
    );
    std::fs::create_dir_all(w.root.join("repos")).unwrap();
    let records: String = ["alpha", "beta", "gamma"]
        .iter()
        .map(|id| {
            let text = format!("The {id} service runs part of billing. ").repeat(20);
            format!("{{\"id\": \"{id}\", \"text\": \"{text}\"}}\n")
        })
        .collect();
    std::fs::write(w.root.join("repos/list.jsonl"), records).unwrap();
    let spec = w.root.join(format!("{name}.yaml"));
    std::fs::write(
        &spec,
        format!(
            "format: cortex.instance/1\nname: {name}\ndescription: Test brain.\n\
             ekr: {{version: \"0.0.34\", bin: \"{bin}\"}}\nseed: {{documents: []}}\n\
             model: {{model: claude-haiku-4-5-20251001, budget_usd: \"1\", timeout_s: 60, \
             batch_chars: 1000}}\n\
             sources:\n  - name: repos\n    schedule: daily\n    settings:\n      kind: files\n      \
             value:\n        paths: [repos]\n        glob: \"*.jsonl\"\n        records: \
             {{format: JsonLines, id: id, text: [\"{{text}}\"], filters: []}}\n    {POLICY}\n\
             serve: {{view_port: 18994}}\n{redaction}",
            bin = stand_in.display()
        ),
    )
    .unwrap();
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    w
}

/// The `ekr` commands logged since the log was last removed.
fn commands(w: &World) -> Vec<String> {
    w.lines("ekr-argv.log")
}

fn argv_log(w: &World) -> PathBuf {
    w.root.join("ekr-argv.log")
}

fn prompts(w: &World) -> Vec<String> {
    std::fs::read_to_string(w.root.join("prompt.log"))
        .unwrap_or_default()
        .split("=== end of prompt ===\n")
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect()
}

/// The known-entity line of type `ty` in `prompt`, if any.
fn known_line<'p>(prompt: &'p str, ty: &str) -> Option<&'p str> {
    let known = prompt.split("Known entities:\n").nth(1)?;
    known
        .lines()
        .take_while(|l| l.starts_with("- "))
        .find(|l| l.starts_with(&format!("- {ty}: ")))
}

/// Acceptance 1: a three-batch run under a `refuse_if_left` policy, whose pre-check and batch
/// loop both look up each batch's known entities, issues at most one `ekr snapshot`.
#[test]
fn a_three_batch_run_under_refuse_if_left_reads_the_store_at_most_once() {
    let policy = "redaction:\n  classes: [Phone]\n  rules: []\n  refuse_if_left: [Phone]\n";
    let w = world("once", policy);
    let _ = std::fs::remove_file(argv_log(&w));

    let (code, ran) = w.cortex(&["run", "once/repos"]);
    assert_eq!(code, 0, "{ran}");
    assert_eq!(prompts(&w).len(), 3, "three batches: {ran}");
    assert_eq!(ran["detail"]["documents_applied"], 3, "{ran}");

    let commands = commands(&w);
    let snapshots = commands.iter().filter(|c| *c == "snapshot").count();
    assert!(
        snapshots <= 1,
        "{snapshots} `ekr snapshot` calls in one run: {commands:#?}"
    );
}

/// Acceptance 2: a name applied in batch k is offered to batch k+1 as a node in the store
/// (origin `Store`), from the run's index updated after the apply, not from a new snapshot.
#[test]
fn a_name_applied_in_one_batch_is_known_from_the_store_in_the_next() {
    for (name, redaction) in [
        ("plain", ""),
        (
            "gated",
            "redaction:\n  classes: [Phone]\n  rules: []\n  refuse_if_left: [Phone]\n",
        ),
    ] {
        let w = world(name, redaction);
        let _ = std::fs::remove_file(argv_log(&w));

        let (code, ran) = w.cortex(&["run", &format!("{name}/repos")]);
        assert_eq!(code, 0, "{name}: {ran}");
        let prompts = prompts(&w);
        assert_eq!(prompts.len(), 3, "{name}: {prompts:#?}");
        for (k, applied) in [(1, "beta"), (2, "gamma")] {
            let prompt = &prompts[k];
            let line = known_line(prompt, "Service")
                .unwrap_or_else(|| panic!("{name}: no known Service in batch {k}:\n{prompt}"));
            assert!(
                line.contains(&format!("{applied} (in the store)")),
                "{name}: batch {k} is offered {applied} from the store: {line}"
            );
        }
        let snapshots = commands(&w).iter().filter(|c| *c == "snapshot").count();
        assert!(snapshots <= 1, "{name}: {snapshots} snapshots");
    }
}
