//! A spec names a Connectors connection by id or by profile of its adapter; a profile is resolved,
//! each time cortex needs the connection, to the one live connection `connections list` names with
//! it (`story:connection-by-profile`). None or several is refused with a reason naming the adapter,
//! the profile, the count and every matching id.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{answer, ekr, executable, World, PAGES};
use serde_json::Value;

/// A stand-in `connectors` whose `connections list --adapter <a>` answers the entries in
/// `<root>/list-<a>.json` (written by [`list`]). Every argument list goes to
/// `<root>/connectors-argv.log`, every invocation to `<root>/invocations.log`. An invocation
/// answers `<root>/answer-<connector>.json`. `connections launch --consumer ekr --args '<JSON>'`
/// opens `<state>/<connection>.json` on descriptor 3 and runs `ekr` with the array's elements and
/// only the `EKR_*` variables; a connection with no such file is refused.
const CONNECTORS: &str = r#"R="@ROOT@"; STATE="@STATE@"; EKR="@EKR@"
printf '%s\n' "$*" >> "$R/connectors-argv.log"
case "$1 $2" in
  "connections list")
    printf '{"ok":true,"result":{"connections":[%s]}}' "$(cat "$R/list-$4.json" 2>/dev/null)" ;;
  "connections revalidate") echo '{"ok":true,"result":{}}' ;;
  "invoke "*)
    printf '%s\n' "$*" >> "$R/invocations.log"
    cat "$R/answer-${2%%/*}.json" ;;
  "connections launch")
    shift 2; C=""; CONSUMER=""; ARGS="[]"
    while [ $# -gt 0 ]; do
      case "$1" in
        --adapter) shift ;;
        --connection) C="$2"; shift ;;
        --consumer) CONSUMER="$2"; shift ;;
        --args) ARGS="$2"; shift ;;
        *) echo "unexpected $1" >&2; exit 2 ;;
      esac
      shift
    done
    [ "$CONSUMER" = ekr ] && [ -e "$STATE/$C.json" ] || { echo "no consumer ekr for $C" >&2; exit 2; }
    exec 3<"$STATE/$C.json"
    L=$(printf '%s' "$ARGS" | sed -e 's/^\["//' -e 's/"\]$//' -e 's/","/\n/g')
    set --
    for v in $(env | sed -n 's/^\(EKR_[A-Za-z0-9_]*\)=.*/\1/p'); do set -- "$@" "$v=$(printenv "$v")"; done
    set -- "$@" "$EKR"
    if [ "$ARGS" != "[]" ]; then
      while IFS= read -r a; do set -- "$@" "$a"; done <<EOF
$L
EOF
    fi
    exec env -i "$@" ;;
  *) echo '{"ok":false}'; exit 2 ;;
esac
"#;

/// A world with the [`CONNECTORS`] stand-in launching `launched` (the real `ekr` when `None`), its
/// launch state at `<root>/connectors-state`.
fn world_launching(launched: Option<&Path>) -> World {
    let w = World::new();
    let state = w.root.join("connectors-state");
    std::fs::create_dir_all(&state).unwrap();
    let ekr = launched.map(Path::to_path_buf).unwrap_or_else(ekr);
    executable(
        &w.bin.join("connectors"),
        &CONNECTORS
            .replace("@ROOT@", &w.root.display().to_string())
            .replace("@STATE@", &state.display().to_string())
            .replace("@EKR@", &ekr.display().to_string()),
    );
    std::fs::write(w.root.join("answer-tavily.json"), answer(PAGES)).unwrap();
    w
}

fn world() -> World {
    world_launching(None)
}

/// Makes `adapter`'s `connections list` answer `entries`: (id, profile, state).
fn list(w: &World, adapter: &str, entries: &[(&str, &str, &str)]) {
    let entries: Vec<String> = entries
        .iter()
        .map(|(id, profile, state)| {
            serde_json::json!({"adapter": adapter, "connector": adapter, "instance": "main", "instance_id": "local", "connection": id,
                "profile": profile, "revision": "rev1", "state": state})
            .to_string()
        })
        .collect();
    std::fs::write(
        w.root.join(format!("list-{adapter}.json")),
        entries.join(","),
    )
    .unwrap();
}

/// The world's web spec `name`, its source naming `connection` as written (`profile: <p>` or
/// `connection: <id>`).
fn web_spec(w: &World, name: &str, connection: &str) -> PathBuf {
    let path = w.spec(name, "@CONNECTION@");
    let text = std::fs::read_to_string(&path)
        .unwrap()
        .replace("connection: @CONNECTION@", connection);
    std::fs::write(&path, text).unwrap();
    path
}

/// `cortex` with the world's stand-ins: exit code, the last stdout line as JSON, stderr.
fn cortex(w: &World, args: &[&str]) -> (Option<i32>, Value, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(args)
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let last = stdout.lines().last().unwrap_or("null");
    (
        out.status.code(),
        serde_json::from_str(last).unwrap_or(Value::Null),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn create(w: &World, spec: &Path) -> (Option<i32>, Value, String) {
    cortex(
        w,
        &[
            "create",
            "--spec",
            spec.to_str().unwrap(),
            "--no-units",
            "--no-extract",
        ],
    )
}

fn assert_names(reason: &str, parts: &[&str]) {
    for part in parts {
        assert!(
            reason.contains(part),
            "{part:?} is not in the reason {reason:?}"
        );
    }
}

#[test]
fn a_web_source_named_by_profile_runs_through_its_one_live_connection() {
    let w = world();
    // The one live `tavily.api-key` connection is conn_B; conn_A has another profile and conn_R
    // is revoked.
    list(
        &w,
        "tavily",
        &[
            ("conn_A", "tavily.other", "ready"),
            ("conn_B", "tavily.api-key", "pending"),
            ("conn_R", "tavily.api-key", "revoked"),
        ],
    );
    let spec = web_spec(&w, "pb", "profile: tavily.api-key");
    let (code, out, err) = create(&w, &spec);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (Some(0), Some("created")),
        "{out}{err}"
    );
    let (code, ran, err) = cortex(&w, &["run", "pb/news"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (Some(0), Some("ran")),
        "{ran}{err}"
    );
    let invocations = w.lines("invocations.log");
    assert!(!invocations.is_empty(), "nothing was invoked");
    for line in &invocations {
        assert!(line.contains("--connection conn_B "), "{line}");
    }
    // The pending connection was revalidated by its resolved id.
    let argv = w.lines("connectors-argv.log");
    assert!(
        argv.iter()
            .any(|l| l.starts_with("connections revalidate --adapter tavily --connection conn_B ")),
        "{argv:?}"
    );
    assert!(
        !argv
            .iter()
            .any(|l| l.contains("conn_A") || l.contains("conn_R")),
        "{argv:?}"
    );
    // The frozen spec keeps the reference, not the id it resolved to.
    let frozen = std::fs::read_to_string(w.home.join("instances/pb/instance.yaml")).unwrap();
    assert!(frozen.contains("profile: tavily.api-key"), "{frozen}");
    assert!(!frozen.contains("conn_B"), "{frozen}");
}

#[test]
fn create_refuses_a_profile_with_two_or_no_live_connections_naming_them() {
    let w = world();
    list(
        &w,
        "tavily",
        &[
            ("conn_B", "tavily.api-key", "ready"),
            ("conn_C", "tavily.api-key", "pending"),
        ],
    );
    let spec = web_spec(&w, "two", "profile: tavily.api-key");
    let (code, out, err) = create(&w, &spec);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (Some(1), Some("connection-missing")),
        "{out}{err}"
    );
    assert_eq!(out["detail"]["connection"], "tavily:profile tavily.api-key");
    let reason = out["detail"]["reason"].as_str().unwrap_or_default();
    assert_names(
        reason,
        &["tavily", "tavily.api-key", " 2 ", "conn_B", "conn_C"],
    );
    assert!(!w.home.join("instances/two").exists());

    list(&w, "tavily", &[("conn_R", "tavily.api-key", "revoked")]);
    let spec = web_spec(&w, "none", "profile: tavily.api-key");
    let (code, out, err) = create(&w, &spec);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (Some(1), Some("connection-missing")),
        "{out}{err}"
    );
    let reason = out["detail"]["reason"].as_str().unwrap_or_default();
    assert_names(reason, &["tavily", "tavily.api-key", " 0 "]);
    assert!(!reason.contains("conn_R"), "{reason}");
    assert!(!w.home.join("instances/none").exists());
    assert!(w.lines("invocations.log").is_empty());
}

#[test]
fn a_run_whose_profile_is_no_longer_one_connection_fails_its_fetch_naming_them() {
    let w = world();
    list(&w, "tavily", &[("conn_B", "tavily.api-key", "ready")]);
    let spec = web_spec(&w, "later", "profile: tavily.api-key");
    let (code, out, err) = create(&w, &spec);
    assert_eq!(code, Some(0), "{out}{err}");

    // A second pod connected the same profile: the run cannot tell which credential is meant.
    list(
        &w,
        "tavily",
        &[
            ("conn_B", "tavily.api-key", "ready"),
            ("conn_C", "tavily.api-key", "ready"),
        ],
    );
    let (code, ran, err) = cortex(&w, &["run", "later/news"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (Some(1), Some("fetch-failed")),
        "{ran}{err}"
    );
    assert_names(
        &ran.to_string(),
        &["tavily", "tavily.api-key", " 2 ", "conn_B", "conn_C"],
    );

    list(&w, "tavily", &[]);
    let (code, ran, err) = cortex(&w, &["run", "later/news"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (Some(1), Some("fetch-failed")),
        "{ran}{err}"
    );
    assert_names(&ran.to_string(), &["tavily", "tavily.api-key", " 0 "]);
    assert!(
        w.lines("invocations.log").is_empty(),
        "a refused profile invoked"
    );
}

#[test]
fn a_spec_that_names_its_connection_by_id_runs_as_before() {
    let w = world();
    list(
        &w,
        "tavily",
        &[
            ("conn_test", "tavily.api-key", "ready"),
            ("conn_C", "tavily.api-key", "ready"),
        ],
    );
    let spec = web_spec(&w, "byid", "connection: conn_test");
    let (code, out, err) = create(&w, &spec);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (Some(0), Some("created")),
        "{out}{err}"
    );
    let (code, ran, err) = cortex(&w, &["run", "byid/news"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (Some(0), Some("ran")),
        "{ran}{err}"
    );
    for line in w.lines("invocations.log") {
        assert!(line.contains("--connection conn_test "), "{line}");
    }
}

#[test]
fn a_spec_naming_both_an_id_and_a_profile_or_neither_is_refused() {
    let w = world();
    list(&w, "tavily", &[("conn_B", "tavily.api-key", "ready")]);
    for (name, connection) in [
        (
            "both",
            "connection: conn_B\n        profile: tavily.api-key",
        ),
        ("neither", "adapter: tavily"),
    ] {
        let spec = web_spec(&w, name, connection);
        let (code, out, err) = create(&w, &spec);
        assert_eq!(code, Some(2), "{name}: {out}{err}");
        assert!(
            err.contains("connection") && err.contains("profile"),
            "{name}: {err}"
        );
        assert!(!w.home.join("instances").join(name).exists());
    }
}

/// A `connectors` source and a `structured` one name their connection by profile of their own
/// adapter; each run invokes through the one live connection of that profile.
#[test]
fn connectors_and_structured_sources_named_by_profile_invoke_their_resolved_id() {
    let w = world();
    list(
        &w,
        "tracker",
        &[
            ("conn_T", "tracker.token", "ready"),
            ("conn_U", "tracker.other", "ready"),
        ],
    );
    list(
        &w,
        "directory",
        &[
            ("conn_D", "directory.token", "ready"),
            ("conn_E", "directory.token", "revoked"),
        ],
    );
    std::fs::write(
        w.root.join("answer-tracker.json"),
        serde_json::json!({"ok": true, "result": {"connector": "tracker", "instance": "main", "operation": "issues.list",
            "revision": "r", "result": {"issues": [{"key": "I-1", "summary": "Widget engine"}]}}})
        .to_string(),
    )
    .unwrap();
    std::fs::write(
        w.root.join("answer-directory.json"),
        serde_json::json!({"ok": true, "result": {"connector": "directory", "instance": "main",
            "operation": "people.list", "revision": "r",
            "result": {"people": [{"id": "P-1", "name": "Ada Lovelace"}]}}})
        .to_string(),
    )
    .unwrap();
    let path = w.root.join("kinds.yaml");
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: kinds
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
        profile: tracker.token
        operation: issues.list
        inputs: [{{}}]
        records: issues
        id: key
        text: ["{{key}}: {{summary}}"]
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
  - name: people
    schedule: daily
    settings:
      kind: structured
      value:
        input:
          from: connectors
          value: {{adapter: directory, profile: directory.token, operation: people.list, inputs: [{{}}]}}
        records: "$.people"
        mapping:
          node_type: Person
          id: "$.id"
          name: "$.name"
          aliases: []
          properties: []
          relations: []
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
serve: {{view_port: 18994}}
"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    let (code, out, err) = create(&w, &path);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (Some(0), Some("created")),
        "{out}{err}"
    );
    let (_, ran, err) = cortex(&w, &["run", "kinds/tracker"]);
    assert_ne!(ran["outcome"], "fetch-failed", "{ran}{err}");
    let (code, ran, err) = cortex(&w, &["run", "kinds/people"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (Some(0), Some("ran")),
        "{ran}{err}"
    );
    let invocations = w.lines("invocations.log");
    let tracker: Vec<_> = invocations
        .iter()
        .filter(|l| l.contains("invoke tracker/main "))
        .collect();
    let directory: Vec<_> = invocations
        .iter()
        .filter(|l| l.contains("invoke directory/main "))
        .collect();
    assert!(
        !tracker.is_empty() && !directory.is_empty(),
        "{invocations:?}"
    );
    for line in tracker {
        assert!(line.contains("--connection conn_T "), "{line}");
    }
    for line in directory {
        assert!(line.contains("--connection conn_D "), "{line}");
    }
}

/// An `ekr` at `<bin>/ekr-pg` that answers a `postgres` store from `<root>/pg-as-sqlite.sqlite`
/// through the real `ekr`, as `tests/store_backend.rs` does: `head` before that file exists answers
/// PostgreSQL's "the lineage has no seed", and `postgres-schema` answers ready.
fn sqlite_backed_ekr(w: &World) -> PathBuf {
    let path = w.bin.join("ekr-pg");
    executable(
        &path,
        &format!(
            r#"R="{root}"; EKR="{ekr}"; DB="$R/pg-as-sqlite.sqlite"
if [ "$1" = postgres-schema ]; then echo '{{"format":"ekr.postgres-schema/1","ready":true}}'; exit 0; fi
if [ "$EKR_BACKEND" = postgres ]; then
  if [ "$1" = head ] && [ ! -e "$DB" ]; then echo "the lineage has no seed" >&2; exit 1; fi
  EKR_BACKEND=sqlite; EKR_STORE="$DB"; export EKR_BACKEND EKR_STORE
fi
exec "$EKR" "$@"
"#,
            root = w.root.display(),
            ekr = ekr().display()
        ),
    );
    path
}

/// An `ekr.postgres/1` file at `path` that takes its password from the launch's descriptor 3.
fn pg_config(path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let document = serde_json::json!({
        "format": "ekr.postgres/1",
        "connection_file": "app.dsn",
        "password_file": "/proc/self/fd/3",
        "schema": "ekr",
        "database_connections": 100,
        "replicas": 1,
        "reserved_connections": 10,
    });
    std::fs::write(path, document.to_string()).unwrap();
}

/// A world with a `postgres` store named by profile: `connection` by `app-role`,
/// `schema_connection` by `owner-role`. The launch state holds a document for every listed id, so
/// a launch through the wrong one is not refused by the stand-in and only the arguments tell.
fn postgres_world(entries: &[(&str, &str, &str)]) -> (World, PathBuf, PathBuf) {
    let w = World::new();
    let ekr_pg = sqlite_backed_ekr(&w);
    let state = w.root.join("connectors-state");
    std::fs::create_dir_all(&state).unwrap();
    executable(
        &w.bin.join("connectors"),
        &CONNECTORS
            .replace("@ROOT@", &w.root.display().to_string())
            .replace("@STATE@", &state.display().to_string())
            .replace("@EKR@", &ekr_pg.display().to_string()),
    );
    for (id, ..) in entries {
        std::fs::write(state.join(format!("{id}.json")), r#"{"password":"x"}"#).unwrap();
    }
    list(&w, "postgres", entries);
    let config = w.root.join("pg/app.json");
    let owner = w.root.join("pg/owner.json");
    pg_config(&config);
    pg_config(&owner);
    let docs = w.root.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(
        docs.join("a.md"),
        "Example Labs develops the Widget engine.\n",
    )
    .unwrap();
    let spec = w.root.join("pg.yaml");
    std::fs::write(
        &spec,
        format!(
            r#"format: cortex.instance/1
name: pg
description: Test brain.
ekr: {{version: "0.0.34", bin: "{ekr}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
  - name: docs
    schedule: daily
    settings:
      kind: files
      value: {{paths: [docs], glob: "**/*.md"}}
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
serve: {{view_port: 18993}}
store:
  backend: postgres
  value:
    config: "{config}"
    connection: {{adapter: postgres, profile: app-role}}
    schema_connection: {{adapter: postgres, profile: owner-role}}
"#,
            ekr = ekr_pg.display(),
            config = config.display()
        ),
    )
    .unwrap();
    (w, spec, owner)
}

#[test]
fn a_postgres_store_named_by_profile_launches_ekr_through_the_matching_ids() {
    let (w, spec, owner) = postgres_world(&[
        ("conn_app", "app-role", "ready"),
        ("conn_old", "app-role", "revoked"),
        ("conn_owner", "owner-role", "pending"),
        ("conn_other", "other-role", "ready"),
    ]);
    let (code, out, err) = cortex(
        &w,
        &[
            "create",
            "--spec",
            spec.to_str().unwrap(),
            "--no-units",
            "--no-extract",
            "--postgres-schema-config",
            owner.to_str().unwrap(),
        ],
    );
    assert_eq!(
        (code, out["outcome"].as_str()),
        (Some(0), Some("created")),
        "{out}{err}"
    );
    let (code, ran, err) = cortex(&w, &["run", "pg/docs"]);
    assert_eq!(code, Some(0), "{ran}{err}");
    let argv = w.lines("connectors-argv.log");
    let launches: Vec<_> = argv
        .iter()
        .filter(|l| l.starts_with("connections launch "))
        .collect();
    let through = |id: &str, args: &str| {
        launches.iter().any(|l| {
            l.starts_with(&format!(
                "connections launch --adapter postgres --connection {id} --consumer ekr --args {args}"
            ))
        })
    };
    assert!(
        through("conn_owner", r#"["postgres-schema","#),
        "{launches:#?}"
    );
    for args in [r#"["head"]"#, r#"["seed","#, r#"["apply-extraction","#] {
        assert!(through("conn_app", args), "{args} in {launches:#?}");
    }
    for line in &launches {
        assert!(
            line.contains("--connection conn_app ") || line.contains("--connection conn_owner "),
            "{line}"
        );
    }
    assert!(
        !argv
            .iter()
            .any(|l| l.contains("conn_old") || l.contains("conn_other")),
        "{argv:#?}"
    );
    // The MCP line names the id the profile resolves to when it is printed.
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["mcp-line", "pg"])
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .output()
        .unwrap();
    let line = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{line}");
    assert!(
        line.contains("connections launch --adapter postgres --connection conn_app --consumer ekr"),
        "{line}"
    );
}

#[test]
fn a_postgres_store_whose_profile_matches_two_connections_is_not_created() {
    let (w, spec, owner) = postgres_world(&[
        ("conn_app", "app-role", "ready"),
        ("conn_app2", "app-role", "ready"),
        ("conn_owner", "owner-role", "ready"),
    ]);
    let (code, out, err) = cortex(
        &w,
        &[
            "create",
            "--spec",
            spec.to_str().unwrap(),
            "--no-units",
            "--no-extract",
            "--postgres-schema-config",
            owner.to_str().unwrap(),
        ],
    );
    assert_eq!(
        (code, out["outcome"].as_str()),
        (Some(1), Some("connection-missing")),
        "{out}{err}"
    );
    let reason = out["detail"]["reason"].as_str().unwrap_or_default();
    assert_names(
        reason,
        &["postgres", "app-role", " 2 ", "conn_app", "conn_app2"],
    );
    assert!(
        !w.lines("connectors-argv.log")
            .iter()
            .any(|l| l.starts_with("connections launch")),
        "a refused profile launched ekr"
    );
}
