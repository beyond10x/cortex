//! Adversary pass 2 on unit read of wave 20261030a (`story:one-store-read-per-run`, correction
//! 2bf0426a), at the merged head where unit cost re-applies replacements with nothing to replace
//! (`extract::reapply_rounds`). The story's safety condition: the run's grown index gives batch
//! k+1 the same `Store` hits a fresh `ekr snapshot` would. A node created by a re-apply round is in
//! the store, so the next batch that names it must be offered it as in the store, and one that
//! does not name it must not (D22).

mod common;

use std::process::Command;

use common::{ekr, executable, World};
use serde_json::Value;

/// The stand-in `claude`: copies each prompt to `prompt.log`. For the batch of record `alpha` it
/// answers one `!Property` fact `owner` of the `Service` named `widget`, marked `replaces: true`,
/// with no entity: `widget` is named by that fact alone. EKR rejects it for nothing to replace,
/// before anything is resolved, and cortex applies it again, which creates `widget`. Every other
/// batch gets no fact.
const CLAUDE: &str = r##"R="@ROOT@"
cat > "$R/prompt.cur"
cat "$R/prompt.cur" >> "$R/prompt.log"
echo "=== end of prompt ===" >> "$R/prompt.log"
F=""
if grep -q '#alpha' "$R/prompt.cur"; then
  id=$(awk '/evidence id / { id = $0; sub(/.*evidence id /, "", id); sub(/ .*/, "", id); print id; exit }' "$R/prompt.cur")
  F="{\"!Property\":{\"subject\":{\"node_type\":\"Service\",\"aliases\":[\"widget\"]},\"property\":\"owner\",\"value\":{\"value_kind\":\"String\",\"value\":\"Alice\"},\"replaces\":true,\"evidence\":[\"$id\"]}}"
fi
cat <<EOF
{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{"format":"ekr.extraction-document/1","ontology":{"node_types":[{"name":"Service","parents":[],"abstract_type":false,"properties":[{"name":"owner","value":{"value_kind":"String"},"cardinality":"One","required":false}]}],"edge_types":[]},"entities":[],"facts":[$F]}}
EOF
"##;

fn world() -> World {
    world_of(&["alpha", "beta"])
}

/// [`world`] with the records `ids`, in that order.
fn world_of(ids: &[&str]) -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(&w.bin.join("claude"), &CLAUDE.replace("@ROOT@", &root));
    std::fs::create_dir_all(w.root.join("repos")).unwrap();
    let records: String = ids
        .iter()
        .map(|id| {
            let text = format!("The {id} service runs part of billing. ").repeat(20);
            format!("{{\"id\": \"{id}\", \"text\": \"{text}\"}}\n")
        })
        .collect();
    std::fs::write(w.root.join("repos/list.jsonl"), records).unwrap();
    let spec = w.root.join("r.yaml");
    std::fs::write(
        &spec,
        format!(
            "format: cortex.instance/1\nname: r\ndescription: Test brain.\n\
             ekr: {{version: \"0.0.34\", bin: \"{bin}\"}}\nseed: {{documents: []}}\n\
             model: {{model: claude-haiku-4-5-20251001, budget_usd: \"1\", timeout_s: 60, \
             batch_chars: 1000}}\n\
             sources:\n  - name: repos\n    schedule: daily\n    settings:\n      kind: files\n      \
             value:\n        paths: [repos]\n        glob: \"*.jsonl\"\n        records: \
             {{format: JsonLines, id: id, text: [\"{{text}}\"], filters: []}}\n    \
             policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, \
             max_chars_per_document: 5000}}\n\
             serve: {{view_port: 18997}}\n",
            bin = ekr().display()
        ),
    )
    .unwrap();
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    w
}

fn prompts(w: &World) -> Vec<String> {
    std::fs::read_to_string(w.root.join("prompt.log"))
        .unwrap_or_default()
        .split("=== end of prompt ===\n")
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect()
}

/// The canonical names of the store's nodes, from a fresh `ekr snapshot`.
fn store_nodes(w: &World) -> Vec<String> {
    let dir = w.home.join("instances/r");
    let out = Command::new(ekr())
        .arg("snapshot")
        .env("EKR_HOST", dir.join("host.json"))
        .env("EKR_BACKEND", "sqlite")
        .env("EKR_STORE", dir.join("store.sqlite"))
        .output()
        .unwrap();
    assert!(out.status.success());
    let snap: Value = serde_json::from_slice(&out.stdout).unwrap();
    snap["graph"]["graph"]["nodes"]
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(_, n)| n["canonical_name"].as_str().map(str::to_string))
        .collect()
}

/// Batch 1 (record alpha) names `widget` only in a replacement with nothing to replace; the
/// re-apply round stores it, so `widget` is a node of the store when batch 2 (record beta) is
/// built. Batch 2 names only `beta` (its key and record id, `known::terms`), and store hits are
/// filtered by those terms, so a fresh snapshot offers it no `widget`; the run's grown index must
/// not either. Coordinator decision D22 retargeted this case; the positive direction, a batch
/// that names `widget`, is
/// [`a_node_created_by_a_reapply_round_is_a_store_hit_of_the_next_batch_that_names_it`].
#[test]
fn a_node_created_by_a_reapply_round_is_not_offered_to_a_batch_that_does_not_name_it() {
    let w = world();
    let (code, ran) = w.cortex(&["run", "r/repos"]);
    assert_eq!(code, 0, "{ran}");
    assert_eq!(ran["detail"]["facts_reapplied"], 1, "{ran}");
    assert!(
        store_nodes(&w).iter().any(|n| n == "widget"),
        "the re-apply created widget: {:?}; {ran}",
        store_nodes(&w)
    );
    let prompts = prompts(&w);
    assert_eq!(prompts.len(), 2, "two batches: {ran}");
    let second = &prompts[1];
    assert!(
        !second.contains("widget (in the store"),
        "batch 2 names only beta and is offered widget from the store:\n{}",
        second
            .split("Known entities:\n")
            .nth(1)
            .unwrap_or("(no known entities)")
            .lines()
            .take(5)
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The same shape where batch 2 names `widget`: its record's id is `widget`, a term of the batch
/// (`known::terms`), so a fresh snapshot would offer the node the re-apply round created. The
/// run's grown index must offer it too: each re-apply round's document and report feed it, as
/// the first apply's do.
#[test]
fn a_node_created_by_a_reapply_round_is_a_store_hit_of_the_next_batch_that_names_it() {
    let w = world_of(&["alpha", "widget"]);
    let (code, ran) = w.cortex(&["run", "r/repos"]);
    assert_eq!(code, 0, "{ran}");
    assert_eq!(ran["detail"]["facts_reapplied"], 1, "{ran}");
    assert!(store_nodes(&w).iter().any(|n| n == "widget"), "{ran}");
    let prompts = prompts(&w);
    assert_eq!(prompts.len(), 2, "two batches: {ran}");
    let known = prompts[1]
        .split("Known entities:\n")
        .nth(1)
        .unwrap_or("(no known entities)");
    assert!(
        known.contains("widget (in the store"),
        "batch 2 names widget and is not offered it from the store:\n{}",
        known.lines().take(5).collect::<Vec<_>>().join("\n")
    );
}
