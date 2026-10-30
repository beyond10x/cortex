//! Adversary probes of `story:apply-outcome-unknown-not-counted`: an `apply-extraction` or a
//! transaction commit whose outcome is unknown, on a SQLite store (EKR's SQLite event log answers
//! `UnknownCommit` too), on the model path and the structured one: a later batch of a run, a run
//! held to a gate, an unconverted record beside the unknown batch, and the commit of the
//! `dropped: Supersede` transaction.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{ekr, executable, World};
use serde_json::{json, Value};

/// What EKR 0.0.34 prints for `StoreError::UnknownCommit` (a fault, `ekr: <message>`, exit 1).
const UNKNOWN: &str = "ekr: publication outcome is unknown; resolve the exact retained occurrence";

/// A stand-in `ekr` in front of the real one. Hooks are files under `<root>`:
/// - `unknown-apply-at`: holds `n`; the `n`th `apply-extraction` counted since the file was
///   written applies, then answers the unknown outcome.
/// - `unknown-commit`: every `commit` commits, then answers the unknown outcome.
fn unknown_ekr(w: &World) -> PathBuf {
    let path = w.bin.join("ekr-unknown");
    let body = r#"R="@R@"; EKR="@EKR@"; UNKNOWN="@UNKNOWN@"
if [ "$1" = apply-extraction ] && [ -e "$R/unknown-apply-at" ]; then
  n=$(( $(cat "$R/applies" 2>/dev/null || echo 0) + 1 )); echo "$n" > "$R/applies"
  if [ "$n" = "$(cat "$R/unknown-apply-at")" ]; then
    "$EKR" "$@" > /dev/null || exit 1; echo "$UNKNOWN" >&2; exit 1
  fi
fi
if [ "$1" = commit ] && [ -e "$R/unknown-commit" ]; then
  "$EKR" "$@" > /dev/null || exit 1; echo "$UNKNOWN" >&2; exit 1
fi
exec "$EKR" "$@"
"#
    .replace("@R@", &w.root.display().to_string())
    .replace("@EKR@", &ekr().display().to_string())
    .replace("@UNKNOWN@", UNKNOWN);
    executable(&path, &format!("{}{body}", common::EKR_SESSION_SHIM));
    path
}

/// Every JSON line `cortex <args>` printed, and its exit status.
fn cortex(w: &World, args: &[&str]) -> (Option<i32>, Vec<Value>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(args)
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let lines = stdout
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    let all = format!("{stdout}{}", String::from_utf8_lossy(&out.stderr));
    (out.status.code(), lines, all)
}

/// `cortex run <source> --record-failure`, as the timers run it: its exit status, the run's own
/// line (the first printed) and everything printed.
fn run(w: &World, source: &str) -> (Option<i32>, Value, String) {
    let (code, lines, all) = cortex(w, &["run", source, "--record-failure"]);
    (code, lines.first().cloned().unwrap_or(Value::Null), all)
}

/// The `sources` row of `source`.
fn row(w: &World, source: &str) -> Value {
    let (code, lines, all) = cortex(w, &["sources"]);
    assert_eq!(code, Some(0), "{all}");
    lines
        .into_iter()
        .find(|r| r["source_id"] == source)
        .expect("a row for the source")
}

fn seen_keys(dir: &Path, source: &str) -> Vec<String> {
    let text =
        std::fs::read_to_string(dir.join(format!("state/{source}.json"))).unwrap_or_default();
    let state: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    state["documents"]
        .as_object()
        .map(|d| d.keys().cloned().collect())
        .unwrap_or_default()
}

/// A web source `t/news` whose search answers 30 documents of about 4,500 characters each, three
/// batches of a run, through the stand-in `ekr`, with `extra` appended to the spec.
fn web_instance(w: &World, extra: &str) -> PathBuf {
    let results: Vec<Value> = (0..30)
        .map(|i| {
            json!({"url": format!("https://example.org/doc-{i}"), "title": format!("D{i}"),
                "description": "Example Labs ships Widget.",
                "content": format!("Example Labs develops the Widget engine {i}. {}", "word ".repeat(900)),
                "content_truncated": false, "published": null, "score": "0.5"})
        })
        .collect();
    let pages = json!({"results": results, "complete": true, "truncation": [],
        "provenance": {"instance": "t", "profile": "tavily/2026-10", "received_at": "2026-10-05T00:00:00.000Z"}});
    std::fs::write(
        w.root.join("answer.json"),
        common::answer(&pages.to_string()),
    )
    .unwrap();
    let spec = w.spec("t", "conn_test");
    let text = std::fs::read_to_string(&spec).unwrap();
    let real = ekr().display().to_string();
    assert!(text.contains(&real), "{text}");
    let text = text
        .replace(&real, &unknown_ekr(w).display().to_string())
        .replace("max_documents_per_run: 10", "max_documents_per_run: 30");
    std::fs::write(&spec, format!("{text}{extra}")).unwrap();
    let (code, lines, all) = cortex(w, &["create", "--spec", spec.to_str().unwrap()]);
    assert_eq!(code, Some(0), "{all}");
    assert_eq!(
        lines.last().and_then(|l| l["outcome"].as_str()),
        Some("created")
    );
    w.home.join("instances/t")
}

/// Asserts the run exited 4 as `store-outcome-unknown`, uncounted.
fn unknown_not_counted(w: &World, source: &str, (code, line, all): (Option<i32>, Value, String)) {
    assert_eq!(
        (code, line["outcome"].as_str()),
        (Some(4), Some("store-outcome-unknown")),
        "{all}"
    );
    let row = row(w, source);
    assert_eq!(row["state"], "Enabled", "{row}");
    assert_eq!(row["consecutive_failures"], 0, "{row}");
}

/// Outside a stage, the earlier batches of a run whose later `apply-extraction` has an unknown
/// outcome stay seen, the unknown batch does not, and the next run extracts only what was not
/// marked seen.
#[test]
fn an_unknown_later_batch_keeps_the_earlier_batches_seen_and_only_the_rest_is_extracted_again() {
    let w = World::new();
    let dir = web_instance(&w, "");
    std::fs::write(w.root.join("unknown-apply-at"), "2").unwrap();
    unknown_not_counted(&w, "t/news", run(&w, "t/news"));
    let calls = w.lines("claude-calls.log").len();
    assert_eq!(calls, 2, "the run stopped at its second batch");
    let seen = seen_keys(&dir, "news").len();
    assert!(
        seen > 0 && seen < 30,
        "seen after the unknown batch: {seen}"
    );

    std::fs::remove_file(w.root.join("unknown-apply-at")).unwrap();
    let (code, line, all) = run(&w, "t/news");
    assert_eq!(
        (code, line["outcome"].as_str()),
        (Some(0), Some("ran")),
        "{all}"
    );
    assert_eq!(
        line["detail"]["documents_applied"],
        30 - seen as i64,
        "{all}"
    );
    assert_eq!(seen_keys(&dir, "news").len(), 30);
}

/// A run held to a gate on a SQLite store: its first batch commits one refused fact, over a
/// `facts_refused max 0` gate, and its second batch's `apply-extraction` answers the unknown
/// outcome. Before this story the second batch's error stopped the run and the gate undid it, the
/// first batch included (`tests/run_gate.rs`, `a_late_batch_failing_the_gate_undoes_every_batch`).
/// Now the run returns before its gate: what the first batch committed stays in the store without
/// ever being held to the gate, and the next run, which measures only its own batches, passes it.
#[test]
fn a_gated_sqlite_run_whose_later_batch_outcome_is_unknown_leaves_no_ungated_batch_in_the_store() {
    let w = World::new();
    let dir = web_instance(
        &w,
        "gate:\n  checks:\n    - {measure: facts_refused, max: \"0\"}\n",
    );
    let before = w.head("t");
    std::fs::write(w.root.join("unknown-apply-at"), "2").unwrap();
    let (code, line, all) = run(&w, "t/news");
    assert!(code.is_some_and(|c| c != 0), "{all}");
    assert_eq!(w.lines("claude-calls.log").len(), 2, "{all}");
    assert_eq!(
        w.head("t"),
        before,
        "a batch over the gate stayed in the store, never held to it ({}): {line}",
        line["outcome"]
    );
    assert!(
        seen_keys(&dir, "news").is_empty(),
        "state/ marks documents of an undone run seen"
    );
}

const ITEMS_CLAUDE: &str = r#"echo call >> "@ROOT@/claude-calls.log"
echo '{"type":"result","subtype":"error","is_error":true}'
exit 1
"#;

/// A structured source `s/items` over `items/items.json` (`Item`, `count` an `Integer`), through
/// the stand-in `ekr`, held to no gate.
fn items_instance(w: &World, supersede: bool) -> PathBuf {
    let root = w.root.display().to_string();
    executable(
        &w.bin.join("claude"),
        &ITEMS_CLAUDE.replace("@ROOT@", &root),
    );
    std::fs::create_dir_all(w.root.join("items")).unwrap();
    let dropped = if supersede {
        "        dropped: Supersede\n"
    } else {
        ""
    };
    let path = w.root.join("s.yaml");
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: s
description: Test brain.
ekr: {{version: "0.0.34", bin: "{ekr}"}}
seed: {{documents: []}}
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
            - {{property: count, path: "$.count", value_kind: Integer}}
          relations: []
{dropped}    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
serve: {{view_port: 18993}}
"#,
            ekr = unknown_ekr(w).display(),
        ),
    )
    .unwrap();
    let (code, lines, all) = cortex(
        w,
        &["create", "--spec", path.to_str().unwrap(), "--no-units"],
    );
    assert_eq!(code, Some(0), "{all}");
    assert_eq!(
        lines.last().and_then(|l| l["outcome"].as_str()),
        Some("created")
    );
    w.home.join("instances/s")
}

fn items(w: &World, records: Value) {
    std::fs::write(
        w.root.join("items/items.json"),
        json!({ "items": records }).to_string(),
    )
    .unwrap();
}

/// The structured path, with the value-kinds unit beside it: an unconverted record and a good one
/// in one run whose only batch's `apply-extraction` has an unknown outcome. The run exits 4
/// uncounted and marks neither seen; the next run applies the good record and names the
/// unconverted one again, so neither is lost.
#[test]
fn an_unknown_structured_apply_loses_neither_the_good_record_nor_the_unconverted_one() {
    let w = World::new();
    let dir = items_instance(&w, false);
    items(
        &w,
        json!([
            {"id": "I-1", "name": "One", "count": "x1"},
            {"id": "I-2", "name": "Two", "count": 2},
        ]),
    );
    std::fs::write(w.root.join("unknown-apply-at"), "1").unwrap();
    unknown_not_counted(&w, "s/items", run(&w, "s/items"));
    assert!(
        seen_keys(&dir, "items").is_empty(),
        "{:?}",
        seen_keys(&dir, "items")
    );

    std::fs::remove_file(w.root.join("unknown-apply-at")).unwrap();
    let (code, line, all) = run(&w, "s/items");
    assert_eq!(
        (code, line["outcome"].as_str()),
        (Some(0), Some("ran")),
        "{all}"
    );
    assert_eq!(line["detail"]["documents_applied"], 1, "{all}");
    assert_eq!(
        line["detail"]["unconverted"].as_array().map(Vec::len),
        Some(1),
        "{all}"
    );
    assert_eq!(seen_keys(&dir, "items").len(), 2);
    assert_eq!(w.lines("claude-calls.log").len(), 0);
}

/// `dropped: Supersede`: the commit of the transaction that ends a dropped record's values
/// answers the unknown outcome after it landed. The run exits 4 uncounted; the run after it, with no fault,
/// runs, and the source stays enabled.
#[test]
fn an_unknown_commit_of_the_dropped_values_transaction_is_not_counted() {
    let w = World::new();
    items_instance(&w, true);
    items(
        &w,
        json!([
            {"id": "I-1", "name": "One", "count": 1},
            {"id": "I-2", "name": "Two", "count": 2},
        ]),
    );
    let (code, line, all) = run(&w, "s/items");
    assert_eq!(
        (code, line["outcome"].as_str()),
        (Some(0), Some("ran")),
        "{all}"
    );
    items(&w, json!([{"id": "I-1", "name": "One", "count": 1}]));
    std::fs::write(w.root.join("unknown-commit"), "").unwrap();
    {
        let ran = run(&w, "s/items");
        let reason = ran.1["detail"]["reason"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        unknown_not_counted(&w, "s/items", ran);
        assert!(
            reason.contains("publication outcome is unknown"),
            "{reason}"
        );
    }
    std::fs::remove_file(w.root.join("unknown-commit")).unwrap();
    let (code, line, all) = run(&w, "s/items");
    assert_eq!(
        (code, line["outcome"].as_str()),
        (Some(0), Some("ran")),
        "{all}"
    );
    assert_eq!(row(&w, "s/items")["consecutive_failures"], 0);
}
