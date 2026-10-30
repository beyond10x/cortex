//! Adversary pass 2 on unit fail of wave 20261030a (`story:poison-document-skipped`, correction
//! 88415357). The correction stopped asking a batch that holds a marked document whole: its
//! marked documents are each asked alone, on every later run, until each applies. So one answered
//! failure of a batch of healthy documents (an error answer that cost something, as a model that
//! erred once gives) makes the next run ask every document of that batch alone: one call per
//! document instead of one for the batch. Release 0.6.1, and the unit before the correction, asked
//! that batch whole on the next run and split it only if it failed again ("A transient failure
//! costs no split").

mod common;

use std::path::Path;

use common::{executable, World};
use serde_json::{json, Value};

/// The stand-in `claude` wrapped: while the file `flaky` exists, the first call removes it and is
/// answered with an error that cost something (the model was asked and failed once); every other
/// call is answered as `tests/common`'s stand-in does. Every call is appended to
/// `wrapped-calls.log`.
const WRAPPER: &str = r#"R="@ROOT@"
cat > "$R/req.txt"
echo call >> "$R/wrapped-calls.log"
if [ -e "$R/flaky" ]; then
  rm -f "$R/flaky"
  echo '{"type":"result","subtype":"success","is_error":true,"result":"The model erred once.","total_cost_usd":0.01}'
  exit 1
fi
exec "@INNER@" "$@" < "$R/req.txt"
"#;

fn serve(w: &World, n: usize) {
    let results: Vec<Value> = (0..n)
        .map(|i| {
            json!({"url": format!("https://example.org/d{i}"), "title": format!("D{i}"),
                "description": "A page.", "content": "Example Labs develops the Widget engine.",
                "content_truncated": false, "published": null, "score": "0.5"})
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

fn run(w: &World) -> (i32, Value) {
    let wrapped = w.bin.join("claude-wrapped");
    w.cortex_env(
        &["--claude", wrapped.to_str().unwrap(), "run", "p/news"],
        &[("CORTEX_MODEL_RETRY_MS", Path::new("10"))],
    )
}

/// Six healthy documents in one batch. Run 1: the model errs once on the batch, so the run fails
/// and marks the six. Run 2: the model answers every call. Run 2 applies the six with one call,
/// as release 0.6.1 did, not one call per document.
#[test]
fn one_failed_answer_on_a_batch_does_not_make_the_next_run_ask_each_document_alone() {
    let w = World::new();
    executable(
        &w.bin.join("claude-wrapped"),
        &WRAPPER
            .replace("@ROOT@", &w.root.display().to_string())
            .replace("@INNER@", &w.bin.join("claude").display().to_string()),
    );
    serve(&w, 6);
    let spec = w.spec("p", "conn_test");
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");

    std::fs::write(w.root.join("flaky"), "").unwrap();
    let (_, first) = run(&w);
    assert_eq!(w.lines("wrapped-calls.log").len(), 1, "{first}");
    assert!(!w.root.join("flaky").exists());

    let (code, second) = run(&w);
    assert_eq!(code, 0, "{second}");
    assert_eq!(second["detail"]["documents_applied"], 6, "{second}");
    let calls = w.lines("wrapped-calls.log").len() - 1;
    assert_eq!(
        calls, 1,
        "run 2 asked the model {calls} times for one batch of six healthy documents: {second}"
    );
}
