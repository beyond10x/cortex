//! Adversarial cases for unit `tail` of wave 20261030a (`story:connectors-call-deadline`,
//! `story:window-relative-placeholders`). Each case drives the built binary and asserts what the
//! unit's own documents promise: `CORTEX_CONNECTORS_TIMEOUT_S` bounds every `connectors` call
//! (`website/docs/operating.md`, the `fetch-failed` comment in `spec/domains/instance.yaml`), and
//! `{since_minutes_ago}` never narrows the window (`website/docs/spec-file.md`).

mod common;

use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use common::{ekr, executable, World, LISTED};

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

/// A `connectors` whose `invoke` records when it started (Unix ms), leaves a background process
/// holding its standard output and standard error for 15 s, and exits 1 at once with nothing on
/// stdout: a wrapper or adapter that leaves a descendant behind.
const EXITS_LEAVING_A_DESCENDANT: &str = r#"R="@ROOT@"
case "$*" in
  *"connections list"*) printf '{"ok":true,"result":{"connections":[{"adapter":"tavily",@LISTED@,"connection":"conn_test","state":"ready","revision":"rev1"}]}}' ;;
  "invoke tavily/main "*) cat > /dev/null; date +%s%3N > "$R/started"; sleep 15 & echo '{"ok":false,"error":{"code":"failure","data":{"code":"unavailable","stage":"execution"}}}' >&2; exit 1 ;;
  *) echo '{"ok":false}'; exit 2 ;;
esac
"#;

/// The deadline holds for a `connectors` that exited but left a descendant holding its output
/// pipes: the call is over within `CORTEX_CONNECTORS_TIMEOUT_S` (1 s here) plus a margin, not
/// when the descendant lets go of the pipe 15 s later. `wait_with` checks the deadline only while
/// `try_wait` answers `None`; once the direct child exited it joins the stdout and stderr readers
/// without one.
#[test]
fn adv_the_deadline_holds_when_connectors_exits_and_a_descendant_keeps_its_output_open() {
    let w = World::new();
    executable(
        &w.bin.join("connectors"),
        &EXITS_LEAVING_A_DESCENDANT
            .replace("@ROOT@", &w.root.display().to_string())
            .replace("@LISTED@", LISTED),
    );
    let spec = w.spec("d", "conn_test");
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");

    let (code, ran) = w.cortex_env(
        &["run", "d/news"],
        &[("CORTEX_CONNECTORS_TIMEOUT_S", Path::new("1"))],
    );
    let ended = now_ms();
    let started: i64 = std::fs::read_to_string(w.root.join("started"))
        .expect("the invoke started")
        .trim()
        .parse()
        .unwrap();
    let waited_ms = ended - started;
    assert!(
        waited_ms <= 3_000,
        "the run ended {waited_ms} ms after the connectors call started, with \
         CORTEX_CONNECTORS_TIMEOUT_S=1: the call was waited for until its descendant closed the \
         pipe (exit {code}, {ran})"
    );
}

/// `CORTEX_CONNECTORS_TIMEOUT_S` is documented as a number of seconds, at least 1, with no upper
/// bound. One past what `Instant` can hold (`u64::MAX`, an operator's "never") overflows
/// `Instant::now() + limit` in `wait_within` and panics the run (exit 101) instead of running or
/// being refused.
#[test]
fn adv_an_unbounded_connectors_timeout_does_not_panic_the_run() {
    let w = World::new();
    let spec = w.spec("u", "conn_test");
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let started = Instant::now();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["run", "u/news"])
        .env("CORTEX_CONNECTORS_TIMEOUT_S", "18446744073709551615")
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&out.stderr);
    assert_ne!(
        out.status.code(),
        Some(101),
        "the run panicked after {:?}: {said}",
        started.elapsed()
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && stdout.contains(r#""outcome":"ran""#),
        "{stdout} {said}"
    );
}

/// A tracker with one ready connection whose `issues.list` logs `<unix ms> <argv> <input>` when it
/// is invoked and answers no issue; the input with `"part":"first"` takes 5 s to answer, as a
/// first input with many pages or child calls does.
const TRACKER: &str = r#"R="@ROOT@"
IN=""; if [ "$1" = invoke ]; then IN=$(cat); fi; A="$* $IN"
case "$A" in
  *"connections list"*) echo '{"ok":true,"result":{"connections":[{"adapter":"tracker","connector":"tracker","instance":"main","connection":"conn_test","state":"ready","revision":"rev1"}]}}' ;;
  *"connections revalidate"*) echo '{"ok":true,"result":{}}' ;;
  "invoke "*" issues.list "*)
    echo "$(date +%s%3N) $A" >> "$R/issues.log"
    case "$A" in *'"part":"first"'*) sleep 5 ;; esac
    printf '{"ok":true,"result":{"connector":"tracker","instance":"main","operation":"issues.list","revision":"r","result":{"issues":[]}}}\n' ;;
  *) echo '{"ok":false}'; exit 2 ;;
esac
"#;

const OVERLAP_MS: i64 = 5 * 60 * 1000;

/// `{since_minutes_ago}` is documented as "rounded up from the milliseconds so the window is never
/// narrowed" (`website/docs/spec-file.md`). It is computed once, from the run's start, but the
/// query it fills is evaluated relative to when the call is made. A second input invoked 5 s after
/// the run started therefore asks from `invoke − N min`, later than the window's `{since}`: the
/// window is narrowed by the time before the call less the rounding slack (here the slack is held
/// under 3 s by the planted last start). Records changed in the cut-off part are missed whenever
/// that delay exceeds the previous run's by more than the 5-minute overlap.
#[test]
fn adv_since_minutes_ago_still_reaches_since_when_the_call_starts_after_the_run() {
    let w = World::new();
    executable(
        &w.bin.join("connectors"),
        &TRACKER.replace("@ROOT@", &w.root.display().to_string()),
    );
    let path = w.root.join("m.yaml");
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: m
description: Test brain.
ekr: {{version: "0.0.34", bin: "{ekr}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
  - name: tracker
    schedule: daily
    settings:
      kind: connectors
      value:
        adapter: tracker
        connection: conn_test
        operation: issues.list
        inputs:
          - {{part: "first", jql: "updated >= -{{since_minutes_ago}}m"}}
          - {{part: "second", jql: "updated >= -{{since_minutes_ago}}m"}}
        records: issues
        id: key
        text: ["{{key}}: {{summary}}"]
    policy: {{refresh_after_days: 3, change: ContentHash, max_documents_per_run: 50, max_chars_per_document: 5000}}
serve: {{view_port: 18995}}
"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");

    // The last successful run started 10 min less 3 s ago: this run's window is 15 min less 3 s
    // plus however long cortex takes to start, which rounds up to 15 min while that is under 3 s.
    let last_start = now_ms() - 10 * 60_000 + 3_000;
    let since_ms = last_start - OVERLAP_MS;
    let state = w.home.join("instances/m/state/tracker.json");
    std::fs::create_dir_all(state.parent().unwrap()).unwrap();
    std::fs::write(
        &state,
        format!(
            r#"{{"format":"cortex.seen/1","documents":{{}},"last_success_started_at":{last_start}}}"#
        ),
    )
    .unwrap();
    let (code, ran) = w.cortex(&["run", "m/tracker"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");

    let log = w.lines("issues.log");
    let second = log
        .iter()
        .find(|l| l.contains(r#""part":"second""#))
        .unwrap_or_else(|| panic!("the second input was invoked: {log:?}"));
    let invoked_ms: i64 = second.split(' ').next().unwrap().parse().unwrap();
    let minutes: i64 = second
        .split("updated >= -")
        .nth(1)
        .and_then(|rest| rest.split('m').next())
        .unwrap_or_else(|| panic!("a rendered jql in {second}"))
        .parse()
        .unwrap_or_else(|_| panic!("a number of minutes in {second}"));
    let asked_from = invoked_ms - minutes * 60_000;
    assert!(
        asked_from <= since_ms,
        "the second input asked from {asked_from} (invoked at {invoked_ms}, -{minutes}m), \
         {} ms after the window's since {since_ms}: the window was narrowed",
        asked_from - since_ms
    );
}
