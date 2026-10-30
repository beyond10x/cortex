//! A run's store reads go through one `ekr session` (`story:ekr-session-per-run`): `ontology`,
//! `snapshot`, `schema`, `mint`, `propose`, `validate`, `commit`, `quality` and `head` are requests
//! to one process the run starts, joined to its stage when one is open; `apply-extraction` and the
//! `ekr stage` verbs stay processes of their own, each preceded by the readiness check of a
//! launched store's connection.
//!
//! Argv-logging stand-ins count what a run starts: an `ekr` in front of the real one, and, for a
//! launched store, a `connectors` that answers `connections list` and runs `connections launch`.

mod common;

use common::{ekr, executable, World};
use cortex_cli::ekr::{Backend, Store};

/// The stand-in `claude`: one `Service` entity named by the next record's id, so every batch
/// applies something.
const CLAUDE: &str = r#"R="@ROOT@"
cat > "$R/prompt.cur"
echo call >> "$R/claude-calls.log"
N=x
if grep -q '#beta' "$R/prompt.cur"; then N=gamma; elif grep -q '#alpha' "$R/prompt.cur"; then N=beta; fi
cat <<EOF
{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{"format":"ekr.extraction-document/1","ontology":{"node_types":[{"name":"Service","parents":[],"abstract_type":false,"properties":[]}],"edge_types":[]},"entities":[{"node_type":"Service","aliases":["$N"]}],"facts":[]}}
EOF
"#;

/// The stand-in `ekr`: logs the stage it was started in (`EKR_STAGE`, or `-`) and its arguments,
/// `<stage>|<args>`, one process per line, to `ekr-argv.log`. A `postgres` store is answered from a
/// SQLite file through the real `ekr`, whose stages work on SQLite too. A session's requests are
/// logged as they are sent, one per line, to `ekr-session.log`.
const EKR: &str = r#"R="@ROOT@"; EKR="@EKR@"; DB="$R/pg-as-sqlite.sqlite"
printf '%s|%s\n' "${EKR_STAGE:--}" "$*" >> "$R/ekr-argv.log"
if [ "$1" = postgres-schema ]; then echo '{"format":"ekr.postgres-schema/1","ready":true}'; exit 0; fi
if [ "$EKR_BACKEND" = postgres ]; then
  if [ "$1" = head ] && [ ! -e "$DB" ]; then echo "the lineage has no seed" >&2; exit 1; fi
  EKR_BACKEND=sqlite; EKR_STORE="$DB"; export EKR_BACKEND EKR_STORE
fi
if [ "$1" = session ]; then tee -a "$R/ekr-session.log" | "$EKR" "$@"; exit $?; fi
exec "$EKR" "$@"
"#;

/// A stand-in `connectors` for a launched store: logs every argument list to
/// `connectors-argv.log`; `connections list` answers connection `app` as ready;
/// `connections launch --consumer ekr --args '<JSON>'` runs the stand-in `ekr` with the array's
/// elements as its arguments and only the `EKR_*` variables (`pass_env`), the password document on
/// descriptor 3.
const CONNECTORS: &str = r#"R="@ROOT@"; EKR="@EKR@"
printf '%s\n' "$*" >> "$R/connectors-argv.log"
case "$1 $2" in
  "connections list")
    printf '{"ok":true,"result":{"connections":[{"adapter":"postgres","connection":"app","state":"ready","revision":"r1"}]}}' ;;
  "connections launch")
    shift 2; ARGS="[]"
    while [ $# -gt 0 ]; do
      case "$1" in
        --args) ARGS="$2"; shift ;;
        --adapter|--connection|--consumer) shift ;;
      esac
      shift
    done
    exec 3<"$R/password.json"
    L=$(printf '%s' "$ARGS" | sed -e 's/^\["//' -e 's/"\]$//' -e 's/","/\n/g')
    set --
    for v in $(env | sed -n 's/^\(EKR_[A-Za-z0-9_]*\)=.*/\1/p'); do set -- "$@" "$v=$(printenv "$v")"; done
    set -- "$@" "$EKR"
    while IFS= read -r a; do set -- "$@" "$a"; done <<EOF
$L
EOF
    exec env -i "$@" ;;
  *) echo '{"ok":false}'; exit 2 ;;
esac
"#;

/// A world whose `files` source `repos` holds three records, `alpha`, `beta` and `gamma`, each of
/// about 800 characters, so a `batch_chars` of 1,000 makes three batches. `store` writes the
/// spec's store block in the world and `gate` is its gate block, both empty for a SQLite store
/// held to no gate.
fn world(store: impl FnOnce(&World) -> String, gate: &str) -> World {
    let w = World::new();
    let store = store(&w);
    let root = w.root.display().to_string();
    executable(&w.bin.join("claude"), &CLAUDE.replace("@ROOT@", &root));
    let stand_in = w.bin.join("ekr");
    executable(
        &stand_in,
        &EKR.replace("@ROOT@", &root)
            .replace("@EKR@", &ekr().display().to_string()),
    );
    executable(
        &w.bin.join("connectors"),
        &CONNECTORS
            .replace("@ROOT@", &root)
            .replace("@EKR@", &stand_in.display().to_string()),
    );
    std::fs::write(w.root.join("password.json"), r#"{"password":"x"}"#).unwrap();
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
             serve: {{view_port: 18993}}\n{store}{gate}",
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
    for log in ["ekr-argv.log", "connectors-argv.log", "ekr-session.log"] {
        let _ = std::fs::remove_file(w.root.join(log));
    }
    w
}

/// A `postgres` store launched through connection `app`, whose `ekr.postgres/1` file names
/// descriptor 3 as its `password_file`.
fn launched_store(w: &World) -> String {
    let config = w.root.join("pg/app.json");
    std::fs::create_dir_all(config.parent().unwrap()).unwrap();
    let document = serde_json::json!({
        "format": "ekr.postgres/1",
        "connection_file": "app.dsn",
        "password_file": "/proc/self/fd/3",
        "schema": "ekr",
        "database_connections": 100,
        "replicas": 1,
        "reserved_connections": 10,
    });
    std::fs::write(&config, document.to_string()).unwrap();
    format!(
        "store: {{backend: postgres, value: {{config: \"{}\", connection: {{adapter: postgres, \
         connection: app}}}}}}\n",
        config.display()
    )
}

/// The `ekr` processes logged since the world was created, as `(stage, verb, args)`.
fn ekrs(w: &World) -> Vec<(String, String, String)> {
    w.lines("ekr-argv.log")
        .into_iter()
        .map(|l| {
            let (stage, args) = l.split_once('|').unwrap_or(("-", &l));
            let verb = args.split(' ').next().unwrap_or_default().to_string();
            (stage.to_string(), verb, args.to_string())
        })
        .collect()
}

fn prompts(w: &World) -> usize {
    w.lines("claude-calls.log").len()
}

/// Acceptance 1, on a SQLite store: a three-batch run starts one `ekr session` and one
/// `apply-extraction` per batch, and no other `ekr`.
#[test]
fn a_sqlite_run_starts_one_session_and_one_apply_per_batch() {
    let w = world(|_| String::new(), "");
    let (code, ran) = w.cortex(&["run", "s/repos"]);
    assert_eq!(code, 0, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 3, "{ran}");
    assert_eq!(prompts(&w), 3, "three batches: {ran}");

    let ekrs = ekrs(&w);
    let count = |verb: &str| ekrs.iter().filter(|(_, v, _)| v == verb).count();
    assert_eq!(count("session"), 1, "one session per run: {ekrs:#?}");
    assert_eq!(
        count("apply-extraction"),
        3,
        "one apply per batch: {ekrs:#?}"
    );
    let others: Vec<_> = ekrs
        .iter()
        .filter(|(_, v, _)| v != "session" && v != "apply-extraction")
        .collect();
    assert!(others.is_empty(), "no other ekr process: {others:#?}");

    // The reads went to the session, and the run still takes one snapshot
    // (`story:one-store-read-per-run`), now a request rather than a process.
    let requests = requests(&w);
    let asked = |verb: &str| requests.iter().filter(|v| *v == verb).count();
    assert_eq!(asked("snapshot"), 1, "one snapshot per run: {requests:#?}");
    assert!(asked("ontology") >= 1, "{requests:#?}");
    assert_eq!(asked("schema"), 1, "{requests:#?}");
    assert_eq!(asked("apply-extraction"), 0, "{requests:#?}");
}

/// The verb of each request the run's session was sent.
fn requests(w: &World) -> Vec<String> {
    w.lines("ekr-session.log")
        .iter()
        .map(|l| {
            let request: serde_json::Value = serde_json::from_str(l).expect("a request line");
            request["argv"][0].as_str().unwrap_or_default().to_string()
        })
        .collect()
}

/// Acceptance 1, on a launched `postgres` store held to a gate on a measure of `ekr quality`: the
/// run runs in a stage, its one session is launched once and joins the stage, `ekr quality` is a
/// request to it, and every other launch is an `apply-extraction` or a stage verb. `connections
/// list`, the readiness check, precedes each launch of its own: the session's requests add none.
#[test]
fn a_launched_staged_run_launches_one_session_in_its_stage_and_checks_readiness_once_for_it() {
    let w = world(
        launched_store,
        "gate:\n  checks:\n    - {measure: assertions.active, min: \"0\"}\n",
    );
    let (code, ran) = w.cortex(&["run", "s/repos"]);
    assert_eq!(code, 0, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 3, "{ran}");
    assert_eq!(prompts(&w), 3, "three batches: {ran}");

    let ekrs = ekrs(&w);
    let sessions: Vec<_> = ekrs.iter().filter(|(_, v, _)| v == "session").collect();
    assert_eq!(sessions.len(), 1, "one session per run: {ekrs:#?}");
    assert_ne!(
        sessions[0].0, "-",
        "the session joins the run's stage: {ekrs:#?}"
    );
    let others: Vec<_> = ekrs
        .iter()
        .filter(|(_, v, _)| !["session", "apply-extraction", "stage"].contains(&v.as_str()))
        .collect();
    assert!(others.is_empty(), "no other ekr process: {others:#?}");
    assert_eq!(
        ekrs.iter()
            .filter(|(_, v, _)| v == "apply-extraction")
            .count(),
        3,
        "one apply per batch: {ekrs:#?}"
    );
    // The stage was published: the gate passed on `ekr quality`, asked through the session.
    assert!(
        ekrs.iter().any(|(_, _, a)| a.starts_with("stage publish ")),
        "{ekrs:#?}"
    );
    let requests = requests(&w);
    assert_eq!(
        requests.iter().filter(|v| *v == "quality").count(),
        1,
        "{requests:#?}"
    );

    let connectors = w.lines("connectors-argv.log");
    let launches: Vec<_> = connectors
        .iter()
        .filter(|l| l.starts_with("connections launch "))
        .collect();
    let session_launches = launches
        .iter()
        .filter(|l| l.ends_with(r#"--args ["session"]"#))
        .count();
    assert_eq!(session_launches, 1, "{connectors:#?}");
    let lists = connectors
        .iter()
        .filter(|l| l.starts_with("connections list"))
        .count();
    let one_shot = launches.len() - session_launches;
    assert!(
        lists <= one_shot + 1,
        "{lists} readiness checks for {one_shot} one-shot launches and one session: \
         {connectors:#?}"
    );
}

/// Acceptance 2, against the pinned `ekr`: a session joined to an open stage reads the stage as
/// it stands when each request is read, so what a one-shot `apply-extraction` in that stage applied
/// after the session's first `snapshot` is in its second.
#[test]
fn a_session_in_a_stage_sees_what_a_one_shot_apply_committed_to_it() {
    let w = World::new();
    let spec = w.spec("t", "conn_test");
    let (code, created) = w.cortex(&[
        "create",
        "--spec",
        spec.to_str().unwrap(),
        "--no-units",
        "--no-extract",
    ]);
    assert_eq!(code, 0, "{created}");
    let dir = w.home.join("instances/t");
    let mut store = Store {
        bin: ekr(),
        host: dir.join("host.json"),
        backend: Backend::Sqlite,
        store: dir.join("store.sqlite"),
        launch: None,
        stage: None,
    };
    let (stage, base) = store.begin_stage().expect("a stage");
    store.stage = Some(stage.clone());

    let session = store.session().expect("a session");
    let names = |snapshot: &cortex_cli::ekr::Snapshot| -> Vec<String> {
        snapshot
            .graph
            .graph
            .nodes
            .values()
            .filter_map(|n| n.canonical_name.clone())
            .collect()
    };
    assert!(names(&session.snapshot().unwrap()).is_empty());

    let doc = w.root.join("batch-k.yaml");
    std::fs::write(
        &doc,
        "format: ekr.extraction-document/1\nontology:\n  node_types:\n  - name: Service\n    \
         parents: []\n    abstract_type: false\n    properties: []\n  edge_types: []\n\
         entities:\n- node_type: Service\n  aliases: [beta]\nfacts: []\nevidence: []\n",
    )
    .unwrap();
    let applied = store.apply(&doc).expect("applied in the stage");
    assert!(
        applied["committed"]
            .as_array()
            .is_some_and(|c| !c.is_empty()),
        "{applied}"
    );

    assert_eq!(
        names(&session.snapshot().unwrap()),
        ["beta"],
        "batch k+1's snapshot through the session sees batch k's apply"
    );
    assert!(session.head().unwrap() > base, "the stage's head moved");
    session.close().expect("the session ends cleanly");

    // The store itself, outside the stage, does not hold it until the stage is published.
    let outside = Store {
        stage: None,
        ..store.clone()
    };
    assert_eq!(outside.head().unwrap(), base);
    store.abandon_stage(&stage).unwrap();
}
