//! Adversary probes of `story:structured-value-kinds`: the edges of the per-kind conversion, a
//! record left out as unconverted and read again on every run, `dropped: Supersede` comparing what
//! it wrote with what the store holds, and a kind declared on another
//! node type. The stand-in `claude` fails when called, and is never called.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{ekr, executable, World};
use serde_json::{json, Value};

const CLAUDE: &str = r#"echo call >> "@ROOT@/claude-calls.log"
echo '{"type":"result","subtype":"error","is_error":true}'
exit 1
"#;

/// `Item.count` `Integer`, declared on `Item` itself.
const SCHEMA: &str = r#"format: ekr.extraction-document/1
ontology:
  node_types:
  - name: Item
    parents: []
    abstract_type: false
    properties:
    - {name: count, value: {value_kind: Integer}, cardinality: One, required: false}
  edge_types: []
entities: []
facts: []
evidence: []
"#;

struct Opts<'a> {
    node_type: &'a str,
    properties: &'a [&'a str],
    schema: Option<&'a str>,
    max: usize,
    supersede: bool,
}

impl Default for Opts<'_> {
    fn default() -> Self {
        Opts {
            node_type: "Item",
            properties: &[],
            schema: None,
            max: 10,
            supersede: false,
        }
    }
}

fn world() -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(&w.bin.join("claude"), &CLAUDE.replace("@ROOT@", &root));
    std::fs::create_dir_all(w.root.join("items")).unwrap();
    w
}

/// Writes `items/items.json` as raw text, so a number keeps the form it is written in.
fn items_raw(w: &World, records: &str) {
    std::fs::write(
        w.root.join("items/items.json"),
        format!("{{\"items\": {records}}}"),
    )
    .unwrap();
}

fn items(w: &World, records: Value) {
    items_raw(w, &records.to_string());
}

fn spec(w: &World, name: &str, o: &Opts<'_>) -> PathBuf {
    let seed = match o.schema {
        Some(schema) => {
            std::fs::write(w.root.join(format!("{name}-schema.yaml")), schema).unwrap();
            format!("{{schema: {name}-schema.yaml, documents: []}}")
        }
        None => "{documents: []}".to_string(),
    };
    let properties: String = o
        .properties
        .iter()
        .map(|p| format!("            - {p}\n"))
        .collect();
    let dropped = if o.supersede {
        "        dropped: Supersede\n"
    } else {
        ""
    };
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
          node_type: {node_type}
          id: "$.id"
          name: "$.name"
          aliases: []
          properties:
{properties}          relations: []
{dropped}    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: {max}, max_chars_per_document: 5000}}
serve: {{view_port: 18994}}
"#,
            ekr = ekr().display(),
            node_type = o.node_type,
            max = o.max,
        ),
    )
    .unwrap();
    path
}

fn created(w: &World, path: &Path) {
    let (code, out) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("created")),
        "{out}"
    );
}

fn ekr_cmd(w: &World, name: &str, args: &[&str]) -> Value {
    let dir = w.home.join("instances").join(name);
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

/// The kind the store declares for `<node_type>.<property>`.
fn declared(w: &World, name: &str, node_type: &str, property: &str) -> Option<String> {
    let ontology = ekr_cmd(w, name, &["ontology"]);
    ontology["node_types"]
        .as_array()?
        .iter()
        .filter(|t| t["name"] == node_type)
        .flat_map(|t| t["properties"].as_array().cloned().unwrap_or_default())
        .find(|p| p["name"] == property)
        .and_then(|p| p["value_type"]["value_kind"].as_str().map(str::to_string))
}

/// Every active value of any property named `property`, as `ekr snapshot` writes it.
fn values(w: &World, name: &str, property: &str) -> Vec<Value> {
    let ontology = ekr_cmd(w, name, &["ontology"]);
    let ids: Vec<Value> = ontology["node_types"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|t| t["properties"].as_array().cloned().unwrap_or_default())
        .filter(|p| p["name"] == property)
        .map(|p| p["id"].clone())
        .collect();
    let snapshot = ekr_cmd(w, name, &["snapshot"]);
    let mut out: Vec<Value> = snapshot["graph"]["graph"]["assertions"]
        .as_object()
        .unwrap()
        .values()
        .filter(|a| ids.contains(&a["predicate"]["Property"]) && a["lifecycle"] == "Active")
        .map(|a| a["object"]["Value"].clone())
        .collect();
    out.sort_by_key(|v| v.to_string());
    out.dedup();
    out
}

fn run(w: &World, name: &str) -> (i32, Value) {
    w.cortex(&["run", &format!("{name}/items")])
}

/// Two unconverted records ahead of a good one, with `max_documents_per_run: 2`: the unconverted
/// ones are never marked seen, so every run selects them again and the good record behind them is
/// never read. A record that is refused alone must not hold the rest of the source back for ever.
#[test]
fn unconverted_records_at_the_head_do_not_starve_the_rest() {
    let w = world();
    items(
        &w,
        json!([
            {"id": "I-1", "name": "One", "count": "x1"},
            {"id": "I-2", "name": "Two", "count": "x2"},
            {"id": "I-3", "name": "Three", "count": 3},
        ]),
    );
    let path = spec(
        &w,
        "hol",
        &Opts {
            properties: &["{property: count, path: \"$.count\", value_kind: Integer}"],
            max: 2,
            ..Opts::default()
        },
    );
    created(&w, &path);
    let mut reports = Vec::new();
    for _ in 0..4 {
        let (code, ran) = run(&w, "hol");
        assert_eq!(code, 0, "{ran}");
        reports.push(ran);
    }
    assert_eq!(
        values(&w, "hol", "count"),
        [json!({"value_kind": "Integer", "value": 3})],
        "I-3 never applied after four runs: {reports:?}"
    );
}

/// Every record of the batch is unconverted: the run reports each and fails nothing.
#[test]
fn a_batch_of_only_unconverted_records_runs_and_names_each() {
    let w = world();
    items(
        &w,
        json!([
            {"id": "I-1", "name": "One", "count": "1.5"},
            {"id": "I-2", "name": "Two", "count": "9223372036854775808"},
        ]),
    );
    let path = spec(
        &w,
        "all",
        &Opts {
            properties: &["{property: count, path: \"$.count\", value_kind: Integer}"],
            ..Opts::default()
        },
    );
    created(&w, &path);
    let (code, ran) = run(&w, "all");
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 0, "{ran}");
    assert_eq!(
        ran["detail"]["unconverted"].as_array().map(Vec::len),
        Some(2),
        "{ran}"
    );
}

/// A `Decimal` cortex accepts must be one EKR takes, or be named unconverted: a JSON number in
/// exponent form (`1e-7`, `1e21`) is written by cortex as the text `"1e-7"`, and a decimal string
/// of 40 digits passes cortex's check. Either way the record with `2.5` beside them is applied and
/// the batch is not refused.
#[test]
fn a_decimal_cortex_accepts_never_refuses_the_batch() {
    let w = world();
    items_raw(
        &w,
        r#"[
            {"id": "I-1", "name": "One", "price": 1e-7},
            {"id": "I-2", "name": "Two", "price": 1e21},
            {"id": "I-3", "name": "Three", "price": "1234567890123456789012345678901234567890.5"},
            {"id": "I-4", "name": "Four", "price": 2.5}
        ]"#,
    );
    let path = spec(
        &w,
        "dec",
        &Opts {
            properties: &["{property: price, path: \"$.price\", value_kind: Decimal}"],
            ..Opts::default()
        },
    );
    created(&w, &path);
    let (code, ran) = run(&w, "dec");
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let applied = ran["detail"]["documents_applied"].as_i64().unwrap_or(0);
    let unconverted = ran["detail"]["unconverted"]
        .as_array()
        .map_or(0, Vec::len) as i64;
    let rejected = ran["detail"]["parts_rejected"].as_i64().unwrap_or(0);
    assert_eq!(applied + unconverted, 4, "{ran}");
    assert_eq!(rejected, 0, "{ran}");
    let held = values(&w, "dec", "price");
    assert!(
        held.iter().any(|v| v["value_kind"] == "Decimal"),
        "{held:?} {ran}"
    );
}

/// `dropped: Supersede` compares the values it lists with those the store holds: a `Decimal`
/// written as `"12.50"` and a JSON `2` must still be active after the run that wrote them, and
/// after a second run with nothing changed.
#[test]
fn supersede_does_not_end_the_decimals_it_just_wrote() {
    let w = world();
    items(
        &w,
        json!([
            {"id": "I-1", "name": "One", "price": "12.50"},
            {"id": "I-2", "name": "Two", "price": 2},
            {"id": "I-3", "name": "Three", "price": " +3.0 "},
        ]),
    );
    let path = spec(
        &w,
        "sup",
        &Opts {
            properties: &["{property: price, path: \"$.price\", value_kind: Decimal}"],
            supersede: true,
            ..Opts::default()
        },
    );
    created(&w, &path);
    let (code, first) = run(&w, "sup");
    assert_eq!((code, first["outcome"].as_str()), (0, Some("ran")), "{first}");
    assert_eq!(first["detail"]["documents_applied"], 3, "{first}");
    assert_eq!(values(&w, "sup", "price").len(), 3, "{first}");
    let (_, second) = run(&w, "sup");
    let held = values(&w, "sup", "price");
    assert_eq!(held.len(), 3, "{held:?} {second}");
    assert_eq!(second["detail"]["retracted"].as_i64().unwrap_or(0), 0, "{second}");
}

/// The same for a `Timestamp` with an offset and a `Boolean` in upper case: what the run wrote is
/// what it lists, so nothing is ended.
#[test]
fn supersede_does_not_end_timestamps_or_booleans_it_just_wrote() {
    let w = world();
    items(
        &w,
        json!([
            {"id": "I-1", "name": "One", "due": "2026-10-01T02:00:00+02:00", "done": "TRUE"},
        ]),
    );
    let path = spec(
        &w,
        "sut",
        &Opts {
            properties: &[
                "{property: due, path: \"$.due\", value_kind: Timestamp}",
                "{property: done, path: \"$.done\", value_kind: Boolean}",
            ],
            supersede: true,
            ..Opts::default()
        },
    );
    created(&w, &path);
    let (code, first) = run(&w, "sut");
    assert_eq!((code, first["outcome"].as_str()), (0, Some("ran")), "{first}");
    run(&w, "sut");
    assert_eq!(
        values(&w, "sut", "due"),
        [json!({"value_kind": "Timestamp", "value": 1_790_812_800_000_i64})]
    );
    assert_eq!(
        values(&w, "sut", "done"),
        [json!({"value_kind": "Boolean", "value": true})]
    );
}

/// The store declares `Item.count` `Integer`; a source mapping `count` on another node type,
/// `Other`, with no `value_kind`, writes it as `String` there, and must leave `Item.count` an
/// `Integer` (EKR 0.0.34 on SQLite changes a redeclared property's kind without refusing).
#[test]
fn a_string_on_another_node_type_leaves_the_declared_kind_alone() {
    let w = world();
    items(&w, json!([{"id": "O-1", "name": "One", "count": "abc"}]));
    let path = spec(
        &w,
        "oth",
        &Opts {
            node_type: "Other",
            properties: &["{property: count, path: \"$.count\"}"],
            schema: Some(SCHEMA),
            ..Opts::default()
        },
    );
    created(&w, &path);
    let (code, ran) = run(&w, "oth");
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(
        declared(&w, "oth", "Item", "count").as_deref(),
        Some("Integer"),
        "{ran}"
    );
}

/// Integer text with a sign, whitespace and leading zeros converts; `i64::MAX` converts; one past
/// it, a fraction and an exponent are named unconverted; the batch is applied otherwise.
#[test]
fn integer_text_edges_convert_or_are_named() {
    let w = world();
    items(
        &w,
        json!([
            {"id": "I-1", "name": "One", "count": " +007 "},
            {"id": "I-2", "name": "Two", "count": "9223372036854775807"},
            {"id": "I-3", "name": "Three", "count": "-9223372036854775809"},
            {"id": "I-4", "name": "Four", "count": "1e3"},
            {"id": "I-5", "name": "Five", "count": null},
        ]),
    );
    let path = spec(
        &w,
        "int",
        &Opts {
            properties: &["{property: count, path: \"$.count\"}"],
            schema: Some(SCHEMA),
            ..Opts::default()
        },
    );
    created(&w, &path);
    let (code, ran) = run(&w, "int");
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 3, "{ran}");
    assert_eq!(
        ran["detail"]["unconverted"].as_array().map(Vec::len),
        Some(2),
        "{ran}"
    );
    assert_eq!(
        values(&w, "int", "count"),
        [
            json!({"value_kind": "Integer", "value": 7}),
            json!({"value_kind": "Integer", "value": i64::MAX}),
        ]
    );
}
