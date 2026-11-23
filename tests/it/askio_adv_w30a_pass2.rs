//! Adversary cases, pass 2, for unit `askio` of wave 20261030a, `story:ask-records-deadline`,
//! against the correction that bounds a record call's three waits each on its own: "a record
//! call waits at most 30 seconds for the calls before it, for a start of the records process and
//! for the answer to its statement" (`spec/domains/ask.yaml`, `website/docs/ask-service.md`).
//!
//! `Remote::call` takes the records mutex with an untimed `lock()` and only compares the clock
//! once it holds it, so the wait for earlier calls is bounded by however long they hold the
//! mutex, never by the deadline.

use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use crate::common::executable;
use cortex_cli::ask::records::postgres::{Launch, Remote, Sql};
use cortex_cli::ask::records::SCHEMA_VERSION;
use serde_json::Value;

/// The deadline of each wait of a record call in this case; the service's is 30 s. A call held
/// longest holds the records process for a start and a statement, each bounded by it, while the
/// call behind it waits one deadline: the case puts half a deadline between the two ends, so
/// the order of the ends does not turn on how loaded the machine is.
const DEADLINE: Duration = Duration::from_secs(3);
/// How long a case waits for a call or a file before it calls it blocked.
const BLOCKED: Duration = Duration::from_secs(20);

fn scratch(prefix: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(prefix)
        .tempdir_in(
            std::env::var_os("CARGO_TARGET_TMPDIR")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir),
        )
        .expect("a scratch directory")
}

/// A stand-in `connectors` launch in `root`: it logs the launch, takes its number `$n` (1 for
/// the first), runs `before_ready`, prints the ready line and answers each `query` with
/// `on_query`; both are `/bin/sh` fragments.
fn records(root: &Path, before_ready: &str, on_query: &str) -> Launch {
    let log = root.join("launches.log");
    executable(
        &root.join("connectors"),
        &format!(
            r#"printf 'launch\n' >> "{log}"
n=$(wc -l < "{log}" | tr -d ' ')
{before_ready}
printf '{{"ready":{{"schema_version":{SCHEMA_VERSION}}}}}\n'
while IFS= read -r line; do
  case "$line" in
    *'"op":"query"'*) {on_query} ;;
  esac
done
"#,
            log = log.display(),
        ),
    );
    Launch {
        connectors: root.join("connectors"),
        adapter: "postgres".into(),
        connection: "records".into(),
        url: "postgres://cortex@db.invalid/brain".into(),
        schema: "cortex_ask".into(),
        ca_file: None,
        remove: Vec::new(),
        deadline: DEADLINE,
    }
}

type Answer = Result<Vec<Vec<Value>>, String>;

/// A query on `remote` on a thread of its own; the receiver yields its result and its duration.
fn query(remote: &Arc<Remote>) -> mpsc::Receiver<(Answer, Duration)> {
    let r = remote.clone();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let started = Instant::now();
        let result = r.query("SELECT 1", &[]).map_err(|e| e.message.clone());
        let _ = tx.send((result, started.elapsed()));
    });
    rx
}

/// Waits until `ready` holds, and fails the case when it still does not after [`BLOCKED`].
fn until(what: &str, ready: impl Fn() -> bool) {
    let started = Instant::now();
    while !ready() {
        assert!(started.elapsed() < BLOCKED, "{what}: not after {BLOCKED:?}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// A database that stalls stalls the start too. The call before ours starts the records process
/// and then waits out the deadline on its statement, so it holds the process for its start and a
/// whole deadline after it. Ours is made while that start runs and, by the correction's own
/// words, waits at most one deadline for it: it fails as "waited … behind earlier ones" while the
/// call before it still holds the process. Under the untimed `lock()` ours could not end before
/// the call before it released the mutex. In the service: a vote, a review or a close answers
/// `store-failed` after about 60 s, not the 30 s the ask-service page states.
///
/// Launch 1 is ready at once and ends, so the first call finds the process gone. Launch 2 starts
/// when the case lets it: half a deadline after ours is queued. So the call before ours ends a
/// deadline and a half after ours was queued, and ours one deadline after; the start ends half a
/// deadline inside its own deadline. Nothing is compared with a bound on the time but the
/// deadline itself, from below.
#[test]
fn a_call_behind_a_start_and_a_stall_waits_at_most_one_deadline_for_it() {
    let tmp = scratch("cortex-askio-adv2");
    let go = tmp.path().join("go");
    // Launch 1 ends at its first query; every later one waits for `go` to start and answers no
    // query: the database stalls.
    let launch = records(
        tmp.path(),
        &format!(
            r#"[ "$n" = 1 ] || while [ ! -e "{}" ]; do sleep 0.01; done"#,
            go.display()
        ),
        r#"[ "$n" = 1 ] && exit 0"#,
    );
    let remote = Arc::new(Remote::start(launch).expect("the stand-in starts"));
    // The process is found gone once, and the next call starts another.
    let first = remote.query("SELECT 1", &[]);
    assert!(first.is_err(), "a process that ended answered: {first:?}");
    let launches = || {
        std::fs::read_to_string(tmp.path().join("launches.log"))
            .unwrap_or_default()
            .lines()
            .count()
    };
    let before = query(&remote);
    until("the start of the call before ours", || launches() == 2);
    let ours = query(&remote);
    until("our call queued", || remote.waiting() == 1);
    std::thread::sleep(DEADLINE / 2);
    std::fs::write(&go, "").unwrap();
    let (answer, took) = ours.recv_timeout(BLOCKED).expect("our call ends");
    let before_still_held = before.try_recv().is_err();
    let (before_answer, _) = before.recv_timeout(BLOCKED).expect("the call before ends");
    assert!(
        before_answer
            .as_ref()
            .is_err_and(|e| e.contains("did not answer")),
        "the call before ours did not start the process and stall on its statement: \
         {before_answer:?}"
    );
    assert!(
        answer
            .as_ref()
            .is_err_and(|e| e.contains("behind earlier ones")),
        "our call did not fail waiting for the one before it: {answer:?}"
    );
    assert!(took >= DEADLINE, "our call gave up after {took:?}");
    assert!(
        before_still_held,
        "our call ended after {took:?}, after the call before it let the process go: it waited \
         for it past its deadline of {DEADLINE:?}"
    );
}
