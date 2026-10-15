//! A failed or skipped run is visible without reading `cortex.log`: `cortex sources` shows each
//! source's last outcome, its time and its reason; with `monitoring.metrics` every run rewrites
//! `metrics.prom` in the instance directory; and `monitoring.hook` is called once for each run
//! that fails, with `disabled` when the run's recorded failure disabled the source. A hook that
//! fails or times out is logged and changes neither the run's outcome nor its exit status.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use common::{executable, World};
use cortex_cli::instance::now_ms;
use cortex_cli::sources::rfc3339;
use serde_json::Value;

/// The world's spec for instance `t`, with `monitoring` appended as written.
fn spec_with(w: &World, monitoring: &str) -> PathBuf {
    let path = w.spec("t", "conn_test");
    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str(monitoring);
    std::fs::write(&path, text).unwrap();
    path
}

/// A hook that appends its arguments, one call per line, to `hook.log` and exits with `status`.
fn hook(w: &World, status: i32) -> PathBuf {
    let path = w.bin.join("hook");
    executable(
        &path,
        &format!(
            "echo \"$*\" >> \"{}/hook.log\"\nexit {status}\n",
            w.root.display()
        ),
    );
    path
}

/// `monitoring` with the metric file and `hook` called with `timeout_s`.
fn monitoring(hook: &Path, timeout_s: u32) -> String {
    format!(
        "monitoring:\n  metrics: true\n  hook: {{command: [\"{}\", \"--from\", \"cortex\"], timeout_s: {timeout_s}}}\n",
        hook.display()
    )
}

/// Creates `t` from `spec` with no units and no seed extraction.
fn create(w: &World, spec: &Path) {
    let (code, out) = w.cortex(&[
        "create",
        "--spec",
        spec.to_str().unwrap(),
        "--no-extract",
        "--no-units",
    ]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("created")),
        "{out}"
    );
}

/// `cortex run t/news --record-failure`: its exit status and its first line (the run's).
fn run(w: &World) -> (i32, Value, Duration) {
    let started = Instant::now();
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["run", "t/news", "--record-failure"])
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let took = started.elapsed();
    let first = String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .and_then(|l| serde_json::from_str(l).ok())
        .unwrap_or(Value::Null);
    (out.status.code().unwrap_or(-1), first, took)
}

/// The `sources` row of `t/news`.
fn news(w: &World) -> Value {
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .arg("sources")
        .env("CORTEX_HOME", &w.home)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .find(|row| row["source_id"] == "t/news")
        .expect("a row for t/news")
}

fn instance_dir(w: &World) -> PathBuf {
    w.home.join("instances").join("t")
}

fn metrics(w: &World) -> String {
    std::fs::read_to_string(instance_dir(w).join("metrics.prom")).unwrap_or_default()
}

/// The value of the sample `name{labels}` in the metric file, `None` when it has none.
fn sample(text: &str, series: &str) -> Option<f64> {
    text.lines().find_map(|l| {
        l.strip_prefix(series)?
            .strip_prefix(' ')?
            .trim()
            .parse()
            .ok()
    })
}

fn runs_of(text: &str, outcome: &str) -> f64 {
    sample(
        text,
        &format!("cortex_source_runs_total{{source=\"t/news\",outcome=\"{outcome}\"}}"),
    )
    .unwrap_or(0.0)
}

/// The `cortex.log` lines of `t` that record a hook call.
fn hook_lines(w: &World) -> Vec<Value> {
    std::fs::read_to_string(instance_dir(w).join("cortex.log"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|l| l.get("hook").is_some())
        .collect()
}

#[test]
fn after_a_fetch_failed_run_sources_shows_its_last_outcome_its_time_and_its_reason() {
    let w = World::new();
    create(&w, &w.spec("t", "conn_test"));
    let before = news(&w);
    assert!(before["last_outcome"].is_null(), "{before}");
    assert!(before["last_run_at"].is_null(), "{before}");

    std::fs::write(w.root.join("fail"), "").unwrap();
    let from = rfc3339(now_ms());
    let (code, out, _) = run(&w);
    let to = rfc3339(now_ms());
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("fetch-failed")),
        "{out}"
    );

    let row = news(&w);
    assert_eq!(row["last_outcome"], "fetch-failed", "{row}");
    let at = row["last_run_at"].as_str().expect("a time");
    assert!(
        from.as_str() <= at && at <= to.as_str(),
        "{at} not in {from}..{to}"
    );
    assert_eq!(row["last_reason"], out["detail"]["reason"], "{row}");
    assert!(
        row["last_reason"]
            .as_str()
            .is_some_and(|r| r.contains("unavailable")),
        "{row}"
    );

    // A run that succeeds next clears the reason.
    std::fs::remove_file(w.root.join("fail")).unwrap();
    let (code, out, _) = run(&w);
    assert_eq!((code, out["outcome"].as_str()), (0, Some("ran")), "{out}");
    let row = news(&w);
    assert_eq!(row["last_outcome"], "ran", "{row}");
    assert!(row["last_reason"].is_null(), "{row}");
}

#[test]
fn after_a_fetch_failed_run_the_metric_file_counts_one_more_failed_run_for_the_source() {
    let w = World::new();
    create(&w, &spec_with(&w, "monitoring: {metrics: true}\n"));
    assert_eq!(metrics(&w), "", "no run, no metric file");

    let (code, _, _) = run(&w);
    assert_eq!(code, 0);
    let first = metrics(&w);
    assert_eq!(runs_of(&first, "ran"), 1.0, "{first}");
    assert_eq!(runs_of(&first, "fetch-failed"), 0.0, "{first}");

    std::fs::write(w.root.join("fail"), "").unwrap();
    let started = now_ms() / 1000;
    let (code, _, _) = run(&w);
    assert_eq!(code, 1);
    let second = metrics(&w);
    assert_eq!(runs_of(&second, "fetch-failed"), 1.0, "{second}");
    assert_eq!(runs_of(&second, "ran"), 1.0, "{second}");
    let last = sample(
        &second,
        "cortex_source_last_run_timestamp_seconds{source=\"t/news\"}",
    )
    .expect("the last run's timestamp");
    assert!(last >= started as f64, "{second}");
    assert!(
        second.contains("# TYPE cortex_source_runs_total counter"),
        "{second}"
    );

    let (code, _, _) = run(&w);
    assert_eq!(code, 1);
    assert_eq!(
        runs_of(&metrics(&w), "fetch-failed"),
        2.0,
        "{}",
        metrics(&w)
    );

    // Rewritten whole by a rename: no temporary file is left beside it.
    let left: Vec<_> = std::fs::read_dir(instance_dir(&w))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains("metrics") && n != "metrics.prom")
        .collect();
    assert!(left.is_empty(), "{left:?}");
}

#[test]
fn when_the_second_failure_in_a_row_disables_the_source_the_hook_is_called_once_with_disabled() {
    let w = World::new();
    let hook = hook(&w, 0);
    create(&w, &spec_with(&w, &monitoring(&hook, 10)));
    std::fs::write(w.root.join("fail"), "").unwrap();

    let (code, out, _) = run(&w);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("fetch-failed")),
        "{out}"
    );
    assert_eq!(w.lines("hook.log"), ["--from cortex t/news fetch-failed"]);

    let (code, _, _) = run(&w);
    assert_eq!(code, 1);
    assert_eq!(news(&w)["state"], "Disabled");
    assert_eq!(
        w.lines("hook.log"),
        [
            "--from cortex t/news fetch-failed",
            "--from cortex t/news disabled"
        ],
        "one call per failed run, `disabled` for the run that disabled the source"
    );
    let logged = hook_lines(&w);
    assert_eq!(logged.len(), 2, "{logged:?}");
    assert_eq!(logged[1]["hook"]["result"], "called", "{logged:?}");
    assert_eq!(logged[1]["outcome"], "disabled", "{logged:?}");

    // A run of the disabled source is not a failure: no hook call.
    let (code, out, _) = run(&w);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (3, Some("disabled")),
        "{out}"
    );
    assert_eq!(w.lines("hook.log").len(), 2);
}

#[test]
fn a_run_of_a_disabled_source_records_disabled_as_its_last_outcome() {
    let w = World::new();
    create(&w, &spec_with(&w, "monitoring: {metrics: true}\n"));
    std::fs::write(w.root.join("fail"), "").unwrap();
    run(&w);
    run(&w);
    assert_eq!(news(&w)["last_outcome"], "fetch-failed");

    let (code, out, _) = run(&w);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (3, Some("disabled")),
        "{out}"
    );
    let row = news(&w);
    assert_eq!(row["last_outcome"], "disabled", "{row}");
    assert!(row["last_reason"].is_null(), "{row}");
    assert_eq!(row["consecutive_failures"], 2, "{row}");
    assert_eq!(runs_of(&metrics(&w), "disabled"), 1.0, "{}", metrics(&w));
}

#[test]
fn a_hook_that_fails_is_logged_and_changes_neither_the_outcome_nor_the_exit_status() {
    let w = World::new();
    let hook = hook(&w, 7);
    create(&w, &spec_with(&w, &monitoring(&hook, 10)));
    std::fs::write(w.root.join("fail"), "").unwrap();

    let (code, out, _) = run(&w);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("fetch-failed")),
        "{out}"
    );
    assert_eq!(w.lines("hook.log").len(), 1);
    let logged = hook_lines(&w);
    assert_eq!(logged.len(), 1, "{logged:?}");
    assert_eq!(logged[0]["source"], "t/news", "{logged:?}");
    assert_eq!(logged[0]["outcome"], "fetch-failed", "{logged:?}");
    assert_eq!(logged[0]["hook"]["result"], "failed", "{logged:?}");
    assert_eq!(logged[0]["hook"]["status"], 7, "{logged:?}");
    assert_eq!(news(&w)["consecutive_failures"], 1);

    // A hook that cannot start fails the same way.
    let (code, _, _) = {
        std::fs::remove_file(&hook).unwrap();
        run(&w)
    };
    assert_eq!(code, 1);
    let logged = hook_lines(&w);
    assert_eq!(logged.len(), 2, "{logged:?}");
    assert_eq!(logged[1]["hook"]["result"], "failed", "{logged:?}");
    assert_eq!(logged[1]["outcome"], "disabled", "{logged:?}");
    assert_eq!(news(&w)["state"], "Disabled");
}

#[test]
fn a_hook_that_outlives_its_timeout_is_killed_and_logged_and_the_run_ends() {
    let w = World::new();
    let path = w.bin.join("hook");
    executable(
        &path,
        &format!(
            "echo \"$*\" >> \"{}/hook.log\"\nexec sleep 30\n",
            w.root.display()
        ),
    );
    create(&w, &spec_with(&w, &monitoring(&path, 1)));
    std::fs::write(w.root.join("fail"), "").unwrap();

    let (code, out, took) = run(&w);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("fetch-failed")),
        "{out}"
    );
    assert!(took < Duration::from_secs(15), "the run took {took:?}");
    let logged = hook_lines(&w);
    assert_eq!(logged.len(), 1, "{logged:?}");
    assert_eq!(logged[0]["hook"]["result"], "timed-out", "{logged:?}");
}

#[test]
fn without_monitoring_a_run_writes_no_metric_file_and_calls_no_hook() {
    let w = World::new();
    hook(&w, 0);
    create(&w, &w.spec("t", "conn_test"));
    std::fs::write(w.root.join("fail"), "").unwrap();
    run(&w);
    run(&w);
    assert!(!instance_dir(&w).join("metrics.prom").exists());
    assert!(w.lines("hook.log").is_empty());
    assert!(hook_lines(&w).is_empty());
}

#[test]
fn a_hook_with_no_command_is_refused_by_the_spec() {
    let w = World::new();
    let spec = spec_with(&w, "monitoring:\n  hook: {command: []}\n");
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args([
            "create",
            "--spec",
            spec.to_str().unwrap(),
            "--no-extract",
            "--no-units",
        ])
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("monitoring.hook.command"), "{stderr}");
    assert!(!instance_dir(&w).exists());
}
