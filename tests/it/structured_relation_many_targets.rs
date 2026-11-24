//! A structured mapping may name one relation with several target types
//! (`story:structured-relation-many-targets`): the ontology a batch proposes declares one edge
//! type of that name whose `target_types` lists every target type, and the batch commits. Before,
//! the top mapping declared one edge type per relation entry, two of one name, and EKR refused the
//! extraction with `extraction-name-duplicate`. No model is called.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::Command;

use crate::common::{ekr, executable, World};
use serde_json::Value;

/// A stand-in `connectors` with one ready `wiki` connection whose `pages.list` answers the file
/// `pages.json` as `{"pages": [...]}`.
const CONNECTORS: &str = r#"R="@ROOT@"
IN=""; if [ "$1" = invoke ]; then IN=$(cat); fi; A="$* $IN"
case "$A" in
  *"connections list"*) echo '{"ok":true,"result":{"connections":[{"adapter":"wiki","connector":"wiki","instance":"main","connection":"conn_test","state":"ready","revision":"rev1"}]}}' ;;
  *"connections revalidate"*) echo '{"ok":true,"result":{}}' ;;
  "invoke "*" pages.list "*)
    printf '{"ok":true,"result":{"connector":"wiki","instance":"main","operation":"pages.list","revision":"r","result":{"pages":%s}}}\n' "$(cat "$R/pages.json")" ;;
  *) echo '{"ok":false}'; exit 2 ;;
esac
"#;

/// A stand-in `claude` that logs the call and fails: a structured source never calls it.
const CLAUDE: &str = r#"echo call >> "@ROOT@/claude-calls.log"
echo '{"type":"result","subtype":"error","is_error":true}'
exit 1
"#;

/// Two pages; `alpha` links to the page `beta` and to the file `diagram.png`.
const PAGES: &str = r#"[
 {"id": "alpha", "name": "alpha", "page": "beta", "file": "diagram.png"},
 {"id": "beta", "name": "beta"}
]"#;

fn create(w: &World, name: &str) {
    let path = w.root.join(format!("{name}.yaml"));
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: {name}
description: Test brain.
ekr: {{version: "0.0.43", bin: "{ekr}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
  - name: pages
    schedule: daily
    settings:
      kind: structured
      value:
        input:
          from: connectors
          value: {{adapter: wiki, connection: conn_test, operation: pages.list, inputs: [{{}}]}}
        records: "$.pages"
        mapping:
          node_type: Page
          id: "$.id"
          name: "$.name"
          aliases: []
          properties: [{{property: title, path: "$.name"}}]
          relations:
            - {{relation: LINKS_TO, target_type: Page, target_name: "$.page"}}
            - {{relation: LINKS_TO, target_type: File, target_name: "$.file"}}
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
serve: {{view_port: 18993}}
"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
}

fn instance(w: &World, name: &str) -> PathBuf {
    w.home.join("instances").join(name)
}

fn ekr_json(w: &World, name: &str, args: &[&str]) -> Value {
    let dir = instance(w, name);
    let out = Command::new(ekr())
        .args(args)
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

#[test]
fn one_relation_name_with_two_target_types_is_one_edge_type_and_commits() {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(
        &w.bin.join("connectors"),
        &CONNECTORS.replace("@ROOT@", &root),
    );
    executable(&w.bin.join("claude"), &CLAUDE.replace("@ROOT@", &root));
    std::fs::write(w.root.join("pages.json"), PAGES).unwrap();
    create(&w, "s");

    let (code, ran) = w.cortex(&["run", "s/pages"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
    assert_eq!(ran["detail"]["facts_refused"], 0, "{ran}");
    assert!(
        w.lines("claude-calls.log").is_empty(),
        "the stand-in model was called"
    );

    // One edge type `LINKS_TO`, from `Page` to both `Page` and `File`.
    let ontology = ekr_json(&w, "s", &["ontology"]);
    let type_names: BTreeMap<String, String> = ontology["node_types"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            (
                t["id"].as_str().unwrap().to_string(),
                t["name"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    let links: Vec<&Value> = ontology["edge_types"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| t["name"] == "LINKS_TO")
        .collect();
    assert_eq!(links.len(), 1, "one edge type LINKS_TO: {ontology}");
    // An end is written as a type id, or as `{id, name}`.
    let names = |key: &str| -> BTreeSet<String> {
        links[0][key]
            .as_array()
            .unwrap_or_else(|| panic!("{key}: {ontology}"))
            .iter()
            .map(|t| {
                let id = t
                    .as_str()
                    .or_else(|| t["id"].as_str())
                    .unwrap_or_else(|| panic!("{key}: {ontology}"));
                type_names
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| id.to_string())
            })
            .collect()
    };
    assert_eq!(
        names("target_types"),
        BTreeSet::from(["File".to_string(), "Page".to_string()]),
        "{ontology}"
    );
    assert_eq!(
        names("source_types"),
        BTreeSet::from(["Page".to_string()]),
        "{ontology}"
    );

    // Both edges committed under it: alpha to the page beta and to the file diagram.png.
    let snapshot = ekr_json(&w, "s", &["snapshot"]);
    let graph = &snapshot["graph"]["graph"];
    let node_name = |id: &str| -> String {
        let n = &graph["nodes"][id];
        let canonical = n["canonical_name"].as_str().unwrap_or_default();
        format!(
            "{} {}",
            type_names[n["type_id"].as_str().unwrap()],
            canonical
        )
    };
    let link_id = links[0]["id"].as_str().unwrap();
    let edges: BTreeSet<(String, String)> = graph["edges"]
        .as_object()
        .unwrap()
        .values()
        .filter(|e| e["type_id"] == link_id)
        .map(|e| {
            (
                node_name(e["source"].as_str().unwrap()),
                node_name(e["target"].as_str().unwrap()),
            )
        })
        .collect();
    let edges: Vec<(String, String)> = edges.into_iter().collect();
    assert_eq!(edges.len(), 2, "{edges:?}");
    assert!(
        edges
            .iter()
            .all(|(source, _)| source.starts_with("Page alpha")),
        "{edges:?}"
    );
    let targets: Vec<&str> = edges.iter().map(|(_, t)| t.as_str()).collect();
    assert!(
        targets.iter().any(|t| t.starts_with("Page beta")),
        "{edges:?}"
    );
    assert!(
        targets.iter().any(|t| t.starts_with("File diagram.png")),
        "{edges:?}"
    );
}
