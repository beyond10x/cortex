//! Findings 4, 9 and 11 of the external review of cortex 0.7.2 (`story:review-change-detection`):
//! a document is applied again whenever what cortex stores from it would change, and fetch-only
//! forecasts the documents the run will send to the model. The binary-level tests replay the
//! reviewed scenarios against the real `ekr` with stand-in `connectors` and `claude`; the others
//! hold the seen state's rules for a document applied before its fields were remembered.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::{ekr, executable, World};
use cortex_cli::sources::{Document, Origin};
use cortex_cli::state::{page_fields, text_hash, Seen, SeenState};
use serde_json::{json, Value};

/// A stand-in `connectors` with one ready `directory` connection; no structured test here calls
/// it, since the source reads files.
const CONNECTORS: &str = r#"case "$*" in
  *"connections list"*) echo '{"ok":true,"result":{"connections":[{"adapter":"directory","connector":"directory","instance":"main","connection":"conn_test","state":"ready","revision":"rev1"}]}}' ;;
  *"connections revalidate"*) echo '{"ok":true,"result":{}}' ;;
  *) echo '{"ok":false}'; exit 2 ;;
esac
"#;

/// A structured source over `registry/*.json` whose mapping holds `@PROPERTIES@`, with a refresh
/// window of 30 days.
const STRUCTURED: &str = r#"format: cortex.instance/1
name: s
description: Test brain.
ekr: {version: "0.0.43", bin: "@EKR@"}
seed: {documents: []}
model: {model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}
sources:
  - name: registry
    schedule: daily
    settings:
      kind: structured
      value:
        input:
          from: files
          value: {paths: [registry], glob: "*.json"}
        records: "$.people"
        mapping:
          node_type: Person
          id: "$.id"
          name: "$.name"
          aliases: []
          properties: @PROPERTIES@
          relations: []
        dropped: Keep
    policy: {refresh_after_days: 30, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}
serve: {view_port: 18991}
"#;

const TEAM: &str = r#"[{property: team, path: "$.team"}]"#;
const TEAM_AND_OFFICE: &str =
    r#"[{property: team, path: "$.team"}, {property: office, path: "$.office"}]"#;

fn structured_spec(w: &World, properties: &str) -> PathBuf {
    let path = w.root.join("s.yaml");
    let spec = STRUCTURED
        .replace("@EKR@", &ekr().display().to_string())
        .replace("@PROPERTIES@", properties);
    std::fs::write(&path, spec).unwrap();
    path
}

fn instance(w: &World, name: &str) -> PathBuf {
    w.home.join("instances").join(name)
}

/// The values of `predicate` the store of instance `name` holds, read back through `ekr sample`.
fn values(w: &World, name: &str, predicate: &str) -> BTreeSet<String> {
    let dir = instance(w, name);
    let out = Command::new(ekr())
        .args(["sample", "--seed", "1", "--size", "1000"])
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
    let sample: Value = serde_json::from_slice(&out.stdout).unwrap();
    sample["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|f| f["predicate_name"] == predicate)
        .filter_map(|f| f["assertion"]["object_value"]["value"].as_str())
        .map(str::to_string)
        .collect()
}

fn set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|v| v.to_string()).collect()
}

fn count(value: &Value, key: &str) -> i64 {
    value["detail"][key].as_i64().unwrap_or(0)
}

/// Finding 4: a property added to a structured source's mapping is applied to every record at the
/// next run, records whose text did not change included, and without waiting for
/// `refresh_after_days`. Fetch-only forecasts them as changed; the run after that applies nothing.
#[test]
fn a_mapping_change_applies_every_unchanged_record_again_at_once() {
    let w = World::new();
    executable(&w.bin.join("connectors"), CONNECTORS);
    std::fs::create_dir_all(w.root.join("registry")).unwrap();
    std::fs::write(
        w.root.join("registry/people.json"),
        json!({"people": [
            {"id": "A", "name": "Alpha", "team": "red", "office": "north"},
            {"id": "B", "name": "Bravo", "team": "blue", "office": "south"},
        ]})
        .to_string(),
    )
    .unwrap();
    let spec = structured_spec(&w, TEAM);
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let (code, first) = w.cortex(&["run", "s/registry"]);
    assert_eq!(code, 0, "{first}");
    assert_eq!(count(&first, "documents_applied"), 2, "{first}");
    assert!(values(&w, "s", "office").is_empty());

    let spec = structured_spec(&w, TEAM_AND_OFFICE);
    let (code, updated) = w.cortex(&[
        "update",
        "s",
        "--spec",
        spec.to_str().unwrap(),
        "--no-units",
    ]);
    assert_eq!(
        (code, updated["outcome"].as_str()),
        (0, Some("updated")),
        "{updated}"
    );

    let (code, forecast) = w.cortex(&["run", "s/registry", "--fetch-only"]);
    assert_eq!(code, 0, "{forecast}");
    assert_eq!(
        (
            count(&forecast, "changed"),
            count(&forecast, "held"),
            count(&forecast, "unchanged")
        ),
        (2, 0, 0),
        "{forecast}"
    );

    let (code, remap) = w.cortex(&["run", "s/registry"]);
    assert_eq!(code, 0, "{remap}");
    assert_eq!(count(&remap, "documents_applied"), 2, "{remap}");
    assert_eq!(values(&w, "s", "office"), set(&["north", "south"]));

    let (code, idle) = w.cortex(&["run", "s/registry"]);
    assert_eq!(code, 0, "{idle}");
    assert_eq!(count(&idle, "documents_new"), 0, "{idle}");
    assert!(
        w.lines("claude-calls.log").is_empty(),
        "the model was called"
    );
}

/// One search result at `https://example.org/report`, with `title`, `description` and `published`
/// and the same body every time.
fn serve_page(w: &World, title: &str, description: &str, published: Option<&str>) {
    let pages = json!({"results": [{"url": "https://example.org/report", "title": title,
        "description": description, "content": "Example Labs develops the Widget engine.",
        "content_truncated": false, "published": published, "score": "0.9"}],
        "complete": true, "truncation": [],
        "provenance": {"instance": "t", "profile": "tavily/2026-10",
            "received_at": "2026-10-05T00:00:00.000Z"}});
    std::fs::write(
        w.root.join("answer.json"),
        common::answer(&pages.to_string()),
    )
    .unwrap();
}

/// Finding 9: a Web page whose URL and body stayed the same while its title and description changed
/// and a publication date appeared is a changed document: with `refresh_after_days: 0` fetch-only
/// counts it changed and the run extracts it again, and the store holds the new title, description
/// and date.
#[test]
fn a_web_page_whose_metadata_alone_changed_is_extracted_again() {
    let w = World::new();
    serve_page(&w, "Report", "A report", None);
    let spec = w.spec("t", "conn_test");
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let (code, first) = w.cortex(&["run", "t/news"]);
    assert_eq!(code, 0, "{first}");
    assert_eq!(count(&first, "documents_applied"), 1, "{first}");
    assert_eq!(values(&w, "t", "title"), set(&["Report"]));

    serve_page(
        &w,
        "Corrected report",
        "A corrected report",
        Some("2026-11-20"),
    );
    let (code, forecast) = w.cortex(&["run", "t/news", "--fetch-only"]);
    assert_eq!(code, 0, "{forecast}");
    assert_eq!(
        (count(&forecast, "changed"), count(&forecast, "unchanged")),
        (1, 0),
        "{forecast}"
    );
    let (code, second) = w.cortex(&["run", "t/news"]);
    assert_eq!(code, 0, "{second}");
    assert_eq!(count(&second, "documents_applied"), 1, "{second}");
    assert!(values(&w, "t", "title").contains("Corrected report"));
    assert!(values(&w, "t", "description").contains("A corrected report"));
    assert!(values(&w, "t", "published").contains("2026-11-20"));

    let (code, idle) = w.cortex(&["run", "t/news"]);
    assert_eq!(code, 0, "{idle}");
    assert_eq!(count(&idle, "documents_new"), 0, "{idle}");
}

/// The stand-in `claude` of finding 11: every call is logged and answered with an error that cost
/// something, so the model was asked and failed on the document.
const FAILING_CLAUDE: &str = r#"cat > /dev/null
echo call >> "@ROOT@/failing-calls.log"
echo '{"type":"result","subtype":"success","is_error":true,"result":"The model could not answer this batch.","total_cost_usd":0.01}'
exit 1
"#;

/// Finding 11: a document skipped after failing alone on 3 consecutive runs is not forecast as
/// model work. Fetch-only names it in `skipped`, as the run does, and counts it neither new nor
/// changed; the run after it selects nothing and calls no model.
#[test]
fn fetch_only_leaves_out_a_document_the_run_skips_after_its_failures() {
    let w = World::new();
    executable(
        &w.bin.join("claude-failing"),
        &FAILING_CLAUDE.replace("@ROOT@", &w.root.display().to_string()),
    );
    serve_page(&w, "Report", "A report", None);
    let spec = w.spec("p", "conn_test");
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let failing = w.bin.join("claude-failing");
    let run = |w: &World, extra: &[&str]| {
        let mut args = vec!["--claude", failing.to_str().unwrap(), "run", "p/news"];
        args.extend_from_slice(extra);
        w.cortex_env(&args, &[("CORTEX_MODEL_RETRY_MS", Path::new("10"))])
    };
    for _ in 0..3 {
        run(&w, &[]);
    }
    let calls = w.lines("failing-calls.log").len();
    assert!(calls >= 3, "the model was asked on each run: {calls}");

    let (code, forecast) = run(&w, &["--fetch-only"]);
    assert_eq!(code, 0, "{forecast}");
    assert_eq!(
        (
            count(&forecast, "fetched"),
            count(&forecast, "new"),
            count(&forecast, "changed")
        ),
        (1, 0, 0),
        "{forecast}"
    );
    let skipped = forecast["detail"]["skipped"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(skipped.len(), 1, "{forecast}");
    assert!(
        skipped[0]
            .as_str()
            .is_some_and(|s| s.starts_with("https://example.org/report: extraction failed")),
        "{forecast}"
    );

    let (code, actual) = run(&w, &[]);
    assert_eq!(code, 0, "{actual}");
    assert_eq!(count(&actual, "documents_new"), 0, "{actual}");
    assert_eq!(
        actual["detail"]["skipped"].as_array(),
        Some(&skipped),
        "{actual}"
    );
    assert_eq!(
        w.lines("failing-calls.log").len(),
        calls,
        "the run called the model"
    );
}

fn page(title: &str) -> Document {
    Document {
        key: "https://example.org/report".into(),
        origin: Origin::Url,
        title: Some(title.into()),
        description: None,
        published: None,
        text: "body".into(),
        hash: None,
    }
}

fn record() -> Document {
    Document {
        key: "files:registry:*.json:A".into(),
        origin: Origin::Record,
        title: None,
        description: None,
        published: None,
        text: r#"{"id":"A"}"#.into(),
        hash: None,
    }
}

/// A state that saw `doc`'s text, applied at 0 by a cortex that did not remember its fields.
fn seen_before_fields(doc: &Document) -> SeenState {
    let mut state = SeenState::default();
    state.documents.insert(
        doc.key.clone(),
        Seen {
            hash: Some(text_hash(&doc.text)),
            applied_at: 0,
            unlinked_hash: None,
            fields_hash: None,
        },
    );
    state
}

/// `state`'s fields of this run: those of the Web page `doc`.
fn fetched(state: &mut SeenState, doc: &Document) {
    state.fields = page_fields(doc)
        .map(|f| (doc.key.clone(), f))
        .into_iter()
        .collect();
}

/// A Web page applied before cortex remembered its fields is not extracted again for them: the
/// first run that fetches it takes the fields it fetched as applied, and a later change of them is
/// a change that waits for the refresh window, as a change of its text does.
#[test]
fn a_page_seen_without_its_fields_takes_them_from_its_next_fetch() {
    let doc = page("Report");
    let mut state = seen_before_fields(&doc);
    fetched(&mut state, &doc);
    state.adopt_fields(std::slice::from_ref(&doc));
    assert!(!state.wants(&doc, 1, 0));
    assert!(state.applied_as_now(&doc));
    assert_eq!(state.documents[&doc.key].fields_hash, page_fields(&doc));

    let changed = page("Corrected report");
    fetched(&mut state, &changed);
    state.adopt_fields(std::slice::from_ref(&changed));
    assert!(!state.applied_as_now(&changed));
    assert!(state.wants(&changed, 1, 0));
    assert!(
        !state.wants(&changed, 1, 30),
        "a page's change waits for the window"
    );
    assert!(state.holds(&changed, 1, 30));
    assert_eq!(state.wanted(&changed).fields, page_fields(&changed));

    state.record(&changed, 1);
    assert!(state.applied_as_now(&changed));
    assert!(!state.wants(&changed, 2, 0));
}

/// A structured record applied before cortex remembered its mapping is applied again at once: the
/// mapping it was applied with is unknown, and applying a record calls no model. Once applied, the
/// same mapping is no change.
#[test]
fn a_record_seen_without_its_mapping_is_applied_again_at_once() {
    let doc = record();
    assert_eq!(page_fields(&doc), None);
    let mut state = seen_before_fields(&doc);
    state
        .fields
        .insert(doc.key.clone(), text_hash("the mapping"));
    state.adopt_fields(std::slice::from_ref(&doc));
    assert!(state.wants(&doc, 1, 30));
    assert!(!state.holds(&doc, 1, 30));
    state.record(&doc, 1);
    assert!(!state.wants(&doc, 2, 0));

    state
        .fields
        .insert(doc.key.clone(), text_hash("another mapping"));
    assert!(
        state.wants(&doc, 2, 30),
        "a changed mapping is applied at once"
    );
}
