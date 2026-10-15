//! Adversary cases for `story:connection-by-profile`: the places a profile reference is resolved
//! (or written down) that `tests/connection_profile.rs` does not drive.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{answer, ekr, executable, World, PAGES};
use cortex_cli::connectors::pick;
use serde_json::{json, Value};

/// A stand-in `connectors`: `connections list --adapter <a>` answers `<root>/list-<a>.json`;
/// every argument list goes to `<root>/connectors-argv.log`, every invocation to
/// `<root>/invocations.log` and answers `<root>/answer-<adapter>.json`; `connections launch`
/// opens `<state>/<connection>.json` on descriptor 3 and runs `ekr` with the `--args` array.
const CONNECTORS: &str = r#"R="@ROOT@"; STATE="@STATE@"; EKR="@EKR@"
printf '%s\n' "$*" >> "$R/connectors-argv.log"
case "$1 $2" in
  "connections list")
    printf '{"ok":true,"result":{"connections":[%s]}}' "$(cat "$R/list-$4.json" 2>/dev/null)" ;;
  "connections revalidate") echo '{"ok":true,"result":{}}' ;;
  "operations describe") echo '{"ok":true,"result":{"schema":"s","revision":"r"}}' ;;
  "operations invoke")
    printf '%s\n' "$*" >> "$R/invocations.log"
    cat "$R/answer-$4.json" ;;
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

fn stand_in(w: &World, launched: &Path) -> PathBuf {
    let state = w.root.join("connectors-state");
    std::fs::create_dir_all(&state).unwrap();
    executable(
        &w.bin.join("connectors"),
        &CONNECTORS
            .replace("@ROOT@", &w.root.display().to_string())
            .replace("@STATE@", &state.display().to_string())
            .replace("@EKR@", &launched.display().to_string()),
    );
    state
}

/// Makes `adapter`'s `connections list` answer `entries`: (id, profile, state).
fn list(w: &World, adapter: &str, entries: &[(&str, &str, &str)]) {
    let entries: Vec<String> = entries
        .iter()
        .map(|(id, profile, state)| {
            json!({"adapter": adapter, "instance_id": "local", "connection": id,
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

/// `cortex` with the world's stand-ins: exit code, stdout, stderr.
fn cortex(w: &World, args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(args)
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn last_json(stdout: &str) -> Value {
    serde_json::from_str(stdout.lines().last().unwrap_or("null")).unwrap_or(Value::Null)
}

fn assert_names(text: &str, parts: &[&str]) {
    for part in parts {
        assert!(text.contains(part), "{part:?} is not in {text:?}");
    }
}

/// An `ekr` that answers a `postgres` store from a SQLite file through the real `ekr`, as
/// `tests/connection_profile.rs` does.
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

fn pg_config(path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let document = json!({
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

/// A `postgres` world whose spec `pg` names `store` (the `connection` and `schema_connection`
/// lines). The launch state holds a document for every listed id. Answers the world, the spec and
/// the schema role's config.
fn postgres_world(
    entries: &[(&str, &str, &str)],
    store: &str,
    port: u16,
) -> (World, PathBuf, PathBuf) {
    let w = World::new();
    let ekr_pg = sqlite_backed_ekr(&w);
    let state = stand_in(&w, &ekr_pg);
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
serve: {{view_port: {port}}}
store:
  backend: postgres
  value:
    config: "{config}"
{store}
"#,
            ekr = ekr_pg.display(),
            config = config.display()
        ),
    )
    .unwrap();
    (w, spec, owner)
}

const BY_PROFILE: &str = "    connection: {adapter: postgres, profile: app-role}\n    \
                          schema_connection: {adapter: postgres, profile: owner-role}";

fn create_pg(w: &World, spec: &Path, owner: &Path, units: bool) -> (Option<i32>, String, String) {
    let mut args = vec![
        "create",
        "--spec",
        spec.to_str().unwrap(),
        "--no-extract",
        "--postgres-schema-config",
        owner.to_str().unwrap(),
    ];
    if !units {
        args.push("--no-units");
    }
    cortex(w, &args)
}

fn launches(w: &World) -> Vec<String> {
    w.lines("connectors-argv.log")
        .into_iter()
        .filter(|l| l.starts_with("connections launch "))
        .collect()
}

/// The story: a profile "is resolved each time cortex needs the connection". The viewer unit
/// `create` writes holds the id the profile resolved to then. After the connection is connected
/// anew (the old one revoked, a new one with the same profile), `cortex update` with the unchanged
/// spec is the operator's way to have cortex write its units again; the viewer must then launch
/// through the live connection, not the revoked one.
#[test]
fn adv_update_after_a_reconnect_points_the_viewer_unit_at_the_live_connection() {
    let (w, spec, owner) = postgres_world(
        &[
            ("conn_app", "app-role", "ready"),
            ("conn_owner", "owner-role", "ready"),
        ],
        BY_PROFILE,
        18981,
    );
    let (code, out, err) = create_pg(&w, &spec, &owner, true);
    assert_eq!(code, Some(0), "{out}{err}");
    let unit_path = w.units.join("cortex-pg-view.service");
    let before = std::fs::read_to_string(&unit_path).unwrap();
    assert!(before.contains(r#"--connection "conn_app" "#), "{before}");

    // Reconnected: conn_app revoked, conn_new is the one live `app-role` connection.
    list(
        &w,
        "postgres",
        &[
            ("conn_app", "app-role", "revoked"),
            ("conn_new", "app-role", "ready"),
            ("conn_owner", "owner-role", "ready"),
        ],
    );
    std::fs::write(
        w.root.join("connectors-state/conn_new.json"),
        r#"{"password":"x"}"#,
    )
    .unwrap();
    let (code, out, err) = cortex(&w, &["update", "pg", "--spec", spec.to_str().unwrap()]);
    assert_eq!(code, Some(0), "{out}{err}");
    let after = std::fs::read_to_string(&unit_path).unwrap();
    assert!(
        !after.contains(r#"--connection "conn_app" "#),
        "after a reconnect and an update, the viewer unit still launches through the revoked \
         connection:\n{after}"
    );
}

/// A run of a `postgres` store whose profile became ambiguous after `create` refuses before any
/// `ekr` is launched, naming the adapter, the profile, the count and both ids.
#[test]
fn adv_a_run_of_a_postgres_store_whose_profile_became_two_launches_nothing() {
    let (w, spec, owner) = postgres_world(
        &[
            ("conn_app", "app-role", "ready"),
            ("conn_owner", "owner-role", "ready"),
        ],
        BY_PROFILE,
        18982,
    );
    let (code, out, err) = create_pg(&w, &spec, &owner, false);
    assert_eq!(code, Some(0), "{out}{err}");
    list(
        &w,
        "postgres",
        &[
            ("conn_app", "app-role", "ready"),
            ("conn_app2", "app-role", "pending"),
            ("conn_owner", "owner-role", "ready"),
        ],
    );
    std::fs::write(
        w.root.join("connectors-state/conn_app2.json"),
        r#"{"password":"x"}"#,
    )
    .unwrap();
    let launched_before = launches(&w).len();
    let (code, out, err) = cortex(&w, &["run", "pg/docs"]);
    assert_eq!(code, Some(1), "{out}{err}");
    assert_names(
        &format!("{out}{err}"),
        &["postgres", "app-role", " 2 ", "conn_app", "conn_app2"],
    );
    assert_eq!(
        launches(&w).len(),
        launched_before,
        "a refused profile launched ekr: {:#?}",
        launches(&w)
    );
}

/// `cortex mcp-line` of a store whose profile matches two connections prints no line and names
/// both; with one live match it names that one.
#[test]
fn adv_mcp_line_refuses_an_ambiguous_profile_and_follows_a_reconnect() {
    let (w, spec, owner) = postgres_world(
        &[
            ("conn_app", "app-role", "ready"),
            ("conn_owner", "owner-role", "ready"),
        ],
        BY_PROFILE,
        18983,
    );
    let (code, out, err) = create_pg(&w, &spec, &owner, false);
    assert_eq!(code, Some(0), "{out}{err}");
    list(
        &w,
        "postgres",
        &[
            ("conn_app", "app-role", "revoked"),
            ("conn_new", "app-role", "ready"),
        ],
    );
    let (code, out, err) = cortex(&w, &["mcp-line", "pg"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert!(out.contains("--connection conn_new "), "{out}");
    list(
        &w,
        "postgres",
        &[
            ("conn_app", "app-role", "ready"),
            ("conn_new", "app-role", "ready"),
        ],
    );
    let (code, out, err) = cortex(&w, &["mcp-line", "pg"]);
    assert_ne!(code, Some(0), "{out}{err}");
    assert!(!out.contains("connections launch"), "{out}");
    assert_names(
        &format!("{out}{err}"),
        &["postgres", "app-role", " 2 ", "conn_app", "conn_new"],
    );
}

/// `create` checks the `schema_connection` profile as well as the `connection` one.
#[test]
fn adv_create_refuses_an_ambiguous_schema_connection_profile() {
    let (w, spec, owner) = postgres_world(
        &[
            ("conn_app", "app-role", "ready"),
            ("conn_o1", "owner-role", "ready"),
            ("conn_o2", "owner-role", "reauthorization_required"),
        ],
        BY_PROFILE,
        18984,
    );
    let (code, out, err) = create_pg(&w, &spec, &owner, false);
    let done = last_json(&out);
    assert_eq!(
        (code, done["outcome"].as_str()),
        (Some(1), Some("connection-missing")),
        "{out}{err}"
    );
    assert_eq!(done["detail"]["connection"], "postgres:profile owner-role");
    assert_names(
        done["detail"]["reason"].as_str().unwrap_or_default(),
        &["postgres", "owner-role", " 2 ", "conn_o1", "conn_o2"],
    );
    assert!(launches(&w).is_empty(), "{:#?}", launches(&w));
    assert!(!w.home.join("instances/pg").exists());
}

/// Both an id and a profile, or neither, is refused (exit 2) on a `postgres` store reference, a
/// `connectors` source and a structured source, and by `update` as by `create`.
#[test]
fn adv_both_or_neither_is_refused_on_every_place_and_by_update() {
    for (i, store) in [
        "    connection: {adapter: postgres, connection: conn_app, profile: app-role}",
        "    connection: {adapter: postgres}",
        "    connection: {adapter: postgres, connection: conn_app}\n    \
         schema_connection: {adapter: postgres, connection: conn_owner, profile: owner-role}",
    ]
    .into_iter()
    .enumerate()
    {
        let (w, spec, owner) = postgres_world(
            &[
                ("conn_app", "app-role", "ready"),
                ("conn_owner", "owner-role", "ready"),
            ],
            store,
            18985,
        );
        let (code, out, err) = create_pg(&w, &spec, &owner, false);
        assert_eq!(code, Some(2), "store form {i}: {out}{err}");
        assert!(err.contains("store.value."), "store form {i}: {err}");
        assert!(launches(&w).is_empty(), "store form {i}");
    }

    let w = World::new();
    stand_in(&w, &ekr());
    std::fs::write(w.root.join("answer-tavily.json"), answer(PAGES)).unwrap();
    list(&w, "tracker", &[("conn_T", "tracker.token", "ready")]);
    let source = |value: &str| {
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
{value}
serve: {{view_port: 18986}}
"#,
                ekr = ekr().display()
            ),
        )
        .unwrap();
        path
    };
    let connectors = |reference: &str| {
        format!(
            r#"  - name: tracker
    schedule: daily
    settings:
      kind: connectors
      value:
        adapter: tracker
{reference}        operation: issues.list
        inputs: [{{}}]
        records: issues
        id: key
        text: ["{{key}}"]
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}"#
        )
    };
    let structured = |reference: &str| {
        format!(
            r#"  - name: people
    schedule: daily
    settings:
      kind: structured
      value:
        input:
          from: connectors
          value: {{adapter: tracker, {reference}operation: people.list, inputs: [{{}}]}}
        records: "$.people"
        mapping:
          node_type: Person
          id: "$.id"
          name: "$.name"
          aliases: []
          properties: []
          relations: []
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}"#
        )
    };
    for (what, value) in [
        (
            "connectors both",
            connectors("        connection: conn_T\n        profile: tracker.token\n"),
        ),
        ("connectors neither", connectors("")),
        (
            "structured both",
            structured("connection: conn_T, profile: tracker.token, "),
        ),
        ("structured neither", structured("")),
    ] {
        let path = source(&value);
        let (code, out, err) = cortex(
            &w,
            &[
                "create",
                "--spec",
                path.to_str().unwrap(),
                "--no-units",
                "--no-extract",
            ],
        );
        assert_eq!(code, Some(2), "{what}: {out}{err}");
        assert!(
            err.contains("connection") && err.contains("profile"),
            "{what}: {err}"
        );
        assert!(!w.home.join("instances/kinds").exists(), "{what}");
    }

    // `update` refuses both as `create` does.
    let path = source(&connectors("        profile: tracker.token\n"));
    let (code, out, err) = cortex(
        &w,
        &[
            "create",
            "--spec",
            path.to_str().unwrap(),
            "--no-units",
            "--no-extract",
        ],
    );
    assert_eq!(code, Some(0), "{out}{err}");
    let path = source(&connectors(
        "        connection: conn_T\n        profile: tracker.token\n",
    ));
    let (code, out, err) = cortex(
        &w,
        &[
            "update",
            "kinds",
            "--spec",
            path.to_str().unwrap(),
            "--no-units",
        ],
    );
    assert_eq!(code, Some(2), "update both: {out}{err}");
    let frozen = std::fs::read_to_string(w.home.join("instances/kinds/instance.yaml")).unwrap();
    assert!(!frozen.contains("conn_T"), "{frozen}");
}

/// A web source with its own `adapter` resolves its profile among that adapter's connections, never
/// the default `tavily`'s, and a profile only `tavily` lists is no match for it.
#[test]
fn adv_a_web_source_resolves_among_its_own_adapters_connections() {
    let w = World::new();
    stand_in(&w, &ekr());
    std::fs::write(w.root.join("answer-exa.json"), answer(PAGES)).unwrap();
    std::fs::write(w.root.join("answer-tavily.json"), answer(PAGES)).unwrap();
    list(&w, "tavily", &[("conn_T", "search.key", "ready")]);
    list(&w, "exa", &[("conn_X", "search.key", "ready")]);
    let spec = |name: &str, adapter: &str| {
        let path = w.spec(name, "@C@");
        let text = std::fs::read_to_string(&path)
            .unwrap()
            .replace("connection: @C@", &format!("{adapter}profile: search.key"));
        std::fs::write(&path, text).unwrap();
        path
    };
    let path = spec("exa", "adapter: exa\n        ");
    let (code, out, err) = cortex(
        &w,
        &[
            "create",
            "--spec",
            path.to_str().unwrap(),
            "--no-units",
            "--no-extract",
        ],
    );
    assert_eq!(code, Some(0), "{out}{err}");
    let (code, out, err) = cortex(&w, &["run", "exa/news"]);
    assert_eq!(code, Some(0), "{out}{err}");
    let invocations = w.lines("invocations.log");
    assert!(!invocations.is_empty());
    for line in &invocations {
        assert!(
            line.contains("--adapter exa ") && line.contains("--connection conn_X "),
            "{line}"
        );
    }

    // Only `exa` lists the profile now: a web source without `adapter` (tavily) finds 0.
    list(&w, "tavily", &[]);
    let path = spec("dflt", "");
    let (code, out, err) = cortex(
        &w,
        &[
            "create",
            "--spec",
            path.to_str().unwrap(),
            "--no-units",
            "--no-extract",
        ],
    );
    let done = last_json(&out);
    assert_eq!(
        (code, done["outcome"].as_str()),
        (Some(1), Some("connection-missing")),
        "{out}{err}"
    );
    assert_eq!(done["detail"]["connection"], "tavily:profile search.key");
    let reason = done["detail"]["reason"].as_str().unwrap_or_default();
    assert_names(reason, &["tavily", "search.key", " 0 "]);
    assert!(!reason.contains("conn_X"), "{reason}");
}

/// `pick` over list shapes the real Connectors answers: an explicit `next_cursor: null` is no
/// further page; one connection listed twice is one; no field but the ids reaches the reason.
#[test]
fn adv_pick_reads_null_cursor_duplicates_and_names_only_ids() {
    let entry = |id: &str, state: &str| {
        json!({"adapter": "tavily", "instance_id": "inst-SECRET-1", "connection": id,
               "profile": "tavily.api-key", "revision": "rev-SECRET-2", "state": state,
               "external_identity": "someone-SECRET-3"})
    };
    let listed = json!({"connections": [entry("conn_B", "ready")], "next_cursor": null});
    assert_eq!(
        pick("tavily", "tavily.api-key", &listed),
        Ok("conn_B".to_string())
    );
    let listed = json!({"connections": [entry("conn_B", "ready"), entry("conn_B", "pending")]});
    assert_eq!(
        pick("tavily", "tavily.api-key", &listed),
        Ok("conn_B".to_string())
    );
    let listed = json!({"connections": [entry("conn_B", "ready"), entry("conn_C", "disabled"),
        entry("conn_R", "revoked")]});
    let why = pick("tavily", "tavily.api-key", &listed).unwrap_err();
    assert_names(
        &why,
        &["tavily", "tavily.api-key", " 2 ", "conn_B", "conn_C"],
    );
    assert!(!why.contains("SECRET"), "{why}");
    assert!(!why.contains("conn_R"), "{why}");
    let listed = json!({"connections": [entry("conn_B", "ready")], "next_cursor": "c-SECRET-4"});
    let why = pick("tavily", "tavily.api-key", &listed).unwrap_err();
    assert!(!why.contains("SECRET"), "{why}");
}
