//! Review of 0.7.2, findings 7 and 10.
//!
//! 7: a `datasource.websearch/v1alpha1` answer names the URLs it could not read in `failed` and
//! says in `complete` whether every requested item is present. A fetch-only run lists each failed
//! URL and an incomplete answer in `unread`; a run names them in `stopped`.
//!
//! 10: a page's description is part of its evidence header. The header is cut to what the
//! 16,384-byte evidence payload holds, so the model is shown, and the store is given, no
//! description text the evidence does not hold.


use std::path::PathBuf;

use crate::common::{answer, executable, World};
use serde_json::{json, Value};

/// EKR's bound on one evidence payload, in bytes.
const BOUND: usize = 16_384;

const SEARCH: &str = "input: search
          value:
            queries: [widget engine]
            policy: {topic: news, max_results: 3, include_domains: [], exclude_domains: []}";

/// The world's spec with its web source reading `urls` in `mode` (`Pages` or `Crawl`).
fn sites_spec(w: &World, mode: &str, urls: &[&str]) -> PathBuf {
    let path = w.spec("t", "conn_test");
    let sites = format!(
        "input: sites\n          value: {{urls: {}, mode: {mode}}}",
        serde_json::to_string(urls).unwrap()
    );
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains(SEARCH), "the world's spec changed");
    std::fs::write(&path, text.replace(SEARCH, &sites)).unwrap();
    path
}

fn provider(w: &World, result: Value) {
    std::fs::write(w.root.join("answer.json"), answer(&result.to_string())).unwrap();
}

fn create(w: &World, spec: &std::path::Path) {
    let (code, out) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("created")),
        "{out}"
    );
}

fn unread(fetched: &Value) -> Vec<String> {
    fetched["detail"]["unread"]
        .as_array()
        .unwrap_or_else(|| panic!("no unread list: {fetched}"))
        .iter()
        .map(|u| u.as_str().unwrap().to_string())
        .collect()
}

/// The review's scenario: `websearch.fetch` answers the contract's per-URL failure for the only
/// URL. Fetch-only lists it in `unread` and the run names it in `stopped`; the run is not a
/// silent success.
#[test]
fn a_url_the_provider_failed_is_unread_in_fetch_only_and_stops_the_run() {
    let w = World::new();
    let spec = sites_spec(&w, "Pages", &["https://example.invalid/offline"]);
    create(&w, &spec);
    provider(
        &w,
        json!({"pages": [], "failed": [{"url": "https://example.invalid/offline",
            "reason": "provider_refused"}], "complete": true, "truncation": []}),
    );

    let (code, fetched) = w.cortex(&["run", "t/news", "--fetch-only"]);
    assert_eq!(
        (code, fetched["outcome"].as_str()),
        (0, Some("fetched")),
        "{fetched}"
    );
    assert_eq!(fetched["detail"]["fetched"], 0, "{fetched}");
    let lines = unread(&fetched);
    assert_eq!(lines.len(), 1, "{fetched}");
    assert!(
        lines[0].contains("https://example.invalid/offline")
            && lines[0].contains("provider_refused"),
        "{fetched}"
    );

    let (code, ran) = w.cortex(&["run", "t/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let stopped = ran["detail"]["stopped"].as_str().unwrap_or_default();
    assert!(
        stopped.contains("https://example.invalid/offline") && stopped.contains("provider_refused"),
        "{ran}"
    );
}

/// The control beside it: one URL read and one failed. The page read is new and applied; only
/// the failed URL is named.
#[test]
fn a_page_read_beside_a_failed_url_is_applied_and_only_the_failure_is_named() {
    let w = World::new();
    let spec = sites_spec(
        &w,
        "Pages",
        &["https://example.org/a", "https://example.invalid/offline"],
    );
    create(&w, &spec);
    provider(
        &w,
        json!({"pages": [{"url": "https://example.org/a", "title": "A",
            "content": "Example Labs develops the Widget engine.", "content_truncated": false}],
            "failed": [{"url": "https://example.invalid/offline", "reason": "timeout"}],
            "complete": true, "truncation": []}),
    );

    let (_, fetched) = w.cortex(&["run", "t/news", "--fetch-only"]);
    assert_eq!(
        (&fetched["detail"]["fetched"], &fetched["detail"]["new"]),
        (&json!(1), &json!(1)),
        "{fetched}"
    );
    let lines = unread(&fetched);
    assert_eq!(lines.len(), 1, "{fetched}");
    assert!(
        lines[0].contains("https://example.invalid/offline") && !lines[0].contains("example.org/a"),
        "{fetched}"
    );

    let (code, ran) = w.cortex(&["run", "t/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    let stopped = ran["detail"]["stopped"].as_str().unwrap_or_default();
    assert!(stopped.contains("https://example.invalid/offline"), "{ran}");
}

/// The review's second scenario: a crawl the provider answered partially (`complete: false`,
/// `provider_partial`). Fetch-only lists it in `unread` with its cause, and the run names it.
#[test]
fn an_incomplete_crawl_is_unread_in_fetch_only_and_stops_the_run() {
    let w = World::new();
    let spec = sites_spec(&w, "Crawl", &["https://example.org/"]);
    create(&w, &spec);
    provider(
        &w,
        json!({"pages": [], "failed": [], "complete": false, "truncation": ["provider_partial"]}),
    );

    let (_, fetched) = w.cortex(&["run", "t/news", "--fetch-only"]);
    assert_eq!(fetched["outcome"], "fetched", "{fetched}");
    let lines = unread(&fetched);
    assert_eq!(lines.len(), 1, "{fetched}");
    assert!(
        lines[0].contains("websearch.crawl") && lines[0].contains("provider_partial"),
        "{fetched}"
    );

    let (code, ran) = w.cortex(&["run", "t/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let stopped = ran["detail"]["stopped"].as_str().unwrap_or_default();
    assert!(stopped.contains("provider_partial"), "{ran}");
}

/// A search that found fewer results than `max_results` (`complete: false` with `provider_limit`
/// alone) left nothing the source asked for unread, and clipped content (`content_bytes`) omits no
/// item: neither is named. Any other cause on a search is.
#[test]
fn a_search_short_of_max_results_is_not_unread_but_a_partial_search_is() {
    let w = World::new();
    let spec = w.spec("t", "conn_test");
    create(&w, &spec);
    let page = json!({"url": "https://example.org/a", "title": "A", "description": "d",
        "content": "Example Labs develops the Widget engine.", "content_truncated": true,
        "published": null});
    provider(
        &w,
        json!({"results": [page], "complete": false,
            "truncation": ["provider_limit", "content_bytes"]}),
    );
    let (_, fetched) = w.cortex(&["run", "t/news", "--fetch-only"]);
    assert_eq!(unread(&fetched), Vec::<String>::new(), "{fetched}");

    provider(
        &w,
        json!({"results": [page], "complete": false, "truncation": ["result_limit"]}),
    );
    let (_, fetched) = w.cortex(&["run", "t/news", "--fetch-only"]);
    let lines = unread(&fetched);
    assert_eq!(lines.len(), 1, "{fetched}");
    assert!(
        lines[0].contains("websearch.search") && lines[0].contains("result_limit"),
        "{fetched}"
    );
}

/// The review's fixture: a 17,471-byte description and a short body. The description is cut so
/// the whole header and the body fit the payload; the prompt and the stored `description` hold
/// only what the evidence holds, and the body is kept whole.
#[test]
fn a_description_over_the_payload_is_cut_to_what_the_evidence_holds() {
    let w = World::new();
    // claude: answers no facts of its own; cortex adds the page's `WebPage` facts.
    executable(
        &w.bin.join("claude"),
        r#"cat > /dev/null
echo '{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0,"structured_output":{"format":"ekr.extraction-document/1","ontology":{"node_types":[],"edge_types":[]},"entities":[],"facts":[]}}'
"#,
    );
    let spec = w.spec("t", "conn_test");
    create(&w, &spec);
    let marker = "Final location: Harbor library.";
    let description = format!("{}{marker}", "A field report. ".repeat(1_090));
    assert_eq!(description.len(), 17_471);
    provider(
        &w,
        json!({"results": [{"url": "https://example.test/report", "title": "Report",
            "description": description, "content": "Field notes", "content_truncated": false,
            "published": null}], "complete": true, "truncation": []}),
    );

    let (code, ran) = w.cortex(&["run", "t/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");

    let batch = batch_dir(&w);
    let extraction: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(batch.join("extraction.yaml")).unwrap())
            .unwrap();
    let payload: Vec<u8> = extraction["evidence"][0]["payload"]
        .as_sequence()
        .unwrap()
        .iter()
        .map(|b| b.as_u64().unwrap() as u8)
        .collect();
    assert!(payload.len() <= BOUND, "{}", payload.len());
    let evidence = String::from_utf8(payload).expect("cut at a character boundary");
    assert!(
        evidence.ends_with("\n\nField notes"),
        "the body is kept whole"
    );

    let prompt = std::fs::read_to_string(batch.join("prompt.txt")).unwrap();
    assert!(
        !prompt.contains(marker),
        "the model is shown no text outside the evidence"
    );
    let shown = prompt
        .lines()
        .find_map(|l| l.strip_prefix("Description: "))
        .expect("a description line");
    assert!(evidence.contains(&format!("Description: {shown}\n")));

    let stored = described(&extraction).expect("a stored description");
    assert!(!stored.contains(marker), "{}", stored.len());
    assert!(evidence.contains(&format!("Description: {stored}\n")));
    assert!(description.starts_with(&stored) && stored.len() > 15_000);
}

/// The newest run directory's first batch.
fn batch_dir(w: &World) -> PathBuf {
    let runs = w.home.join("instances/t/runs");
    let mut dirs: Vec<_> = std::fs::read_dir(&runs)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.file_name().unwrap().to_string_lossy().ends_with("-news"))
        .collect();
    dirs.sort();
    dirs.pop().expect("a run directory").join("batch-0")
}

/// The `description` property the batch writes on its `WebPage`.
fn described(extraction: &serde_yaml_ng::Value) -> Option<String> {
    let text = serde_json::to_value(extraction).ok()?;
    let mut found = None;
    walk(&text, &mut |v| {
        if v["property"] == "description" {
            found = v["value"]["value"].as_str().map(str::to_string);
        }
    });
    found
}

fn walk(v: &Value, f: &mut impl FnMut(&Value)) {
    f(v);
    match v {
        Value::Array(a) => a.iter().for_each(|x| walk(x, f)),
        Value::Object(o) => o.values().for_each(|x| walk(x, f)),
        _ => {}
    }
}
