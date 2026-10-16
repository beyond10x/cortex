//! A structured source writes each mapped property in the value kind the store holds for it
//! (`story:structured-value-kinds`): the kind a mapping names (`value_kind`), else the kind the
//! store's ontology declares for that property of the node type, else `String`. A record whose
//! value does not convert to the kind is left out alone and named with its key and the property in
//! the run's report and `cortex.log`; a `value_kind` that differs from the kind the store declares
//! is refused when the spec is read, naming the node type, the property and both kinds. The
//! stand-in `claude` fails when called, and is never called.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{ekr, executable, World};
use serde_json::{json, Value};

const CLAUDE: &str = r#"echo call >> "@ROOT@/claude-calls.log"
echo '{"type":"result","subtype":"error","is_error":true}'
exit 1
"#;

/// A seed schema declaring `Item.count` `Integer` and `Item.open` `Boolean`.
const SCHEMA: &str = r#"format: ekr.extraction-document/1
ontology:
  node_types:
  - name: Item
    parents: []
    abstract_type: false
    properties:
    - {name: count, value: {value_kind: Integer}, cardinality: One, required: false}
    - {name: open, value: {value_kind: Boolean}, cardinality: One, required: false}
  edge_types: []
entities: []
facts: []
evidence: []
"#;

fn world() -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(&w.bin.join("claude"), &CLAUDE.replace("@ROOT@", &root));
    std::fs::create_dir_all(w.root.join("items")).unwrap();
    std::fs::write(w.root.join("schema.yaml"), SCHEMA).unwrap();
    w
}

/// Writes `items/items.json`: `{"items": [...]}`.
fn items(w: &World, records: Value) {
    std::fs::write(
        w.root.join("items/items.json"),
        json!({ "items": records }).to_string(),
    )
    .unwrap();
}

/// The spec of instance `name` with one structured source, `items`, reading `items/*.json`, its
/// mapping's `properties` as given (YAML flow items, one per line), seeded with [`SCHEMA`] when
/// `seeded`.
fn spec(w: &World, name: &str, properties: &[&str], seeded: bool) -> PathBuf {
    let seed = if seeded {
        "{schema: schema.yaml, documents: []}"
    } else {
        "{documents: []}"
    };
    let properties: String = properties
        .iter()
        .map(|p| format!("            - {p}\n"))
        .collect();
    let path = w.root.join(format!("{name}.yaml"));
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: {name}
description: Test brain.
ekr: {{version: "0.0.34", bin: "{ekr}"}}
seed: {seed}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
  - name: items
    schedule: daily
    settings:
      kind: structured
      value:
        input:
          from: files
          value: {{paths: [items], glob: "*.json"}}
        records: "$.items"
        mapping:
          node_type: Item
          id: "$.id"
          name: "$.name"
          aliases: []
          properties:
{properties}          relations: []
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
serve: {{view_port: 18993}}
"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    path
}

fn create(w: &World, path: &Path) -> (i32, Value) {
    w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"])
}

fn created(w: &World, path: &Path) {
    let (code, out) = create(w, path);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("created")),
        "{out}"
    );
}

fn instance(w: &World, name: &str) -> PathBuf {
    w.home.join("instances").join(name)
}

fn ekr_cmd(w: &World, name: &str, args: &[&str]) -> Value {
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

/// The kind the store declares for `Item.<property>`.
fn declared(w: &World, name: &str, property: &str) -> Option<String> {
    let ontology = ekr_cmd(w, name, &["ontology"]);
    ontology["node_types"]
        .as_array()?
        .iter()
        .filter(|t| t["name"] == "Item")
        .flat_map(|t| t["properties"].as_array().cloned().unwrap_or_default())
        .find(|p| p["name"] == property)
        .and_then(|p| p["value_type"]["value_kind"].as_str().map(str::to_string))
}

/// Every active value of `Item.<property>` the store holds, each as `ekr snapshot` writes it
/// (`{"value_kind": …, "value": …}`).
fn values(w: &World, name: &str, property: &str) -> Vec<Value> {
    let ontology = ekr_cmd(w, name, &["ontology"]);
    let Some(id) = ontology["node_types"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|t| t["properties"].as_array().cloned().unwrap_or_default())
        .find(|p| p["name"] == property)
        .map(|p| p["id"].clone())
    else {
        return Vec::new();
    };
    let snapshot = ekr_cmd(w, name, &["snapshot"]);
    let mut out: Vec<Value> = snapshot["graph"]["graph"]["assertions"]
        .as_object()
        .unwrap()
        .values()
        .filter(|a| a["predicate"]["Property"] == id && a["lifecycle"] == "Active")
        .map(|a| a["object"]["Value"].clone())
        .collect();
    out.sort_by_key(|v| v.to_string());
    out
}

fn run(w: &World, name: &str) -> (i32, Value) {
    w.cortex(&["run", &format!("{name}/items")])
}

fn log_lines(w: &World, name: &str) -> Vec<Value> {
    std::fs::read_to_string(instance(w, name).join("cortex.log"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

const BOTH: [&str; 2] = [
    "{property: count, path: \"$.count\"}",
    "{property: open, path: \"$.open\"}",
];

/// The store declares `count` `Integer` and `open` `Boolean`; a mapping of both with no
/// `value_kind` writes `3` and `"4"` as Integers and `true` and `"false"` as Booleans, every
/// record applied.
#[test]
fn a_mapping_without_a_kind_writes_the_kinds_the_store_declares() {
    let w = world();
    items(
        &w,
        json!([
            {"id": "I-1", "name": "One", "count": 3, "open": true},
            {"id": "I-2", "name": "Two", "count": "4", "open": "false"},
        ]),
    );
    let path = spec(&w, "a", &BOTH, true);
    created(&w, &path);
    let (code, ran) = run(&w, "a");
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
    assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");
    assert!(ran["detail"].get("unconverted").is_none(), "{ran}");
    assert_eq!(declared(&w, "a", "count").as_deref(), Some("Integer"));
    assert_eq!(declared(&w, "a", "open").as_deref(), Some("Boolean"));
    assert_eq!(
        values(&w, "a", "count"),
        [
            json!({"value_kind": "Integer", "value": 3}),
            json!({"value_kind": "Integer", "value": 4}),
        ]
    );
    assert_eq!(
        values(&w, "a", "open"),
        [
            json!({"value_kind": "Boolean", "value": false}),
            json!({"value_kind": "Boolean", "value": true}),
        ]
    );
    assert!(w.lines("claude-calls.log").is_empty());
}

/// One record's `count` is `"12a"`: that record alone is left out, named with its key and the
/// property in the run's report and `cortex.log`; the others are applied. Its next run tries it
/// again.
#[test]
fn a_record_whose_value_does_not_convert_is_refused_alone() {
    let w = world();
    items(
        &w,
        json!([
            {"id": "I-1", "name": "One", "count": 3, "open": true},
            {"id": "I-2", "name": "Two", "count": "12a", "open": "false"},
            {"id": "I-3", "name": "Three", "count": "5", "open": false},
        ]),
    );
    let path = spec(&w, "b", &BOTH, true);
    created(&w, &path);
    let (_, ran) = run(&w, "b");
    assert_eq!(ran["outcome"].as_str(), Some("ran"), "{ran}");
    assert_eq!(ran["detail"]["documents_new"], 3, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
    let unconverted = ran["detail"]["unconverted"].as_array().cloned();
    let unconverted = unconverted.unwrap_or_else(|| panic!("no unconverted: {ran}"));
    assert_eq!(unconverted.len(), 1, "{ran}");
    let one = &unconverted[0];
    assert!(
        one["document"]
            .as_str()
            .is_some_and(|d| d.ends_with(":I-2")),
        "{ran}"
    );
    assert_eq!(one["property"], "count", "{ran}");
    assert_eq!(one["value_kind"], "Integer", "{ran}");

    assert_eq!(
        values(&w, "b", "count"),
        [
            json!({"value_kind": "Integer", "value": 3}),
            json!({"value_kind": "Integer", "value": 5}),
        ]
    );
    // No part of record I-2 is in the store: its `open` is not either.
    assert_eq!(
        values(&w, "b", "open"),
        [
            json!({"value_kind": "Boolean", "value": false}),
            json!({"value_kind": "Boolean", "value": true}),
        ]
    );
    let logged = log_lines(&w, "b");
    let line = logged
        .iter()
        .rev()
        .find(|l| l["source"] == "items")
        .unwrap_or_else(|| panic!("no run line in cortex.log: {logged:?}"));
    assert_eq!(line["unconverted"], json!(unconverted), "{line}");

    // The record is skipped until its text changes, then applied once its value converts.
    let (_, same) = run(&w, "b");
    assert_eq!(same["detail"]["documents_applied"], 0, "{same}");
    assert!(same["detail"].get("unconverted").is_none(), "{same}");
    items(
        &w,
        json!([
            {"id": "I-1", "name": "One", "count": 3, "open": true},
            {"id": "I-2", "name": "Two", "count": "12", "open": "false"},
            {"id": "I-3", "name": "Three", "count": "5", "open": false},
        ]),
    );
    let (_, again) = run(&w, "b");
    assert_eq!(again["detail"]["documents_applied"], 1, "{again}");
    assert!(again["detail"].get("unconverted").is_none(), "{again}");
}

/// `value_kind` on properties the store does not declare yet: each is declared in that kind and
/// written in it (`Integer`, `Decimal`, `Boolean`, `Timestamp` from RFC 3339). A property with no
/// `value_kind` that the store does not declare stays `String`.
#[test]
fn a_named_kind_is_declared_on_a_property_the_store_does_not_hold() {
    let w = world();
    items(
        &w,
        json!([
            {"id": "I-1", "name": "One", "size": "7", "price": "12.50", "done": "TRUE",
             "due": "2026-10-01T00:00:00Z", "team": "Core"},
        ]),
    );
    let path = spec(
        &w,
        "c",
        &[
            "{property: size, path: \"$.size\", value_kind: Integer}",
            "{property: price, path: \"$.price\", value_kind: Decimal}",
            "{property: done, path: \"$.done\", value_kind: Boolean}",
            "{property: due, path: \"$.due\", value_kind: Timestamp}",
            "{property: team, path: \"$.team\"}",
        ],
        false,
    );
    created(&w, &path);
    let (code, ran) = run(&w, "c");
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");
    for (property, kind) in [
        ("size", "Integer"),
        ("price", "Decimal"),
        ("done", "Boolean"),
        ("due", "Timestamp"),
        ("team", "String"),
    ] {
        assert_eq!(
            declared(&w, "c", property).as_deref(),
            Some(kind),
            "{property}"
        );
    }
    assert_eq!(
        values(&w, "c", "size"),
        [json!({"value_kind": "Integer", "value": 7})]
    );
    assert_eq!(values(&w, "c", "price")[0]["value_kind"], "Decimal");
    assert_eq!(
        values(&w, "c", "done"),
        [json!({"value_kind": "Boolean", "value": true})]
    );
    assert_eq!(
        values(&w, "c", "due"),
        [json!({"value_kind": "Timestamp", "value": 1_790_812_800_000_i64})]
    );
    assert_eq!(
        values(&w, "c", "team"),
        [json!({"value_kind": "String", "value": "Core"})]
    );
}

fn names_the_conflict(text: &str) -> bool {
    ["Item", "count", "String", "Integer"]
        .iter()
        .all(|n| text.contains(n))
}

/// `value_kind: String` on `count`, which the store declares `Integer`: `create` refuses the spec,
/// naming the node type, the property and both kinds, and leaves no instance.
#[test]
fn create_refuses_a_kind_that_differs_from_the_stores() {
    let w = world();
    let path = spec(
        &w,
        "d",
        &["{property: count, path: \"$.count\", value_kind: String}"],
        true,
    );
    let (code, out) = create(&w, &path);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("seed-refused")),
        "{out}"
    );
    assert!(names_the_conflict(&out.to_string()), "{out}");
    assert!(!instance(&w, "d").exists(), "{out}");
}

/// The same on `update` of an instance whose store declares `count` `Integer`: refused, naming
/// the conflict, and the frozen spec is unchanged.
#[test]
fn update_refuses_a_kind_that_differs_from_the_stores() {
    let w = world();
    let path = spec(&w, "e", &BOTH, true);
    created(&w, &path);
    let frozen = std::fs::read_to_string(instance(&w, "e").join("instance.yaml")).unwrap();
    spec(
        &w,
        "e",
        &["{property: count, path: \"$.count\", value_kind: String}"],
        true,
    );
    let (code, out) = w.cortex(&[
        "update",
        "e",
        "--spec",
        path.to_str().unwrap(),
        "--no-units",
    ]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("seed-change-refused")),
        "{out}"
    );
    assert!(names_the_conflict(&out.to_string()), "{out}");
    let after = std::fs::read_to_string(instance(&w, "e").join("instance.yaml")).unwrap();
    assert_eq!(frozen, after);
}

/// A run whose store came to declare `count` `Integer` after the instance was created with
/// `value_kind: String` on it: the run is refused, naming the conflict, and nothing is applied.
#[test]
fn a_run_refuses_a_kind_that_differs_from_the_stores_and_applies_nothing() {
    let w = world();
    items(&w, json!([{"id": "I-1", "name": "One", "count": "3"}]));
    let path = spec(
        &w,
        "f",
        &["{property: count, path: \"$.count\", value_kind: String}"],
        false,
    );
    created(&w, &path);
    ekr_cmd(
        &w,
        "f",
        &[
            "apply-extraction",
            w.root.join("schema.yaml").to_str().unwrap(),
        ],
    );
    let head = w.head("f");
    let (code, ran) = run(&w, "f");
    assert_ne!(code, 0, "{ran}");
    assert!(names_the_conflict(&ran.to_string()), "{ran}");
    assert_eq!(w.head("f"), head, "{ran}");
    assert!(values(&w, "f", "count").is_empty(), "{ran}");
}
