//! Adversary cases, pass 2, for unit `askio` of wave 20261030a, `story:ask-records-deadline`,
//! against the correction that bounds a record call's three waits each on its own: "a record
//! call waits at most 30 seconds for the calls before it, for a start of the records process and
//! for the answer to its statement" (`spec/domains/ask.yaml`, `website/docs/ask-service.md`).
//!
//! `Remote::call` takes the records mutex with an untimed `lock()` and only compares the clock
//! once it holds it, so the wait for earlier calls is bounded by however long they hold the
//! mutex, never by the deadline.

mod common;

use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use common::executable;
use cortex_cli::ask::records::postgres::{Launch, Remote, Sql};
use cortex_cli::ask::records::SCHEMA_VERSION;
use serde_json::Value;

/// The deadline of each wait of a record call in these cases; the service's is 30 s.
const DEADLINE: Duration = Duration::from_secs(1);
/// What a call that failed in the queue may take past its deadline: taking the clock and
/// answering.
const SLACK: Duration = Duration::from_millis(400);
/// How long a case waits for a call before it calls the call blocked.
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

/// A database that stalls stalls the start too. The call before ours starts the records process
/// (0.9 s, inside its deadline) and then waits out the deadline on its statement, so it holds the
/// mutex for 1.9 s. Ours arrived 0.1 s after it and, by the correction's own words, waits at
/// most one deadline for it; it waits 1.8 s, and only then fails as "waited 1s behind earlier
/// ones". In the service: a vote, a review or a close answers `store-failed` after about 60 s,
/// not the 30 s the ask-service page states.
#[test]
fn a_call_behind_a_start_and_a_stall_waits_at_most_one_deadline_for_it() {
    let tmp = scratch("cortex-askio-adv2");
    // Launch 1 is ready at once; every later launch takes 0.9 s to start. No launch answers a
    // query: the database stalls.
    let launch = records(tmp.path(), r#"[ "$n" = 1 ] || sleep 0.9"#, ":");
    let remote = Arc::new(Remote::start(launch).expect("the stand-in starts"));
    // The stall is found once: the process is stopped and the next call starts another.
    let first = remote.query("SELECT 1", &[]);
    assert!(first.is_err(), "a stalled process answered: {first:?}");
    let before = query(&remote);
    std::thread::sleep(Duration::from_millis(100));
    let (ours, took) = query(&remote).recv_timeout(BLOCKED).expect("our call ends");
    let _ = before.recv_timeout(BLOCKED);
    assert!(
        took < DEADLINE + SLACK,
        "a call that arrived 0.1 s after the one before it waited {took:?} for it, past the \
         deadline of {DEADLINE:?} for the calls before it; it answered {ours:?}"
    );
}
