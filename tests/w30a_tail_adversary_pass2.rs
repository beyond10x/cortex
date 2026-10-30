//! Adversarial cases, pass 2, for unit `tail` of wave 20261030a, against the correction of pass 1
//! (D34 to D39). Each case drives the built binary and asserts what the unit's documents promise:
//!
//! - `{since_minutes_ago}` is filled "at the moment each call is made", "so the window is never
//!   narrowed" (`website/docs/spec-file.md`), also when the call is made again after a renewal.
//! - A run sent SIGTERM during its fetch stops "at once" (`website/docs/operating.md`, "Stopping a
//!   run"), also while the `connectors` child has exited and the deadline loop of D35 waits for
//!   its output.
//! - A session's stderr is masked before it is cut (D34), also when the cut of the 1 MiB hold
//!   drops the line that marks a credential and keeps the lines that hold it.

mod common;

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use common::{ekr, executable, World, LISTED};

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

fn start(w: &World, args: &[&str], env: &[(&str, &str)]) -> Child {
    Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(args)
        .envs(env.iter().copied())
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

/// Waits for `child` at most `limit` (killing it then): its exit code, or `None`, and what it
/// printed on stdout and stderr.
fn finish(mut child: Child, limit: Duration) -> (Option<i32>, String, String) {
    let pipe = |mut p: Box<dyn Read + Send>| {
        std::thread::spawn(move || {
            let mut s = Vec::new();
            let _ = p.read_to_end(&mut s);
            String::from_utf8_lossy(&s).into_owned()
        })
    };
    let out = pipe(Box::new(child.stdout.take().unwrap()));
    let err = pipe(Box::new(child.stderr.take().unwrap()));
    let started = Instant::now();
    let code = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status.code();
        }
        if started.elapsed() > limit {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    (code, out.join().unwrap(), err.join().unwrap())
}

fn wait_for(path: &Path) {
    let until = Instant::now() + Duration::from_secs(60);
    while !path.exists() {
        assert!(Instant::now() < until, "{} never appeared", path.display());
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Every file under `dir` whose bytes contain `needle`.
fn holding(dir: &Path, needle: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            found.extend(holding(&path, needle));
        } else if kind.is_file()
            && std::fs::read(&path).is_ok_and(|b| String::from_utf8_lossy(&b).contains(needle))
        {
            found.push(path);
        }
    }
    found
}

// ---------------------------------------------------------------------------------------------
// D36: `{since_minutes_ago}` filled per call, and a call made again.

/// A tracker with one ready connection. Its first `issues.list` invocation is refused as a lapse
/// (`not_granted` at admission); `connections revalidate` takes 5 s, as a renewal over the network
/// can; every invocation logs `<unix ms> <argv> <input>`, and the second answers no issue.
const LAPSING_TRACKER: &str = r#"R="@ROOT@"
IN=""; if [ "$1" = invoke ]; then IN=$(cat); fi; A="$* $IN"
case "$A" in
  *"connections list"*) echo '{"ok":true,"result":{"connections":[{"adapter":"tracker","connector":"tracker","instance":"main","connection":"conn_test","state":"ready","revision":"rev1"}]}}' ;;
  *"connections revalidate"*) sleep 5; echo '{"ok":true,"result":{}}' ;;
  "invoke "*" issues.list "*)
    echo "$(date +%s%3N) $A" >> "$R/issues.log"
    if [ ! -e "$R/lapsed" ]; then
      touch "$R/lapsed"
      echo '{"ok":false,"error":{"code":"failure","data":{"code":"not_granted","stage":"admission"}}}' >&2
      exit 1
    fi
    printf '{"ok":true,"result":{"connector":"tracker","instance":"main","operation":"issues.list","revision":"r","result":{"issues":[]}}}\n' ;;
  *) echo '{"ok":false}'; exit 2 ;;
esac
"#;

const OVERLAP_MS: i64 = 5 * 60 * 1000;

/// `Connectors::invoke` serialises its input once and sends the same bytes again after it renewed
/// a lapsed connection (`src/connectors.rs` `invoke`, the `Lapsed` arm), so the minutes `at_call`
/// filled before the renewal are sent with the retry made after it: the retry asks from later than
/// `{since}` by the renewal's time less the rounding slack (held under 3 s by the planted last
/// start, as in pass 1). The same holds for the retry of a timeout at admission and for the
/// revalidation pauses before the first invocation.
#[test]
fn adv2_since_minutes_ago_still_reaches_since_when_the_call_is_retried_after_a_renewal() {
    let w = World::new();
    executable(
        &w.bin.join("connectors"),
        &LAPSING_TRACKER.replace("@ROOT@", &w.root.display().to_string()),
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
          - {{jql: "updated >= -{{since_minutes_ago}}m"}}
        records: issues
        id: key
        text: ["{{key}}: {{summary}}"]
    policy: {{refresh_after_days: 3, change: ContentHash, max_documents_per_run: 50, max_chars_per_document: 5000}}
serve: {{view_port: 18994}}
"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");

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
    assert_eq!(log.len(), 2, "a lapse and its retry: {log:?}");
    let retry = &log[1];
    let invoked_ms: i64 = retry.split(' ').next().unwrap().parse().unwrap();
    let minutes: i64 = retry
        .split("updated >= -")
        .nth(1)
        .and_then(|rest| rest.split('m').next())
        .unwrap_or_else(|| panic!("a rendered jql in {retry}"))
        .parse()
        .unwrap_or_else(|_| panic!("a number of minutes in {retry}"));
    let asked_from = invoked_ms - minutes * 60_000;
    assert!(
        asked_from <= since_ms,
        "the retry after the renewal asked from {asked_from} (invoked at {invoked_ms}, \
         -{minutes}m), {} ms after the window's since {since_ms}: the minutes were filled before \
         the renewal and the window was narrowed",
        asked_from - since_ms
    );
}

// ---------------------------------------------------------------------------------------------
// D35: the deadline loop after the child exited, and SIGTERM.

/// A `connectors` whose connection is ready and whose `invoke` answers at once and exits, leaving
/// a process it started (its pid in `grandchild`) holding its stdout for 30 s; `started` marks the
/// moment it exited.
const LEAVES_A_CHILD: &str = r#"R="@ROOT@"
case "$*" in
  *"connections list"*) printf '{"ok":true,"result":{"connections":[{"adapter":"tavily",@LISTED@,"connection":"conn_test","state":"ready","revision":"rev1"}]}}' ;;
  "invoke tavily/main "*) cat > /dev/null; sleep 30 & echo $! > "$R/grandchild"; cat "$R/answer.json"; touch "$R/started" ;;
  *) echo '{"ok":false}'; exit 2 ;;
esac
"#;

/// A run sent SIGTERM while its `connectors` child has exited and a process the child started
/// still holds the output stops at once, as "Stopping a run" promises for the fetch, and exits 8
/// (`terminated`). The loop D35 added after the child's exit (`src/terminate.rs` `wait_with`,
/// `until(deadline)`) reads neither `requested()` nor anything else but the clock, so the run
/// waits for the deadline (60 s here, 120 s by default), past a systemd or Kubernetes grace
/// period, and is then killed with nothing recorded.
#[test]
fn adv2_sigterm_ends_a_connectors_call_whose_child_exited_and_left_its_output_held() {
    let w = World::new();
    executable(
        &w.bin.join("connectors"),
        &LEAVES_A_CHILD
            .replace("@ROOT@", &w.root.display().to_string())
            .replace("@LISTED@", LISTED),
    );
    let spec = w.spec("d", "conn_test");
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");

    let child = start(
        &w,
        &["run", "d/news"],
        &[("CORTEX_CONNECTORS_TIMEOUT_S", "60")],
    );
    wait_for(&w.root.join("started"));
    // The direct child has exited by now; only its leftover holds the pipe.
    std::thread::sleep(Duration::from_millis(500));
    let pid = child.id().to_string();
    assert!(Command::new("kill")
        .args(["-TERM", &pid])
        .status()
        .unwrap()
        .success());
    let sent = Instant::now();
    let (code, stdout, stderr) = finish(child, Duration::from_secs(90));
    let took = sent.elapsed();
    if let Ok(grandchild) = std::fs::read_to_string(w.root.join("grandchild")) {
        let _ = Command::new("kill")
            .args(["-KILL", grandchild.trim()])
            .stderr(Stdio::null())
            .status();
    }
    assert!(
        took < Duration::from_secs(10) && code == Some(8),
        "the run ended {took:?} after SIGTERM with exit {code:?} (8, terminated, expected within \
         10 s): {stdout} {stderr}"
    );
}

// ---------------------------------------------------------------------------------------------
// D34: the 1 MiB hold drops the marking line and keeps the credential's lines.

/// A stand-in `ekr`: with the file `break-session`, `ekr session` writes to stderr one line of
/// 1 MiB of `x` that ends in a private key's `BEGIN` line, then the key's base64 lines, and exits
/// 1; everything else is the real `ekr`.
const BREAKS_SESSION: &str = r#"R="@ROOT@"
if [ "$1" = session ] && [ -e "$R/break-session" ]; then
  {
    head -c 1048576 /dev/zero | tr '\0' x
    printf ' -----BEGIN RSA PRIVATE %s-----\n' KEY
    printf '%s\n' "@BODY1@" "@BODY2@" "@BODY3@"
  } >&2
  exit 1
fi
exec "@EKR@" "$@"
"#;

/// A base64 line of 64 characters, built at runtime so no key-shaped literal sits in the source.
fn body_line(seed: usize) -> String {
    const B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    (0..64)
        .map(|i| B64[(i * 37 + seed * 11 + 5) % 64] as char)
        .collect()
}

/// D34 holds whole lines up to 1 MiB and masks them before the 16 KiB cut, so that "a cut cannot
/// part a credential from its name". A private key is marked by its `BEGIN` line, and its base64
/// lines are masked only after it (`src/mask.rs` `KEY_LINES`). `keep_read` drops the line that
/// ends in `BEGIN` whole once it passes the 1 MiB hold and keeps the key's lines, so `window`
/// masks lines nothing marks any more, and they reach the run's answer and its status file.
#[test]
fn adv2_a_private_key_whose_begin_line_is_dropped_from_the_hold_is_not_kept_unmasked() {
    let w = World::new();
    let bodies = [body_line(1), body_line(2), body_line(3)];
    executable(
        &w.bin.join("ekr"),
        &BREAKS_SESSION
            .replace("@ROOT@", &w.root.display().to_string())
            .replace("@BODY1@", &bodies[0])
            .replace("@BODY2@", &bodies[1])
            .replace("@BODY3@", &bodies[2])
            .replace("@EKR@", &ekr().display().to_string()),
    );
    let spec = {
        let path = w.spec("m", "conn_test");
        let text = std::fs::read_to_string(&path).unwrap().replace(
            &ekr().display().to_string(),
            &w.bin.join("ekr").display().to_string(),
        );
        std::fs::write(&path, text).unwrap();
        path
    };
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    std::fs::write(w.root.join("break-session"), "").unwrap();

    let child = start(&w, &["run", "m/news"], &[]);
    let (code, stdout, stderr) = finish(child, Duration::from_secs(120));
    assert!(code.is_some(), "the run did not end");
    let said = format!("{stdout}\n{stderr}");
    let reached = said.contains("session ended") || !holding(&w.home, "session ended").is_empty();
    assert!(reached, "the run did not report the ended session: {said}");
    let printed = bodies.iter().filter(|b| said.contains(b.as_str())).count();
    let files = holding(&w.home, &bodies[0]);
    assert!(
        printed == 0 && files.is_empty(),
        "the private key's lines are kept unmasked: {printed} of 3 printed, the first written to \
         {files:?}; the answer ends {:?}",
        said.lines().find(|l| l.contains("session ended")).map(|l| {
            let tail: Vec<char> = l.chars().rev().take(300).collect();
            tail.into_iter().rev().collect::<String>()
        })
    );
}
