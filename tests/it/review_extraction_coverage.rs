//! `story:extraction-coverage`: one round leaves one active value per single-valued property, and
//! the claims in thread replies and long multi-item messages reach the model with an instruction
//! to extract them.
//!
//! A refresh round read one merge request from several records: its tracker's record, a chat
//! message and a later reply. The model stated the merge request's `state` from each, none marked
//! `replaces`, and EKR 0.0.34 kept every value active: `ekr apply-extraction` adds a `!Property`
//! fact without `replaces` as one more assertion, and the kernel's cardinality check does not
//! count assertions. cortex now marks every fact of a property declared `cardinality: One`
//! `replaces: true` (`extract::mark_single_valued`), so EKR supersedes the stored value, rejects a
//! value older than it (`invalid-supersession`), and a value with nothing stored is applied again
//! by `extract::reapply_rounds` in time order. Every record here is synthetic.

use std::path::PathBuf;
use std::process::Command;

use crate::common::{ekr, executable, World};
use serde_json::Value;

/// A stand-in `claude` that reads its documents: a line `state: <value>` is a `!Property` fact
/// `state` (declared `cardinality: One`) of the merge request `demo!1`, and `label: <value>` one of
/// `label` (declared `Many`), neither marked `replaces`. Every document also gives one
/// `CHANGES` relation. It keeps each prompt as `prompt-<n>.txt` and its arguments in
/// `claude-args.log`.
const CLAUDE: &str = r##"R=$(cd "$(dirname "$0")/.." && pwd)
printf '%s' "$*" | tr '\n' ' ' >> "$R/claude-args.log"; echo >> "$R/claude-args.log"
n=$(ls "$R" | grep -c '^prompt-')
cat > "$R/prompt-$n.txt"
facts=$(awk '
BEGIN { m = "{\"node_type\":\"MergeRequest\",\"aliases\":[\"demo!1\"]}";
        r = "{\"node_type\":\"Repository\",\"aliases\":[\"demo\"]}" }
/evidence id / { id = $0; sub(/.*evidence id /, "", id); sub(/ .*/, "", id);
  printf "%s{\"!Relation\":{\"subject\":%s,\"relation\":\"CHANGES\",\"object\":%s,\"evidence\":[\"%s\"]}}", sep, m, r, id; sep = "," }
/^(state|label): / { p = $0; sub(/:.*/, "", p); v = $0; sub(/^[a-z]*: /, "", v);
  printf ",{\"!Property\":{\"subject\":%s,\"property\":\"%s\",\"value\":{\"value_kind\":\"String\",\"value\":\"%s\"},\"evidence\":[\"%s\"]}}", m, p, v, id }
' "$R/prompt-$n.txt")
cat <<EOF
{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{"format":"ekr.extraction-document/1","ontology":{"node_types":[{"name":"Repository","parents":[],"abstract_type":false,"properties":[]},{"name":"MergeRequest","parents":[],"abstract_type":false,"properties":[{"name":"state","value":{"value_kind":"String"},"cardinality":"One","required":false},{"name":"label","value":{"value_kind":"String"},"cardinality":"Many","required":false}]}],"edge_types":[{"name":"CHANGES","source_types":["MergeRequest"],"target_types":["Repository"],"cardinality":"Many","properties":[]}]},"entities":[{"node_type":"MergeRequest","aliases":["demo!1"]},{"node_type":"Repository","aliases":["demo"]}],"facts":[$facts]}}
EOF
"##;

/// An instance `review` with a `files` source over `chat/*.jsonl` (records with `id`, `time`,
/// `text` and, for a reply, `thread`), and the reading stand-in `claude`.
fn world() -> World {
    let w = World::new();
    executable(&w.bin.join("claude"), CLAUDE);
    std::fs::create_dir_all(w.root.join("chat")).unwrap();
    let path = w.root.join("review.yaml");
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: review
description: Test brain.
ekr: {{version: "0.0.34", bin: "{ekr}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
  - name: chat
    schedule: daily
    settings:
      kind: files
      value:
        paths: [chat]
        glob: "*.jsonl"
        records: {{format: JsonLines, id: id, time: time, text: ["{{text}}"], thread: thread, filters: []}}
    policy: {{refresh_after_days: 7, change: ContentHash, max_documents_per_run: 50, max_chars_per_document: 12000}}
serve: {{view_port: 18994}}
"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    w
}

/// Writes `records` as the export, then runs the source once: its report.
fn run_with(w: &World, records: &[String]) -> Value {
    std::fs::write(w.root.join("chat/export.jsonl"), records.join("\n") + "\n").unwrap();
    let (code, ran) = w.cortex(&["run", "review/chat"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    ran
}

/// One record: `text` is its lines.
fn record(id: &str, time: &str, text: &[&str]) -> String {
    serde_json::json!({"id": id, "time": time, "text": text.join("\n")}).to_string()
}

fn ekr_read(w: &World, verb: &str) -> Value {
    let dir: PathBuf = w.home.join("instances/review");
    let out = Command::new(ekr())
        .arg(verb)
        .env("EKR_HOST", dir.join("host.json"))
        .env("EKR_BACKEND", "sqlite")
        .env("EKR_STORE", dir.join("store.sqlite"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

/// The store's graph, with its ontology (`ekr ontology`) as `ontology`.
fn graph(w: &World) -> Value {
    let mut graph = ekr_read(w, "snapshot")["graph"]["graph"].clone();
    graph["ontology"] = ekr_read(w, "ontology");
    graph
}

/// The values of the store's active assertions of `property`, sorted, each with the record ids
/// its evidence was issued for.
fn active(graph: &Value, property: &str) -> Vec<(String, Vec<String>)> {
    let names: std::collections::BTreeMap<&str, &str> = graph["ontology"]["node_types"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|t| t["properties"].as_array().into_iter().flatten())
        .filter_map(|p| Some((p["id"].as_str()?, p["name"].as_str()?)))
        .collect();
    let record = |id: &Value| -> String {
        let item = &graph["evidence"][id.as_str().unwrap()];
        let identity = item["source"]
            .as_object()
            .into_iter()
            .flat_map(|s| s.values())
            .find_map(|s| s["identity"].as_str())
            .expect("an identity");
        identity.rsplit('#').next().unwrap().to_string()
    };
    let mut out: Vec<(String, Vec<String>)> = graph["assertions"]
        .as_object()
        .expect("assertions")
        .values()
        .filter(|a| a["lifecycle"] == "Active")
        .filter(|a| {
            a["predicate"]["Property"]
                .as_str()
                .is_some_and(|p| names.get(p) == Some(&property))
        })
        .map(|a| {
            (
                a["object"]["Value"]["value"].as_str().unwrap().to_string(),
                a["evidence"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(record)
                    .collect(),
            )
        })
        .collect();
    out.sort();
    out
}

/// The record ids the run lists in `rejected`, each with its refusal.
fn refusals(ran: &Value) -> Vec<(String, String)> {
    ran["detail"]["rejected"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| {
            (
                r["document"]
                    .as_str()
                    .unwrap()
                    .rsplit('#')
                    .next()
                    .unwrap()
                    .to_string(),
                r["refusal"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

/// The reviewed scenario: a tracker record says the merge request is merged; a later run reads a
/// chat message that says it is merged to main and not released. Neither fact is marked
/// `replaces`. One `state` stays active, the newer, while the two labels of the `Many` property
/// both stay.
#[test]
fn a_later_value_of_a_single_valued_property_leaves_one_active_value() {
    let w = world();
    let tracker = record("mr-1", "2026-01-02", &["state: merged", "label: backend"]);
    run_with(&w, std::slice::from_ref(&tracker));
    let chat = record(
        "msg-1",
        "2026-01-03",
        &["state: merged to main, not released", "label: urgent"],
    );
    let ran = run_with(&w, &[tracker, chat]);
    assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");

    let g = graph(&w);
    assert_eq!(
        active(&g, "state"),
        [(
            "merged to main, not released".to_string(),
            vec!["msg-1".to_string()]
        )],
        "{g}"
    );
    assert_eq!(
        active(&g, "label"),
        [
            ("backend".to_string(), vec!["mr-1".to_string()]),
            ("urgent".to_string(), vec!["msg-1".to_string()])
        ]
    );
}

/// One batch, an empty store: two records state two values, a third repeats the newest. One value
/// stays active, the newest, whatever order the records come in.
#[test]
fn values_of_one_batch_leave_the_newest_active() {
    let w = world();
    let ran = run_with(
        &w,
        &[
            record("msg-3", "2026-01-04", &["state: released"]),
            record("mr-1", "2026-01-02", &["state: merged"]),
            record("msg-2", "2026-01-05", &["state: released"]),
        ],
    );
    assert_eq!(ran["detail"]["documents_applied"], 3, "{ran}");
    assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");
    let g = graph(&w);
    assert_eq!(
        active(&g, "state"),
        [("released".to_string(), vec!["msg-2".to_string()])],
        "{g}"
    );
}

/// A record older than the stored value, read in a later run: EKR rejects its value as
/// `invalid-supersession`, the run lists it and goes on, and the newer value stays the one active.
#[test]
fn an_older_value_read_later_leaves_the_newer_active_and_is_listed() {
    let w = world();
    let newer = record("msg-1", "2026-01-03", &["state: released"]);
    run_with(&w, std::slice::from_ref(&newer));
    let ran = run_with(
        &w,
        &[newer, record("mr-1", "2026-01-02", &["state: merged"])],
    );
    let refusals = refusals(&ran);
    assert!(
        refusals.len() == 1
            && refusals[0].0 == "mr-1"
            && refusals[0].1.contains("invalid-supersession"),
        "{ran}"
    );
    let g = graph(&w);
    assert_eq!(
        active(&g, "state"),
        [("released".to_string(), vec!["msg-1".to_string()])],
        "{g}"
    );
}

/// The prompt side of coverage: every record of a thread, and every item of a long message that
/// lists several, reaches the model; each reply's own text comes before its `[context]`; and the
/// system prompt tells the model to extract every document's own facts, replies and list items
/// included, reading `[context]` as context. Measured: records whose own text reaches the prompt,
/// of the records, and list items that reach it, of the items.
#[test]
fn every_reply_and_every_list_item_reaches_the_prompt_with_the_rules_to_extract_them() {
    let w = world();
    let items: Vec<String> = (1..=12)
        .map(|n| format!("- item {n}: the widget build {n} passed its checks on the staging lane"))
        .collect();
    let mut status = vec!["Daily status"];
    status.extend(items.iter().map(String::as_str));
    let records = [
        record("t-1", "2026-01-02", &["Shall we merge demo!1 today?"]),
        serde_json::json!({"id": "t-2", "time": "2026-01-02T01:00:00Z",
            "text": "state: merged", "thread": "t-1"})
        .to_string(),
        serde_json::json!({"id": "t-3", "time": "2026-01-02T02:00:00Z",
            "text": status.join("\n"), "thread": "t-1"})
        .to_string(),
    ];
    let ran = run_with(&w, &records);
    assert_eq!(ran["detail"]["documents_applied"], 3, "{ran}");
    let prompt = std::fs::read_to_string(w.root.join("prompt-0.txt")).unwrap();
    let blocks: Vec<&str> = prompt
        .split("\n=== Document — evidence id ")
        .skip(1)
        .collect();
    assert_eq!(blocks.len(), 3, "{prompt}");
    let own = [
        "Shall we merge demo!1 today?",
        "state: merged",
        "Daily status",
    ];
    let reached = own
        .iter()
        .filter(|text| blocks.iter().any(|b| b.contains(*text)))
        .count();
    assert_eq!(reached, 3, "records whose own text reaches the prompt");
    let listed = items.iter().filter(|i| prompt.contains(i.as_str())).count();
    assert_eq!(listed, 12, "list items that reach the prompt");
    for (block, text) in blocks.iter().zip(own) {
        let at = block.find(text).expect("own text");
        if let Some(context) = block.find("[context]") {
            assert!(at < context, "own text before its context: {block}");
        }
    }

    let args = w.lines("claude-args.log").join("\n");
    for rule in [
        "A reply in a thread states facts of its own",
        "states the facts of every item",
        "A paragraph that starts with `[context]`",
    ] {
        assert!(args.contains(rule), "the system prompt holds {rule:?}");
    }
}
