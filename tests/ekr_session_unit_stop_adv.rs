//! Adversarial case (pass 2) for `story:ekr-session-per-run`, against the correction that lets a
//! run's SIGTERM end a wait for its session's answer to a read (`src/ekr.rs`, `Answers`).
//!
//! A stop of the whole unit (`systemctl stop`, which signals every process of the unit's cgroup)
//! sends SIGTERM to the run and to its `ekr session` at once. `src/terminate.rs` already treats a
//! child "that ended after the run was sent SIGTERM" as ended "because of it, whatever its status
//! says". A session that dies of that SIGTERM while the run waits for its answer to a read closes
//! its output; `Answers::read` answers end of file on `Disconnected` without looking at the run's
//! SIGTERM, so the read fails as "the run's ekr session ended", which `store_failure` maps to
//! `Failure::Apply`. The run ends `apply-refused` instead of `terminated`, and a gated run is not
//! undone: the batch it applied before the stop stays in the store.
//!
//! The stand-in `ekr` models the stop: once a batch was applied (`applied` exists), the session,
//! on its next request, sends the run SIGTERM and then SIGTERM to its own process group (the
//! session runs in a group of its own), so it dies with the run's SIGTERM in flight, as the unit's
//! processes do.

mod common;

use std::io::Read;
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

/// The stand-in `ekr`. While `unit-stop` or `hang-after-apply` exists: an `apply-extraction` that
/// ran writes `applied`; the session forwards its requests to the real `ekr session` until
/// `applied` exists, then logs the request it got to `stopped-at.log` and, under `unit-stop`, sends
/// the run (its parent) SIGTERM and SIGTERM to its own process group; under `hang-after-apply` it
/// writes `session-hung` and never answers.
const EKR: &str = r#"R="@ROOT@"; EKR="@EKR@"
printf '%s\n' "$*" >> "$R/ekr-argv.log"
if { [ -e "$R/unit-stop" ] || [ -e "$R/hang-after-apply" ]; } && [ "$1" = apply-extraction ]; then
  "$EKR" "$@"; s=$?; touch "$R/applied"; exit $s
fi
if { [ -e "$R/unit-stop" ] || [ -e "$R/hang-after-apply" ]; } && [ "$1" = session ]; then
  RUN=$PPID
  while IFS= read -r l; do
    if [ -e "$R/applied" ] && [ -e "$R/hang-after-apply" ]; then
      printf '%s\n' "$l" >> "$R/stopped-at.log"
      touch "$R/session-hung"; cat > /dev/null; exit 0
    fi
    if [ -e "$R/applied" ]; then
      printf '%s\n' "$l" >> "$R/stopped-at.log"
      kill -TERM "$RUN"
      kill -TERM 0
      exit 0
    fi
    printf '%s\n' "$l"
  done | "$EKR" "$@"
  exit 0
fi
exec "$EKR" "$@"
"#;

/// A world with instance `s` on a `files` source `repos` of three records, `alpha`, `beta` and
/// `gamma`, each of about 800 characters, so a `batch_chars` of 1,000 makes three batches, held
/// to a gate any of its runs passes.
fn world() -> World {
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
             serve: {{view_port: 18994}}\n\
             gate:\n  checks:\n    - {{measure: facts_refused, max: \"0\"}}\n",
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

/// Waits for `child` at most `bound`: its exit status, the last JSON line it printed and its
/// stderr, or `None` (the child killed) when it was still running at the bound.
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

/// A gated run whose unit is stopped while it waits for its session's answer to a read, after it
/// applied its first batch, ends `terminated` (exit 8) and is undone, as the correction says of a
/// run sent SIGTERM while it waits on a read ("the run stops `terminated`, undone when gated") and
/// as `src/terminate.rs` says of a child the unit's stop ended.
#[test]
fn a_gated_run_whose_unit_is_stopped_during_a_session_read_ends_terminated_and_undone() {
    let w = world();
    let head = w.head("s");
    std::fs::write(w.root.join("unit-stop"), "").unwrap();

    let (status, ran, err) =
        finish(start(&w), Duration::from_secs(60)).expect("the run ended within 60 s");

    assert!(
        w.root.join("applied").exists(),
        "a batch was applied before the stop: {ran}\n{err}"
    );
    let stopped_at = w.lines("stopped-at.log");
    let verb = stopped_at
        .first()
        .and_then(|l| serde_json::from_str::<Value>(l).ok())
        .and_then(|r| r["argv"][0].as_str().map(str::to_string))
        .unwrap_or_default();
    assert!(
        ["ontology", "snapshot", "schema", "quality", "head"].contains(&verb.as_str()),
        "the stop came while the run waited on a read, not `{verb}`"
    );
    assert_eq!(
        (status.code(), ran["outcome"].as_str(), w.head("s") == head),
        (Some(8), Some("terminated"), true),
        "a gated run its unit's stop ended while it waited on `{verb}` ends terminated with the \
         store at its head from before the run ({head}): {ran}\n{err}"
    );
}

/// The path the correction covers, gated: SIGTERM to the run alone (a Kubernetes Job's deadline)
/// while its session hangs on a read after a batch was applied ends the session's process group,
/// and the run ends `terminated` (exit 8), undone to its head from before the run, with no session
/// process left holding the store.
#[test]
fn a_gated_run_sent_sigterm_while_its_session_hangs_on_a_read_is_undone_and_leaves_no_session() {
    let w = world();
    let head = w.head("s");
    std::fs::write(w.root.join("hang-after-apply"), "").unwrap();

    let child = start(&w);
    let hung = w.root.join("session-hung");
    let until = Instant::now() + Duration::from_secs(60);
    while !hung.exists() {
        assert!(Instant::now() < until, "the session never hung");
        std::thread::sleep(Duration::from_millis(20));
    }
    std::thread::sleep(Duration::from_millis(300));
    let pid = child.id().to_string();
    assert!(Command::new("kill")
        .args(["-TERM", &pid])
        .status()
        .unwrap()
        .success());

    let (status, ran, err) =
        finish(child, Duration::from_secs(30)).expect("the run ended within 30 s of SIGTERM");
    assert_eq!(
        (status.code(), ran["outcome"].as_str(), w.head("s") == head),
        (Some(8), Some("terminated"), true),
        "{ran}\n{err}"
    );
    let left = Command::new("pgrep")
        .args(["-f", &format!("{} session", w.bin.join("ekr").display())])
        .output()
        .unwrap();
    assert!(
        left.stdout.is_empty(),
        "a session process outlived the run: {}",
        String::from_utf8_lossy(&left.stdout)
    );
}
