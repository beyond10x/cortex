//! `story:connections-revalidate-retry`: a Connectors failure of `connections revalidate` (or of
//! the `connections list` read before it) that Connectors marks retryable (`kind: operational`,
//! `next_action: retry_explicitly`) is tried again, at most 4 attempts with doubling pauses
//! (`CORTEX_REVALIDATE_ATTEMPTS`, `CORTEX_REVALIDATE_PAUSE_MS`). One that remains before the run's
//! first fetch fails the run as `connection-unavailable`, exit status 5, which `--record-failure`
//! does not count; a refusal is not retried and is counted as before.
//!
//! The stand-in `connectors` is driven by files under the world's root:
//! - `list-fail`: `connections list` answers the retryable failure once per line, removing one.
//! - `fail`: `connections revalidate` answers the retryable failure once per line, removing one.
//! - `fail-always`: `connections revalidate` answers the retryable failure on every call.
//! - `refuse`: `connections revalidate` answers the envelope the file holds.
//! - `ready`: the connection is listed `ready` (else `pending` until a revalidation succeeds).
//! - `lapse-second`: the second `invoke` answers `not_granted` at admission.

mod common;

use std::path::{Path, PathBuf};

use common::{executable, World};
use serde_json::Value;

/// Connectors' answer to `connections revalidate` on the deployment that found this
/// (beyond10x/connectors#174).
const RETRYABLE: &str = r#"{"ok":false,"error":{"code":"failure","data":{"code":"unavailable","kind":"operational","next_action":"retry_explicitly","stage":"dispatch"}}}"#;

/// A lapsed connection's refusal: not retried.
const NOT_GRANTED: &str =
    r#"{"ok":false,"error":{"code":"failure","data":{"code":"not_granted","stage":"admission"}}}"#;

/// An authentication failure, as Connectors writes it: operational, but it asks for a repair.
const UNAUTHENTICATED: &str = r#"{"ok":false,"error":{"code":"failure","data":{"code":"unauthenticated","kind":"operational","next_action":"repair_connection","stage":"readiness"}}}"#;

fn world() -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(
        &w.bin.join("connectors"),
        &format!(
            r#"R="{root}"
RETRYABLE='{RETRYABLE}'
case "$*" in
  *"connections list"*)
    echo "$*" >> "$R/lists.log"
    if [ -s "$R/list-fail" ]; then sed -i '1d' "$R/list-fail"; echo "$RETRYABLE" >&2; exit 1; fi
    if [ -e "$R/revalidated" ] || [ -e "$R/ready" ]; then S=ready; else S=pending; fi
    printf '{{"ok":true,"result":{{"connections":[{{"adapter":"tavily","connector":"tavily","instance":"main","connection":"conn_test","state":"%s","revision":"rev1"}}]}}}}' "$S" ;;
  *"connections revalidate"*)
    date +%s%N >> "$R/revalidations.log"
    if [ -e "$R/refuse" ]; then cat "$R/refuse" >&2; exit 1; fi
    if [ -e "$R/fail-always" ]; then echo "$RETRYABLE" >&2; exit 1; fi
    if [ -s "$R/fail" ]; then sed -i '1d' "$R/fail"; echo "$RETRYABLE" >&2; exit 1; fi
    touch "$R/revalidated"
    echo '{{"ok":true,"result":{{}}}}' ;;
  *"connections status"*) printf '{{"ok":true,"result":{{"connection":{{"summary":{{"state":"ready"}}}}}}}}' ;;
  "invoke "*)
    echo "$*" >> "$R/invocations.log"
    if [ -e "$R/lapse-second" ] && [ "$(wc -l < "$R/invocations.log")" -eq 2 ]; then
      echo '{NOT_GRANTED}' >&2; exit 1
    fi
    cat "$R/answer.json" ;;
  *) echo '{{"ok":false}}'; exit 2 ;;
esac
"#
        ),
    );
    w
}

/// A hook that appends its arguments, one call per line, to `hook.log`.
fn hook(w: &World) -> PathBuf {
    let path = w.bin.join("hook");
    executable(
        &path,
        &format!("echo \"$*\" >> \"{}/hook.log\"\n", w.root.display()),
    );
    path
}

/// Creates instance `name` over the world's web source (two queries, so a run invokes twice),
/// with the metric file and the hook; answers its directory.
fn created(w: &World, name: &str) -> PathBuf {
    let spec = w.spec(name, "conn_test");
    let text = std::fs::read_to_string(&spec).unwrap();
    let text = text.replace(
        "queries: [widget engine]",
        "queries: [widget engine, gadget runtime]",
    );
    let text = format!(
        "{text}monitoring:\n  metrics: true\n  hook: {{command: [\"{}\"], timeout_s: 10}}\n",
        hook(w).display()
    );
    std::fs::write(&spec, text).unwrap();
    let (code, out) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{out}");
    // `create` may revalidate; each case counts from here.
    for log in ["revalidations.log", "lists.log", "invocations.log"] {
        let _ = std::fs::remove_file(w.root.join(log));
    }
    let _ = std::fs::remove_file(w.root.join("revalidated"));
    w.home.join("instances").join(name)
}

/// `cortex run <source> --record-failure`, as the timers run it, pausing `pause_ms` between
/// revalidation attempts: its exit status and the run's own line, the first it prints (a recorded
/// failure prints a second).
fn run(w: &World, source: &str, pause_ms: &str) -> (i32, Value) {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["run", source, "--record-failure"])
        .env("CORTEX_REVALIDATE_PAUSE_MS", pause_ms)
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let line = stdout
        .lines()
        .next()
        .and_then(|l| serde_json::from_str(l).ok())
        .unwrap_or(Value::Null);
    assert!(
        out.status.code() != Some(2),
        "cortex run {source} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code().unwrap_or(-1), line)
}

/// The `sources` row of `source`.
fn row(w: &World, source: &str) -> Value {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_cortex"))
        .arg("sources")
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .find(|r| r["source_id"] == source)
        .unwrap_or_else(|| panic!("a row for {source}"))
}

fn status_file(dir: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(dir.join("status/news.json")).unwrap()).unwrap()
}

fn runs_of(dir: &Path, source: &str, outcome: &str) -> f64 {
    let text = std::fs::read_to_string(dir.join("metrics.prom")).unwrap_or_default();
    let series = format!("cortex_source_runs_total{{source=\"{source}\",outcome=\"{outcome}\"}} ");
    text.lines()
        .find_map(|l| l.strip_prefix(&series)?.trim().parse().ok())
        .unwrap_or(0.0)
}

/// The instants of the revalidations, in milliseconds.
fn revalidations(w: &World) -> Vec<u128> {
    w.lines("revalidations.log")
        .iter()
        .map(|l| l.parse::<u128>().unwrap() / 1_000_000)
        .collect()
}

/// Acceptance 2: revalidate answers the retryable failure twice, then succeeds: the run applies,
/// and revalidate was invoked 3 times.
#[test]
fn a_revalidation_failing_twice_retryably_is_retried_and_the_run_applies() {
    let w = world();
    created(&w, "a");
    std::fs::write(w.root.join("fail"), "f\n".repeat(2)).unwrap();
    let (code, ran) = run(&w, "a/news", "5");
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(revalidations(&w).len(), 3, "two retries");
    assert_eq!(row(&w, "a/news")["consecutive_failures"], 0);
}

/// The retryable failure from the `connections list` read before the revalidation is retried too.
#[test]
fn a_connections_list_failing_retryably_before_the_revalidation_is_retried() {
    let w = world();
    created(&w, "l");
    std::fs::write(w.root.join("list-fail"), "f\n").unwrap();
    let (code, ran) = run(&w, "l/news", "5");
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(
        revalidations(&w).len(),
        1,
        "one revalidation after the retried list"
    );
}

/// Acceptance 3: revalidate answers the retryable failure on every call: it is invoked 4 times;
/// the run ends `connection-unavailable`, exit 5, naming the last error. With `--record-failure`
/// the failure count does not move, and two such runs leave the source enabled. Its status, metric
/// and hook report the outcome; nothing was fetched.
#[test]
fn a_revalidation_failing_on_every_attempt_ends_connection_unavailable_and_is_not_counted() {
    let w = world();
    let dir = created(&w, "b");
    std::fs::write(w.root.join("fail-always"), "").unwrap();
    for n in 1..=2 {
        let (code, ran) = run(&w, "b/news", "5");
        assert_eq!(
            (code, ran["outcome"].as_str()),
            (5, Some("connection-unavailable")),
            "run {n}: {ran}"
        );
        let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
        assert!(
            reason.contains("connections revalidate: unavailable at dispatch"),
            "run {n}: {reason}"
        );
        assert_eq!(revalidations(&w).len(), 4 * n, "run {n}: four attempts");
        let r = row(&w, "b/news");
        assert_eq!(r["state"], "Enabled", "run {n}: {r}");
        assert_eq!(r["consecutive_failures"], 0, "run {n}: {r}");
        assert_eq!(r["last_outcome"], "connection-unavailable", "run {n}: {r}");
        let status = status_file(&dir);
        assert_eq!(status["last_outcome"], "connection-unavailable", "{status}");
        assert_eq!(status["runs"]["connection-unavailable"], n, "{status}");
        assert_eq!(runs_of(&dir, "b/news", "connection-unavailable"), n as f64);
    }
    assert!(w.lines("invocations.log").is_empty(), "nothing was fetched");
    assert_eq!(
        w.lines("hook.log"),
        [
            "b/news connection-unavailable",
            "b/news connection-unavailable"
        ]
    );
    let systemctl = std::fs::read_to_string(w.root.join("systemctl.log")).unwrap_or_default();
    assert!(!systemctl.contains("disable"), "{systemctl}");

    // The next run starts as usual.
    std::fs::remove_file(w.root.join("fail-always")).unwrap();
    let (code, ran) = run(&w, "b/news", "5");
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
}

/// `CORTEX_REVALIDATE_ATTEMPTS` bounds the attempts; at least 1.
#[test]
fn the_attempts_follow_cortex_revalidate_attempts() {
    let w = world();
    created(&w, "n");
    std::fs::write(w.root.join("fail-always"), "").unwrap();
    for (attempts, expected) in [("2", 2), ("0", 1)] {
        let before = revalidations(&w).len();
        let (code, ran) = w.cortex_env(
            &["run", "n/news"],
            &[
                ("CORTEX_REVALIDATE_PAUSE_MS", Path::new("5")),
                ("CORTEX_REVALIDATE_ATTEMPTS", Path::new(attempts)),
            ],
        );
        assert_eq!(
            (code, ran["outcome"].as_str()),
            (5, Some("connection-unavailable")),
            "{ran}"
        );
        assert_eq!(revalidations(&w).len() - before, expected, "{attempts}");
    }
}

/// Acceptance 4: a refusal (`not_granted`, or an authentication failure) is not retried: revalidate
/// is invoked once, and the run fails and is counted as before.
#[test]
fn a_refused_revalidation_is_not_retried_and_is_counted() {
    for (name, refusal) in [("c", NOT_GRANTED), ("d", UNAUTHENTICATED)] {
        let w = world();
        created(&w, name);
        std::fs::write(w.root.join("refuse"), refusal).unwrap();
        let source = format!("{name}/news");
        let (code, ran) = run(&w, &source, "5");
        assert_eq!(
            (code, ran["outcome"].as_str()),
            (1, Some("fetch-failed")),
            "{name}: {ran}"
        );
        assert_eq!(revalidations(&w).len(), 1, "{name}: not retried");
        assert_eq!(row(&w, &source)["consecutive_failures"], 1, "{name}");
    }
}

/// Acceptance 5: the pauses follow `CORTEX_REVALIDATE_PAUSE_MS` and double.
#[test]
fn the_pauses_follow_cortex_revalidate_pause_ms_and_double() {
    let w = world();
    created(&w, "p");
    std::fs::write(w.root.join("fail-always"), "").unwrap();
    let (code, ran) = run(&w, "p/news", "150");
    assert_eq!(code, 5, "{ran}");
    let at = revalidations(&w);
    assert_eq!(at.len(), 4, "{at:?}");
    let gaps: Vec<u128> = at.windows(2).map(|p| p[1] - p[0]).collect();
    for (i, gap) in gaps.iter().enumerate() {
        let pause = 150 << i;
        assert!(
            *gap >= pause,
            "pause {i} is {gap} ms, at least {pause}: {gaps:?}"
        );
    }
    // Not the default 5 s.
    assert!(gaps[0] < 5_000, "{gaps:?}");
}

/// Outcome 3: a revalidation that fails after the run fetched documents (a lapse renewed mid-run)
/// is retried the same way and then fails the run as before: `fetch-failed`, counted.
#[test]
fn a_revalidation_failing_after_a_fetch_is_retried_and_fails_the_run_as_before() {
    let w = world();
    created(&w, "m");
    std::fs::write(w.root.join("ready"), "").unwrap();
    std::fs::write(w.root.join("lapse-second"), "").unwrap();
    std::fs::write(w.root.join("fail-always"), "").unwrap();
    let (code, ran) = run(&w, "m/news", "5");
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("fetch-failed")),
        "{ran}"
    );
    assert_eq!(
        w.lines("invocations.log").len(),
        2,
        "the first query fetched"
    );
    assert_eq!(revalidations(&w).len(), 4, "the renewal retried");
    assert_eq!(row(&w, "m/news")["consecutive_failures"], 1);
}
