//! Adversary cases for `story:batch-fits-model-window` (issue #72): an extraction batch fits the
//! model's window, and a part that still does not fit ends the run `batch-too-large`.
//!
//! Attacked: a single document larger than the window; a split three deep that applies half and
//! fails half; the error texts matched; the derived size at its clamps; the batch size in
//! characters against the bytes it was before; token budgets across splits; the gated path with
//! a split.

mod common;

use std::path::Path;
use std::process::Command;

use common::{executable, World};
use serde_json::{json, Value};

/// The stand-in `claude` wrapped, as `tests/batch_fits_model_window.rs` wraps it: each call `n`
/// keeps its prompt in `req-<n>.txt`; with the file `mode` holding `always` it answers `too.json`;
/// with `limit`, for a prompt of more bytes than the file `limit` holds; with `marker`, for a
/// prompt holding `HUGE`. Otherwise it answers as `tests/common`'s stand-in does, with a `usage`.
const WRAPPER: &str = r#"R="@ROOT@"
n=$(cat "$R/n" 2>/dev/null || echo 0)
echo $((n + 1)) > "$R/n"
cat > "$R/req-$n.txt"
size=$(wc -c < "$R/req-$n.txt")
mode=$(cat "$R/mode" 2>/dev/null || echo ok)
too=no
case "$mode" in
  always) too=yes ;;
  limit) [ "$size" -gt "$(cat "$R/limit")" ] && too=yes ;;
  marker) grep -q HUGE "$R/req-$n.txt" && too=yes ;;
esac
if [ "$too" = yes ]; then
  cat "$R/too.json"
  exit 1
fi
"@INNER@" "$@" < "$R/req-$n.txt" | sed 's/"is_error":false,/"is_error":false,"usage":{"input_tokens":10,"output_tokens":10,"cache_read_input_tokens":0,"cache_creation_input_tokens":0},/'
"#;

const TOO_LONG: &str = "Prompt is too long";

fn text(i: usize, len: usize) -> String {
    let mut t = format!("doc{i} ");
    while t.len() < len {
        t.push_str("lorem ipsum dolor sit amet ");
    }
    t.truncate(len);
    t
}

fn texts(n: usize, len: usize) -> Vec<String> {
    (0..n).map(|i| text(i, len)).collect()
}

/// A world whose connection answers one page per entry of `texts`, and whose instance `b` is
/// created with `model` appended to its `model` mapping, its policy allowing 20 documents of
/// `max_chars` characters, and `extra` appended to the spec.
fn world_with(texts: &[String], model: &str, max_chars: usize, extra: &str) -> World {
    let w = World::new();
    executable(
        &w.bin.join("claude-wrapped"),
        &WRAPPER
            .replace("@ROOT@", &w.root.display().to_string())
            .replace("@INNER@", &w.bin.join("claude").display().to_string()),
    );
    let results: Vec<Value> = texts
        .iter()
        .enumerate()
        .map(|(i, t)| {
            json!({"url": format!("https://example.org/d{i}"), "title": format!("D{i}"),
                "description": "A page.", "content": t, "content_truncated": false,
                "published": null, "score": "0.5"})
        })
        .collect();
    let pages = json!({"results": results, "complete": true, "truncation": [],
        "provenance": {"instance": "t", "profile": "tavily/2026-10",
            "received_at": "2026-10-05T00:00:00.000Z"}});
    std::fs::write(
        w.root.join("answer.json"),
        common::answer(&pages.to_string()),
    )
    .unwrap();
    let spec = w.spec("b", "conn_test");
    let mut s = std::fs::read_to_string(&spec).unwrap();
    for (from, to) in [
        ("timeout_s: 60}", format!("timeout_s: 60{model}}}")),
        (
            "max_documents_per_run: 10, max_chars_per_document: 5000}",
            format!("max_documents_per_run: 20, max_chars_per_document: {max_chars}}}"),
        ),
    ] {
        assert!(s.contains(from), "{s}");
        s = s.replacen(from, &to, 1);
    }
    s.push_str(extra);
    std::fs::write(&spec, s).unwrap();
    let (code, out) = w.cortex_env(
        &["create", "--spec", spec.to_str().unwrap(), "--no-units"],
        &[("GW_TOKEN", Path::new("t1"))],
    );
    assert_eq!(code, 0, "{out}");
    w
}

fn world(texts: &[String], model: &str, max_chars: usize) -> World {
    world_with(texts, model, max_chars, "")
}

fn answers_too_long(w: &World, mode: &str, text: &str) {
    let answer = json!({"type": "result", "subtype": "success", "is_error": true, "result": text,
        "total_cost_usd": 0, "usage": {"input_tokens": 5, "output_tokens": 0,
            "cache_read_input_tokens": 0, "cache_creation_input_tokens": 0}});
    std::fs::write(w.root.join("too.json"), answer.to_string()).unwrap();
    std::fs::write(w.root.join("mode"), mode).unwrap();
}

fn run(w: &World, record_failure: bool) -> (i32, Value) {
    let wrapped = w.bin.join("claude-wrapped");
    let mut args = vec!["--claude", wrapped.to_str().unwrap(), "run", "b/news"];
    if record_failure {
        args.push("--record-failure");
    }
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(&args)
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .env("CORTEX_MODEL_RETRY_MS", "10")
        .env("GW_TOKEN", "t1")
        .output()
        .unwrap();
    let line = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .find(|v| v["command"] == "run")
        .unwrap_or(Value::Null);
    assert_ne!(
        out.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code().unwrap_or(-1), line)
}

fn requests(w: &World) -> Vec<String> {
    let n: usize = std::fs::read_to_string(w.root.join("n"))
        .map(|s| s.trim().parse().unwrap())
        .unwrap_or(0);
    (0..n)
        .map(|i| std::fs::read_to_string(w.root.join(format!("req-{i}.txt"))).unwrap())
        .collect()
}

fn docs_of(prompt: &str) -> Vec<&str> {
    prompt
        .split("\n=== Document — evidence id ")
        .skip(1)
        .map(|p| {
            let at = p.find("\n\n").expect("a blank line before the text") + 2;
            p[at..].strip_suffix('\n').unwrap_or(&p[at..])
        })
        .collect()
}

fn sizes(w: &World) -> Vec<usize> {
    requests(w).iter().map(|r| docs_of(r).len()).collect()
}

fn consecutive_failures(w: &World) -> Value {
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .arg("sources")
        .env("CORTEX_HOME", &w.home)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .find(|row| row["source_id"] == "b/news")
        .expect("a row for b/news")["consecutive_failures"]
        .clone()
}

fn state(w: &World) -> Value {
    let path = w.home.join("instances/b/state/news.json");
    std::fs::read_to_string(path)
        .map(|s| serde_json::from_str(&s).unwrap())
        .unwrap_or(Value::Null)
}

fn seen(w: &World) -> Vec<String> {
    let mut keys: Vec<String> = state(w)["documents"]
        .as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default();
    keys.sort();
    keys
}

fn keys(range: std::ops::Range<usize>) -> Vec<String> {
    let mut k: Vec<String> = range.map(|i| format!("https://example.org/d{i}")).collect();
    k.sort();
    k
}

fn last_log(w: &World) -> Value {
    let log = std::fs::read_to_string(w.home.join("instances/b/cortex.log")).unwrap();
    serde_json::from_str(log.lines().last().unwrap()).unwrap()
}

/// One document whose text alone is over the window, first among the source's documents, at the
/// smallest `batch_chars` the spec allows. The story and the docs say the part that did not fit and
/// the batches after it are extracted by a later run, "with a smaller `model.batch_chars`". A part
/// of one document is never split and no size makes it smaller, so every later run ends
/// `batch-too-large` on the same document before any other, counted against nothing: the other
/// documents are never extracted and the source never says it is stuck.
///
/// Corrected: that document is left out alone, named in the run's `too_large` and its `cortex.log`
/// line, and marked seen until its text changes; the run goes on and ends `ran`. It is seen, so
/// it is among the keys of the source's state, and a later run does not ask it again.
#[test]
fn a_single_document_over_the_window_does_not_block_the_documents_after_it() {
    let mut docs = texts(4, 600);
    docs[0] = format!("HUGE {}", text(0, 590));
    let w = world(&docs, ", batch_chars: 1000", 5000);
    answers_too_long(&w, "marker", TOO_LONG);
    for attempt in 0..3 {
        let (code, ran) = run(&w, true);
        assert_eq!(consecutive_failures(&w), 0, "attempt {attempt}: {ran}");
        if code == 0 {
            break;
        }
        assert_eq!(
            (code, ran["outcome"].as_str()),
            (6, Some("batch-too-large")),
            "attempt {attempt}: {ran}"
        );
    }
    assert_eq!(
        seen(&w),
        keys(0..4),
        "after three runs at the smallest batch_chars, the documents after the one that can never \
         fit are extracted; requests per run: {:?}",
        sizes(&w)
    );
    // One run did it: d0 asked once and left out, d1..d3 applied.
    assert_eq!(sizes(&w), [1, 1, 1, 1]);
    let log = last_log(&w);
    assert_eq!(log["documents_applied"], 3, "{log}");
    assert_eq!(log["too_large"], json!(["https://example.org/d0"]), "{log}");
    // d0's text is unchanged: the next run does not ask it again.
    let asked = requests(&w).len();
    let (code, ran) = run(&w, true);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_new"], 0, "{ran}");
    assert_eq!(requests(&w).len(), asked, "d0 is asked again: {ran}");
}

/// 16 documents, the 10th of which (d9) never fits: 16 -> 8 + 8; the first 8 apply; 8 -> 4 + 4;
/// 4 (d8..d11) -> 2 + 2; 2 (d8, d9) at depth 3 ends the run. d0..d7 are applied and seen, d8 (an
/// innocent document in the failing part) and d9..d15 are not; the log line says why it stopped and
/// the window stays pending; a later run whose model answers extracts exactly the eight left, none
/// of them twice.
#[test]
fn a_split_three_deep_that_applies_half_keeps_that_half_and_leaves_the_rest_for_a_later_run() {
    let mut docs = texts(16, 200);
    docs[9] = format!("HUGE {}", text(9, 190));
    let w = world(&docs, "", 5000);
    answers_too_long(&w, "marker", TOO_LONG);
    let (code, ran) = run(&w, true);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (6, Some("batch-too-large")),
        "{ran}"
    );
    assert_eq!(sizes(&w), [16, 8, 8, 4, 2]);
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("after 3 splits"), "{reason}");
    assert!(reason.contains("https://example.org/d8"), "{reason}");
    assert!(reason.contains("https://example.org/d9"), "{reason}");
    assert!(!reason.contains("https://example.org/d10"), "{reason}");
    assert_eq!(seen(&w), keys(0..8));
    assert_eq!(consecutive_failures(&w), 0);
    let log = last_log(&w);
    assert_eq!(log["documents_applied"], 8, "{log}");
    assert_eq!(log["splits"], 3, "{log}");
    assert!(
        log["stopped"]
            .as_str()
            .is_some_and(|s| s.contains("did not fit")),
        "{log}"
    );
    let st = state(&w);
    assert!(
        st["pending_since"].is_i64(),
        "the window stays pending: {st}"
    );
    assert!(st["last_success_started_at"].is_null(), "{st}");

    std::fs::write(w.root.join("mode"), "ok").unwrap();
    let (code, ran) = run(&w, true);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 8, "{ran}");
    assert_eq!(&sizes(&w)[5..], [8], "only the eight left are asked");
    let asked = &requests(&w)[5];
    for i in 0..8 {
        assert!(
            !asked.contains(&format!("doc{i} ")),
            "d{i} applied by the first run is asked again"
        );
    }
    assert_eq!(seen(&w), keys(0..16));
}

/// Every wording the spec names (`Input is too long`, `reached its context window limit`) and the
/// API's own lower-case `prompt is too long: N tokens > M maximum` split the batch; the suite
/// tests only `Prompt is too long` and the output-token-maximum text, so dropping either of the
/// other two from `extract::unfit`, or its lower-casing, stays green.
#[test]
fn every_wording_the_spec_names_splits_the_batch() {
    for said in [
        "Input is too long for requested model.",
        "API Error: 400 {\"type\":\"error\",\"error\":{\"type\":\"invalid_request_error\",\"message\":\"prompt is too long: 41537 tokens > 41536 maximum\"}}",
        "Claude reached its context window limit.",
    ] {
        let w = world(&texts(5, 3000), "", 5000);
        answers_too_long(&w, "limit", said);
        std::fs::write(w.root.join("limit"), "8000").unwrap();
        let (code, ran) = run(&w, true);
        assert_eq!(
            (code, ran["outcome"].as_str()),
            (0, Some("ran")),
            "{said}: {ran}"
        );
        assert_eq!(ran["detail"]["splits"], 2, "{said}: {ran}");
        assert_eq!(sizes(&w), [5, 2, 3, 1, 2], "{said}");
    }
}

/// An OpenAI-compatible server behind a gateway (vLLM, a LiteLLM proxy) words a request over the
/// window `This model's maximum context length is N tokens`: issue #72's setting, a self-hosted
/// model behind `model.gateway`. `extract::unfit` does not know it, so the run fails as before and
/// is counted.
#[test]
fn an_openai_compatible_context_length_error_splits_the_batch() {
    let w = world(&texts(5, 3000), "", 5000);
    answers_too_long(
        &w,
        "limit",
        "API Error: 400 {\"error\":{\"message\":\"This model's maximum context length is 65536 \
         tokens. However, you requested 66001 tokens (42001 in the messages, 24000 in the \
         completion). Please reduce the length of the messages or completion.\",\
         \"type\":\"BadRequestError\",\"code\":400}}",
    );
    std::fs::write(w.root.join("limit"), "8000").unwrap();
    let (code, ran) = run(&w, true);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(consecutive_failures(&w), 0);
}

/// A document whose own text says `Prompt is too long` is extracted as any other: only an error
/// answer's text is matched.
#[test]
fn a_document_that_says_prompt_is_too_long_is_not_split() {
    let mut docs = texts(3, 300);
    docs[1] = "Prompt is too long. Input is too long. Claude's response exceeded the 16000 output \
               token maximum."
        .into();
    let w = world(&docs, "", 5000);
    let (code, ran) = run(&w, true);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["splits"], 0, "{ran}");
    assert_eq!(sizes(&w), [3]);
}

/// A window so small that 1.5 characters per token of it less the fixed part is below 4,000: the
/// derived size is 4,000. No case of the suite reaches the lower clamp.
#[test]
fn a_tiny_window_derives_the_least_size_4000() {
    let w = world(
        &texts(5, 1500),
        r#", budget_tokens: 100000000, gateway: {base_url: "http://gw.test", token_env: GW_TOKEN, max_context_tokens: 2000}"#,
        5000,
    );
    let (code, ran) = run(&w, false);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["batch_chars"], 4000, "{ran}");
    // 1,500 characters each: two to a batch of at most 4,000.
    assert_eq!(sizes(&w), [2, 2, 1]);
}

/// A window so large that the derived size is over 60,000: it is 60,000. No case of the suite
/// reaches the upper clamp.
#[test]
fn a_huge_window_derives_the_most_size_60000() {
    let w = world(
        &texts(5, 14_000),
        r#", budget_tokens: 100000000, gateway: {base_url: "http://gw.test", token_env: GW_TOKEN, max_context_tokens: 9000000000000000000, max_output_tokens: 1000}"#,
        15_000,
    );
    let (code, ran) = run(&w, false);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["batch_chars"], 60_000, "{ran}");
    assert_eq!(sizes(&w), [4, 1]);
}

/// Without a gateway or `batch_chars` the story and the docs keep the batch "60,000, as before".
/// Before this unit `batches` counted bytes (`text.len()`); now it counts characters, so a
/// document of two-byte text (Cyrillic, Greek, umlauts) or three-byte text (CJK) goes to the
/// model in a batch two or three times as large as before, on every instance with no gateway.
#[test]
fn without_a_gateway_a_batch_of_multibyte_text_is_as_large_as_before() {
    // Two-byte text; the fetch cuts each to about 8,000 characters (16,000 bytes).
    let docs: Vec<String> = (0..5)
        .map(|i| format!("doc{i} {}", "ä".repeat(13_995)))
        .collect();
    let w = world(&docs, "", 30_000);
    let (code, ran) = run(&w, false);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    // Before: a request held at most 60,000 bytes of document text, or one document.
    for (i, r) in requests(&w).iter().enumerate() {
        let docs = docs_of(r);
        let bytes: usize = docs.iter().map(|d| d.len()).sum();
        let chars: usize = docs.iter().map(|d| d.chars().count()).sum();
        assert!(
            bytes <= 60_000 || docs.len() == 1,
            "request {i}: {} documents, {chars} characters, {bytes} bytes, over 60,000 bytes",
            docs.len()
        );
    }
}

/// A token budget the first, failed attempt spends: the halves are not asked, the run stops with
/// nothing applied and nothing seen, and is counted against nothing.
#[test]
fn a_token_budget_spent_by_a_failed_attempt_asks_no_half() {
    let w = world(
        &texts(4, 300),
        r#", budget_tokens: 5, gateway: {base_url: "http://gw.test", token_env: GW_TOKEN}"#,
        5000,
    );
    answers_too_long(&w, "always", TOO_LONG);
    let (_, ran) = run(&w, true);
    assert_eq!(sizes(&w), [4], "{ran}");
    assert!(seen(&w).is_empty(), "{ran}");
    assert_eq!(consecutive_failures(&w), 0, "{ran}");
}

/// A gated run that split, applied a part, left out a one-document part that did not fit, and
/// failed its gate: the run is undone, nothing it applied stays seen, the gate's reason names the
/// document left out, and a later run extracts every document.
#[test]
fn a_gated_run_that_split_and_failed_its_gate_is_undone_and_loses_nothing() {
    let mut docs = texts(8, 200);
    docs[5] = format!("HUGE {}", text(5, 190));
    let w = world_with(
        &docs,
        "",
        5000,
        "gate:\n  checks:\n    - {measure: documents_applied, min: \"8\"}\n",
    );
    answers_too_long(&w, "marker", TOO_LONG);
    let (code, ran) = run(&w, false);
    // 8 -> 4 + 4; the first 4 apply; 4 -> 2 + 2; (d4, d5) -> 1 + 1; d4 applies; d5 at depth 3 is
    // left out alone; (d6, d7) apply. 7 applied, under the gate's 8.
    assert_eq!(sizes(&w), [8, 4, 4, 2, 1, 1, 2], "{ran}");
    assert_ne!(code, 0, "{ran}");
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("https://example.org/d5"), "{reason}");
    assert!(
        seen(&w).is_empty(),
        "the undone run leaves nothing seen: {ran}"
    );
    std::fs::write(w.root.join("mode"), "ok").unwrap();
    let (code, ran) = run(&w, false);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 8, "{ran}");
    assert_eq!(seen(&w), keys(0..8));
}
