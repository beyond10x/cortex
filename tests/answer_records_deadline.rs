//! `story:ask-records-deadline`: a records process that stops answering fails each record call
//! within the call's deadline, as `store-failed` on the record routes, and the call after it is
//! not blocked behind it.
//!
//! The records process is a stand-in `connectors` launch that prints the ready line, answers the
//! purge the service makes when it starts, and then reads every statement and answers none, as
//! a records process does whose database stalls. No PostgreSQL is needed.

mod common;

use std::path::PathBuf;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use common::executable;
use cortex_cli::ask::model::Route;
use cortex_cli::ask::records::postgres::{Launch, PgRecords};
use cortex_cli::ask::records::{Keeper, SCHEMA_VERSION};
use cortex_cli::ask::{ReaderCommand, Settings};
use cortex_model::ask as ma;
use serde_json::Value;

/// The deadline of one record call in these cases; the service's is 30 s.
const DEADLINE: Duration = Duration::from_secs(1);
/// What a call may take past its deadline: stopping the stalled process and answering.
const SLACK: Duration = Duration::from_secs(4);
/// How long a case waits for a call before it calls the call blocked.
const BLOCKED: Duration = Duration::from_secs(15);
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
            took < DEADLINE + SLACK,
            "{call} took {took:?}, past its deadline of {DEADLINE:?}"
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
            took < DEADLINE + SLACK,
            "call {i} took {took:?}, past its deadline of {DEADLINE:?}"
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
            took < DEADLINE + SLACK,
            "{route} took {took:?}, past the records' deadline of {DEADLINE:?}"
        );
    }
}
