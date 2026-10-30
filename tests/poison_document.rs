//! `story:poison-document-skipped`: a document whose extraction fails the same way on consecutive
//! runs stops blocking the source. A batch of several documents that the model answers it failed
//! on marks them and fails as before; a later run asks that batch whole once more, and when it
//! fails again each of its documents alone, those that failed alone on fewer runs first; a
//! document is counted only when it fails alone. Its count lives in `failures/<source>.json`
//! beside `state/`, so a run that is undone does not reset it; from the
//! [`DOCUMENT_FAILURE_LIMIT`]th consecutive failure on, the document is left out, named in
//! `skipped` with its count and reason, and the batches after it run. A
//! changed text is asked once more alone and keeps the count.

mod common;

use std::path::{Path, PathBuf};

use common::{executable, World};
use cortex_cli::sources::DOCUMENT_FAILURE_LIMIT;
use serde_json::{json, Value};

/// The stand-in `claude` wrapped: a prompt holding `POISON` is answered with an error that cost
/// something (the model was asked and failed on it); with the file `free` present, every prompt is
/// answered with an error that cost nothing (a sign-in or a usage limit, before any model was
/// asked); with the file `unstructured` present, a prompt holding `POISON` is answered with no
/// structured output. Anything else is answered as `tests/common`'s stand-in does. Every call is appended to
/// `wrapped-calls.log`, and its prompt to `prompts.log`.
const WRAPPER: &str = r#"R="@ROOT@"
cat > "$R/req.txt"
echo call >> "$R/wrapped-calls.log"
cat "$R/req.txt" >> "$R/prompts.log"; echo '=== END' >> "$R/prompts.log"
if [ -e "$R/free" ]; then
  echo '{"type":"result","subtype":"success","is_error":true,"result":"Not logged in.","total_cost_usd":0}'
  exit 1
fi
if grep -q POISON "$R/req.txt"; then
  if [ -e "$R/unstructured" ]; then
    echo '{"type":"result","subtype":"success","is_error":false,"result":"","total_cost_usd":0.01}'
    exit 0
  fi
  echo '{"type":"result","subtype":"success","is_error":true,"result":"The model could not answer this batch.","total_cost_usd":0.01}'
  exit 1
fi
exec "@INNER@" "$@" < "$R/req.txt"
"#;

/// The key of document `i`.
fn key(i: usize) -> String {
    format!("https://example.org/d{i}")
}

/// Makes the connection answer one result per entry of `texts`, document `i` at [`key`] `i`.
fn serve(w: &World, texts: &[&str]) {
    let results: Vec<Value> = texts
        .iter()
        .enumerate()
        .map(|(i, t)| {
            json!({"url": key(i), "title": format!("D{i}"), "description": "A page.",
                "content": t, "content_truncated": false, "published": null, "score": "0.5"})
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
}

/// A world whose wrapped `claude` fails on `POISON`, and whose instance `p` is created from the
/// common spec with `extra` appended.
fn world(texts: &[&str], extra: &str) -> World {
    world_edited(texts, |s| format!("{s}{extra}"))
}

/// A world whose wrapped `claude` fails on `POISON`, and whose instance `p` is created from the
/// common spec after `edit`.
fn world_edited(texts: &[&str], edit: impl Fn(String) -> String) -> World {
    let w = World::new();
    executable(
        &w.bin.join("claude-wrapped"),
        &WRAPPER
            .replace("@ROOT@", &w.root.display().to_string())
            .replace("@INNER@", &w.bin.join("claude").display().to_string()),
    );
    serve(&w, texts);
    let spec = w.spec("p", "conn_test");
    let text = std::fs::read_to_string(&spec).unwrap();
    std::fs::write(&spec, edit(text)).unwrap();
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    w
}

/// `cortex run p/news` through the wrapped `claude`: its exit status and its `run` line.
fn run(w: &World) -> (i32, Value) {
    let wrapped = w.bin.join("claude-wrapped");
    w.cortex_env(
        &["--claude", wrapped.to_str().unwrap(), "run", "p/news"],
        &[("CORTEX_MODEL_RETRY_MS", Path::new("10"))],
    )
}

fn instance(w: &World) -> PathBuf {
    w.home.join("instances/p")
}

/// The source's seen documents, by key.
fn seen(w: &World) -> Value {
    let path = instance(w).join("state/news.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn seen_keys(w: &World) -> Vec<String> {
    seen(w)["documents"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect()
}

/// The source's failure counts: `failures/news.json`, `Null` when absent.
fn failures(w: &World) -> Value {
    std::fs::read(instance(w).join("failures/news.json"))
        .map(|b| serde_json::from_slice(&b).unwrap())
        .unwrap_or(Value::Null)
}

fn runs_of(w: &World, i: usize) -> Value {
    failures(w)["documents"][key(i)]["runs"].clone()
}

const CLEAN: &str = "Example Labs develops the Widget engine.";
const POISONED: &str = "Example Labs develops the Widget engine. POISON";

#[test]
fn a_document_failing_alone_on_consecutive_runs_is_skipped_and_later_batches_run() {
    assert_eq!(DOCUMENT_FAILURE_LIMIT, 3);
    let w = world(&[CLEAN, POISONED, CLEAN, CLEAN], "");

    // Run 1: the batch of four fails for the first time. It is not split: the run fails as
    // before, after one call, and its documents are marked with no count.
    let (code, ran) = run(&w);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("extraction-failed")),
        "{ran}"
    );
    assert_eq!(w.lines("wrapped-calls.log").len(), 1);
    for i in 0..4 {
        assert_eq!(runs_of(&w, i), 0, "{}", failures(&w));
    }

    // Run 2: the batch failed once, so it is asked whole once more (the pass-2 rule: one failed
    // answer costs no call per document). It fails again, so each document is asked alone in
    // this run: d0 is applied, and the run stops at d1, its first failure alone, counted.
    let (code, ran) = run(&w);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    let stopped = ran["detail"]["stopped"].as_str().unwrap_or_default();
    assert!(
        stopped.contains("The model could not answer this batch.") && stopped.contains(&key(1)),
        "{ran}"
    );
    assert_eq!(
        w.lines("wrapped-calls.log").len(),
        1 + 3,
        "[0-3], [0-3], [0], [1]"
    );
    assert_eq!(runs_of(&w, 1), 1, "{}", failures(&w));
    assert!(runs_of(&w, 0).is_null(), "applied: {}", failures(&w));
    assert_eq!(seen_keys(&w), [key(0)]);

    // Run 3: d2 and d3, in two failed batches and with no count, are asked alone before d1 and
    // applied; d1 fails alone again, counted, and the run stops at it.
    let (code, ran) = run(&w);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
    assert!(
        ran["detail"]["stopped"]
            .as_str()
            .is_some_and(|s| s.contains(&key(1))),
        "{ran}"
    );
    assert_eq!(w.lines("wrapped-calls.log").len(), 4 + 3, "[2], [3], [1]");
    assert_eq!(runs_of(&w, 1), 2, "{}", failures(&w));
    assert_eq!(seen_keys(&w), [key(0), key(2), key(3)]);

    // Run N+1: d1 fails alone a third time and is skipped; the run does not stop at it, every
    // other document is applied, and d1 is named in `skipped` with its count and the reason.
    let (code, ran) = run(&w);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 0, "{ran}");
    assert!(ran["detail"]["stopped"].is_null(), "{ran}");
    let skipped = ran["detail"]["skipped"].to_string();
    assert!(
        skipped.contains(&key(1))
            && skipped.contains("3 consecutive runs")
            && skipped.contains("The model could not answer this batch."),
        "{ran}"
    );
    assert_eq!(runs_of(&w, 1), 3, "{}", failures(&w));
    assert_eq!(seen_keys(&w), [key(0), key(2), key(3)]);

    // The run after: a new document d4 is applied, d1 is not sent to the model and is named in
    // `skipped` with its count and the model's reason.
    serve(
        &w,
        &[
            CLEAN,
            POISONED,
            CLEAN,
            CLEAN,
            "Example Labs ships the Gadget runtime.",
        ],
    );
    let calls = w.lines("wrapped-calls.log").len();
    let (code, ran) = run(&w);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    assert!(ran["detail"]["stopped"].is_null(), "{ran}");
    let skipped: Vec<String> = ran["detail"]["skipped"]
        .as_array()
        .unwrap_or_else(|| panic!("no skipped: {ran}"))
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect();
    assert_eq!(skipped.len(), 1, "{skipped:?}");
    assert!(
        skipped[0].contains(&key(1))
            && skipped[0].contains("3 consecutive runs")
            && skipped[0].contains("The model could not answer this batch."),
        "{skipped:?}"
    );
    assert_eq!(w.lines("wrapped-calls.log").len(), calls + 1, "d4 alone");
    assert!(!seen(&w)["documents"]
        .as_object()
        .unwrap()
        .contains_key(&key(1)));
    assert_eq!(seen_keys(&w), [key(0), key(2), key(3), key(4)]);

    // Its text changed and the model answers it: d1 is asked again, applied, and its count
    // removed.
    serve(
        &w,
        &[
            CLEAN,
            "Example Labs develops the Widget engine, again.",
            CLEAN,
            CLEAN,
        ],
    );
    let (code, ran) = run(&w);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    assert!(ran["detail"].get("skipped").is_none(), "{ran}");
    assert!(seen(&w)["documents"]
        .as_object()
        .unwrap()
        .contains_key(&key(1)));
    assert!(runs_of(&w, 1).is_null(), "{}", failures(&w));
}

/// A changed text is asked again, alone, and keeps the count: two failures, a change that fails
/// too, and the document is left out at the limit; the same text is then not asked, a further
/// change is asked once more, and a text the model answers is applied and drops the count.
#[test]
fn a_changed_text_is_asked_once_more_alone_and_keeps_its_count() {
    let w = world(&[POISONED], "");
    for _ in 0..2 {
        run(&w);
    }
    assert_eq!(runs_of(&w, 0), 2, "{}", failures(&w));
    serve(
        &w,
        &["Example Labs develops the Widget engine. POISON, changed"],
    );
    let (code, ran) = run(&w);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert!(
        ran["detail"]["skipped"]
            .to_string()
            .contains("3 consecutive runs"),
        "{ran}"
    );
    assert_eq!(runs_of(&w, 0), 3, "{}", failures(&w));
    assert_eq!(w.lines("wrapped-calls.log").len(), 3);

    run(&w);
    assert_eq!(
        w.lines("wrapped-calls.log").len(),
        3,
        "the same text is not asked"
    );

    serve(
        &w,
        &["Example Labs develops the Widget engine. POISON, changed again"],
    );
    let (code, ran) = run(&w);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(w.lines("wrapped-calls.log").len(), 4, "asked once more");
    assert_eq!(runs_of(&w, 0), 4, "{}", failures(&w));

    serve(&w, &[CLEAN]);
    let (code, ran) = run(&w);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    assert!(runs_of(&w, 0).is_null(), "{}", failures(&w));
}

/// A run its gate refuses puts `state/` back; the failure count is outside it and stays.
#[test]
fn a_run_its_gate_refuses_keeps_the_failure_count() {
    let w = world(
        &[CLEAN, POISONED],
        "gate:\n  checks:\n    - {measure: documents_applied, max: \"0\"}\n",
    );
    // The first failure of the batch marks both documents and applies nothing.
    let (code, ran) = run(&w);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("extraction-failed")),
        "{ran}"
    );
    // Each later run splits it, applies d0, counts d1, and is refused by its gate and undone.
    for n in 1..=DOCUMENT_FAILURE_LIMIT {
        let (code, ran) = run(&w);
        assert_eq!(
            (code, ran["outcome"].as_str()),
            (1, Some("apply-refused")),
            "{ran}"
        );
        assert_eq!(runs_of(&w, 1), n, "{}", failures(&w));
        assert!(
            !std::fs::read_to_string(instance(&w).join("state/news.json"))
                .unwrap_or_default()
                .contains("example.org/d1")
        );
    }
}

/// An answer with no structured output is the model failing on the request, as an error answer
/// is: the batch asked whole once more fails again, each document is then asked alone, the one
/// it fails on is counted, and the others are applied.
#[test]
fn an_answer_with_no_structured_output_marks_and_counts_as_an_error_answer_does() {
    let w = world(&[CLEAN, POISONED], "");
    std::fs::write(w.root.join("unstructured"), "").unwrap();
    let (code, ran) = run(&w);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("extraction-failed")),
        "{ran}"
    );
    assert!(
        ran["detail"]["reason"]
            .as_str()
            .is_some_and(|r| r.contains("no structured output")),
        "{ran}"
    );
    assert_eq!((runs_of(&w, 0), runs_of(&w, 1)), (json!(0), json!(0)));
    let (code, ran) = run(&w);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(seen_keys(&w), [key(0)]);
    assert_eq!(runs_of(&w, 1), 1, "{}", failures(&w));
    assert_eq!(
        w.lines("wrapped-calls.log").len(),
        1 + 3,
        "[0-1], [0-1], [0], [1]"
    );
}

/// An error answer that cost nothing came before any model was asked: the batch is not split and
/// no document is counted.
#[test]
fn an_error_answer_that_cost_nothing_is_not_split_and_counts_no_document() {
    let w = world(&[CLEAN, CLEAN, CLEAN, CLEAN], "");
    std::fs::write(w.root.join("free"), "").unwrap();
    let (code, ran) = run(&w);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("extraction-failed")),
        "{ran}"
    );
    assert_eq!(
        w.lines("wrapped-calls.log").len(),
        2,
        "asked and retried once"
    );
    assert!(failures(&w).is_null(), "{}", failures(&w));
}

/// Adversary: the stand-in `claude` wrapped as [`WRAPPER`] is, with two more modes. While the
/// file `outage` exists, every call outlives `model.timeout_s` and is ended by `timeout`, as a
/// model that is slow for a few hours is. With the file `burns-budget`, a prompt holding `POISON`
/// is answered with an error that spent the run's whole `budget_usd` (1).
const ADVERSARY_WRAPPER: &str = r#"R="@ROOT@"
cat > "$R/req.txt"
echo call >> "$R/wrapped-calls.log"
if [ -e "$R/outage" ]; then exec sleep 5; fi
if grep -q POISON "$R/req.txt"; then
  if [ -e "$R/burns-budget" ]; then
    echo '{"type":"result","subtype":"error_max_budget_usd","is_error":true,"result":"","total_cost_usd":1}'
  else
    echo '{"type":"result","subtype":"success","is_error":true,"result":"The model could not answer this batch.","total_cost_usd":0.01}'
  fi
  exit 1
fi
exec "@INNER@" "$@" < "$R/req.txt"
"#;

/// A world as [`world_edited`] makes it, whose wrapped `claude` is [`ADVERSARY_WRAPPER`].
fn adversary_world(texts: &[&str], edit: impl Fn(String) -> String) -> World {
    let w = World::new();
    executable(
        &w.bin.join("claude-wrapped"),
        &ADVERSARY_WRAPPER
            .replace("@ROOT@", &w.root.display().to_string())
            .replace("@INNER@", &w.bin.join("claude").display().to_string()),
    );
    serve(&w, texts);
    let spec = w.spec("p", "conn_test");
    let text = std::fs::read_to_string(&spec).unwrap();
    std::fs::write(&spec, edit(text)).unwrap();
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    w
}

/// The source's seen keys, none when no run wrote its state.
fn seen_keys_if_any(w: &World) -> Vec<String> {
    match std::fs::read(instance(w).join("state/news.json")) {
        Ok(bytes) => serde_json::from_slice::<Value>(&bytes).unwrap()["documents"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Adversary: a model that times out for a few hours (here four runs in a row) is not a fault
/// of any document. Every call is ended by `timeout`, which leaves no JSON answer: an uncosted
/// failure, so not a free error answer, so marked, split and counted. On the fourth run the
/// first document of the batch reaches the limit and is skipped until its text changes, though
/// nothing is wrong with it; once the model answers again it is never asked.
#[test]
fn adversary_a_model_timing_out_on_consecutive_runs_skips_no_healthy_document() {
    let w = adversary_world(
        &[
            CLEAN,
            "Example Labs ships the Gadget runtime.",
            "Example Labs ships the Sprocket toolkit.",
            "Example Labs ships the Gizmo service.",
        ],
        |s| {
            assert!(s.contains("timeout_s: 60}"), "{s}");
            s.replacen("timeout_s: 60}", "timeout_s: 1}", 1)
        },
    );
    std::fs::write(w.root.join("outage"), "").unwrap();
    for _ in 0..=DOCUMENT_FAILURE_LIMIT {
        let (code, ran) = run(&w);
        assert_eq!(code, 1, "every run of the outage fails: {ran}");
    }
    std::fs::remove_file(w.root.join("outage")).unwrap();
    let (code, ran) = run(&w);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(
        seen_keys_if_any(&w),
        [key(0), key(1), key(2), key(3)],
        "a document was left out after the outage: skipped {}; failures {}",
        ran["detail"]["skipped"],
        failures(&w)
    );
}

/// Adversary: a document the model fails on every time, whose text changes on every run (a page
/// with a view counter, a thread that gains a reply). Each changed text starts its count again
/// and is not marked, so it never reaches the limit; and every split of its batch puts it first
/// and stops the run at it before the documents after it are asked. The source stays blocked
/// for good, as before the change.
#[test]
fn adversary_a_poison_document_whose_text_changes_every_run_blocks_no_other_document() {
    let w = world(&[POISONED, CLEAN, CLEAN], "");
    for n in 0..6 {
        let poisoned = format!("{POISONED} Read {n} times.");
        serve(&w, &[&poisoned, CLEAN, CLEAN]);
        run(&w);
    }
    assert_eq!(
        seen_keys_if_any(&w),
        [key(1), key(2)],
        "after 6 runs; failures {}",
        failures(&w)
    );
}

/// Adversary: a batch on which the model spends the run's whole `budget_usd` before it fails
/// (`error_max_budget_usd`, the call bounded by `--max-budget-usd`). Each later run splits it and
/// stops at the budget before either half is asked, so the poison document never fails alone,
/// is never counted, and the healthy document beside it is never applied.
#[test]
fn adversary_a_failing_batch_that_spends_the_run_budget_does_not_block_the_source_for_good() {
    let w = adversary_world(&[CLEAN, POISONED], |s| s);
    std::fs::write(w.root.join("burns-budget"), "").unwrap();
    for _ in 0..6 {
        run(&w);
    }
    assert_eq!(
        seen_keys_if_any(&w),
        [key(0)],
        "after 6 runs, {} calls; failures {}",
        w.lines("wrapped-calls.log").len(),
        failures(&w)
    );
}
