//! `story:ask-records-deadline`: a records process that stops answering fails each record call
//! within the call's deadline, as `store-failed` on the record routes, and the call after it is
//! not blocked behind it.
//!
//! The records process is a stand-in `connectors` launch that prints the ready line, answers the
//! purge the service makes when it starts, and then reads every statement and answers none, as
//! a records process does whose database stalls. No PostgreSQL is needed.

use std::path::PathBuf;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use crate::common::executable;
use cortex_cli::ask::model::Route;
use cortex_cli::ask::records::postgres::{Launch, PgRecords};
use cortex_cli::ask::records::{Keeper, SCHEMA_VERSION};
use cortex_cli::ask::{ReaderCommand, Settings};
use cortex_model::ask as ma;
use serde_json::Value;

/// The deadline of one record call in these cases; the service's is 30 s.
const DEADLINE: Duration = Duration::from_secs(1);
/// A call to a process that answers nothing ends only through a deadline; one that ended within
/// ten deadlines was ended by its own (or, queued behind a stalled call, by that one's, which
/// makes two). The rest is room for stopping the stalled process and answering on a loaded
/// machine: a call is timed from below by its deadline and from above by this bound only.
const FIRED: Duration = Duration::from_secs(10);
/// The deadline of the cases that order calls: long enough that no call reaches it while the
/// case holds the records process, which these cases release themselves.
const HELD: Duration = Duration::from_secs(10);
/// How long a case waits for a call before it calls the call blocked.
const BLOCKED: Duration = Duration::from_secs(20);
const HEADER: &str = "x-asker";

/// A temporary root holding the stand-in `connectors` and the log of its launches.
struct World {
    _tmp: tempfile::TempDir,
    root: PathBuf,
}

impl World {
    fn new() -> Self {
        let tmp = tempfile::Builder::new()
            .prefix("cortex-records-deadline")
            .tempdir_in(
                std::env::var_os("CARGO_TARGET_TMPDIR")
                    .map(PathBuf::from)
                    .unwrap_or_else(std::env::temp_dir),
            )
            .unwrap();
        let root = tmp.path().to_path_buf();
        // Each launch is logged, prints the ready line, answers a `batch` (the purge) and reads
        // every other statement without answering it.
        executable(
            &root.join("connectors"),
            &format!(
                r#"printf 'launch\n' >> "{log}"
printf '{{"ready":{{"schema_version":{SCHEMA_VERSION}}}}}\n'
while IFS= read -r line; do
  case "$line" in
    *'"op":"batch"'*) printf '{{"counts":[0,0,0]}}\n' ;;
  esac
done
"#,
                log = root.join("launches.log").display(),
            ),
        );
        Self { _tmp: tmp, root }
    }

    fn launch(&self) -> Launch {
        Launch {
            connectors: self.root.join("connectors"),
            adapter: "postgres".into(),
            connection: "records".into(),
            url: "postgres://cortex@db.invalid/brain".into(),
            schema: "cortex_ask".into(),
            ca_file: None,
            remove: Vec::new(),
            deadline: DEADLINE,
        }
    }

    fn keeper(&self) -> Keeper {
        Keeper {
            store: Arc::new(PgRecords::launched(self.launch()).expect("the stand-in starts")),
            instance: "asker".into(),
            retention_days: 30,
            slow_after_ms: None,
        }
    }

    fn launches(&self) -> usize {
        std::fs::read_to_string(self.root.join("launches.log"))
            .unwrap_or_default()
            .lines()
            .count()
    }
}

/// Runs `f` on a thread of its own: what it returned and how long it took, or a failed case when
/// it is still running after [`BLOCKED`].
fn timed<T: Send + 'static>(what: &str, f: impl FnOnce() -> T + Send + 'static) -> (T, Duration) {
    let (tx, rx) = mpsc::channel();
    let started = Instant::now();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    match rx.recv_timeout(BLOCKED) {
        Ok(v) => (v, started.elapsed()),
        Err(_) => panic!("{what}: still blocked after {BLOCKED:?}"),
    }
}

#[test]
fn a_stalled_records_process_fails_a_call_within_the_deadline_and_the_next_call_too() {
    let world = World::new();
    let keeper = world.keeper();
    for call in ["the first call", "the call after it"] {
        let k = keeper.clone();
        let (result, took) = timed(call, move || k.open_reviews());
        assert!(
            result.is_err(),
            "{call}: a records process that answers nothing answered: {result:?}"
        );
        assert!(
            took >= DEADLINE && took < FIRED,
            "{call} took {took:?}: not ended by its deadline of {DEADLINE:?}"
        );
    }
    // The stalled process was stopped at the first deadline and another started for the next.
    assert!(
        world.launches() >= 2,
        "launches: {}; the stalled records process was kept",
        world.launches()
    );
}

/// The records mutex is not held past a call's deadline: two calls made at once both end within
/// their own deadline, the second not one deadline after the first.
#[test]
fn two_calls_at_once_on_a_stalled_records_process_each_end_within_their_deadline() {
    let world = World::new();
    let keeper = world.keeper();
    let calls: Vec<_> = (0..2)
        .map(|i| {
            let k = keeper.clone();
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || {
                let started = Instant::now();
                let result = k.open_reviews();
                let _ = tx.send((result.is_err(), started.elapsed()));
            });
            (i, rx)
        })
        .collect();
    for (i, rx) in calls {
        let (failed, took) = rx
            .recv_timeout(BLOCKED)
            .unwrap_or_else(|_| panic!("call {i}: still blocked after {BLOCKED:?}"));
        assert!(
            failed,
            "call {i}: a records process that answers nothing answered"
        );
        assert!(
            took >= DEADLINE && took < FIRED,
            "call {i} took {took:?}: not ended by a deadline of {DEADLINE:?}"
        );
    }
}

/// A free port on `127.0.0.1`.
fn free_addr() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().to_string()
}

fn settings(listen: &str) -> Settings {
    Settings {
        ask: ma::AskSpec {
            listen: listen.into(),
            identity_header: HEADER.into(),
            limits: ma::AskLimits {
                max_steps: 4,
                max_result_bytes: 4096,
                max_seconds: 30,
                per_identity_per_minute: 60,
                max_tool_calls: None,
            },
            link_templates: None,
            guidance: None,
            retention_days: None,
            slow_after_ms: None,
            records: None,
            max_in_flight: None,
        },
        model: Route {
            base_url: "http://127.0.0.1:9".into(),
            token_env: "CORTEX_RECORDS_DEADLINE_UNSET_TOKEN".into(),
            model: "test-model".into(),
        },
        reader: ReaderCommand {
            program: "/bin/false".into(),
            args: Vec::new(),
            env: Vec::new(),
            remove: Vec::new(),
        },
        system: String::new(),
        redactor: None,
    }
}

/// One record route answered: its status and its JSON body.
fn call(
    client: &reqwest::blocking::Client,
    request: reqwest::blocking::RequestBuilder,
) -> (u16, Value) {
    let response = client
        .execute(request.header(HEADER, "alice").build().unwrap())
        .expect("the service answers before the case's timeout");
    let status = response.status().as_u16();
    let body = response.text().unwrap_or_default();
    (status, serde_json::from_str(&body).unwrap_or(Value::Null))
}

#[test]
fn the_record_routes_answer_store_failed_within_the_deadline_when_the_records_stall() {
    let world = World::new();
    let keeper = world.keeper();
    let addr = free_addr();
    let settings = settings(&addr);
    std::thread::spawn(move || cortex_cli::ask::serve::serve(settings, keeper));
    let started = Instant::now();
    while std::net::TcpStream::connect(&addr).is_err() {
        assert!(
            started.elapsed() < BLOCKED,
            "the service did not listen on {addr}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(BLOCKED)
        .build()
        .unwrap();
    let base = format!("http://{addr}");
    let routes = [
        ("GET /review", client.get(format!("{base}/review"))),
        (
            "POST /ask/{answer_id}/vote",
            client
                .post(format!(
                    "{base}/ask/0190a0a0-0000-7000-8000-000000000000/vote"
                ))
                .header("content-type", "application/json")
                .body(r#"{"value": "up"}"#),
        ),
        ("GET /review again", client.get(format!("{base}/review"))),
    ];
    for (route, request) in routes {
        let started = Instant::now();
        let (status, body) = call(&client, request);
        let took = started.elapsed();
        assert_eq!(status, 500, "{route}: {body}");
        assert_eq!(body["error"], "store-failed", "{route}: {body}");
        assert!(
            took >= DEADLINE && took < FIRED,
            "{route} took {took:?}: not ended by the records' deadline of {DEADLINE:?}"
        );
    }
}

/// Waits until `ready` holds, and fails the case when it still does not after [`BLOCKED`].
fn until(what: &str, ready: impl Fn() -> bool) {
    let started = Instant::now();
    while !ready() {
        assert!(started.elapsed() < BLOCKED, "{what}: not after {BLOCKED:?}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// The lines of `path`, none when it does not exist yet.
fn lines(path: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

/// Record calls are served in the order they arrive. A caller making statements back to back
/// (a vote reads the answer, writes the vote and may open a review item) does not get ahead of a
/// call that was already waiting: that call is served after the statement in progress, not after
/// the whole run of them, and so never fails in the queue behind a healthy process.
///
/// The runner makes 8 statements in a row; the records process holds the first until the case
/// releases it. The waiter's statement is made while the first is held, and the case releases
/// the first only once the waiter is queued behind it, so served in order the waiter's statement
/// is the second the process reads, whatever the machine's load; a caller let in ahead of it
/// again and again puts it after the runner's next one. The process logs each statement as it
/// reads it, and nothing here is timed.
#[test]
fn a_waiting_call_is_served_before_the_next_statement_of_a_caller_making_them_back_to_back() {
    use cortex_cli::ask::records::postgres::{Remote, Sql};
    let tmp = tempfile::Builder::new()
        .prefix("cortex-records-order")
        .tempdir_in(
            std::env::var_os("CARGO_TARGET_TMPDIR")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir),
        )
        .unwrap();
    let root = tmp.path().to_path_buf();
    let (served, go) = (root.join("served.log"), root.join("go"));
    executable(
        &root.join("connectors"),
        &format!(
            r#"printf '{{"ready":{{"schema_version":{SCHEMA_VERSION}}}}}\n'
while IFS= read -r line; do
  case "$line" in
    *'"op":"query"'*)
      printf '%s\n' "$line" >> "{served}"
      while [ ! -e "{go}" ]; do sleep 0.01; done
      printf '{{"rows":[[1]]}}\n' ;;
  esac
done
"#,
            served = served.display(),
            go = go.display(),
        ),
    );
    let launch = Launch {
        connectors: root.join("connectors"),
        adapter: "postgres".into(),
        connection: "records".into(),
        url: "postgres://cortex@db.invalid/brain".into(),
        schema: "cortex_ask".into(),
        ca_file: None,
        remove: Vec::new(),
        deadline: HELD,
    };
    let remote = Arc::new(Remote::start(launch).expect("the stand-in starts"));
    let r = remote.clone();
    let runner = std::thread::spawn(move || {
        (0..8)
            .map(|_| r.query("SELECT 1", &[]).is_ok())
            .collect::<Vec<_>>()
    });
    until("the runner's first statement", || {
        !lines(&served).is_empty()
    });
    let r = remote.clone();
    let (tx, waited) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(r.query("SELECT 2", &[]).map_err(|e| e.message));
    });
    until("the waiting call queued", || remote.waiting() == 1);
    std::fs::write(&go, "").unwrap();
    let waiter = waited.recv_timeout(BLOCKED).expect("the waiting call ends");
    let runs = runner.join().expect("the runner ends");
    assert!(
        waiter.is_ok(),
        "the waiting call failed behind a caller making statements back to back: {waiter:?}"
    );
    assert!(
        runs.iter().all(|ok| *ok),
        "the caller's statements: {runs:?}"
    );
    let order: Vec<&str> = lines(&served)
        .iter()
        .map(|l| {
            if l.contains("SELECT 2") {
                "waiter"
            } else {
                "runner"
            }
        })
        .collect();
    assert_eq!(order.len(), 9, "{order:?}");
    assert_eq!(
        order[..2],
        ["runner", "waiter"],
        "the waiting call was not served right after the statement in progress: {order:?}"
    );
}

/// A start that fails fails every call that waited through it, at once: the call queued behind
/// it neither starts the records process again nor waits a start of its own. One outage costs
/// one start, not one per caller queued behind it.
///
/// Launch 1 is ready at once and then ends, so the first call finds the process gone. Call B
/// starts launch 2, which waits until the case lets it refuse to start; call C is made while it
/// waits, and the case lets launch 2 refuse only once C is queued behind B. The deadline is long
/// enough that neither call reaches it: C fails because the start it waited through failed.
#[test]
fn a_call_queued_behind_a_failed_start_fails_with_it_and_starts_nothing() {
    use cortex_cli::ask::records::postgres::{Remote, Sql};
    let tmp = tempfile::Builder::new()
        .prefix("cortex-records-failed-start")
        .tempdir_in(
            std::env::var_os("CARGO_TARGET_TMPDIR")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir),
        )
        .unwrap();
    let root = tmp.path().to_path_buf();
    let (log, refuse) = (root.join("launches.log"), root.join("refuse"));
    executable(
        &root.join("connectors"),
        &format!(
            r#"printf 'launch\n' >> "{log}"
n=$(wc -l < "{log}" | tr -d ' ')
if [ "$n" != 1 ]; then
  while [ ! -e "{refuse}" ]; do sleep 0.01; done
  printf '{{"err":"the records database refused the connection"}}\n'
  exit 1
fi
printf '{{"ready":{{"schema_version":{SCHEMA_VERSION}}}}}\n'
"#,
            log = log.display(),
            refuse = refuse.display(),
        ),
    );
    let launch = Launch {
        connectors: root.join("connectors"),
        adapter: "postgres".into(),
        connection: "records".into(),
        url: "postgres://cortex@db.invalid/brain".into(),
        schema: "cortex_ask".into(),
        ca_file: None,
        remove: Vec::new(),
        deadline: HELD,
    };
    let remote = Arc::new(Remote::start(launch).expect("the stand-in starts"));
    assert!(
        remote.query("SELECT 1", &[]).is_err(),
        "a process that ended answered"
    );
    let r = remote.clone();
    let b = std::thread::spawn(move || r.query("SELECT 1", &[]).map_err(|e| e.message));
    until("launch 2", || lines(&log).len() == 2);
    let r = remote.clone();
    let (tx, queued) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(r.query("SELECT 1", &[]).map_err(|e| e.message));
    });
    until("call C queued behind the start", || remote.waiting() == 1);
    std::fs::write(&refuse, "").unwrap();
    let c = queued.recv_timeout(BLOCKED).expect("call C ends");
    let b = b.join().expect("call B ends");
    assert_eq!(
        b,
        Err("the records database refused the connection".to_string()),
        "call B did not fail with the refused start"
    );
    assert_eq!(
        c, b,
        "the queued call did not fail with the start it waited through"
    );
    assert_eq!(
        lines(&log).len(),
        2,
        "the queued call started the records process again"
    );
}
