//! The model sees the store's existing entities for a batch's keys, and a structured mapping can
//! add templated aliases (`story:store-entities-known-to-model`): a node another writer put in the
//! store is shown to the model as a known entity of its own type when a batch's record key names
//! it, so a model that reuses known names creates no node; every store hit is shown however many
//! names the run itself knows of that type; a structured record's template alias reaches the
//! store's node of that alias; and a template of another placeholder syntax is refused when the
//! spec is read.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{ekr, executable, World};
use serde_json::Value;

/// A stand-in `claude` that copies each prompt to `prompt.log` and answers one entity named
/// `acme/api`: of the node type the prompt lists it under as a known entity, or of type `Service`
/// when the prompt lists no such entity. It never creates a type it was shown.
const CLAUDE: &str = r#"R="@ROOT@"
cat > "$R/prompt.cur"
cat "$R/prompt.cur" >> "$R/prompt.log"
echo "=== end of prompt ===" >> "$R/prompt.log"
T=$(grep -E '^- [A-Za-z]+: (.*; )?acme/api( |;|$)' "$R/prompt.cur" | head -1 | sed 's/^- \([A-Za-z]*\):.*/\1/')
ONT=""
if [ -z "$T" ]; then T=Service; ONT='{"name":"Service","parents":[],"abstract_type":false,"properties":[]}'; fi
cat <<EOF
{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{"format":"ekr.extraction-document/1","ontology":{"node_types":[$ONT],"edge_types":[]},"entities":[{"node_type":"$T","aliases":["acme/api"]}],"facts":[]}}
EOF
"#;

/// The node another writer put in the store: a `Repository` named `acme/api`, also known as
/// `repo:acme/api`.
const FOREIGN: &str = r#"format: ekr.extraction-document/1
ontology:
  node_types:
  - {name: Repository, parents: [], abstract_type: false, properties: []}
  edge_types: []
entities:
- {node_type: Repository, aliases: [acme/api, "repo:acme/api"]}
facts: []
evidence: []
"#;

const POLICY: &str = "policy: {refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}";

fn world() -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(&w.bin.join("claude"), &CLAUDE.replace("@ROOT@", &root));
    w
}

fn instance(w: &World, name: &str) -> PathBuf {
    w.home.join("instances").join(name)
}

/// `ekr <args>` on instance `name`'s store, as JSON.
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

/// Writes [`FOREIGN`] into instance `name`'s store with `ekr` itself, as another writer would.
fn foreign_writer(w: &World, name: &str) {
    let doc = w.root.join("foreign.yaml");
    std::fs::write(&doc, FOREIGN).unwrap();
    let report = ekr_json(w, name, &["apply-extraction", doc.to_str().unwrap()]);
    assert!(
        report["rejected"].as_array().is_some_and(Vec::is_empty)
            && report["committed"]
                .as_array()
                .is_some_and(|c| !c.is_empty()),
        "{report}"
    );
}

/// Every node of instance `name`'s store: `(type name, canonical name, aliases)`.
fn nodes(w: &World, name: &str) -> Vec<(String, String, Vec<String>)> {
    let ontology = ekr_json(w, name, &["ontology"]);
    let snapshot = ekr_json(w, name, &["snapshot"]);
    let type_name = |id: &str| -> String {
        ontology["node_types"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|t| t["id"] == id)
            .and_then(|t| t["name"].as_str())
            .unwrap_or("?")
            .to_string()
    };
    snapshot["graph"]["graph"]["nodes"]
        .as_object()
        .into_iter()
        .flatten()
        .map(|(_, n)| {
            (
                type_name(n["type_id"].as_str().unwrap_or_default()),
                n["canonical_name"].as_str().unwrap_or_default().to_string(),
                n["aliases"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect(),
            )
        })
        .collect()
}

fn prompts(w: &World) -> Vec<String> {
    std::fs::read_to_string(w.root.join("prompt.log"))
        .unwrap_or_default()
        .split("=== end of prompt ===\n")
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect()
}

fn write_spec(w: &World, name: &str, sources: &str) -> PathBuf {
    let path = w.root.join(format!("{name}.yaml"));
    std::fs::write(
        &path,
        format!(
            "format: cortex.instance/1\nname: {name}\ndescription: Test brain.\n\
             ekr: {{version: \"0.0.34\", bin: \"{ekr}\"}}\nseed: {{documents: []}}\n\
             model: {{model: claude-haiku-4-5-20251001, budget_usd: \"1\", timeout_s: 60}}\n\
             sources:\n{sources}serve: {{view_port: 18995}}\n",
            ekr = ekr().display()
        ),
    )
    .unwrap();
    path
}

fn create(w: &World, name: &str, sources: &str) {
    let path = write_spec(w, name, sources);
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
}

/// A `files` source `repos` reading `repos/*.jsonl` as records keyed by `id`.
fn records_source() -> String {
    format!(
        "  - name: repos\n    schedule: daily\n    settings:\n      kind: files\n      value:\n        \
         paths: [repos]\n        glob: \"*.jsonl\"\n        records: {{format: JsonLines, id: id, \
         text: [\"{{text}}\"], filters: []}}\n    {POLICY}\n"
    )
}

/// A world with instance `name` on [`records_source`], whose one record's key is `acme/api`, and
/// the store holding [`FOREIGN`].
fn records_world(name: &str) -> World {
    let w = world();
    std::fs::create_dir_all(w.root.join("repos")).unwrap();
    std::fs::write(
        w.root.join("repos/list.jsonl"),
        "{\"id\": \"acme/api\", \"text\": \"The acme/api repository serves the billing API.\"}\n",
    )
    .unwrap();
    create(&w, name, &records_source());
    foreign_writer(&w, name);
    w
}

/// The known-entity line of type `ty` in `prompt`, if any.
fn known_line<'p>(prompt: &'p str, ty: &str) -> Option<&'p str> {
    let known = prompt.split("Known entities:\n").nth(1)?;
    known
        .lines()
        .take_while(|l| l.starts_with("- "))
        .find(|l| l.starts_with(&format!("- {ty}: ")))
}

#[test]
fn a_node_another_writer_stored_is_shown_under_its_own_type_and_a_reusing_model_adds_no_node() {
    let w = records_world("kn");
    let before = nodes(&w, "kn");
    assert_eq!(
        before
            .iter()
            .filter(|(ty, name, _)| ty == "Repository" && name == "acme/api")
            .count(),
        1,
        "{before:?}"
    );

    let (code, ran) = w.cortex(&["run", "kn/repos"]);
    assert_eq!(code, 0, "{ran}");

    let prompts = prompts(&w);
    assert_eq!(prompts.len(), 1, "{prompts:?}");
    let line = known_line(&prompts[0], "Repository")
        .unwrap_or_else(|| panic!("no known Repository in the prompt:\n{}", prompts[0]));
    assert!(line.contains("acme/api"), "{line}");
    assert!(line.contains("repo:acme/api"), "the aliases are shown: {line}");
    assert!(line.contains("store"), "the origin is shown: {line}");

    let after = nodes(&w, "kn");
    assert_eq!(after.len(), before.len(), "{after:?}");
    assert!(
        !after.iter().any(|(ty, _, _)| ty == "Service"),
        "{after:?}"
    );
}

#[test]
fn every_store_hit_is_shown_beyond_the_fifty_run_names_of_its_type() {
    let w = records_world("cap");
    let names: Vec<String> = (0..60).map(|n| format!("repo-{n:02}")).collect();
    let entities = serde_json::json!({ "Repository": names });
    let state = instance(&w, "cap").join("state");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(state.join("entities.json"), entities.to_string()).unwrap();

    let (code, ran) = w.cortex(&["run", "cap/repos"]);
    assert_eq!(code, 0, "{ran}");

    let prompts = prompts(&w);
    let line = known_line(&prompts[0], "Repository").expect("a Repository line");
    let (store, run) = (line.find("acme/api"), line.find("repo-00"));
    assert!(store.is_some() && run.is_some(), "{line}");
    assert!(store < run, "store hits come first: {line}");
    // The cap holds for the run's own names.
    assert!(!line.contains("repo-55"), "{line}");
}

/// A structured source over `registry/*.json` whose mapping adds `repo:{namespace}/{path}`.
fn structured_source(templates: &str) -> String {
    format!(
        "  - name: registry\n    schedule: daily\n    settings:\n      kind: structured\n      \
         value:\n        input:\n          from: files\n          value: {{paths: [registry], \
         glob: \"*.json\"}}\n        records: \"$.repos\"\n        mapping:\n          \
         node_type: Repository\n          id: \"$.id\"\n          name: \"$.path\"\n          \
         aliases: []\n          properties: [{{property: path, path: \"$.path\"}}]\n          \
         relations: []\n          \
         alias_templates: {templates}\n    {POLICY}\n"
    )
}

#[test]
fn a_template_alias_reaches_the_store_node_another_writer_gave_that_alias() {
    let w = world();
    std::fs::create_dir_all(w.root.join("registry")).unwrap();
    std::fs::write(
        w.root.join("registry/repos.json"),
        r#"{"repos": [{"id": 7, "namespace": "acme", "path": "api"}, {"id": 8, "path": "web"}]}"#,
    )
    .unwrap();
    create(&w, "tpl", &structured_source(r#"["repo:{namespace}/{path}"]"#));
    foreign_writer(&w, "tpl");
    let before = nodes(&w, "tpl");

    let (code, ran) = w.cortex(&["run", "tpl/registry"]);
    assert_eq!(code, 0, "{ran}");

    let after = nodes(&w, "tpl");
    // Record 7 is the foreign node by its template alias; record 8 lacks `namespace`, so it has
    // no template alias and is a node of its own.
    assert_eq!(after.len(), before.len() + 1, "{before:?} -> {after:?}");
    assert!(
        !after.iter().any(|(_, name, _)| name.starts_with("api (")),
        "record 7 made a node of its own: {after:?}"
    );
    let web = after
        .iter()
        .find(|(_, name, aliases)| name.starts_with("web") || aliases.iter().any(|a| a == "web"))
        .expect("record 8's node");
    assert!(
        !web.2.iter().any(|a| a.starts_with("repo:")),
        "a template whose field is missing yields no alias: {web:?}"
    );
}

/// `spec::parse` of a spec whose structured mapping has `templates`.
fn parsed(templates: &str) -> Result<(), String> {
    let w = world();
    let path = write_spec(&w, "bad", &structured_source(templates));
    let text = std::fs::read_to_string(Path::new(&path)).unwrap();
    cortex_cli::spec::parse(&text).map(|_| ())
}

#[test]
fn a_template_of_another_placeholder_syntax_is_refused_when_the_spec_is_read() {
    assert_eq!(parsed(r#"["repo:{namespace}/{path}"]"#), Ok(()));
    assert_eq!(parsed(r#"["{$.namespace}-{a_b.c-d}"]"#), Ok(()));
    for bad in [
        r#"["repo:{namespace"]"#,
        r#"["repo:namespace}"]"#,
        r#"["repo:{}"]"#,
        r#"["repo:{{namespace}}"]"#,
        r#"["repo:{name space}"]"#,
        r#"["repo:{items[0]}"]"#,
        r#"["repo:{a..b}"]"#,
        r#"[""]"#,
        r#"["repo:fixed"]"#,
    ] {
        let refused = parsed(bad).expect_err(bad);
        assert!(
            refused.contains("sources.registry") && refused.contains("alias_templates"),
            "{bad}: {refused}"
        );
    }
}

#[test]
fn a_spec_without_alias_templates_still_reads() {
    let w = world();
    let path = write_spec(&w, "old", &structured_source("[]"));
    let text = std::fs::read_to_string(&path)
        .unwrap()
        .replace("          alias_templates: []\n", "");
    assert!(!text.contains("alias_templates"), "{text}");
    cortex_cli::spec::parse(&text).unwrap();
}
