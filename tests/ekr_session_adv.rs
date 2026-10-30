//! Adversarial cases for `story:ekr-session-per-run`: how a run behaves when its one `ekr session`
//! does not open, never answers or ends part-way, and whether the run still reads the store once
//! (`story:one-store-read-per-run`) when its reads are requests rather than processes.
//!
//! A stand-in `ekr` in front of the real one logs each process it starts and switches by files in
//! the world's root: `store-down` makes every store verb, `session` included, fail as a store
//! that does not open; `session-hangs` makes `ekr session` read its requests and never answer;
//! `session-dies-after` holds a count of requests the session answers before its input ends.
//! Every process a case starts is waited for with a bound.

mod common;

use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use common::{ekr, executable, World};
use serde_json::Value;

/// The stand-in `claude`: one `Service` entity named by the next record's id.
const CLAUDE: &str = r#"R="@ROOT@"
cat > "$R/prompt.cur"
echo call >> "$R/claude-calls.log"
N=x
if grep -q '#beta' "$R/prompt.cur"; then N=gamma; elif grep -q '#alpha' "$R/prompt.cur"; then N=beta; fi
cat <<EOF
{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{"format":"ekr.extraction-document/1","ontology":{"node_types":[{"name":"Service","parents":[],"abstract_type":false,"properties":[]}],"edge_types":[]},"entities":[{"node_type":"Service","aliases":["$N"]}],"facts":[]}}
EOF
"#;

/// The stand-in `ekr`, switched by files in the world's root as the module says. The session's
/// pid (the shell that runs it) is written to `session.pid`, its requests to `ekr-session.log`.
const EKR: &str = r#"R="@ROOT@"; EKR="@EKR@"
printf '%s\n' "$*" >> "$R/ekr-argv.log"
if [ -e "$R/store-down" ]; then
  case "$1" in
    session|head|ontology|snapshot|quality|propose|validate|commit|apply-extraction|stage)
      echo "ekr: the store does not open: connection refused" >&2; exit 1 ;;
  esac
fi
if [ "$1" = session ]; then
  echo $$ > "$R/session.pid"
  if [ -e "$R/session-hangs" ]; then touch "$R/session-started"; cat > /dev/null; exit 0; fi
  if [ -e "$R/session-dies-after" ]; then
    N=$(cat "$R/session-dies-after"); i=0
    while [ "$i" -lt "$N" ] && IFS= read -r l; do printf '%s\n' "$l"; i=$((i+1)); done \
      | tee -a "$R/ekr-session.log" | "$EKR" "$@"
    exit 0
  fi
  tee -a "$R/ekr-session.log" | "$EKR" "$@"; exit $?
fi
exec "$EKR" "$@"
"#;

/// A world with instance `s` on a `files` source `repos` of three records, `alpha`, `beta` and
/// `gamma`, each of about 800 characters, so a `batch_chars` of 1,000 makes three batches. `extra`
/// is appended to the spec (a `redaction` or `gate` block). The logs are emptied after `create`.
fn world(extra: &str) -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(&w.bin.join("claude"), &CLAUDE.replace("@ROOT@", &root));
    let stand_in = w.bin.join("ekr");
    executable(
        &stand_in,
        &EKR.replace("@ROOT@", &root)
            .replace("@EKR@", &ekr().display().to_string()),
    );
    std::fs::create_dir_all(w.root.join("repos")).unwrap();
    let records: String = ["alpha", "beta", "gamma"]
        .iter()
        .map(|id| {
            let text = format!("The {id} service runs part of billing. ").repeat(20);
            format!("{{\"id\": \"{id}\", \"text\": \"{text}\"}}\n")
        })
        .collect();
    std::fs::write(w.root.join("repos/list.jsonl"), records).unwrap();
    let spec = w.root.join("s.yaml");
    std::fs::write(
        &spec,
        format!(
            "format: cortex.instance/1\nname: s\ndescription: Test brain.\n\
             ekr: {{version: \"0.0.34\", bin: \"{bin}\"}}\nseed: {{documents: []}}\n\
             model: {{model: claude-haiku-4-5-20251001, budget_usd: \"1\", timeout_s: 60, \
             batch_chars: 1000}}\n\
             sources:\n  - name: repos\n    schedule: daily\n    settings:\n      kind: files\n      \
             value:\n        paths: [repos]\n        glob: \"*.jsonl\"\n        records: \
             {{format: JsonLines, id: id, text: [\"{{text}}\"], filters: []}}\n    \
             policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, \
             max_chars_per_document: 5000}}\n\
             serve: {{view_port: 18992}}\n{extra}",
            bin = stand_in.display()
        ),
    )
    .unwrap();
    let (code, created) = w.cortex(&[
        "create",
        "--spec",
        spec.to_str().unwrap(),
        "--no-units",
        "--no-extract",
    ]);
    assert_eq!(code, 0, "{created}");
    for log in ["ekr-argv.log", "ekr-session.log", "claude-calls.log"] {
        let _ = std::fs::remove_file(w.root.join(log));
    }
    w
}

/// `cortex run s/repos`, started and not waited for.
fn start(w: &World) -> Child {
    Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["run", "s/repos"])
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

/// Waits for `child` at most `bound`: its exit status and the last JSON line it printed, or
/// `None` (the child killed) when it was still running at the bound.
fn finish(mut child: Child, bound: Duration) -> Option<(ExitStatus, Value, String)> {
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() > bound {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let (mut out, mut err) = (String::new(), String::new());
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut out)
        .unwrap();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut err)
        .unwrap();
    let last = out
        .lines()
        .last()
        .and_then(|l| serde_json::from_str(l).ok())
        .unwrap_or(Value::Null);
    Some((status, last, err))
}

/// Waits at most 30 s for `path` to exist.
fn wait_for(path: &Path) {
    let until = Instant::now() + Duration::from_secs(30);
    while !path.exists() {
        assert!(Instant::now() < until, "{} never appeared", path.display());
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Whether the process `pid` is gone (or a zombie nobody reaped), waiting 5 s at most.
fn gone(pid: &str) -> bool {
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap_or_default();
        let state = stat
            .rsplit_once(") ")
            .and_then(|(_, rest)| rest.chars().next());
        if stat.is_empty() || state == Some('Z') {
            return true;
        }
        if Instant::now() > until {
            return false;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// The verb of each request the run's session was sent.
fn requests(w: &World) -> Vec<String> {
    w.lines("ekr-session.log")
        .iter()
        .map(|l| {
            let request: Value = serde_json::from_str(l).expect("a request line");
            request["argv"][0].as_str().unwrap_or_default().to_string()
        })
        .collect()
}

/// A store that does not open when a model run starts is a failure of the store
/// (`apply-refused`, `Failure::Apply`), as it was when the run's first store read was a one-shot
/// `ekr ontology`. The model was never asked, so it is not `extraction-failed`, which
/// `spec/domains/instance.yaml` (`cortex.instance.ExtractionFailed`) reserves for "the model call
/// failed or returned no valid extraction document". The session's first request is now the
/// extraction schema, mapped to `Failure::Extract` (`src/run.rs`, `session.model_schema()`), so a
/// session that could not open its store is reported as a model failure.
#[test]
fn a_store_that_does_not_open_when_a_run_starts_is_a_store_failure_not_an_extraction_failure() {
    let w = world("");
    std::fs::write(w.root.join("store-down"), "").unwrap();

    let (code, ran) = w.cortex(&["run", "s/repos"]);
    assert_ne!(code, 0, "{ran}");
    assert_eq!(
        w.lines("claude-calls.log").len(),
        0,
        "the model was not asked: {ran}"
    );
    assert_eq!(
        ran["outcome"], "apply-refused",
        "a store that does not open is the store's failure, not the model's: {ran}"
    );
}

/// A run whose session never answers a request stops at SIGTERM, as a run waiting for the model
/// does (`src/terminate.rs`: a run sent SIGTERM "stops at its next safe point" and exits 8). A
/// session request has no deadline and is not a safe point (`Session::request` blocks in
/// `read_answer`), so the first SIGTERM is never acted on and the run stays blocked until it is
/// killed.
#[test]
fn a_run_blocked_on_a_session_that_never_answers_stops_on_sigterm() {
    let w = world("");
    std::fs::write(w.root.join("session-hangs"), "").unwrap();

    let child = start(&w);
    wait_for(&w.root.join("session-started"));
    std::thread::sleep(Duration::from_millis(500));
    let pid = child.id().to_string();
    let sent = Command::new("kill").args(["-TERM", &pid]).status().unwrap();
    assert!(sent.success());

    let ended = finish(child, Duration::from_secs(20));
    let session = std::fs::read_to_string(w.root.join("session.pid")).unwrap();
    assert!(gone(session.trim()), "the hung session outlived the run");
    let (status, ran, err) = ended
        .expect("the run, blocked on its session's answer, did not end within 20 s of SIGTERM");
    assert_eq!(status.code(), Some(8), "{ran}\n{err}");
    assert_eq!(ran["outcome"], "terminated", "{ran}\n{err}");
}

/// A session that ends part-way (its input cut after four requests, so it exits 0 between two
/// requests) fails the run as the store failing, says so in the reason, and leaves no session
/// process behind.
#[test]
fn a_session_that_ends_part_way_fails_the_run_as_the_store_and_leaves_no_process() {
    let w = world("");
    std::fs::write(w.root.join("session-dies-after"), "4").unwrap();

    let ended = finish(start(&w), Duration::from_secs(60)).expect("the run ended");
    let (status, ran, err) = ended;
    assert_ne!(status.code(), Some(0), "{ran}\n{err}");
    assert_eq!(ran["outcome"], "apply-refused", "{ran}\n{err}");
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("ekr session"), "{reason}");
    let session = std::fs::read_to_string(w.root.join("session.pid")).unwrap();
    assert!(gone(session.trim()), "the session outlived the run");
}

/// `story:one-store-read-per-run`, acceptance 1, as requests: a three-batch run under a
/// `refuse_if_left` policy, whose pre-check and batch loop both look up each batch's known
/// entities, asks its session for one `snapshot`. `tests/one_store_read.rs` counts `ekr snapshot`
/// processes, of which a run now starts none, so it holds whatever the run asks its session.
#[test]
fn a_three_batch_run_under_refuse_if_left_asks_its_session_for_one_snapshot() {
    let w = world("redaction:\n  classes: [Phone]\n  rules: []\n  refuse_if_left: [Phone]\n");

    let (code, ran) = w.cortex(&["run", "s/repos"]);
    assert_eq!(code, 0, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 3, "{ran}");
    assert_eq!(w.lines("claude-calls.log").len(), 3, "three batches: {ran}");

    let sessions = w
        .lines("ekr-argv.log")
        .iter()
        .filter(|l| l.split(' ').next() == Some("session"))
        .count();
    assert_eq!(sessions, 1, "one session per run");
    let requests = requests(&w);
    assert!(
        requests.iter().any(|v| v == "ontology"),
        "the run's reads went to its session: {requests:#?}"
    );
    let snapshots = requests.iter().filter(|v| *v == "snapshot").count();
    assert_eq!(snapshots, 1, "one snapshot per run: {requests:#?}");
}
