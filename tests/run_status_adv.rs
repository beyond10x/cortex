//! Adversarial cases for a run's status, its metric file and its hook (`cortex.instance.RunHook`,
//! `cortex.instance.MonitoringSpec`): the hook's timeout bounds the whole call, a hook killed at
//! its timeout leaves nothing of it running, and the metric file and the status name each outcome
//! of the instance's own sources only.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use common::{executable, World};
use serde_json::Value;

/// The world's spec for instance `name`, with `monitoring` appended as written.
fn spec_with(w: &World, name: &str, monitoring: &str) -> PathBuf {
    let path = w.spec(name, "conn_test");
    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str(monitoring);
    std::fs::write(&path, text).unwrap();
    path
}

/// `monitoring` with no metric file and `hook` called with `timeout_s`.
fn hook_only(hook: &Path, timeout_s: u32) -> String {
    format!(
        "monitoring:\n  hook: {{command: [\"{}\"], timeout_s: {timeout_s}}}\n",
        hook.display()
    )
}

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

/// `cortex run <id> --record-failure`: its exit status, its first line and how long it took.
fn run(w: &World, id: &str) -> (i32, Value, Duration) {
    let started = Instant::now();
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["run", id, "--record-failure"])
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

fn row(w: &World, id: &str) -> Value {
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .arg("sources")
        .env("CORTEX_HOME", &w.home)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .find(|row| row["source_id"] == id)
        .expect("a row for the source")
}

fn metrics(w: &World, name: &str) -> String {
    std::fs::read_to_string(w.home.join("instances").join(name).join("metrics.prom"))
        .unwrap_or_default()
}

fn hook_lines(w: &World) -> Vec<Value> {
    std::fs::read_to_string(w.home.join("instances/t/cortex.log"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|l| l.get("hook").is_some())
        .collect()
}

/// Whether process `pid` is alive (and not a zombie).
fn alive(pid: &str) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|s| {
            s.rsplit_once(')')
                .map(|(_, rest)| rest.trim_start().to_string())
        })
        .is_some_and(|rest| !rest.starts_with('Z'))
}

fn kill(pid: &str) {
    let _ = Command::new("kill").arg("-9").arg(pid).status();
}

/// A hook that starts a notifier in the background and exits 0 at once: a common shape of
/// "fire and forget". Its child inherits the hook's stderr. The hook itself ended within its
/// timeout of 1 s, so the run must end soon after; the spec bounds a hook call by `timeout_s`.
#[test]
fn a_hook_that_exits_at_once_leaving_a_background_child_does_not_hold_the_run_past_its_timeout() {
    let w = World::new();
    let path = w.bin.join("hook");
    executable(
        &path,
        &format!(
            "echo \"$*\" >> \"{root}/hook.log\"\nsleep 20 &\necho $! > \"{root}/child.pid\"\nexit 0\n",
            root = w.root.display()
        ),
    );
    create(&w, &spec_with(&w, "t", &hook_only(&path, 1)));
    std::fs::write(w.root.join("fail"), "").unwrap();

    let (code, out, took) = run(&w, "t/news");
    let child = std::fs::read_to_string(w.root.join("child.pid")).unwrap_or_default();
    kill(child.trim());
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("fetch-failed")),
        "{out}"
    );
    assert_eq!(w.lines("hook.log"), ["t/news fetch-failed"]);
    assert!(
        took < Duration::from_secs(10),
        "a hook with timeout_s 1 that exited at once held the run (and the home's lock) for \
         {took:?}: the run waited for the hook's background child to close stderr"
    );
}

/// A hook that runs its work in a child and waits for it. At its timeout the hook is killed;
/// the spec says the hook is killed, and the work it started is the hook's work.
#[test]
fn a_hook_killed_at_its_timeout_leaves_no_process_it_started_running() {
    let w = World::new();
    let path = w.bin.join("hook");
    executable(
        &path,
        &format!(
            "sleep 30 &\necho $! > \"{root}/child.pid\"\nwait\n",
            root = w.root.display()
        ),
    );
    create(&w, &spec_with(&w, "t", &hook_only(&path, 1)));
    std::fs::write(w.root.join("fail"), "").unwrap();

    let (code, _, took) = run(&w, "t/news");
    assert_eq!(code, 1);
    assert!(took < Duration::from_secs(15), "the run took {took:?}");
    let logged = hook_lines(&w);
    assert_eq!(logged.len(), 1, "{logged:?}");
    assert_eq!(logged[0]["hook"]["result"], "timed-out", "{logged:?}");

    let child = std::fs::read_to_string(w.root.join("child.pid")).unwrap();
    let child = child.trim();
    let survived = alive(child);
    kill(child);
    assert!(
        !survived,
        "the hook was reported timed-out and killed, but the process it started (pid {child}) \
         is still running after the run ended"
    );
}

/// `metrics` absent, `hook` present: no metric file. Catches a `metrics` read as "not false".
#[test]
fn a_monitoring_block_with_a_hook_and_no_metrics_writes_no_metric_file() {
    let w = World::new();
    let path = w.bin.join("hook");
    executable(&path, "exit 0\n");
    create(&w, &spec_with(&w, "t", &hook_only(&path, 5)));
    std::fs::write(w.root.join("fail"), "").unwrap();
    let (code, _, _) = run(&w, "t/news");
    assert_eq!(code, 1);
    assert_eq!(hook_lines(&w).len(), 1);
    assert!(
        !w.home.join("instances/t/metrics.prom").exists(),
        "{}",
        metrics(&w, "t")
    );
}

/// Two instances in one home: the metric file of `t` lists `t`'s sources only.
#[test]
fn the_metric_file_of_an_instance_lists_its_own_sources_only() {
    let w = World::new();
    create(&w, &spec_with(&w, "t", "monitoring: {metrics: true}\n"));
    create(&w, &spec_with(&w, "u", "monitoring: {metrics: true}\n"));
    let (code, _, _) = run(&w, "t/news");
    assert_eq!(code, 0);
    let text = metrics(&w, "t");
    assert!(
        text.contains("cortex_source_runs_total{source=\"t/news\",outcome=\"ran\"} 1\n"),
        "{text}"
    );
    assert!(!text.contains("u/news"), "{text}");
    assert_eq!(metrics(&w, "u"), "", "u did not run");
}

/// An extraction failure (the model call fails) is named `extraction-failed` in `cortex sources`,
/// in the metric file and in the hook's arguments, not under another failure's name.
#[test]
fn an_extraction_failure_is_named_extraction_failed_in_sources_metrics_and_hook() {
    let w = World::new();
    executable(&w.bin.join("claude"), "echo model down >&2\nexit 1\n");
    let path = w.bin.join("hook");
    executable(
        &path,
        &format!("echo \"$*\" >> \"{}/hook.log\"\n", w.root.display()),
    );
    let monitoring = format!(
        "monitoring:\n  metrics: true\n  hook: {{command: [\"{}\"]}}\n",
        path.display()
    );
    create(&w, &spec_with(&w, "t", &monitoring));

    let (code, out, _) = run(&w, "t/news");
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("extraction-failed")),
        "{out}"
    );
    let r = row(&w, "t/news");
    assert_eq!(r["last_outcome"], "extraction-failed", "{r}");
    assert_eq!(r["last_reason"], out["detail"]["reason"], "{r}");
    let text = metrics(&w, "t");
    assert!(
        text.contains(
            "cortex_source_runs_total{source=\"t/news\",outcome=\"extraction-failed\"} 1\n"
        ),
        "{text}"
    );
    assert!(
        text.contains("cortex_source_runs_total{source=\"t/news\",outcome=\"apply-refused\"} 0\n"),
        "{text}"
    );
    assert_eq!(w.lines("hook.log"), ["t/news extraction-failed"]);
}
