//! `story:batch-fits-model-window` (issue #72): an extraction batch fits the model's window.
//!
//! - Size: `model.batch_chars` bounds the characters of document text in one request; without it,
//!   behind a gateway with `max_context_tokens`, the size is derived from the window, less the
//!   output reserve and the request's fixed part; otherwise it is 60,000. The run states it.
//! - Split: a request `claude` answers as too long for the window, or whose answer was cut off at
//!   the output limit, is split in two by documents and asked again, at most 3 splits deep.
//! - Bound: a part that still does not fit ends the run `batch-too-large`, exit 6, counted against
//!   nothing; the batches before it stay applied and the rest are extracted by a later run.
//! - `model.gateway.max_output_tokens` reaches `claude` as `CLAUDE_CODE_MAX_OUTPUT_TOKENS`.

mod common;

use std::path::Path;
use std::process::Command;

use common::{executable, World};
use serde_json::{json, Value};

/// The stand-in `claude` wrapped. Each call `n` keeps its prompt in `req-<n>.txt`, its system
/// prompt and schema in `sys.txt` and `schema.txt`, and `CLAUDE_CODE_MAX_OUTPUT_TOKENS` as it got
/// it in `out-env.log`. With the file `mode` holding `always` it answers `too.json`; with `limit`,
/// it answers `too.json` for a prompt of more bytes than the file `limit` holds; with `marker`,
/// for a prompt holding `HUGE`. Otherwise it answers as `tests/common`'s stand-in does, with a
/// `usage` added so a token budget can count it.
const WRAPPER: &str = r#"R="@ROOT@"
n=$(cat "$R/n" 2>/dev/null || echo 0)
echo $((n + 1)) > "$R/n"
cat > "$R/req-$n.txt"
keep() {
  while [ $# -gt 0 ]; do
    case "$1" in
      --system-prompt) printf '%s' "$2" > "$R/sys.txt" ;;
      --json-schema) printf '%s' "$2" > "$R/schema.txt" ;;
    esac
    shift
  done
}
keep "$@"
echo "call ${CLAUDE_CODE_MAX_OUTPUT_TOKENS-unset}" >> "$R/out-env.log"
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

/// What `claude -p` answers for a request over the model's window.
const TOO_LONG: &str = "Prompt is too long";
/// What `claude -p` answers when the model's answer ran past the output limit.
const CUT_OFF: &str = "API Error: Claude's response exceeded the 16000 output token maximum. To \
     configure this behavior, set the CLAUDE_CODE_MAX_OUTPUT_TOKENS environment variable.";

/// A gateway with a 64k-token window and a 24,000-token output cap, as in issue #72.
const GATEWAY: &str = r#"{base_url: "http://gw.test", token_env: GW_TOKEN, max_context_tokens: 65536, max_output_tokens: 24000}"#;

/// The text of document `i`: `len` characters of words, starting with its number.
fn text(i: usize, len: usize) -> String {
    let mut t = format!("doc{i} ");
    while t.len() < len {
        t.push_str("lorem ipsum dolor sit amet ");
    }
    t.truncate(len);
    t
}

/// A world whose connection answers one page per entry of `texts`, and
/// whose instance `b` is created from the spec with `model` appended to its `model` mapping and
/// its policy allowing 20 documents of `max_chars` characters.
fn world(texts: &[String], model: &str, max_chars: usize) -> World {
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
    std::fs::write(&spec, s).unwrap();
    let (code, out) = w.cortex_env(
        &["create", "--spec", spec.to_str().unwrap(), "--no-units"],
        &[("GW_TOKEN", Path::new("t1"))],
    );
    assert_eq!(code, 0, "{out}");
    w
}

/// `claude` answers `text` as an error, at no cost, when it answers too long.
fn answers_too_long(w: &World, mode: &str, text: &str) {
    let answer = json!({"type": "result", "subtype": "success", "is_error": true, "result": text,
        "total_cost_usd": 0, "usage": {"input_tokens": 5, "output_tokens": 0,
            "cache_read_input_tokens": 0, "cache_creation_input_tokens": 0}});
    std::fs::write(w.root.join("too.json"), answer.to_string()).unwrap();
    std::fs::write(w.root.join("mode"), mode).unwrap();
}

/// `cortex run b/news` through the wrapped `claude`: its exit status and its `run` line (with
/// `--record-failure`, a counted failure prints a second line after it).
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

/// The prompts `claude` was sent, in order.
fn requests(w: &World) -> Vec<String> {
    let n: usize = std::fs::read_to_string(w.root.join("n"))
        .map(|s| s.trim().parse().unwrap())
        .unwrap_or(0);
    (0..n)
        .map(|i| std::fs::read_to_string(w.root.join(format!("req-{i}.txt"))).unwrap())
        .collect()
}

/// A prompt's fixed part (everything before its first document) and the text of each document.
fn parts(prompt: &str) -> (&str, Vec<&str>) {
    let mut pieces = prompt.split("\n=== Document — evidence id ");
    let fixed = pieces.next().unwrap();
    let docs = pieces
        .map(|p| {
            let at = p.find("\n\n").expect("a blank line before the text") + 2;
            p[at..].strip_suffix('\n').unwrap_or(&p[at..])
        })
        .collect();
    (fixed, docs)
}

/// Every request holds at most `size` characters of document text, counted as UTF-8 bytes, or
/// one document.
fn assert_within(w: &World, size: usize) {
    let requests = requests(w);
    assert!(!requests.is_empty());
    for (i, r) in requests.iter().enumerate() {
        let (_, docs) = parts(r);
        let chars: usize = docs.iter().map(|d| d.len()).sum();
        assert!(
            chars <= size || docs.len() == 1,
            "request {i}: {} documents, {chars} characters, over {size}",
            docs.len()
        );
    }
}

/// `consecutive_failures` of `b/news`, from `cortex sources`.
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

/// The keys of `b/news`'s seen documents.
fn seen(w: &World) -> Vec<String> {
    let path = w.home.join("instances/b/state/news.json");
    let state: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    state["documents"]
        .as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default()
}

/// The last `cortex.log` line of instance `b`.
fn last_log(w: &World) -> Value {
    let log = std::fs::read_to_string(w.home.join("instances/b/cortex.log")).unwrap();
    serde_json::from_str(log.lines().last().unwrap()).unwrap()
}

fn texts(n: usize, len: usize) -> Vec<String> {
    (0..n).map(|i| text(i, len)).collect()
}

#[test]
fn batch_chars_bounds_the_document_text_of_one_request() {
    let w = world(&texts(5, 3000), ", batch_chars: 5000", 5000);
    let (code, ran) = run(&w, false);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 5, "{ran}");
    assert_eq!(ran["detail"]["batch_chars"], 5000, "{ran}");
    assert_eq!(ran["detail"]["splits"], 0, "{ran}");
    assert_eq!(
        requests(&w).len(),
        5,
        "one 3,000-character document per request"
    );
    assert_within(&w, 5000);
    assert_eq!(last_log(&w)["batch_chars"], 5000);
}

#[test]
fn without_a_gateway_or_batch_chars_a_batch_is_60000_characters_as_before() {
    let w = world(&texts(5, 14_000), "", 15_000);
    let (code, ran) = run(&w, false);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["batch_chars"], 60_000, "{ran}");
    let requests = requests(&w);
    assert_eq!(
        requests.len(),
        2,
        "four documents (56,000 characters), then one"
    );
    assert_eq!(parts(&requests[0]).1.len(), 4);
    assert_within(&w, 60_000);
    let out = w.lines("out-env.log");
    assert!(out.iter().all(|l| l == "call unset"), "{out:?}");
}

#[test]
fn behind_a_gateway_the_size_is_derived_from_the_window_less_the_output_and_the_fixed_part() {
    let w = world(
        &texts(5, 14_000),
        &format!(", budget_tokens: 100000000, gateway: {GATEWAY}"),
        15_000,
    );
    let (code, ran) = run(&w, false);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 5, "{ran}");
    // The fixed part as `claude` received it: the system prompt, the schema and the prompt up to
    // the documents.
    // Counted as UTF-8 bytes, as the batch is.
    let chars = |f: &str| std::fs::read_to_string(w.root.join(f)).unwrap().len();
    let first = &requests(&w)[0];
    let fixed = chars("sys.txt") + chars("schema.txt") + parts(first).0.len();
    // (65,536 - 24,000) tokens of input room, 1.5 characters each.
    let derived = (62_304 - fixed as i64).clamp(4_000, 60_000);
    assert!(
        derived < 56_000,
        "a derived size of {derived} would not tell itself from 60,000 here"
    );
    assert_eq!(ran["detail"]["batch_chars"], derived, "{ran}");
    assert_within(&w, derived as usize);
    assert_eq!(last_log(&w)["batch_chars"], derived);
    let out = w.lines("out-env.log");
    assert!(
        !out.is_empty() && out.iter().all(|l| l == "call 24000"),
        "{out:?}"
    );
}

#[test]
fn a_gateway_without_max_output_tokens_passes_no_output_cap_and_reserves_a_quarter() {
    let w = world(
        &texts(2, 1000),
        r#", budget_tokens: 100000000, gateway: {base_url: "http://gw.test", token_env: GW_TOKEN, max_context_tokens: 16384}"#,
        5000,
    );
    let (code, ran) = run(&w, false);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let chars = |f: &str| {
        std::fs::read_to_string(w.root.join(f))
            .unwrap()
            .chars()
            .count()
    };
    let fixed = chars("sys.txt") + chars("schema.txt") + parts(&requests(&w)[0]).0.chars().count();
    // 16,384 less a quarter is 12,288 tokens of room: 18,432 characters.
    let derived = (18_432 - fixed as i64).clamp(4_000, 60_000);
    assert_eq!(ran["detail"]["batch_chars"], derived, "{ran}");
    let out = w.lines("out-env.log");
    assert!(out.iter().all(|l| l == "call unset"), "{out:?}");
}

/// A stand-in that answers `answer` for any prompt over 8,000 bytes: two 3,000-character
/// documents fit, three do not.
fn split_until_it_fits(answer: &str) {
    let w = world(&texts(5, 3000), "", 5000);
    answers_too_long(&w, "limit", answer);
    std::fs::write(w.root.join("limit"), "8000").unwrap();
    let (code, ran) = run(&w, true);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 5, "{ran}");
    // 5 -> 2 + 3; 3 -> 1 + 2.
    assert_eq!(ran["detail"]["splits"], 2, "{ran}");
    let sizes: Vec<usize> = requests(&w).iter().map(|r| parts(r).1.len()).collect();
    assert_eq!(sizes, [5, 2, 3, 1, 2], "no answer asked again after a wait");
    assert_eq!(ran["detail"]["model_calls"], 5, "{ran}");
    assert_eq!(seen(&w).len(), 5);
    assert_eq!(consecutive_failures(&w), 0);
    assert_eq!(last_log(&w)["splits"], 2);
}

#[test]
fn a_batch_too_long_for_the_window_is_split_and_every_document_extracted() {
    split_until_it_fits(TOO_LONG);
}

#[test]
fn a_batch_whose_answer_is_cut_off_at_the_output_limit_is_split_and_every_document_extracted() {
    split_until_it_fits(CUT_OFF);
}

#[test]
fn after_3_splits_the_run_ends_batch_too_large_counted_against_nothing() {
    let w = world(&texts(16, 200), "", 5000);
    answers_too_long(&w, "always", TOO_LONG);
    for _ in 0..2 {
        let (code, ran) = run(&w, true);
        assert_eq!(
            (code, ran["outcome"].as_str()),
            (6, Some("batch-too-large")),
            "{ran}"
        );
        let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
        // 16 -> 8 -> 4 -> 2: the part of two documents still did not fit.
        assert!(reason.contains("https://example.org/d0"), "{reason}");
        assert!(reason.contains("https://example.org/d1"), "{reason}");
        assert!(!reason.contains("https://example.org/d2"), "{reason}");
        assert!(reason.contains("60000"), "the size used: {reason}");
        assert_eq!(consecutive_failures(&w), 0, "not counted");
    }
    let sizes: Vec<usize> = requests(&w).iter().map(|r| parts(r).1.len()).collect();
    assert_eq!(sizes, [16, 8, 4, 2, 16, 8, 4, 2]);
    assert!(seen(&w).is_empty(), "nothing applied, nothing seen");
    // The source is still enabled, and a run whose model answers extracts every document.
    std::fs::write(w.root.join("mode"), "ok").unwrap();
    let (code, ran) = run(&w, true);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 16, "{ran}");
}

/// A part of one document that does not fit is not split and does not end the run: it is left
/// out alone, named in `too_large` in the report and `cortex.log`, and marked seen until its text
/// changes; the batches before and after it are applied and the run ends `ran`.
#[test]
fn a_one_document_part_is_not_split_but_left_out_alone_and_the_run_goes_on() {
    let mut docs = texts(5, 600);
    docs[2] = format!("HUGE {}", text(2, 590));
    let w = world(&docs, ", batch_chars: 1000", 5000);
    answers_too_long(&w, "marker", TOO_LONG);
    let (code, ran) = run(&w, true);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 4, "{ran}");
    assert_eq!(
        ran["detail"]["too_large"],
        json!(["https://example.org/d2"]),
        "{ran}"
    );
    assert_eq!(last_log(&w)["too_large"], json!(["https://example.org/d2"]));
    let mut seen_keys = seen(&w);
    seen_keys.sort();
    assert_eq!(
        seen_keys,
        (0..5)
            .map(|i| format!("https://example.org/d{i}"))
            .collect::<Vec<_>>(),
        "the document left out is seen with the applied ones"
    );
    assert_eq!(requests(&w).len(), 5, "a one-document part is not split");
    assert_eq!(consecutive_failures(&w), 0);
    // A later run asks nothing: the document left out is unchanged.
    std::fs::write(w.root.join("mode"), "ok").unwrap();
    let (code, ran) = run(&w, true);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_new"], 0, "{ran}");
    assert_eq!(requests(&w).len(), 5, "{ran}");
}

#[test]
fn another_error_answer_is_not_split_and_is_counted_as_before() {
    let w = world(&texts(5, 3000), "", 5000);
    answers_too_long(&w, "always", "The model stopped.");
    let (code, ran) = run(&w, true);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("extraction-failed")),
        "{ran}"
    );
    // A zero-cost error answer is asked once more, as before; never split.
    let sizes: Vec<usize> = requests(&w).iter().map(|r| parts(r).1.len()).collect();
    assert_eq!(sizes, [5, 5]);
    assert_eq!(consecutive_failures(&w), 1);
}

/// `cortex create` of a spec with `model` appended to its `model` mapping: its exit status and
/// stderr.
fn create(model: &str) -> (Option<i32>, String) {
    let w = World::new();
    let spec = w.spec("r", "conn_test");
    let s = std::fs::read_to_string(&spec).unwrap();
    std::fs::write(
        &spec,
        s.replacen("timeout_s: 60}", &format!("timeout_s: 60{model}}}"), 1),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["create", "--spec", spec.to_str().unwrap(), "--no-units"])
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .env("GW_TOKEN", "t1")
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code(), text)
}

#[test]
fn the_spec_check_refuses_a_batch_chars_below_1000_and_an_output_cap_outside_the_window() {
    for (model, field) in [
        (", batch_chars: 999", "model.batch_chars"),
        (
            r#", budget_tokens: 1000, gateway: {base_url: "http://gw.test", token_env: GW_TOKEN, max_context_tokens: 1000, max_output_tokens: 1000}"#,
            "model.gateway.max_output_tokens",
        ),
        (
            r#", budget_tokens: 1000, gateway: {base_url: "http://gw.test", token_env: GW_TOKEN, max_output_tokens: 0}"#,
            "model.gateway.max_output_tokens",
        ),
    ] {
        let (code, text) = create(model);
        assert_eq!(code, Some(2), "{model}: {text}");
        assert!(text.contains(field), "{model}: {text}");
    }
    let (code, text) = create(", batch_chars: 1000");
    assert_eq!(code, Some(0), "{text}");
}
