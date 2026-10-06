//! EKR 0.0.30 takes an evidence payload of at most 16,384 bytes (one YAML sequence element per
//! byte, its `sequence_elements` limit) and rejects every fact citing a longer one. cortex cuts each
//! payload to that bound at a character boundary, on a source run and on the seed alike, and a
//! document a rejected fact cites is not recorded as seen, so the next run tries it again.

mod common;

use std::path::{Path, PathBuf};

use common::{answer, executable, World};
use serde_json::Value;

/// EKR 0.0.30's bound on one evidence payload, in bytes.
const BOUND: usize = 16_384;

fn spec(w: &World, max_chars: usize) -> PathBuf {
    let path = w.spec("t", "conn_test");
    let text = std::fs::read_to_string(&path).unwrap().replace(
        "max_chars_per_document: 5000",
        &format!("max_chars_per_document: {max_chars}"),
    );
    std::fs::write(&path, text).unwrap();
    path
}

fn one_page(w: &World, url: &str, content: &str) {
    let pages = serde_json::json!({"results": [{"url": url, "title": "T", "description": "d",
        "content": content, "published": null}]});
    std::fs::write(w.root.join("answer.json"), answer(&pages.to_string())).unwrap();
}

fn created(w: &World, spec: &Path, extra: &[&str]) -> Value {
    let mut args = vec!["create", "--spec", spec.to_str().unwrap(), "--no-units"];
    args.extend_from_slice(extra);
    let (code, out) = w.cortex(&args);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("created")),
        "{out}"
    );
    out
}

/// The evidence payloads of the newest batch the run labelled `label` applied.
fn payloads(w: &World, label: &str) -> Vec<Vec<u8>> {
    let runs = w.home.join("instances/t/runs");
    let mut dirs: Vec<_> = std::fs::read_dir(&runs)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .ends_with(&format!("-{label}"))
        })
        .collect();
    dirs.sort();
    let path = dirs
        .last()
        .expect("a run directory")
        .join("batch-0/extraction.yaml");
    let doc: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    doc["evidence"]
        .as_sequence()
        .unwrap()
        .iter()
        .map(|item| {
            item["payload"]
                .as_sequence()
                .unwrap()
                .iter()
                .map(|b| b.as_u64().unwrap() as u8)
                .collect()
        })
        .collect()
}

fn log_line(w: &World, source: &str) -> Value {
    std::fs::read_to_string(w.home.join("instances/t/cortex.log"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .rfind(|l| l["source"] == source)
        .expect("a log line")
}

/// An ASCII page of 30,000 characters is cut to exactly the bound, and EKR takes it: 16,384 bytes
/// is a payload EKR accepts.
#[test]
fn a_page_over_the_bound_is_cut_to_exactly_16384_bytes_and_applied() {
    let w = World::new();
    let spec = spec(&w, 30_000);
    one_page(
        &w,
        "https://example.org/long",
        &"Example Labs ships. ".repeat(1_500),
    );
    created(&w, &spec, &[]);
    let (code, ran) = w.cortex(&["run", "t/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    assert_eq!(ran["detail"]["truncated"], 1, "the cut is counted: {ran}");
    let payloads = payloads(&w, "news");
    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0].len(), BOUND);
    assert!(payloads[0].starts_with(b"Source: https://example.org/long\n"));
}

/// The defect hunt's page: 8,000 three-byte characters (24 KB) under `max_chars_per_document`
/// 8000. The payload is cut at a character boundary within the bound, and its facts are stored.
#[test]
fn a_page_of_multibyte_characters_is_cut_at_a_character_boundary_and_its_facts_are_stored() {
    let w = World::new();
    let spec = spec(&w, 8_000);
    let page: String = "示例实验室开发了小部件引擎。"
        .repeat(700)
        .chars()
        .take(8_000)
        .collect();
    one_page(&w, "https://example.org/zh", &page);
    created(&w, &spec, &[]);
    let (code, ran) = w.cortex(&["run", "t/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    let payloads = payloads(&w, "news");
    let text = String::from_utf8(payloads[0].clone()).expect("cut at a character boundary");
    assert!(
        text.len() <= BOUND && text.len() > BOUND - 3,
        "{}",
        text.len()
    );
    assert!(page.starts_with(text.split("\n\n").nth(1).unwrap()));
}

/// The seed takes the same bound: a seed file of 40,000 characters is applied with no part
/// rejected.
#[test]
fn a_seed_document_over_the_bound_is_cut_and_applied() {
    let w = World::new();
    let spec = spec(&w, 5_000);
    let text = std::fs::read_to_string(&spec)
        .unwrap()
        .replace("seed: {documents: []}", "seed: {documents: [\"docs\"]}");
    std::fs::write(&spec, text).unwrap();
    std::fs::create_dir_all(w.root.join("docs")).unwrap();
    std::fs::write(
        w.root.join("docs/big.md"),
        "Example Labs develops the Widget engine. ü ".repeat(1_000),
    )
    .unwrap();
    created(&w, &spec, &[]);
    let seed = log_line(&w, "seed");
    assert_eq!(seed["parts_rejected"], 0, "{seed}");
    assert_eq!(seed["documents_applied"], 1, "{seed}");
    let payloads = payloads(&w, "seed");
    assert!(payloads[0].len() <= BOUND, "{}", payloads[0].len());
    String::from_utf8(payloads[0].clone()).expect("cut at a character boundary");
}

/// A fact EKR rejects (a 70,000-byte value, over its string limit) cites the first document of
/// the batch. EKR would reject it again on the same text, so the document is recorded as seen and
/// not tried again (a retry would cost a model call and a `max_documents_per_run` slot on every
/// run); the run report names it with EKR's refusal, so nothing is lost silently.
#[test]
fn a_document_a_rejected_fact_cites_is_seen_and_named_in_the_report() {
    let w = World::new();
    let spec = spec(&w, 5_000);
    let root = w.root.display();
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
echo call >> "$R/claude-calls.log"
ids=$(grep -oE 'evidence id [0-9a-f-]{{36}}' | cut -d' ' -f3)
facts=""
for id in $ids; do
  facts="$facts{{\"!Relation\":{{\"subject\":{{\"node_type\":\"Organization\",\"aliases\":[\"Example Labs\"]}},\"relation\":\"DEVELOPS\",\"object\":{{\"node_type\":\"Product\",\"aliases\":[\"Widget engine\"]}},\"evidence\":[\"$id\"]}}}},"
done
first=$(echo "$ids" | head -n 1)
if [ -e "$R/long" ]; then
  long=$(head -c 70000 /dev/zero | tr '\0' y)
  facts="$facts{{\"!Property\":{{\"subject\":{{\"node_type\":\"Organization\",\"aliases\":[\"Example Labs\"]}},\"property\":\"website\",\"value\":{{\"value_kind\":\"String\",\"value\":\"$long\"}},\"evidence\":[\"$first\"]}}}},"
fi
facts=${{facts%,}}
cat <<EOF
{{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{{"format":"ekr.extraction-document/1","ontology":{{"node_types":[{{"name":"Organization","parents":[],"abstract_type":false,"properties":[{{"name":"website","value":{{"value_kind":"String"}},"cardinality":"One","required":false}}]}},{{"name":"Product","parents":[],"abstract_type":false,"properties":[]}}],"edge_types":[{{"name":"DEVELOPS","source_types":["Organization"],"target_types":["Product"],"cardinality":"Many","properties":[]}}]}},"entities":[{{"node_type":"Organization","aliases":["Example Labs"]}},{{"node_type":"Product","aliases":["Widget engine"]}}],"facts":[$facts]}}}}
EOF
"#
        ),
    );
    std::fs::write(w.root.join("long"), "").unwrap();
    created(&w, &spec, &[]);

    let (code, ran) = w.cortex(&["run", "t/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_new"], 2, "{ran}");
    assert_eq!(ran["detail"]["parts_rejected"], 1, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
    let rejected = ran["detail"]["rejected"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(rejected.len(), 1, "{ran}");
    assert!(rejected[0]["document"]
        .as_str()
        .is_some_and(|d| d.starts_with("https://example.org/")));
    assert!(
        rejected[0]["refusal"]
            .as_str()
            .is_some_and(|r| r.contains("string_bytes")),
        "{ran}"
    );
    assert_eq!(log_line(&w, "news")["rejected"], ran["detail"]["rejected"]);

    // Nothing changed upstream: nothing is tried again.
    let (_, again) = w.cortex(&["run", "t/news"]);
    assert_eq!(again["detail"]["documents_new"], 0, "{again}");
    assert_eq!(again["detail"]["rejected"], Value::Null, "{again}");
    assert_eq!(w.lines("claude-calls.log").len(), 1);
}
