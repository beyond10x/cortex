//! Adversary cases for PR #30 (`Fixes #24`): the 16,384-byte evidence bound and the retry of a
//! document a rejected fact cites.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{answer, ekr, executable, World};
use serde_json::{json, Value};

const BOUND: usize = 16_384;

fn spec(w: &World, max_chars: usize, max_docs: usize) -> PathBuf {
    let path = w.spec("t", "conn_test");
    let text = std::fs::read_to_string(&path)
        .unwrap()
        .replace(
            "max_chars_per_document: 5000",
            &format!("max_chars_per_document: {max_chars}"),
        )
        .replace(
            "max_documents_per_run: 10",
            &format!("max_documents_per_run: {max_docs}"),
        );
    std::fs::write(&path, text).unwrap();
    path
}

fn create(w: &World, spec: &Path) {
    let (code, out) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("created")),
        "{out}"
    );
}

/// Every run directory of instance `t`, oldest first.
fn run_dirs(w: &World, instance: &str) -> Vec<PathBuf> {
    let mut dirs: Vec<_> = std::fs::read_dir(w.home.join("instances").join(instance).join("runs"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    dirs.sort();
    dirs
}

/// A stand-in `claude` like the world's, plus a fact with a 70,000-byte value (over EKR's string
/// limit, so EKR rejects it whatever the payload size) citing the first evidence id, whenever the
/// prompt names `https://example.org/a`.
fn claude_rejecting_page_a(w: &World) {
    let root = w.root.display();
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
in=$(cat)
echo call >> "$R/claude-calls.log"
printf '%s\n' "$in" | grep -oE 'Source: \S+' | tr '\n' ' ' >> "$R/claude-sources.log"; echo >> "$R/claude-sources.log"
ids=$(printf '%s\n' "$in" | grep -oE 'evidence id [0-9a-f-]{{36}}' | cut -d' ' -f3)
facts=""
for id in $ids; do
  facts="$facts{{\"!Relation\":{{\"subject\":{{\"node_type\":\"Organization\",\"aliases\":[\"Example Labs\"]}},\"relation\":\"DEVELOPS\",\"object\":{{\"node_type\":\"Product\",\"aliases\":[\"Widget engine\"]}},\"evidence\":[\"$id\"]}}}},"
done
first=$(printf '%s\n' "$ids" | head -n 1)
case "$in" in
  *"https://example.org/a"*)
    long=$(head -c 70000 /dev/zero | tr '\0' y)
    facts="$facts{{\"!Property\":{{\"subject\":{{\"node_type\":\"Organization\",\"aliases\":[\"Example Labs\"]}},\"property\":\"website\",\"value\":{{\"value_kind\":\"String\",\"value\":\"$long\"}},\"evidence\":[\"$first\"]}}}},"
    ;;
esac
facts=${{facts%,}}
cat <<EOF
{{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{{"format":"ekr.extraction-document/1","ontology":{{"node_types":[{{"name":"Organization","parents":[],"abstract_type":false,"properties":[{{"name":"website","value":{{"value_kind":"String"}},"cardinality":"One","required":false}}]}},{{"name":"Product","parents":[],"abstract_type":false,"properties":[]}}],"edge_types":[{{"name":"DEVELOPS","source_types":["Organization"],"target_types":["Product"],"cardinality":"Many","properties":[]}}]}},"entities":[{{"node_type":"Organization","aliases":["Example Labs"]}},{{"node_type":"Product","aliases":["Widget engine"]}}],"facts":[$facts]}}}}
EOF
"#
        ),
    );
}

/// A document EKR rejects a fact of for a reason other than size, on every run, holds its slot
/// of `max_documents_per_run` on every run: the page after it is never extracted, and each run
/// spends a model call on the same page. Before the change the rejected page was recorded as seen
/// and the next run took page b.
#[test]
fn adv24_a_document_rejected_on_every_run_does_not_starve_the_page_after_it() {
    let w = World::new();
    let spec = spec(&w, 5_000, 1);
    claude_rejecting_page_a(&w);
    create(&w, &spec);
    let mut applied = 0;
    let mut runs = Vec::new();
    for _ in 0..4 {
        let (code, ran) = w.cortex(&["run", "t/news"]);
        assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
        applied += ran["detail"]["documents_applied"].as_i64().unwrap();
        runs.push(ran["detail"].clone());
    }
    let sources = w.lines("claude-sources.log");
    assert!(
        applied >= 1 && sources.iter().any(|s| s.contains("example.org/b")),
        "four runs, page b never reached the model: documents_applied total {applied}; \
         model calls saw {sources:?}; runs {runs:?}"
    );
    // Page a is recorded as seen, not tried again, and the run that applied it names it with
    // EKR's refusal: nothing is lost silently.
    let tried_a = sources
        .iter()
        .filter(|s| s.contains("example.org/a"))
        .count();
    assert_eq!(tried_a, 1, "{sources:?}");
    let named: Vec<_> = runs
        .iter()
        .flat_map(|r| r["rejected"].as_array().cloned().unwrap_or_default())
        .collect();
    assert_eq!(named.len(), 1, "{runs:?}");
    assert_eq!(named[0]["document"], "https://example.org/a", "{named:?}");
    assert!(
        named[0]["refusal"]
            .as_str()
            .is_some_and(|r| r.contains("string_bytes")),
        "{named:?}"
    );
}

/// Active assertions of instance `t` per (subject, predicate, object).
fn active(w: &World) -> std::collections::BTreeMap<(String, String, String), usize> {
    let dir = w.home.join("instances/t");
    let out = Command::new(ekr())
        .args(["sample", "--seed", "1", "--size", "1000"])
        .env("EKR_HOST", dir.join("host.json"))
        .env("EKR_BACKEND", "sqlite")
        .env("EKR_STORE", dir.join("store.sqlite"))
        .output()
        .unwrap();
    let sample: Value = serde_json::from_slice(&out.stdout).unwrap();
    let mut per = std::collections::BTreeMap::new();
    for i in sample["items"].as_array().into_iter().flatten() {
        let object = match i["assertion"]["object_kind"].as_str() {
            Some("Node") => i["object_name"].to_string(),
            _ => i["assertion"]["object_value"]["value"].to_string(),
        };
        *per.entry((
            i["subject_name"].to_string(),
            i["predicate_name"].to_string(),
            object,
        ))
        .or_default() += 1;
    }
    per
}

/// A document holding both an accepted and a rejected fact: its accepted facts are stored once,
/// by the run that applied it, and later runs over unchanged pages add none again. (Pages a and b
/// each state the DEVELOPS relation, citing their own evidence, so it is held twice from run 1;
/// what must not happen is a count that grows with each run.)
#[test]
fn adv24_a_retried_document_does_not_add_its_accepted_fact_again_on_every_run() {
    let w = World::new();
    let spec = spec(&w, 5_000, 10);
    claude_rejecting_page_a(&w);
    create(&w, &spec);
    let (code, ran) = w.cortex(&["run", "t/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let first = active(&w);
    assert!(!first.is_empty());
    for _ in 0..2 {
        let (code, ran) = w.cortex(&["run", "t/news"]);
        assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    }
    assert_eq!(
        active(&w),
        first,
        "three runs over unchanged pages; active assertions grew after the first"
    );
}

/// Three payloads of the full 16,384 bytes in one batch: EKR's limit is per sequence, so no part
/// is rejected and all three apply.
#[test]
fn adv24_three_full_payloads_in_one_batch_reject_no_part() {
    let w = World::new();
    let spec = spec(&w, 50_000, 10);
    let body = "Example Labs develops the Widget engine. ".repeat(1000);
    let page = |n: usize| {
        json!({"url": format!("https://example.org/{n}"), "title": "T", "description": "D",
            "content": body, "published": null})
    };
    let pages = json!({"results": (0..3).map(page).collect::<Vec<_>>()});
    std::fs::write(w.root.join("answer.json"), answer(&pages.to_string())).unwrap();
    create(&w, &spec);
    let (code, ran) = w.cortex(&["run", "t/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(w.lines("claude-calls.log").len(), 1, "{ran}");
    assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 3, "{ran}");
}

/// The cut lands between a letter and its combining accent. Accepted (coordinator's decision on
/// finding 4): `floor_char_boundary` keeps a character, not a grapheme cluster, so the payload may
/// end in a bare `e`. What holds is that the model is shown exactly the text the payload ends
/// with, cut at the same byte, so every citation stays consistent with its evidence.
#[test]
fn adv24_a_cut_inside_a_grapheme_cluster_cuts_the_model_text_at_the_same_byte() {
    use cortex_cli::evidence::{fit, payload, PAYLOAD_MAX_BYTES};
    use cortex_cli::sources::{Document, Origin};
    // "é" as `e` + U+0301: three bytes. A header of `h` bytes leaves `room`; pad so the cut falls
    // after the `e` and before its accent.
    let mut d = Document {
        key: "https://example.org/a".into(),
        origin: Origin::Url,
        title: None,
        description: None,
        published: None,
        text: String::new(),
        hash: None,
    };
    let head = "Source: https://example.org/a\n\n".len();
    let room = PAYLOAD_MAX_BYTES - head;
    let pad = "x".repeat(room - 1);
    d.text = format!("{pad}e\u{301}tude");
    fit(&mut d);
    let bytes = payload(&d);
    let kept = String::from_utf8(bytes).unwrap();
    assert_eq!(kept.len(), PAYLOAD_MAX_BYTES);
    assert_eq!(
        d.text,
        format!("{pad}e"),
        "the model text is cut at the same byte"
    );
    assert!(
        kept.ends_with(&d.text),
        "the payload ends with the model text"
    );
}

/// A world whose `connectors` answers `people` to `people.list`, and instance `s` created with a
/// structured source mapping each person's id, name and role.
fn structured_world(people: &Value) -> World {
    const CONNECTORS: &str = r#"R="@ROOT@"
case "$*" in
  *"connections list"*) echo '{"ok":true,"result":{"connections":[{"adapter":"directory","connection":"conn_test","state":"ready","revision":"rev1"}]}}' ;;
  *"connections revalidate"*) echo '{"ok":true,"result":{}}' ;;
  *"operations describe"*) echo '{"ok":true,"result":{"schema":"s","revision":"r"}}' ;;
  *"--operation people.list"*)
    printf '{"ok":true,"result":{"adapter":"directory","operation":"people.list","revision":"r","result":{"people":%s}}}\n' "$(cat "$R/people.json")" ;;
  *) echo '{"ok":false}'; exit 2 ;;
esac
"#;
    let w = World::new();
    let root = w.root.display().to_string();
    executable(
        &w.bin.join("connectors"),
        &CONNECTORS.replace("@ROOT@", &root),
    );
    std::fs::write(w.root.join("people.json"), people.to_string()).unwrap();
    let path = w.root.join("s.yaml");
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: s
description: Test brain.
ekr: {{version: "0.0.30", bin: "{ekr}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
  - name: people
    schedule: daily
    settings:
      kind: structured
      value:
        input:
          from: connectors
          value: {{adapter: directory, connection: conn_test, operation: people.list, inputs: [{{}}]}}
        records: "$.people"
        mapping:
          node_type: Person
          id: "$.id"
          name: "$.name"
          aliases: []
          properties:
            - {{property: role, path: "$.role"}}
          relations: []
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
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

/// A structured record is evidence for every fact mapped from it. Over the bound, its mapped
/// values are written first in its evidence, so the cut leaves every value its facts cite.
#[test]
fn adv24_a_structured_records_mapped_values_are_in_the_evidence_they_cite() {
    let about = "Ada wrote the first published program. ".repeat(500);
    let w = structured_world(
        &json!([{"about": about, "id": "P-1", "name": "Ada Lovelace",
        "role": "Engineer"}]),
    );
    let (code, ran) = w.cortex(&["run", "s/people"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let dir = run_dirs(&w, "s").pop().unwrap();
    let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(
        &std::fs::read_to_string(dir.join("batch-0/extraction.yaml")).unwrap(),
    )
    .unwrap();
    let payload: Vec<u8> = doc["evidence"][0]["payload"]
        .as_sequence()
        .unwrap()
        .iter()
        .map(|b| b.as_u64().unwrap() as u8)
        .collect();
    assert!(payload.len() <= BOUND, "{}", payload.len());
    let held = String::from_utf8(payload).unwrap();
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    for value in ["Ada Lovelace", "P-1", "Engineer"] {
        assert!(
            held.contains(value),
            "the record's facts cite evidence of {} bytes that does not hold `{value}`; it ends: {:?}",
            held.len(),
            &held[held.len() - 60..]
        );
    }
}

/// A record whose mapped values alone are over the bound cannot be cited from evidence that holds
/// them: it is refused, named in `skipped` with the reason, recorded as seen so it is not read
/// again, and the run's other records apply.
#[test]
fn adv24_a_structured_record_whose_mapped_values_exceed_the_bound_is_refused_and_named() {
    let role = "Engineer ".repeat(2_000);
    let w = structured_world(&json!([
        {"id": "P-1", "name": "Ada Lovelace", "role": role},
        {"id": "P-2", "name": "Grace Hopper", "role": "Admiral"},
    ]));
    let (code, ran) = w.cortex(&["run", "s/people"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_new"], 2, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");
    let skipped = ran["detail"]["skipped"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(skipped.len(), 1, "{ran}");
    let why = skipped[0].as_str().unwrap();
    assert!(
        why.contains("directory:people.list:P-1") && why.contains("16384"),
        "{why}"
    );
    let (_, again) = w.cortex(&["run", "s/people"]);
    assert_eq!(again["detail"]["documents_new"], 0, "{again}");
}
