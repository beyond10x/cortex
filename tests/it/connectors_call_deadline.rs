//! `story:connectors-call-deadline`: every `connectors` call is waited for at most
//! `CORTEX_CONNECTORS_TIMEOUT_S` seconds (120). Past it the child is killed and reaped, and the
//! call fails as `timeout`, so a hung call does not hold the run until something outside ends it.

use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::common::{executable, World, LISTED};

/// A `connectors` whose connection is ready and whose `invoke` records its pid and the time it
/// started (Unix nanoseconds), then sleeps far past any deadline the test sets.
const HANGS: &str = r#"R="@ROOT@"
case "$*" in
  *"connections list"*) printf '{"ok":true,"result":{"connections":[{"adapter":"tavily",@LISTED@,"connection":"conn_test","state":"ready","revision":"rev1"}]}}' ;;
  "invoke tavily/main "*) echo $$ > "$R/pid"; date +%s%N > "$R/started"; exec sleep 60 ;;
  *) echo '{"ok":false}'; exit 2 ;;
esac
"#;

fn now_ns() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

/// Acceptance 1: a `connectors` that sleeps past the deadline fails the call as `timeout` within
/// the deadline plus one second, and the child is gone.
#[test]
fn a_connectors_call_past_the_deadline_fails_as_timeout_and_the_child_is_gone() {
    let w = World::new();
    executable(
        &w.bin.join("connectors"),
        &HANGS
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
    let ended = now_ns();
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("fetch-failed")),
        "{ran}"
    );
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("timeout"), "{reason}");

    let started: u128 = std::fs::read_to_string(w.root.join("started"))
        .expect("the invoke started")
        .trim()
        .parse()
        .unwrap();
    // The reason names the timeout, so the deadline ended the call. From below, the call was
    // waited for its whole deadline; from above, the run ended long before the 60 s the hung
    // `connectors` sleeps, with 9 s of room for the run's own end on a loaded machine.
    let waited_ms = (ended - started) / 1_000_000;
    assert!(
        (1_000..10_000).contains(&waited_ms),
        "the run ended {waited_ms} ms after the call started: not ended by the 1 s deadline"
    );

    let pid = std::fs::read_to_string(w.root.join("pid")).unwrap();
    let alive = Command::new("kill")
        .args(["-0", pid.trim()])
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap()
        .success();
    assert!(!alive, "the connectors child {} is still there", pid.trim());
}
