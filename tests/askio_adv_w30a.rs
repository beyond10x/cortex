//! Adversary cases for unit `askio` of wave 20261030a, `story:ask-records-deadline`: the deadline
//! of a record call is counted from the call's entry, its wait for the records mutex included,
//! and a call past it stops the records process. These cases hold that a process is stopped for
//! a stall, not for a queue.

mod common;

use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use common::executable;
use cortex_cli::ask::records::postgres::{Launch, Remote, Sql};
use cortex_cli::ask::records::SCHEMA_VERSION;
use serde_json::Value;

/// The deadline of one record call in these cases; the service's is 30 s.
const DEADLINE: Duration = Duration::from_secs(1);
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

/// A stand-in `connectors` launch: it logs the launch, takes its number `$n` (1 for the first),
/// prints the ready line and answers each `query` with `on_query`, a `/bin/sh` fragment.
fn records(root: &Path, on_query: &str) -> Launch {
    let log = root.join("launches.log");
    executable(
        &root.join("connectors"),
        &format!(
            r#"printf 'launch\n' >> "{log}"
n=$(wc -l < "{log}" | tr -d ' ')
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
    launch_at(root)
}

/// The launch of the stand-in `connectors` in `root`.
fn launch_at(root: &Path) -> Launch {
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

fn launches(root: &Path) -> usize {
    std::fs::read_to_string(root.join("launches.log"))
        .unwrap_or_default()
        .lines()
        .count()
}

/// One query's rows, or its error's message.
type Answer = Result<Vec<Vec<Value>>, String>;

/// `n` queries made at once on `remote`: each one's result and how long it took.
fn at_once(remote: &Arc<Remote>, n: usize) -> Vec<(Answer, Duration)> {
    let rxs: Vec<_> = (0..n)
        .map(|_| {
            let r = remote.clone();
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || {
                let started = Instant::now();
                let result = r.query("SELECT 1", &[]).map_err(|e| e.message.clone());
                let _ = tx.send((result, started.elapsed()));
            });
            rx
        })
        .collect();
    rxs.into_iter()
        .enumerate()
        .map(|(i, rx)| {
            rx.recv_timeout(BLOCKED)
                .unwrap_or_else(|_| panic!("call {i}: still blocked after {BLOCKED:?}"))
        })
        .collect()
}

/// The story is about a database that *stalls*. A records process that is healthy and answers
/// every statement well within the deadline must not be stopped because a caller spent its
/// deadline waiting for the records mutex behind other callers: that kills a working process
/// (and its statement in flight) and makes the next caller pay a start and a migration, which
/// under load are again charged against a deadline already partly spent in the queue.
#[test]
fn a_healthy_records_process_is_not_stopped_because_a_caller_queued_behind_others() {
    let tmp = scratch("cortex-askio-adv");
    // Each statement takes 0.6 s: well inside the 1 s deadline on its own.
    let launch = records(tmp.path(), r#"sleep 0.6; printf '{"rows":[[1]]}\n'"#);
    let remote = Arc::new(Remote::start(launch).expect("the stand-in starts"));
    let results = at_once(&remote, 2);
    let ok = results.iter().filter(|(r, _)| r.is_ok()).count();
    assert!(ok >= 1, "no call was answered: {results:?}");
    // A stopped process is started again by the next call, which the log then shows.
    let next = remote.query("SELECT 1", &[]);
    assert!(next.is_ok(), "the call after them: {next:?}");
    assert_eq!(
        launches(tmp.path()),
        1,
        "a records process that answered each statement in 0.6 s (deadline {DEADLINE:?}) was \
         stopped and started again; results: {results:?}"
    );
}

/// One stall needs one restart. Callers that queued behind the stalled call reach the mutex
/// with only the rest of their deadline; a start begun with that sliver must not be killed
/// half-way (in the service: a Connectors launch, a credential read and a migration, each
/// wasted), or every queued caller kills the start the one before it began.
///
/// Launch 1 stalls on its first statement. Each later launch takes 0.5 s to print its ready
/// line, as a start through Connectors with a migration does, and then answers at once.
#[test]
fn a_stall_costs_one_restart_not_one_per_caller_queued_behind_it() {
    let tmp = scratch("cortex-askio-adv");
    let log = tmp.path().join("launches.log");
    executable(
        &tmp.path().join("connectors"),
        &format!(
            r#"printf 'launch\n' >> "{log}"
n=$(wc -l < "{log}" | tr -d ' ')
[ "$n" = 1 ] || sleep 0.5
printf '{{"ready":{{"schema_version":{SCHEMA_VERSION}}}}}\n'
while IFS= read -r line; do
  case "$line" in
    *'"op":"query"'*) if [ "$n" = 1 ]; then printf '{{"rows":[["late'; exec sleep 30; else printf '{{"rows":[[%s]]}}\n' "$n"; fi ;;
  esac
done
"#,
            log = log.display(),
        ),
    );
    let remote = Arc::new(Remote::start(launch_at(tmp.path())).expect("the stand-in starts"));
    // The stalled call, then two callers arriving 0.3 s and 0.4 s into it.
    let mut rxs = Vec::new();
    for wait in [0, 300, 100] {
        std::thread::sleep(Duration::from_millis(wait));
        let r = remote.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(r.query("SELECT 1", &[]).map_err(|e| e.message.clone()));
        });
        rxs.push(rx);
    }
    let results: Vec<_> = rxs
        .into_iter()
        .map(|rx| rx.recv_timeout(BLOCKED).expect("a call ends"))
        .collect();
    for rows in results.iter().flatten() {
        assert!(
            rows[0][0].as_u64().is_some_and(|n| n >= 2),
            "an answer of the stopped process was read: {rows:?}"
        );
    }
    // The call after them, with its whole deadline.
    let rows = remote
        .query("SELECT 1", &[])
        .expect("a new process answers");
    assert_eq!(
        launches(tmp.path()),
        2,
        "one stall, {} launches; the call after them was answered by launch {}; the queued \
         callers: {results:?}",
        launches(tmp.path()),
        rows[0][0]
    );
}
