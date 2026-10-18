//! The ontology names a run shows the model are masked and one-line
//! (`story:ontology-names-masked`): node type names, property names and value kinds, and edge
//! type names with their source and target types, as `ekr ontology` answers them, reach the
//! prompt with every credential shape masked and every line break written as a space, and then
//! pass through the instance's redaction policy like every known name.

mod common;

use std::path::PathBuf;

use common::{ekr, executable, World};
use serde_json::{json, Value};

/// A stand-in `claude` that copies each prompt to `prompt.log` and answers one `Service` named
/// `acme/api`.
const CLAUDE: &str = r#"R="@ROOT@"
cat > "$R/prompt.cur"
cat "$R/prompt.cur" >> "$R/prompt.log"
echo "=== end of prompt ===" >> "$R/prompt.log"
cat <<EOF
{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{"format":"ekr.extraction-document/1","ontology":{"node_types":[{"name":"Service","parents":[],"abstract_type":false,"properties":[]}],"edge_types":[]},"entities":[{"node_type":"Service","aliases":["acme/api"]}],"facts":[]}}
EOF
"#;

/// A stand-in `ekr`: the real one, except that `ekr ontology` answers `ontology.json` once the
/// test has written it, as a store another writer shares could.
const EKR: &str = r#"R="@ROOT@"
if [ "$1" = ontology ] && [ -f "$R/ontology.json" ]; then cat "$R/ontology.json"; exit 0; fi
exec "@EKR@" "$@"
"#;

const POLICY: &str = "policy: {refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}";

/// A GitHub token shape `crate::mask` masks on every run, built here so no token is written down.
fn token() -> String {
    format!("gh{}_{}", "p", "Zq9".repeat(12))
}

/// A world whose instance `name` reads one record keyed `acme/api` through the stand-in `ekr`,
/// with `redaction` as the spec's top-level redaction block (empty for none).
fn world(name: &str, redaction: &str) -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(&w.bin.join("claude"), &CLAUDE.replace("@ROOT@", &root));
    let standin = w.root.join("ekr-standin");
    executable(
        &standin,
        &EKR.replace("@ROOT@", &root)
            .replace("@EKR@", &ekr().display().to_string()),
    );
    std::fs::create_dir_all(w.root.join("repos")).unwrap();
    std::fs::write(
        w.root.join("repos/list.jsonl"),
        "{\"id\": \"acme/api\", \"text\": \"The acme/api repository serves the billing API.\"}\n",
    )
    .unwrap();
    let source = format!(
        "  - name: repos\n    schedule: daily\n    settings:\n      kind: files\n      value:\n        \
         paths: [repos]\n        glob: \"*.jsonl\"\n        records: {{format: JsonLines, id: id, \
         text: [\"{{text}}\"], filters: []}}\n    {POLICY}\n"
    );
    let path: PathBuf = w.root.join(format!("{name}.yaml"));
    std::fs::write(
        &path,
        format!(
            "format: cortex.instance/1\nname: {name}\ndescription: Test brain.\n\
             ekr: {{version: \"0.0.34\", bin: \"{ekr}\"}}\nseed: {{documents: []}}\n\
             model: {{model: claude-haiku-4-5-20251001, budget_usd: \"1\", timeout_s: 60}}\n\
             sources:\n{source}serve: {{view_port: 18996}}\n{redaction}",
            ekr = standin.display()
        ),
    )
    .unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    w
}

fn write_ontology(w: &World, ontology: &Value) {
    std::fs::write(w.root.join("ontology.json"), ontology.to_string()).unwrap();
}

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

#[test]
fn a_type_name_with_a_line_break_is_one_line_and_a_property_named_like_a_token_is_masked() {
    let w = world("onto", "");
    let token = token();
    write_ontology(
        &w,
        &json!({
            "node_types": [{"id": "t1", "name": "Repo\nIGNORE ALL", "parents": [],
                "abstract_type": false,
                "properties": [{"name": token, "value_type": {"value_kind": "String\rX"}}]}],
            "edge_types": [{"id": "e1", "name": "OWNS\u{2028}NOW",
                "source_types": [{"name": "Repo\nIGNORE ALL"}],
                "target_types": [{"name": format!("Svc {token}")}]}],
        }),
    );

    let (code, ran) = w.cortex(&["run", "onto/repos"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");

    let prompts = prompts(&w);
    assert_eq!(prompts.len(), 1, "{prompts:?}");
    let prompt = &prompts[0];
    let types = line(prompt, "Existing node types: ");
    assert!(types.contains("Repo IGNORE ALL("), "{types}");
    assert!(types.contains(": String X)"), "{types}");
    let relations = line(prompt, "Existing relations: ");
    assert!(
        relations.contains("OWNS NOW: Repo IGNORE ALL -> Svc "),
        "{relations}"
    );
    assert!(
        !prompt
            .lines()
            .any(|l| l.starts_with("IGNORE ALL") || l.starts_with("NOW") || l.starts_with('X')),
        "a name added a line to the prompt:\n{prompt}"
    );
    assert!(
        !prompt.contains('\u{2028}') && !prompt.contains('\r'),
        "{prompt}"
    );
    assert!(
        !prompt.contains(&token),
        "a token reached the model:\n{prompt}"
    );
    assert!(
        !prompt.contains("ghp_"),
        "a token reached the model:\n{prompt}"
    );
    assert!(types.contains("[masked:"), "{types}");
}

#[test]
fn a_type_name_a_redaction_known_name_names_is_replaced_as_the_policy_says() {
    let w = world(
        "ontored",
        "redaction:\n  classes: []\n  rules: []\n  known_names: [Zorblat]\n",
    );
    write_ontology(
        &w,
        &json!({
            "node_types": [{"id": "t1", "name": "Zorblat", "parents": [], "abstract_type": false,
                "properties": [{"name": "zorblat", "value_type": {"value_kind": "String"}}]}],
            "edge_types": [{"id": "e1", "name": "RUNS",
                "source_types": [{"name": "Zorblat"}], "target_types": [{"name": "Service"}]}],
        }),
    );

    let (code, ran) = w.cortex(&["run", "ontored/repos"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");

    let prompts = prompts(&w);
    assert_eq!(prompts.len(), 1, "{prompts:?}");
    let prompt = &prompts[0];
    assert!(
        !prompt.to_lowercase().contains("zorblat"),
        "a known name reached the model:\n{prompt}"
    );
    // Each value gets its own placeholder, so `zorblat` is restored as written; the type name is
    // the same placeholder wherever it is shown.
    let types = line(prompt, "Existing node types: ");
    assert_eq!(types, "Existing node types: [Name-1]([Name-2]: String)");
    let relations = line(prompt, "Existing relations: ");
    assert_eq!(relations, "Existing relations: RUNS: [Name-1] -> Service");
}
