//! A model error answer keeps its reason and is retried once (`story:model-error-text-and-retry`):
//! claude's error text, masked for credential shapes and cut to a bound, is the run's `reason` and
//! `batch-0/claude/error.txt`; an error answer that cost zero is asked once more after a wait
//! (`CORTEX_MODEL_RETRY_MS` here, so the test does not wait the default); one with a cost is not.

mod common;

use std::path::Path;

use common::{executable, World};

/// The stand-in `claude` wrapped: while the file `errors` holds a number above zero it counts it
/// down and answers `error.json` (exit 1, as claude does); otherwise it answers as
/// `tests/common`'s stand-in does. Every call is appended to `wrapped-calls.log`.
const WRAPPER: &str = r#"R="@ROOT@"
echo call >> "$R/wrapped-calls.log"
n=$(cat "$R/errors" 2>/dev/null || echo 0)
if [ "$n" -gt 0 ]; then
  echo $((n - 1)) > "$R/errors"
  cat > /dev/null
  cat "$R/error.json"
  exit 1
fi
exec "@INNER@" "$@"
"#;

/// A world whose wrapped `claude` answers `errors` error answers of `cost` (none when `None`)
/// carrying `text`, then valid documents; its instance `e` is created.
fn world(errors: u32, cost: Option<f64>, text: &str) -> World {
    let w = World::new();
    let wrapped = w.bin.join("claude-wrapped");
    executable(
        &wrapped,
        &WRAPPER
            .replace("@ROOT@", &w.root.display().to_string())
            .replace("@INNER@", &w.bin.join("claude").display().to_string()),
    );
    let mut answer = serde_json::json!({"type": "result", "subtype": "success", "is_error": true,
        "result": text});
    if let Some(c) = cost {
        answer["total_cost_usd"] = c.into();
    }
    std::fs::write(w.root.join("error.json"), answer.to_string()).unwrap();
    std::fs::write(w.root.join("errors"), errors.to_string()).unwrap();
    let spec = w.spec("e", "conn_test");
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    w
}

fn run(w: &World, record_failure: bool) -> (i32, serde_json::Value) {
    let wrapped = w.bin.join("claude-wrapped");
    let mut args = vec!["--claude", wrapped.to_str().unwrap(), "run", "e/news"];
    if record_failure {
        args.push("--record-failure");
    }
    w.cortex_env(&args, &[("CORTEX_MODEL_RETRY_MS", Path::new("10"))])
}

/// The only run directory of instance `e`.
fn run_dir(w: &World) -> std::path::PathBuf {
    let runs = w.home.join("instances/e/runs");
    let mut dirs: Vec<_> = std::fs::read_dir(&runs)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(dirs.len(), 1, "{dirs:?}");
    dirs.remove(0)
}

/// `consecutive_failures` of `e/news`, from `cortex sources`.
fn consecutive_failures(w: &World) -> serde_json::Value {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_cortex"))
        .arg("sources")
        .env("CORTEX_HOME", &w.home)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
        .find(|row| row["source_id"] == "e/news")
        .expect("a row for e/news")["consecutive_failures"]
        .clone()
}

#[test]
fn a_zero_cost_error_answer_keeps_its_text_masked_in_the_reason_and_error_txt() {
    // Assembled at runtime so no credential-shaped literal sits in the source.
    let token = format!("sk-{}", "Qw3rTy".repeat(5));
    let text = format!("Not logged in. Please run /login (key {token}).");
    let w = world(2, Some(0.0), &text);
    let (code, ran) = run(&w, false);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("extraction-failed")),
        "{ran}"
    );
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("Not logged in. Please run /login (key [masked:api-key])."),
        "{reason}"
    );
    assert!(!reason.contains(&token), "{reason}");
    let error = std::fs::read_to_string(run_dir(&w).join("batch-0/claude/error.txt")).unwrap();
    assert!(
        error.contains("Not logged in. Please run /login (key [masked:api-key])."),
        "{error}"
    );
    assert!(!error.contains(&token), "{error}");
    assert_eq!(
        w.lines("wrapped-calls.log").len(),
        2,
        "asked once and retried once, no more"
    );
}

#[test]
fn a_long_error_text_is_cut_at_a_character_boundary() {
    let text = "é".repeat(5000);
    let w = world(2, Some(0.0), &text);
    let (code, ran) = run(&w, false);
    assert_eq!(code, 1, "{ran}");
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    assert!(reason.len() < 2000, "bounded: {} bytes", reason.len());
    assert!(reason.contains("éé"), "{reason}");
    let error = std::fs::read_to_string(run_dir(&w).join("batch-0/claude/error.txt")).unwrap();
    assert!(error.len() < 2000, "bounded: {} bytes", error.len());
}

#[test]
fn a_zero_cost_error_answered_once_is_retried_and_counts_no_failure() {
    let w = world(1, Some(0.0), "You've hit your usage limit.");
    let (code, ran) = run(&w, true);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
    assert_eq!(w.lines("wrapped-calls.log").len(), 2, "retried once");
    assert_eq!(consecutive_failures(&w), 0);
}

#[test]
fn an_error_answer_with_a_cost_is_not_retried() {
    let w = world(1, Some(0.02), "The model stopped.");
    let (code, ran) = run(&w, false);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("extraction-failed")),
        "{ran}"
    );
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("The model stopped."), "{reason}");
    assert_eq!(w.lines("wrapped-calls.log").len(), 1, "no retry");
}

#[test]
fn an_error_answer_with_no_cost_reported_is_not_retried() {
    let w = world(1, None, "Something failed.");
    let (code, ran) = run(&w, false);
    assert_eq!(code, 1, "{ran}");
    assert_eq!(w.lines("wrapped-calls.log").len(), 1, "no retry");
}
