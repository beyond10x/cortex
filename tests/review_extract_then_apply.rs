//! `cortex run <instance>/<source> --extract-only` keeps a run's extraction unapplied and writes
//! nothing to the store or the seen state; `cortex apply <instance> <run>` applies it once, and
//! refuses it once the store moved past it (`story:extract-then-apply`). The model is the
//! stand-in `claude` of `tests/common`.

mod common;

use std::path::PathBuf;
use std::process::Command;

use common::{ekr, World};
use serde_json::Value;

const NAME: &str = "keep";

fn world() -> World {
    let w = World::new();
    let spec = w.spec(NAME, "conn_test");
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    w
}

fn dir(w: &World) -> PathBuf {
    w.home.join("instances").join(NAME)
}

/// How many assertions the store holds at its head.
fn assertions(w: &World) -> usize {
    let dir = dir(w);
    let out = Command::new(ekr())
        .arg("snapshot")
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
    let graph: Value = serde_json::from_slice(&out.stdout).unwrap();
    graph["graph"]["graph"]["assertions"]
        .as_object()
        .map_or(0, |a| a.len())
}

/// The source's seen state file as it is on disk, `None` when there is none.
fn state(w: &World) -> Option<String> {
    std::fs::read_to_string(dir(w).join("state/news.json")).ok()
}

/// The keys the source's seen state holds.
fn seen_keys(w: &World) -> Vec<String> {
    state(w)
        .map(|s| serde_json::from_str::<Value>(&s).unwrap())
        .and_then(|s| {
            s["documents"]
                .as_object()
                .map(|d| d.keys().cloned().collect())
        })
        .unwrap_or_default()
}

/// `run --extract-only`, which must answer `extracted`; its run directory's name.
fn extract_only(w: &World) -> (Value, String) {
    let (code, line) = w.cortex(&["run", &format!("{NAME}/news"), "--extract-only"]);
    assert_eq!(
        (code, line["outcome"].as_str()),
        (0, Some("extracted")),
        "{line}"
    );
    let run = line["detail"]["run"]
        .as_str()
        .expect("a kept run")
        .to_string();
    (line, run)
}

fn apply(w: &World, run: &str) -> (i32, Value) {
    w.cortex(&["apply", NAME, run])
}

#[test]
fn extract_only_keeps_the_extraction_unapplied_and_apply_adds_the_facts_once() {
    let w = world();
    let before = assertions(&w);
    let head = w.head(NAME);
    let seen = state(&w);
    let (line, run) = extract_only(&w);
    let d = &line["detail"];
    assert_eq!(
        (&d["documents_new"], &d["documents_kept"], &d["batches"]),
        (&Value::from(2), &Value::from(2), &Value::from(1)),
        "{line}"
    );
    assert_eq!(d["model_calls"], 1, "{line}");
    // Nothing was written to the store or the seen state; the extraction is kept.
    assert_eq!(assertions(&w), before, "the store's assertion count moved");
    assert_eq!(w.head(NAME), head);
    assert_eq!(state(&w), seen, "the seen state was written");
    let kept = dir(&w).join("runs").join(&run);
    assert!(kept.join("kept.json").is_file());
    assert!(kept.join("batch-0/extraction.yaml").is_file());
    // No status, metric or log line for an extract-only run.
    assert!(
        !dir(&w).join("cortex.log").exists()
            || !std::fs::read_to_string(dir(&w).join("cortex.log"))
                .unwrap()
                .contains(&run)
    );

    let (code, applied) = apply(&w, &run);
    assert_eq!(
        (code, applied["outcome"].as_str()),
        (0, Some("applied")),
        "{applied}"
    );
    assert_eq!(applied["detail"]["documents_applied"], 2, "{applied}");
    assert_eq!(applied["detail"]["documents_left"], 0, "{applied}");
    let after = assertions(&w);
    assert!(
        after > before,
        "apply added no assertion: {before} then {after}"
    );
    assert_eq!(
        seen_keys(&w),
        ["https://example.org/a", "https://example.org/b"]
    );
    // The model was asked once, by the extract-only run; the apply asks none.
    assert_eq!(w.lines("claude-calls.log").len(), 1);

    // A second apply of the same run is refused and writes nothing.
    let head = w.head(NAME);
    let (code, again) = apply(&w, &run);
    assert_eq!(
        (code, again["outcome"].as_str()),
        (1, Some("already-applied")),
        "{again}"
    );
    assert_eq!(again["detail"]["run"], run.as_str());
    assert_eq!(w.head(NAME), head);
    assert_eq!(assertions(&w), after);

    // The source's next run finds nothing new: the apply advanced the seen state.
    let (code, ran) = w.cortex(&["run", &format!("{NAME}/news")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_new"], 0, "{ran}");
}

#[test]
fn a_run_between_extract_only_and_apply_sees_the_documents_as_new_and_the_apply_is_stale() {
    let w = world();
    let (_, run) = extract_only(&w);
    let (code, ran) = w.cortex(&["run", &format!("{NAME}/news")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_new"], 2, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
    let head = w.head(NAME);
    let count = assertions(&w);
    let (code, stale) = apply(&w, &run);
    assert_eq!(
        (code, stale["outcome"].as_str()),
        (1, Some("extraction-stale")),
        "{stale}"
    );
    let reason = stale["detail"]["reason"].as_str().unwrap();
    assert!(reason.contains("https://example.org/a"), "{reason}");
    assert_eq!(w.head(NAME), head, "a stale apply wrote to the store");
    assert_eq!(assertions(&w), count);
    // A stale extraction is not consumed: it stays as it was, and is refused again.
    assert!(!dir(&w)
        .join("runs")
        .join(&run)
        .join("applied.json")
        .exists());
    let (code, again) = apply(&w, &run);
    assert_eq!(
        (code, again["outcome"].as_str()),
        (1, Some("extraction-stale"))
    );
}

#[test]
fn apply_refuses_a_run_that_kept_no_extraction() {
    let w = world();
    for run in ["1700000000000-news", "../state", "nothing"] {
        let (code, line) = apply(&w, run);
        assert_eq!(
            (code, line["outcome"].as_str()),
            (1, Some("no-such-run")),
            "{run}: {line}"
        );
    }
    let (code, line) = w.cortex(&["apply", "absent", "1700000000000-news"]);
    assert_eq!(
        (code, line["outcome"].as_str()),
        (1, Some("no-such-instance")),
        "{line}"
    );
    // A normal run's directory holds no kept extraction.
    let (code, ran) = w.cortex(&["run", &format!("{NAME}/news")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let runs: Vec<String> = std::fs::read_dir(dir(&w).join("runs"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.ends_with("-news"))
        .collect();
    assert_eq!(runs.len(), 1);
    let (code, line) = apply(&w, &runs[0]);
    assert_eq!(
        (code, line["outcome"].as_str()),
        (1, Some("no-such-run")),
        "{line}"
    );
}

#[test]
fn extract_only_of_the_seed_or_with_fetch_only_is_refused() {
    let w = world();
    let (code, line) = w.cortex(&["run", &format!("{NAME}/seed"), "--extract-only"]);
    assert_eq!(
        (code, line["outcome"].as_str()),
        (1, Some("extract-only-failed")),
        "{line}"
    );
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args([
            "run",
            &format!("{NAME}/news"),
            "--extract-only",
            "--fetch-only",
        ])
        .env("CORTEX_HOME", &w.home)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(2),
        "clap refuses the two flags together"
    );
    // A failed fetch keeps nothing and records nothing on the source.
    let seen = state(&w);
    std::fs::write(w.root.join("fail"), "").unwrap();
    let (code, line) = w.cortex(&["run", &format!("{NAME}/news"), "--extract-only"]);
    assert_eq!(
        (
            code,
            line["outcome"].as_str(),
            line["detail"]["failure"].as_str()
        ),
        (1, Some("extract-only-failed"), Some("fetch-failed")),
        "{line}"
    );
    assert_eq!(
        state(&w),
        seen,
        "the failed extract-only run wrote the seen state"
    );
}
