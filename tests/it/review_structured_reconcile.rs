//! Findings 1, 2, 3, 5 and 6 of the external review of cortex 0.7.2
//! (`story:review-structured-reconcile`): a structured source retracts a record's values only on
//! evidence that the record is gone, and a record EKR did not apply is tried again, never counted
//! as applied, and does not hold back the others. Each test replays the reviewed scenario against
//! the real `ekr`; the stand-in `connectors` answers the file `answer.json`, and the stand-in
//! `claude` fails and is never called.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

use crate::common::{ekr, executable, World};
use serde_json::{json, Value};

/// A stand-in `connectors` with one ready `directory` connection whose `people.list` answers the
/// file `answer.json` as its result, logging each invocation with its input to `people.log`.
const CONNECTORS: &str = r#"R="@ROOT@"
IN=""; if [ "$1" = invoke ]; then IN=$(cat); fi; A="$* $IN"
case "$A" in
  *"connections list"*) echo '{"ok":true,"result":{"connections":[{"adapter":"directory","connector":"directory","instance":"main","connection":"conn_test","state":"ready","revision":"rev1"}]}}' ;;
  *"connections revalidate"*) echo '{"ok":true,"result":{}}' ;;
  "invoke "*" people.list "*)
    echo "$A" >> "$R/people.log"
    printf '{"ok":true,"result":{"connector":"directory","instance":"main","operation":"people.list","revision":"r","result":%s}}\n' "$(cat "$R/answer.json")" ;;
  *) echo '{"ok":false}'; exit 2 ;;
esac
"#;

const CLAUDE: &str = r#"echo call >> "@ROOT@/claude-calls.log"
echo '{"type":"result","subtype":"error","is_error":true}'
exit 1
"#;

const SPEC: &str = r#"format: cortex.instance/1
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
          @INPUT@
        records: "$.people"
        mapping:
          node_type: Person
          id: "$.id"
          name: "$.name"
          aliases: @ALIASES@
          properties:
            - {property: team, path: "$.team"}
          relations: []
          @LOOKUP@
        dropped: @DROPPED@
    policy: {refresh_after_days: 0, change: ContentHash, max_documents_per_run: @MAX@, max_chars_per_document: 5000}
serve: {view_port: 18992}
"#;

/// `registry/*.json`, a whole listing.
const FILES: &str = r#"from: files
          value: {paths: [registry], glob: "*.json"}"#;

/// `people.list` asked about the run's window: it answers what changed since `{since}`.
const WINDOWED: &str = r#"from: connectors
          value: {adapter: directory, connection: conn_test, operation: people.list, inputs: [{since: "{since}"}]}"#;

/// `people.list` asked for every record, page by page with a token.
const PAGED: &str = r#"from: connectors
          value: {adapter: directory, connection: conn_test, operation: people.list, inputs: [{}], paging: {style: Token, param: page, next: next, max_pages: 5}}"#;

struct Source<'a> {
    input: &'a str,
    dropped: &'a str,
    max: usize,
    aliases: &'a str,
    any_type: bool,
}

const SUPERSEDE: Source<'static> = Source {
    input: FILES,
    dropped: "Supersede",
    max: 10,
    aliases: "[]",
    any_type: false,
};

/// A world with instance `s`: one structured source, `registry`, as `source` says.
fn world(source: Source<'_>) -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(
        &w.bin.join("connectors"),
        &CONNECTORS.replace("@ROOT@", &root),
    );
    executable(&w.bin.join("claude"), &CLAUDE.replace("@ROOT@", &root));
    std::fs::create_dir_all(w.root.join("registry")).unwrap();
    people(&w, &[("A", "Alpha", "red"), ("B", "Bravo", "blue")]);
    let path = w.root.join("s.yaml");
    let lookup = if source.any_type {
        "store_lookup: AnyType"
    } else {
        ""
    };
    let spec = SPEC
        .replace("@EKR@", &ekr().display().to_string())
        .replace("@INPUT@", source.input)
        .replace("@ALIASES@", source.aliases)
        .replace("@LOOKUP@", lookup)
        .replace("@DROPPED@", source.dropped)
        .replace("@MAX@", &source.max.to_string());
    std::fs::write(&path, spec).unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    w
}

/// Writes the records `(id, name, team)` to `registry/people.json` and to `answer.json`.
fn people(w: &World, rows: &[(&str, &str, &str)]) {
    let rows: Vec<Value> = rows
        .iter()
        .map(|(id, name, team)| json!({"id": id, "name": name, "team": team}))
        .collect();
    records(w, &rows);
}

fn records(w: &World, rows: &[Value]) {
    let people = json!({ "people": rows }).to_string();
    std::fs::write(w.root.join("registry/people.json"), &people).unwrap();
    std::fs::write(w.root.join("answer.json"), &people).unwrap();
}

/// `cortex run s/registry`, which must end `ran` with no model call.
fn run(w: &World) -> Value {
    let (code, ran) = w.cortex(&["run", "s/registry"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert!(
        w.lines("claude-calls.log").is_empty(),
        "the model was called"
    );
    ran["detail"].clone()
}

fn count(detail: &Value, key: &str) -> i64 {
    detail[key].as_i64().unwrap_or(0)
}

fn instance(w: &World) -> PathBuf {
    w.home.join("instances/s")
}

fn ekr_in(w: &World) -> Command {
    let dir = instance(w);
    let mut c = Command::new(ekr());
    c.env("EKR_HOST", dir.join("host.json"))
        .env("EKR_BACKEND", "sqlite")
        .env("EKR_STORE", dir.join("store.sqlite"));
    c
}

/// The values of `team` the store holds active, read back through `ekr sample`.
fn teams(w: &World) -> BTreeSet<String> {
    let out = ekr_in(w)
        .args(["sample", "--seed", "1", "--size", "1000"])
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
        .filter(|f| f["predicate_name"] == "team")
        .filter_map(|f| f["assertion"]["object_value"]["value"].as_str())
        .map(str::to_string)
        .collect()
}

fn set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|v| v.to_string()).collect()
}

/// Another writer's node of `node_type`, known by `aliases`.
fn foreign_node(w: &World, node_type: &str, aliases: &[&str]) {
    let path = w
        .root
        .join(format!("foreign-{node_type}-{}.json", aliases[0]));
    let doc = json!({
        "format": "ekr.extraction-document/1",
        "ontology": {"node_types": [{"name": node_type, "parents": [], "abstract_type": false, "properties": []}], "edge_types": []},
        "entities": [{"node_type": node_type, "aliases": aliases}],
        "facts": [],
        "evidence": [],
    });
    std::fs::write(&path, doc.to_string()).unwrap();
    let out = ekr_in(w)
        .arg("apply-extraction")
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The keys the run named in `rejected`.
fn rejected(detail: &Value) -> Vec<String> {
    detail["rejected"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| r["document"].as_str())
        .map(str::to_string)
        .collect()
}

/// Finding 1: a source asked about its window (`{since}`) and `dropped: Supersede` lists what
/// changed in the window; a record it did not list is not gone, and keeps its values. The listed
/// record's changed value still supersedes its old one.
#[test]
fn a_window_listing_only_the_changed_record_keeps_the_values_of_the_others() {
    let w = world(Source {
        input: WINDOWED,
        ..SUPERSEDE
    });
    let first = run(&w);
    assert_eq!(count(&first, "documents_applied"), 2, "{first}");

    people(&w, &[("A", "Alpha", "green")]);
    let second = run(&w);
    assert!(
        w.lines("people.log").iter().all(|l| l.contains("since")),
        "the operation was asked about the window: {:?}",
        w.lines("people.log")
    );
    assert_eq!(count(&second, "retracted"), 0, "{second}");
    assert_eq!(count(&second, "superseded"), 1, "{second}");
    assert_eq!(teams(&w), set(&["blue", "green"]));
}

/// Finding 2: a provider that names a token it already returned answers in a cycle; the pages
/// after it were never read. Fetch-only says the listing is incomplete, and the run ends nothing.
#[test]
fn a_repeated_page_token_is_an_incomplete_listing_that_ends_nothing() {
    let w = world(Source {
        input: PAGED,
        ..SUPERSEDE
    });
    let first = run(&w);
    assert_eq!(count(&first, "documents_applied"), 2, "{first}");

    std::fs::write(
        w.root.join("answer.json"),
        json!({"people": [{"id": "A", "name": "Alpha", "team": "red"}], "next": "one"}).to_string(),
    )
    .unwrap();
    let (code, fetched) = w.cortex(&["run", "s/registry", "--fetch-only"]);
    assert_eq!(code, 0, "{fetched}");
    let unread = fetched["detail"]["unread"].as_array().cloned();
    assert!(
        unread.is_some_and(|u| u
            .iter()
            .any(|l| l.as_str().is_some_and(|l| l.contains("paging cycle")))),
        "{fetched}"
    );

    let cycle = run(&w);
    assert_eq!(count(&cycle, "retracted"), 0, "{cycle}");
    assert!(
        cycle["stopped"]
            .as_str()
            .is_some_and(|s| s.contains("paging cycle")),
        "{cycle}"
    );
    assert_eq!(teams(&w), set(&["blue", "red"]));
}

/// Finding 3: a record a whole listing dropped is retracted and retired; when it is listed again
/// with the same text, it is applied again and its values are active.
#[test]
fn a_record_that_returns_after_its_retirement_is_applied_again() {
    let w = world(SUPERSEDE);
    run(&w);
    people(&w, &[("A", "Alpha", "red")]);
    let removed = run(&w);
    assert_eq!(count(&removed, "retracted"), 1, "{removed}");
    assert_eq!(teams(&w), set(&["red"]));

    people(&w, &[("A", "Alpha", "red"), ("B", "Bravo", "blue")]);
    let (code, fetched) = w.cortex(&["run", "s/registry", "--fetch-only"]);
    assert_eq!(code, 0, "{fetched}");
    assert_eq!(fetched["detail"]["new"], 1, "{fetched}");
    let back = run(&w);
    assert_eq!(count(&back, "documents_applied"), 1, "{back}");
    assert_eq!(teams(&w), set(&["blue", "red"]));

    // Back in the listing and applied: the next run selects nothing.
    let idle = run(&w);
    assert_eq!(count(&idle, "documents_new"), 0, "{idle}");
}

/// Finding 5: EKR reports a record's entity `ambiguous` (its aliases meet two store nodes of one
/// type) and applies nothing of it, though it rejects no part. The run names the record in
/// `rejected`, does not count it applied, and the next run tries it again; its healthy peer is
/// applied.
#[test]
fn an_ambiguous_record_is_not_counted_applied_and_is_tried_again() {
    let w = world(Source {
        dropped: "Keep",
        aliases: r#"["$.also"]"#,
        any_type: true,
        ..SUPERSEDE
    });
    foreign_node(&w, "Project", &["Site West", "alias:left"]);
    foreign_node(&w, "Project", &["Site East", "alias:right"]);
    records(
        &w,
        &[
            json!({"id": "A", "name": "Alpha", "team": "red", "also": ["alias:left", "alias:right"]}),
            json!({"id": "B", "name": "Bravo", "team": "blue"}),
        ],
    );

    let first = run(&w);
    assert_eq!(count(&first, "documents_new"), 2, "{first}");
    assert_eq!(count(&first, "documents_applied"), 1, "{first}");
    assert_eq!(rejected(&first), ["files:registry:*.json:A"], "{first}");
    assert!(first.to_string().contains("ambiguous"), "{first}");
    assert_eq!(teams(&w), set(&["blue"]));

    let second = run(&w);
    assert_eq!(count(&second, "documents_new"), 1, "{second}");
    assert_eq!(count(&second, "documents_applied"), 0, "{second}");
    assert_eq!(rejected(&second), ["files:registry:*.json:A"], "{second}");
}

/// Finding 6: under a one-record run limit, a record refused on every run (its name meets store
/// nodes of two types) is tried after the others, so a valid record behind it is applied on the
/// next run, and the refused one is tried again after it.
#[test]
fn a_refused_record_does_not_block_the_next_one_under_a_run_limit() {
    let w = world(Source {
        dropped: "Keep",
        max: 1,
        any_type: true,
        ..SUPERSEDE
    });
    foreign_node(&w, "Project", &["Site West", "Alpha"]);
    foreign_node(&w, "Service", &["Site East", "Alpha"]);

    let first = run(&w);
    assert_eq!(count(&first, "documents_applied"), 0, "{first}");
    assert_eq!(rejected(&first), ["files:registry:*.json:A"], "{first}");

    let second = run(&w);
    assert_eq!(count(&second, "documents_applied"), 1, "{second}");
    assert!(rejected(&second).is_empty(), "{second}");
    assert_eq!(teams(&w), set(&["blue"]));

    let third = run(&w);
    assert_eq!(count(&third, "documents_new"), 1, "{third}");
    assert_eq!(rejected(&third), ["files:registry:*.json:A"], "{third}");
}
