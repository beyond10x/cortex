//! Adversary cases for `story:connections-revalidate-retry`: a retryable Connectors failure of
//! `connections revalidate` (or of the `connections list` read before it) is retried, at most 4
//! attempts pausing 5, 10 and 20 s; one that remains before any fetch ends the run as
//! `connection-unavailable`, exit 5, not counted; a refusal is never retried.
//!
//! The stand-in `connectors` is driven by files under the world's root:
//! - `list-fail`: `connections list` answers the retryable failure once per line, removing one.
//! - `list-fail-always`: `connections list` answers the retryable failure on every call.
//! - `fail`: `connections revalidate` answers the retryable failure once per line, removing one.
//! - `fail-always`: `connections revalidate` answers the retryable failure on every call.
//! - `refuse`: once `fail` is spent, `connections revalidate` answers the envelope it holds.
//! - `ready`: the connection is listed `ready` (else `pending` until a revalidation succeeds).
//! - `lapse-first`: the first `operations invoke` answers `not_granted` at admission.
//!
//! Every entry `connections list` answers carries the profile `tavily.api-key`, so a spec may
//! name the connection by id or by that profile.

mod common;

use std::path::{Path, PathBuf};

use common::{executable, World};
use serde_json::Value;

/// The retryable failure Connectors answered on the deployment that found the story.
const RETRYABLE: &str = r#"{"ok":false,"error":{"code":"failure","data":{"code":"unavailable","kind":"operational","next_action":"retry_explicitly","stage":"dispatch"}}}"#;

/// A lapsed connection's refusal.
const NOT_GRANTED: &str = r#"{"ok":false,"error":{"code":"failure","data":{"code":"not_granted","kind":"operational","next_action":"repair_connection","stage":"admission"}}}"#;

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
    date +%s%N >> "$R/lists.log"
    if [ -e "$R/list-fail-always" ]; then echo "$RETRYABLE" >&2; exit 1; fi
    if [ -s "$R/list-fail" ]; then sed -i '1d' "$R/list-fail"; echo "$RETRYABLE" >&2; exit 1; fi
    if [ -e "$R/revalidated" ] || [ -e "$R/ready" ]; then S=ready; else S=pending; fi
    printf '{{"ok":true,"result":{{"connections":[{{"adapter":"tavily","connection":"conn_test","profile":"tavily.api-key","state":"%s","revision":"rev1"}}]}}}}' "$S" ;;
  *"connections revalidate"*)
    date +%s%N >> "$R/revalidations.log"
    if [ -e "$R/fail-always" ]; then echo "$RETRYABLE" >&2; exit 1; fi
    if [ -s "$R/fail" ]; then sed -i '1d' "$R/fail"; echo "$RETRYABLE" >&2; exit 1; fi
    if [ -e "$R/refuse" ]; then cat "$R/refuse" >&2; exit 1; fi
    touch "$R/revalidated"
    echo '{{"ok":true,"result":{{}}}}' ;;
  *"connections status"*) printf '{{"ok":true,"result":{{"connection":{{"summary":{{"state":"ready"}}}}}}}}' ;;
  *"operations describe"*) echo '{{"ok":true,"result":{{"schema":"s","revision":"r"}}}}' ;;
  *"operations invoke"*)
    echo "$*" >> "$R/invocations.log"
    if [ -e "$R/lapse-first" ] && [ "$(wc -l < "$R/invocations.log")" -eq 1 ]; then
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

/// Creates instance `name` over the world's web source, its connection named as `connection`
/// (`connection: conn_test` or `profile: tavily.api-key`), with the metric file and the hook.
fn created(w: &World, name: &str, connection: &str) -> PathBuf {
    let spec = w.spec(name, "@CONNECTION@");
    let text = std::fs::read_to_string(&spec)
        .unwrap()
        .replace("connection: @CONNECTION@", connection);
    let text = format!(
        "{text}monitoring:\n  metrics: true\n  hook: {{command: [\"{}\"], timeout_s: 10}}\n",
        hook(w).display()
    );
    std::fs::write(&spec, text).unwrap();
    let (code, out) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{out}");
    for log in ["revalidations.log", "lists.log", "invocations.log"] {
        let _ = std::fs::remove_file(w.root.join(log));
    }
    let _ = std::fs::remove_file(w.root.join("revalidated"));
    w.home.join("instances").join(name)
}

/// `cortex run <source> <extra…>` with `env` on top of the world's variables: its exit status and
/// the run's own line, the first it prints.
fn run_with(w: &World, source: &str, extra: &[&str], env: &[(&str, &str)]) -> (i32, Value) {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_cortex"))
        .arg("run")
        .arg(source)
        .args(extra)
        .envs(env.iter().copied())
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

/// `cortex run <source> --record-failure`, as the timers run it, with pauses of 5 ms.
fn run(w: &World, source: &str) -> (i32, Value) {
    run_with(
        w,
        source,
        &["--record-failure"],
        &[("CORTEX_REVALIDATE_PAUSE_MS", "5")],
    )
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

/// The instants written to `log`, in milliseconds.
fn instants(w: &World, log: &str) -> Vec<u128> {
    w.lines(log)
        .iter()
        .map(|l| l.parse::<u128>().unwrap() / 1_000_000)
        .collect()
}

fn gaps(at: &[u128]) -> Vec<u128> {
    at.windows(2).map(|p| p[1] - p[0]).collect()
}

fn status_file(dir: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(dir.join("status/news.json")).unwrap()).unwrap()
}

/// A source naming its connection by profile: the `connections list` read that resolves the
/// profile is the first read before the revalidation. Answering it the retryable failure once is
/// retried like the list read inside the revalidation, and the run applies.
#[test]
fn a_profile_resolution_failing_retryably_once_is_retried_and_the_run_applies() {
    let w = world();
    created(&w, "pa", "profile: tavily.api-key");
    std::fs::write(w.root.join("list-fail"), "f\n").unwrap();
    let (code, ran) = run(&w, "pa/news");
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(row(&w, "pa/news")["consecutive_failures"], 0);
}

/// The same source when `connections list` answers the retryable failure on every attempt:
/// nothing was fetched, so the run ends `connection-unavailable`, exit 5, and two such runs leave
/// the source enabled with no failure counted. This is the story's symptom for a source that
/// names a profile.
#[test]
fn a_profile_resolution_failing_on_every_attempt_is_not_counted_and_never_disables() {
    let w = world();
    created(&w, "pb", "profile: tavily.api-key");
    std::fs::write(w.root.join("list-fail-always"), "").unwrap();
    for n in 1..=2 {
        let (code, ran) = run(&w, "pb/news");
        assert_eq!(
            (code, ran["outcome"].as_str()),
            (5, Some("connection-unavailable")),
            "run {n}: {ran}"
        );
        let r = row(&w, "pb/news");
        assert_eq!(r["consecutive_failures"], 0, "run {n}: {r}");
        assert_eq!(r["state"], "Enabled", "run {n}: {r}");
    }
    assert!(w.lines("invocations.log").is_empty(), "nothing was fetched");
}

/// A connection named by id whose `connections list` answers the retryable failure on every
/// attempt: 4 list reads, no revalidation, `connection-unavailable`, exit 5, not counted.
#[test]
fn a_connections_list_failing_on_every_attempt_ends_connection_unavailable() {
    let w = world();
    created(&w, "la", "connection: conn_test");
    std::fs::write(w.root.join("list-fail-always"), "").unwrap();
    let (code, ran) = run(&w, "la/news");
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (5, Some("connection-unavailable")),
        "{ran}"
    );
    assert_eq!(w.lines("lists.log").len(), 4, "four attempts of the list");
    assert!(w.lines("revalidations.log").is_empty());
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("connections list: unavailable at dispatch"),
        "{reason}"
    );
    assert_eq!(row(&w, "la/news")["consecutive_failures"], 0);
}

/// The boundary: retryable on attempts 1 to 3, a success on the 4th and last: the run applies.
#[test]
fn a_revalidation_succeeding_on_the_last_attempt_runs() {
    let w = world();
    created(&w, "sl", "connection: conn_test");
    std::fs::write(w.root.join("fail"), "f\n".repeat(3)).unwrap();
    let (code, ran) = run(&w, "sl/news");
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(w.lines("revalidations.log").len(), 4);
}

/// Retryable on attempts 1 to 3 and a refusal on the 4th: the refusal is the run's failure,
/// `fetch-failed`, counted; not `connection-unavailable`.
#[test]
fn a_refusal_on_the_last_attempt_fails_the_run_and_is_counted() {
    let w = world();
    created(&w, "rl", "connection: conn_test");
    std::fs::write(w.root.join("fail"), "f\n".repeat(3)).unwrap();
    std::fs::write(w.root.join("refuse"), NOT_GRANTED).unwrap();
    let (code, ran) = run(&w, "rl/news");
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("fetch-failed")),
        "{ran}"
    );
    assert_eq!(w.lines("revalidations.log").len(), 4);
    assert_eq!(row(&w, "rl/news")["consecutive_failures"], 1);
}

/// A refusal code (`revoked`) is never retried, even in an envelope that says `operational` and
/// `retry_explicitly`: one revalidation, `fetch-failed`, counted.
#[test]
fn a_refusal_marked_retry_explicitly_is_still_not_retried() {
    let w = world();
    created(&w, "rr", "connection: conn_test");
    std::fs::write(
        w.root.join("refuse"),
        r#"{"ok":false,"error":{"code":"failure","data":{"code":"revoked","kind":"operational","next_action":"retry_explicitly","stage":"admission"}}}"#,
    )
    .unwrap();
    let (code, ran) = run(&w, "rr/news");
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("fetch-failed")),
        "{ran}"
    );
    assert_eq!(w.lines("revalidations.log").len(), 1, "not retried");
    assert_eq!(row(&w, "rr/news")["consecutive_failures"], 1);
}

/// The failure count is unchanged, not reset: a source with one counted failure keeps it across
/// a `connection-unavailable` run, and the next counted failure disables it. A mutant that reset
/// the count on the new outcome passes the unit's own cases, which start from 0.
#[test]
fn a_connection_unavailable_run_keeps_an_earlier_failure_count() {
    let w = world();
    let dir = created(&w, "kc", "connection: conn_test");
    std::fs::write(w.root.join("refuse"), NOT_GRANTED).unwrap();
    let (code, _) = run(&w, "kc/news");
    assert_eq!(code, 1);
    assert_eq!(row(&w, "kc/news")["consecutive_failures"], 1);

    std::fs::write(w.root.join("fail-always"), "").unwrap();
    let (code, ran) = run(&w, "kc/news");
    assert_eq!(code, 5, "{ran}");
    let r = row(&w, "kc/news");
    assert_eq!(r["consecutive_failures"], 1, "{r}");
    assert_eq!(r["state"], "Enabled", "{r}");
    assert_eq!(status_file(&dir)["runs"]["fetch-failed"], 1);

    std::fs::remove_file(w.root.join("fail-always")).unwrap();
    let (code, ran) = run(&w, "kc/news");
    assert_eq!(code, 1, "{ran}");
    let r = row(&w, "kc/news");
    assert_eq!(
        r["state"], "Disabled",
        "the second counted failure disables: {r}"
    );
    assert_eq!(
        w.lines("hook.log"),
        [
            "kc/news fetch-failed",
            "kc/news connection-unavailable",
            "kc/news disabled"
        ]
    );
}

/// A lapse at the first read, before anything was fetched, whose renewal stays unavailable: the
/// run fetched nothing, so it is `connection-unavailable`, exit 5, not counted.
#[test]
fn a_renewal_unavailable_before_any_answer_is_connection_unavailable() {
    let w = world();
    created(&w, "lf", "connection: conn_test");
    std::fs::write(w.root.join("ready"), "").unwrap();
    std::fs::write(w.root.join("lapse-first"), "").unwrap();
    std::fs::write(w.root.join("fail-always"), "").unwrap();
    let (code, ran) = run(&w, "lf/news");
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (5, Some("connection-unavailable")),
        "{ran}"
    );
    assert_eq!(w.lines("invocations.log").len(), 1);
    assert_eq!(w.lines("revalidations.log").len(), 4);
    assert_eq!(row(&w, "lf/news")["consecutive_failures"], 0);
}

/// `--fetch-only` records nothing on the source: a revalidation unavailable on every attempt
/// exits 1 as `fetch-failed`, as the operating page says, and counts nothing.
#[test]
fn fetch_only_with_the_connection_unavailable_exits_1_and_records_nothing() {
    let w = world();
    created(&w, "fo", "connection: conn_test");
    std::fs::write(w.root.join("fail-always"), "").unwrap();
    let (code, ran) = run_with(
        &w,
        "fo/news",
        &["--fetch-only"],
        &[("CORTEX_REVALIDATE_PAUSE_MS", "5")],
    );
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("fetch-failed")),
        "{ran}"
    );
    assert_eq!(w.lines("revalidations.log").len(), 4);
    let r = row(&w, "fo/news");
    assert_eq!(r["consecutive_failures"], 0, "{r}");
    assert!(r["last_outcome"].is_null(), "{r}");
}

/// The pauses are the setting, then twice it, then four times it: bounded from above too, so a
/// mutant that doubles before the first pause (10, 20, 40 s) is caught. The unit's own case
/// asserts lower bounds only.
#[test]
fn the_pauses_start_at_the_setting_and_not_at_twice_it() {
    let w = world();
    created(&w, "pu", "connection: conn_test");
    std::fs::write(w.root.join("fail-always"), "").unwrap();
    let (code, ran) = run_with(
        &w,
        "pu/news",
        &["--record-failure"],
        &[("CORTEX_REVALIDATE_PAUSE_MS", "400")],
    );
    assert_eq!(code, 5, "{ran}");
    let gaps = gaps(&instants(&w, "revalidations.log"));
    assert_eq!(gaps.len(), 3, "{gaps:?}");
    for (i, gap) in gaps.iter().enumerate() {
        let pause = 400u128 << i;
        assert!(
            *gap >= pause && *gap < 2 * pause,
            "pause {i} is {gap} ms, expected {pause} up to {}: {gaps:?}",
            2 * pause
        );
    }
}

/// With `CORTEX_REVALIDATE_PAUSE_MS` unset the first pause is the default 5 s. Nothing in the
/// unit's suite runs without the variable, so the default is otherwise untested.
#[test]
fn the_first_pause_defaults_to_five_seconds() {
    let w = world();
    created(&w, "dp", "connection: conn_test");
    std::fs::write(w.root.join("fail-always"), "").unwrap();
    let (code, ran) = run_with(
        &w,
        "dp/news",
        &["--record-failure"],
        &[("CORTEX_REVALIDATE_ATTEMPTS", "2")],
    );
    assert_eq!(code, 5, "{ran}");
    let gaps = gaps(&instants(&w, "revalidations.log"));
    assert_eq!(gaps.len(), 1, "{gaps:?}");
    assert!(
        gaps[0] >= 5_000 && gaps[0] < 9_000,
        "the first pause is {} ms",
        gaps[0]
    );
}
