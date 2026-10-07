//! An instance's store lives on the backend its spec names: one SQLite file (the default), or
//! EKR's PostgreSQL provider configured by an `ekr.postgres/1` file.
//!
//! The PostgreSQL case starts a disposable PostgreSQL 17 container with TLS, an owner role for
//! schema management and a DML-only application role, as EKR 0.0.30's provider requires. It skips
//! with a named reason when no container runtime answers, except on GitHub Actions, where it fails.
//! The other cases need no database.

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::{ekr, World};
use cortex_cli::instance::Layout;
use cortex_cli::schedule::Systemd;
use serde_json::Value;

const IMAGE: &str = "postgres:17";

/// A spec with one `files` source over `docs/` and `store` written as given.
fn spec_text(name: &str, store: &str) -> String {
    format!(
        r#"format: cortex.instance/1
name: {name}
description: Test brain.
ekr: {{version: "0.0.30", bin: "{ekr}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
  - name: docs
    schedule: daily
    settings:
      kind: files
      value: {{paths: [docs], glob: "**/*.md"}}
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
serve: {{view_port: 18996}}
store: {store}
"#,
        ekr = ekr().display()
    )
}

fn write_spec(w: &World, name: &str, store: &str) -> PathBuf {
    let docs = w.root.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    std::fs::write(
        docs.join("a.md"),
        "Example Labs develops the Widget engine.\n",
    )
    .unwrap();
    std::fs::write(docs.join("b.md"), "Example Labs also builds Gadget.\n").unwrap();
    let path = w.root.join(format!("{name}.yaml"));
    std::fs::write(&path, spec_text(name, store)).unwrap();
    path
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

/// Every byte under `dir`, as text, for a search for a credential.
fn all_text(dir: &Path) -> String {
    let mut out = String::new();
    for entry in walkdir::WalkDir::new(dir).into_iter().flatten() {
        if entry.file_type().is_file() {
            out.push_str(&String::from_utf8_lossy(
                &std::fs::read(entry.path()).unwrap_or_default(),
            ));
        }
    }
    out
}

/// Why a container cannot be started here, or `None` when one can.
fn no_container_runtime() -> Option<String> {
    match Command::new("docker")
        .args(["info", "--format", "{{.ServerVersion}}"])
        .output()
    {
        Ok(out) if out.status.success() => None,
        Ok(out) => Some(format!(
            "`docker info` failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )),
        Err(e) => Some(format!("cannot run `docker`: {e}")),
    }
}

fn run(cmd: &mut Command, what: &str) -> String {
    let out = cmd
        .output()
        .unwrap_or_else(|e| panic!("cannot run {what}: {e}"));
    assert!(
        out.status.success(),
        "{what} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// The label every container of this file carries, and the one that dates it.
const LABEL: &str = "cortex.test=store-backend";
const CREATED: &str = "cortex.test.created";
/// A labelled container older than this is left over from an interrupted run.
const STALE_AFTER_S: u64 = 600;

fn now_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Removes every labelled container older than [`STALE_AFTER_S`]: `Drop` does not run when a
/// test run is killed, so the next run clears what it left.
fn sweep_stale_containers() {
    let listed = run(
        Command::new("docker").args([
            "ps",
            "-a",
            "--filter",
            &format!("label={LABEL}"),
            "--format",
            &format!("{{{{.ID}}}} {{{{.Label \"{CREATED}\"}}}}"),
        ]),
        "docker ps",
    );
    for line in listed.lines() {
        let mut parts = line.split_whitespace();
        let (Some(id), created) = (parts.next(), parts.next()) else {
            continue;
        };
        let created: u64 = created.and_then(|c| c.parse().ok()).unwrap_or(0);
        if now_s().saturating_sub(created) > STALE_AFTER_S {
            let _ = Command::new("docker").args(["rm", "-f", id]).output();
        }
    }
}

/// A disposable PostgreSQL container, removed on drop, and by the next run's sweep when a killed
/// run never dropped it.
struct Postgres {
    name: String,
    /// The application role's `ekr.postgres/1` file.
    app: PathBuf,
    /// The schema-management role's `ekr.postgres/1` file.
    owner: PathBuf,
    /// The application role's password, which must appear nowhere cortex writes.
    password: String,
    /// The published port, the CA file and the directory of the configurations.
    port: String,
    ca: PathBuf,
    cfg: PathBuf,
}

impl Drop for Postgres {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["rm", "-f", &self.name])
            .output();
    }
}

impl Postgres {
    fn start(dir: &Path) -> Self {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let name = format!("cortex-store-backend-{}-{stamp}", std::process::id());
        let tls = dir.join("tls");
        let cfg = dir.join("pg");
        std::fs::create_dir_all(&tls).unwrap();
        std::fs::create_dir_all(&cfg).unwrap();
        let openssl = |args: &[&str]| {
            run(
                Command::new("openssl").args(args).current_dir(&tls),
                "openssl",
            )
        };
        openssl(&[
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-days",
            "1",
            "-subj",
            "/CN=cortex-test-ca",
            "-keyout",
            "ca.key",
            "-out",
            "ca.crt",
        ]);
        openssl(&[
            "req",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-subj",
            "/CN=localhost",
            "-keyout",
            "server.key",
            "-out",
            "server.csr",
        ]);
        std::fs::write(
            tls.join("ext"),
            "subjectAltName=DNS:localhost,IP:127.0.0.1\n",
        )
        .unwrap();
        openssl(&[
            "x509",
            "-req",
            "-in",
            "server.csr",
            "-CA",
            "ca.crt",
            "-CAkey",
            "ca.key",
            "-CAcreateserial",
            "-days",
            "1",
            "-extfile",
            "ext",
            "-out",
            "server.crt",
        ]);
        for file in ["server.crt", "server.key"] {
            std::fs::set_permissions(tls.join(file), std::fs::Permissions::from_mode(0o644))
                .unwrap();
        }
        let mount = format!("{}:/tls:ro", tls.display());
        sweep_stale_containers();
        let created = format!("{CREATED}={}", now_s());
        run(
            Command::new("docker").args([
                "run",
                "-d",
                "--name",
                &name,
                "--label",
                LABEL,
                "--label",
                &created,
                "-e",
                "POSTGRES_PASSWORD=superuser",
                "-e",
                "POSTGRES_DB=ekr",
                "-p",
                "127.0.0.1::5432",
                "-v",
                &mount,
                IMAGE,
                "sh",
                "-c",
                "cp /tls/server.crt /tls/server.key /var/lib/postgresql/ \
                 && chown postgres /var/lib/postgresql/server.* \
                 && chmod 600 /var/lib/postgresql/server.key \
                 && exec docker-entrypoint.sh postgres -c ssl=on \
                 -c ssl_cert_file=/var/lib/postgresql/server.crt \
                 -c ssl_key_file=/var/lib/postgresql/server.key",
            ]),
            "docker run",
        );
        let mut pg = Self {
            name,
            app: cfg.join("app.json"),
            owner: cfg.join("owner.json"),
            password: format!("app-secret-{stamp}"),
            port: String::new(),
            ca: tls.join("ca.crt"),
            cfg: cfg.clone(),
        };
        // The entrypoint's initialisation server listens on no TCP port; a TCP answer is the real one.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
        while !Command::new("docker")
            .args([
                "exec",
                &pg.name,
                "pg_isready",
                "-h",
                "127.0.0.1",
                "-U",
                "postgres",
            ])
            .output()
            .is_ok_and(|o| o.status.success())
        {
            assert!(
                std::time::Instant::now() < deadline,
                "PostgreSQL did not start"
            );
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        let roles = format!(
            "CREATE ROLE ekr_owner LOGIN PASSWORD 'owner-secret'; \
             CREATE ROLE ekr_app LOGIN PASSWORD '{}' CONNECTION LIMIT 4; \
             CREATE SCHEMA ekr AUTHORIZATION ekr_owner; \
             GRANT USAGE ON SCHEMA ekr TO ekr_app; \
             ALTER DEFAULT PRIVILEGES FOR ROLE ekr_owner IN SCHEMA ekr \
               GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO ekr_app; \
             ALTER DEFAULT PRIVILEGES FOR ROLE ekr_owner IN SCHEMA ekr \
               GRANT USAGE ON SEQUENCES TO ekr_app;",
            pg.password
        );
        run(
            Command::new("docker").args([
                "exec",
                &pg.name,
                "psql",
                "-v",
                "ON_ERROR_STOP=1",
                "-h",
                "127.0.0.1",
                "-U",
                "postgres",
                "-d",
                "ekr",
                "-c",
                &roles,
            ]),
            "psql",
        );
        let port = run(
            Command::new("docker").args(["port", &pg.name, "5432/tcp"]),
            "docker port",
        );
        let port = port
            .lines()
            .next()
            .and_then(|l| l.rsplit(':').next())
            .expect("a published port")
            .trim()
            .to_string();
        for (role, password, config) in [
            ("ekr_app", pg.password.as_str(), &pg.app),
            ("ekr_owner", "owner-secret", &pg.owner),
        ] {
            let connection = config.with_extension("connection");
            std::fs::write(
                &connection,
                format!("postgres://{role}:{password}@localhost:{port}/ekr\n"),
            )
            .unwrap();
            let document = serde_json::json!({
                "format": "ekr.postgres/1",
                "connection_file": connection.file_name().unwrap().to_str().unwrap(),
                "ca_file": tls.join("ca.crt"),
                "schema": "ekr",
                "database_connections": 100,
                "replicas": 1,
                "reserved_connections": 10,
            });
            std::fs::write(config, document.to_string()).unwrap();
        }
        pg.port = port;
        pg
    }

    /// The application and schema-management `ekr.postgres/1` files of a store whose `ekr` is
    /// started by `connectors connections launch`: their connection strings carry no password,
    /// and `password_file` is descriptor 3. Each role's `{"password": …}` document is written
    /// only into `state`, the stand-in `connectors`' own directory, as `<connection>.json`.
    fn launched(&self, state: &Path) -> (PathBuf, PathBuf) {
        std::fs::create_dir_all(state).unwrap();
        let mut out = Vec::new();
        for (role, password, connection) in [
            ("ekr_app", self.password.as_str(), "app"),
            ("ekr_owner", "owner-secret", "owner"),
        ] {
            std::fs::write(
                state.join(format!("{connection}.json")),
                serde_json::json!({ "password": password }).to_string(),
            )
            .unwrap();
            let dsn = self.cfg.join(format!("{connection}-fd3.dsn"));
            std::fs::write(
                &dsn,
                format!("postgres://{role}@localhost:{}/ekr\n", self.port),
            )
            .unwrap();
            let config = self.cfg.join(format!("{connection}-fd3.json"));
            let document = serde_json::json!({
                "format": "ekr.postgres/1",
                "connection_file": dsn.file_name().unwrap().to_str().unwrap(),
                "password_file": "/proc/self/fd/3",
                "ca_file": self.ca,
                "schema": "ekr",
                "database_connections": 100,
                "replicas": 1,
                "reserved_connections": 10,
            });
            std::fs::write(&config, document.to_string()).unwrap();
            out.push(config);
        }
        (out.remove(0), out.remove(0))
    }
}

/// An `ekr` at `<root>/bin/ekr` that logs its arguments to `<root>/ekr-argv.log` and runs the real
/// one: the spec's `ekr.bin`, and the binary the stand-in `connectors` launches.
fn logging_ekr(w: &World) -> PathBuf {
    let path = w.bin.join("ekr");
    common::executable(
        &path,
        &format!(
            "printf '%s\\n' \"$*\" >> \"{root}/ekr-argv.log\"\nexec \"{ekr}\" \"$@\"\n",
            root = w.root.display(),
            ekr = ekr().display()
        ),
    );
    path
}

/// A stand-in `connectors` for a launched store, replacing the world's. It logs every argument
/// list to `<root>/connectors-argv.log`; `connections list` answers every `<state>/<id>.json` as a
/// ready connection; `connections launch --consumer ekr --args '<JSON>'` opens
/// `<state>/<connection>.json` on descriptor 3 and runs `ekr` (the operator's pinned consumer) with
/// the JSON array's elements as its arguments and only the `EKR_*` variables (`pass_env`).
fn launching_connectors(w: &World, state: &Path, ekr: &Path) {
    common::executable(
        &w.bin.join("connectors"),
        &format!(
            r#"R="{root}"; STATE="{state}"; EKR="{ekr}"
printf '%s\n' "$*" >> "$R/connectors-argv.log"
case "$1 $2" in
  "connections list")
    L=""
    for f in "$STATE"/*.json; do
      c=$(basename "$f" .json)
      L="$L{{\"adapter\":\"postgres\",\"connection\":\"$c\",\"state\":\"ready\",\"revision\":\"r1\"}},"
    done
    printf '{{"ok":true,"result":{{"connections":[%s]}}}}' "${{L%,}}" ;;
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
    [ "$CONSUMER" = ekr ] && [ -e "$STATE/$C.json" ] || {{ echo "no consumer ekr for $C" >&2; exit 2; }}
    case "$ARGS" in *\\*) echo "escapes are not handled here: $ARGS" >&2; exit 2 ;; esac
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
  *) echo '{{"ok":false}}'; exit 2 ;;
esac
"#,
            root = w.root.display(),
            state = state.display(),
            ekr = ekr.display()
        ),
    );
}

/// The store form of a launched store over `config`, through connection `app`, provisioned
/// through connection `owner`.
fn launched_store(config: &Path) -> String {
    format!(
        "{{backend: postgres, value: {{config: \"{}\", connection: {{adapter: postgres, connection: app}}, schema_connection: {{adapter: postgres, connection: owner}}}}}}",
        config.display()
    )
}

/// `ekr head` of tenant `tenant` on PostgreSQL, read by `ekr` directly: the head, or its error.
fn tenant_head(dir: &Path, tenant: &str, config: &Path) -> Result<Value, String> {
    let host = dir.join(format!("host-{tenant}.json"));
    let mut document: Value = serde_json::from_str(&run(
        Command::new(ekr()).args(["example", "ekr.cli-host/1"]),
        "ekr example",
    ))
    .unwrap();
    document["tenant"] = Value::String(tenant.into());
    std::fs::write(&host, document.to_string()).unwrap();
    let out = Command::new(ekr())
        .arg("head")
        .env("EKR_HOST", &host)
        .env("EKR_BACKEND", "postgres")
        .env("EKR_STORE", config)
        .output()
        .unwrap();
    if out.status.success() {
        Ok(serde_json::from_slice(&out.stdout).unwrap())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// `ekr head` of an instance's store on PostgreSQL, through its own host document.
fn postgres_head(dir: &Path, config: &Path) -> Value {
    let out = Command::new(ekr())
        .arg("head")
        .env("EKR_HOST", dir.join("host.json"))
        .env("EKR_BACKEND", "postgres")
        .env("EKR_STORE", config)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "ekr head: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

/// Whether a container can run here. Without one the case skips with the reason, except on
/// GitHub Actions, where it fails.
fn container_runtime(case: &str) -> bool {
    let Some(reason) = no_container_runtime() else {
        return true;
    };
    assert_ne!(
        std::env::var("GITHUB_ACTIONS").as_deref(),
        Ok("true"),
        "the PostgreSQL case must run on GitHub Actions: {reason}"
    );
    eprintln!("SKIP {case}: no container runtime ({reason})");
    false
}

fn postgres_store(pg: &Postgres) -> String {
    format!(
        "{{backend: postgres, value: {{config: \"{}\"}}}}",
        pg.app.display()
    )
}

#[test]
fn a_postgres_instance_is_provisioned_seeded_run_and_served_from_postgres() {
    if !container_runtime("the PostgreSQL store case") {
        return;
    }
    let w = World::new();
    let pg = Postgres::start(&w.root);
    // The passwords are only in the stand-in `connectors`' state directory; the configurations
    // cortex is given name descriptor 3.
    let state = w.root.join("connectors-state");
    let (app, owner) = pg.launched(&state);
    let logged_ekr = logging_ekr(&w);
    launching_connectors(&w, &state, &logged_ekr);
    let spec = write_spec(&w, "pg", &launched_store(&app));
    let text = std::fs::read_to_string(&spec).unwrap().replace(
        &format!("bin: \"{}\"", ekr().display()),
        &format!("bin: \"{}\"", logged_ekr.display()),
    );
    std::fs::write(&spec, text).unwrap();

    let (code, out, err) = cortex(
        &w,
        &[
            "create",
            "--spec",
            spec.to_str().unwrap(),
            "--postgres-schema-config",
            owner.to_str().unwrap(),
        ],
    );
    let created = last_json(&out);
    assert_eq!(
        (code, created["outcome"].as_str()),
        (Some(0), Some("created")),
        "{out}{err}"
    );
    let dir = w.home.join("instances/pg");
    assert!(
        !dir.join("store.sqlite").exists(),
        "a postgres instance wrote a SQLite store"
    );
    assert_eq!(postgres_head(&dir, &pg.app)["revision"], 0);

    let (code, out, err) = cortex(&w, &["run", "pg/docs"]);
    let ran = last_json(&out);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (Some(0), Some("ran")),
        "{out}{err}"
    );
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
    let head = postgres_head(&dir, &pg.app);
    assert!(
        head["revision"].as_i64().unwrap() > 0,
        "the run committed nothing to PostgreSQL: {head}"
    );

    // Every `ekr` that opened the store was launched through its connection: the schema through
    // `owner`, everything else through `app`.
    let launches = std::fs::read_to_string(w.root.join("connectors-argv.log")).unwrap();
    let launch = |connection: &str, args: &str| {
        format!(
            "connections launch --adapter postgres --connection {connection} --consumer ekr --args {args}"
        )
    };
    for expected in [
        launch(
            "owner",
            &format!(r#"["postgres-schema","--config","{}"]"#, owner.display()),
        ),
        launch("app", r#"["head"]"#),
        launch("app", r#"["seed","#),
        launch("app", r#"["apply-extraction","#),
    ] {
        assert!(launches.contains(&expected), "{expected} in {launches}");
    }

    // The viewer and the MCP line read the same store, through the same launch.
    let unit = std::fs::read_to_string(w.units.join("cortex-pg-view.service")).unwrap();
    for part in [
        "Environment=\"EKR_BACKEND=postgres\"".to_string(),
        format!("Environment=\"EKR_STORE={}\"", app.display()),
        format!(
            "Environment=\"CORTEX_CONNECTORS={}\"",
            w.bin.join("connectors").display()
        ),
        format!(
            r#"ExecStart="{}" connections launch --adapter "postgres" --connection "app" --consumer ekr --args "[\"view\",\"--port\",\"18996\"]""#,
            w.bin.join("connectors").display()
        ),
    ] {
        assert!(unit.contains(&part), "{part} in {unit}");
    }
    let (code, line, err) = cortex(&w, &["mcp-line", "pg"]);
    assert_eq!(code, Some(0), "{err}");
    assert!(
        line.contains(&format!(
            "EKR_BACKEND=postgres EKR_STORE={} {} connections launch --adapter postgres \
             --connection app --consumer ekr --args '[\"mcp\"]'",
            app.display(),
            w.bin.join("connectors").display()
        )),
        "{line}"
    );

    // cortex passed file references only: neither password is anywhere it wrote, nor in the
    // arguments of any process it started.
    let argv = [
        "connectors-argv.log",
        "ekr-argv.log",
        "claude-args.log",
        "systemctl.log",
    ]
    .map(|log| {
        (
            log,
            std::fs::read_to_string(w.root.join(log)).unwrap_or_default(),
        )
    });
    assert!(!argv[1].1.is_empty(), "no ekr was started");
    for (place, text) in [
        ("the home", all_text(&w.home)),
        ("the units", all_text(&w.units)),
        ("the MCP line", line),
    ]
    .into_iter()
    .chain(argv.map(|(log, text)| (log, text)))
    {
        for password in [pg.password.as_str(), "owner-secret"] {
            assert!(
                !text.contains(password),
                "the database credential is in {place}"
            );
        }
    }
}

/// A store that names a connection is refused unless its `ekr.postgres/1` file takes the password
/// from descriptor 3 and Connectors lists the connection; a schema connection needs a connection.
/// No refusal prints a value of the configuration.
#[test]
fn a_launched_postgres_store_needs_a_password_file_and_a_listed_connection() {
    const MARK: &str = "mark-3b9e";
    let w = World::new();
    let config = w.root.join("pg/app.json");
    std::fs::create_dir_all(config.parent().unwrap()).unwrap();
    let no_password_file = serde_json::json!({
        "format": "ekr.postgres/1",
        "connection_file": format!("{MARK}.dsn"),
        "schema": "ekr",
        "database_connections": 100,
        "replicas": 1,
        "reserved_connections": 10,
    });
    let store = |connection: &str| {
        format!(
            "{{backend: postgres, value: {{config: \"{}\", connection: {{adapter: postgres, connection: {connection}}}}}}}",
            config.display()
        )
    };
    let create = |name: &str, store: &str| {
        let spec = write_spec(&w, name, store);
        let (code, out, err) = cortex(
            &w,
            &["create", "--spec", spec.to_str().unwrap(), "--no-units"],
        );
        assert!(
            !out.contains(MARK) && !err.contains(MARK),
            "the refusal prints the configuration: {out}{err}"
        );
        assert!(!w.home.join("instances").join(name).exists(), "{out}{err}");
        (code, last_json(&out), format!("{out}{err}"))
    };

    // The world's `connectors` lists only `conn_test`.
    std::fs::write(&config, no_password_file.to_string()).unwrap();
    let (code, refused, all) = create("unlisted", &store("elsewhere"));
    assert_eq!(
        (code, refused["outcome"].as_str()),
        (Some(1), Some("connection-missing")),
        "{all}"
    );
    assert_eq!(
        refused["detail"]["connection"].as_str(),
        Some("postgres:elsewhere"),
        "{all}"
    );

    for (name, document) in [
        ("nofile", no_password_file.clone()),
        (
            "otherfile",
            serde_json::json!({"format": "ekr.postgres/1", "password_file": format!("/srv/{MARK}.json")}),
        ),
    ] {
        std::fs::write(&config, document.to_string()).unwrap();
        let (code, refused, all) = create(name, &store("conn_test"));
        assert_eq!(
            (code, refused["outcome"].as_str()),
            (Some(1), Some("seed-refused")),
            "{all}"
        );
        let reason = refused["detail"]["reason"].as_str().unwrap_or_default();
        assert!(
            reason.contains("store.value.config") && reason.contains("/proc/self/fd/3"),
            "{reason}"
        );
    }

    let (code, refused, all) = create(
        "schemaonly",
        &format!(
            "{{backend: postgres, value: {{config: \"{}\", schema_connection: {{adapter: postgres, connection: conn_test}}}}}}",
            config.display()
        ),
    );
    assert_eq!(
        (code, refused["outcome"].as_str()),
        (Some(1), Some("seed-refused")),
        "{all}"
    );
    let reason = refused["detail"]["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("store.value.schema_connection")
            && reason.contains("store.value.connection"),
        "{reason}"
    );
}

/// The viewer unit and the MCP line of a store that names a connection start `ekr` through
/// `connectors connections launch`, with the unit carrying the variables `connectors` needs.
#[test]
fn a_launched_store_is_viewed_and_served_through_connectors() {
    let w = World::new();
    let config = w.root.join("pg/app.json");
    let dir = w.home.join("instances/pgl");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("instance.yaml"),
        spec_text("pgl", &launched_store(&config)),
    )
    .unwrap();
    let connectors = w.bin.join("connectors");

    let (code, line, err) = cortex(&w, &["mcp-line", "pgl"]);
    assert_eq!(code, Some(0), "{err}");
    assert!(
        line.trim_end().ends_with(&format!(
            "EKR_BACKEND=postgres EKR_STORE={} {} connections launch --adapter postgres \
             --connection app --consumer ekr --args '[\"mcp\"]'",
            config.display(),
            connectors.display()
        )),
        "{line}"
    );

    let layout = Layout::new(dir.clone());
    let spec = layout.load_spec().unwrap();
    let mut store = layout.store_handle(&spec);
    store
        .launch
        .as_mut()
        .expect("a store with a connection is launched")
        .connectors = connectors.clone();
    let systemd = Systemd {
        systemctl: w.bin.join("systemctl"),
        unit_dir: w.units.clone(),
        home_root: w.home.clone(),
        taken_over: Default::default(),
    };
    systemd.install_view("pgl", &store, 18996).unwrap();
    let unit = std::fs::read_to_string(w.units.join("cortex-pgl-view.service")).unwrap();
    for part in [
        "Environment=\"HOME=".to_string(),
        "Environment=\"EKR_BACKEND=postgres\"".to_string(),
        format!("Environment=\"EKR_STORE={}\"", config.display()),
        format!("Environment=\"CORTEX_CONNECTORS={}\"", connectors.display()),
        format!(
            r#"ExecStart="{}" connections launch --adapter "postgres" --connection "app" --consumer ekr --args "[\"view\",\"--port\",\"18996\"]""#,
            connectors.display()
        ),
    ] {
        assert!(unit.contains(&part), "{part} in {unit}");
    }
    assert!(
        !unit.contains(&format!("{} view", ekr().display())),
        "{unit}"
    );
}

#[test]
fn the_viewer_unit_and_the_mcp_line_name_the_postgres_store() {
    let w = World::new();
    let config = w.root.join("pg/app.json");
    let dir = w.home.join("instances/pgline");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("instance.yaml"),
        spec_text(
            "pgline",
            &format!(
                "{{backend: postgres, value: {{config: \"{}\"}}}}",
                config.display()
            ),
        ),
    )
    .unwrap();

    let (code, line, err) = cortex(&w, &["mcp-line", "pgline"]);
    assert_eq!(code, Some(0), "{err}");
    assert!(
        line.contains(&format!(
            "EKR_BACKEND=postgres EKR_STORE={} ",
            config.display()
        )),
        "{line}"
    );

    let layout = Layout::new(dir.clone());
    let spec = layout.load_spec().unwrap();
    let systemd = Systemd {
        systemctl: w.bin.join("systemctl"),
        unit_dir: w.units.clone(),
        home_root: w.home.clone(),
        taken_over: Default::default(),
    };
    systemd
        .install_view("pgline", &layout.store_handle(&spec), 18996)
        .unwrap();
    let unit = std::fs::read_to_string(w.units.join("cortex-pgline-view.service")).unwrap();
    assert!(
        unit.contains("Environment=\"EKR_BACKEND=postgres\""),
        "{unit}"
    );
    assert!(
        unit.contains(&format!("Environment=\"EKR_STORE={}\"", config.display())),
        "{unit}"
    );
}

#[test]
fn a_sqlite_instance_keeps_its_one_file_in_the_viewer_and_the_mcp_line() {
    let w = World::new();
    let spec = write_spec(&w, "lite", "{backend: sqlite}");
    let (code, out, err) = cortex(&w, &["create", "--spec", spec.to_str().unwrap()]);
    assert_eq!(
        (code, last_json(&out)["outcome"].as_str()),
        (Some(0), Some("created")),
        "{out}{err}"
    );
    let store = w.home.join("instances/lite/store.sqlite");
    assert!(store.is_file());
    let unit = std::fs::read_to_string(w.units.join("cortex-lite-view.service")).unwrap();
    assert!(
        unit.contains("Environment=\"EKR_BACKEND=sqlite\""),
        "{unit}"
    );
    assert!(
        unit.contains(&format!("Environment=\"EKR_STORE={}\"", store.display())),
        "{unit}"
    );
    let (code, line, _) = cortex(&w, &["mcp-line", "lite"]);
    assert_eq!(code, Some(0));
    assert!(
        line.contains(&format!(
            "EKR_BACKEND=sqlite EKR_STORE={} ",
            store.display()
        )),
        "{line}"
    );
}

#[test]
fn a_relative_postgres_config_is_refused_before_anything_is_written() {
    let w = World::new();
    let spec = write_spec(&w, "rel", "{backend: postgres, value: {config: pg.json}}");
    let (code, out, err) = cortex(
        &w,
        &["create", "--spec", spec.to_str().unwrap(), "--no-units"],
    );
    let refused = last_json(&out);
    assert_eq!(
        (code, refused["outcome"].as_str()),
        (Some(1), Some("seed-refused")),
        "{out}{err}"
    );
    let reason = refused["detail"]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("store.value.config"), "{reason}");
    assert!(!w.home.join("instances/rel").exists());
}

#[test]
fn a_schema_config_for_a_sqlite_store_is_refused() {
    let w = World::new();
    let spec = write_spec(&w, "sq", "{backend: sqlite}");
    let owner = w.root.join("owner.json");
    let (code, _, err) = cortex(
        &w,
        &[
            "create",
            "--spec",
            spec.to_str().unwrap(),
            "--no-units",
            "--postgres-schema-config",
            owner.to_str().unwrap(),
        ],
    );
    assert_eq!(code, Some(2), "{err}");
    // The refusal is cortex's, not clap's refusal of an unknown flag.
    assert!(
        err.contains("--postgres-schema-config") && err.contains("store.backend: postgres"),
        "{err}"
    );
    assert!(!w.home.join("instances/sq").exists());
}

#[test]
fn an_update_that_moves_the_store_to_another_backend_is_refused() {
    let w = World::new();
    let spec = write_spec(&w, "mv", "{backend: sqlite}");
    let (code, out, err) = cortex(
        &w,
        &["create", "--spec", spec.to_str().unwrap(), "--no-units"],
    );
    assert_eq!(code, Some(0), "{out}{err}");
    let moved = write_spec(
        &w,
        "mv",
        &format!(
            "{{backend: postgres, value: {{config: \"{}\"}}}}",
            w.root.join("pg/app.json").display()
        ),
    );
    let (code, out, err) = cortex(
        &w,
        &[
            "update",
            "mv",
            "--spec",
            moved.to_str().unwrap(),
            "--no-units",
        ],
    );
    assert_eq!(
        (code, last_json(&out)["outcome"].as_str()),
        (Some(1), Some("seed-change-refused")),
        "{out}{err}"
    );
    let frozen = std::fs::read_to_string(w.home.join("instances/mv/instance.yaml")).unwrap();
    assert!(frozen.contains("backend: sqlite"), "{frozen}");
}

/// Adversary pass 1, case 1, adjusted to the coordinator's F1 decision: the second create is
/// `seed-refused` naming the tenant that already holds a store (the case asked only for "not
/// `created`"), and the first home's history is untouched.
#[test]
fn a_second_home_does_not_create_an_instance_over_another_homes_postgres_store() {
    if !container_runtime("the shared-tenant case") {
        return;
    }
    let first = World::new();
    let pg = Postgres::start(&first.root);
    let store = postgres_store(&pg);
    let spec = write_spec(&first, "pg", &store);
    let (code, out, err) = cortex(
        &first,
        &[
            "create",
            "--spec",
            spec.to_str().unwrap(),
            "--no-units",
            "--postgres-schema-config",
            pg.owner.to_str().unwrap(),
        ],
    );
    assert_eq!(code, Some(0), "{out}{err}");
    let (code, out, err) = cortex(&first, &["run", "pg/docs"]);
    assert_eq!(code, Some(0), "{out}{err}");
    let first_head = postgres_head(&first.home.join("instances/pg"), &pg.app);
    assert!(
        first_head["revision"].as_i64().unwrap_or(0) > 0,
        "{first_head}"
    );

    let second = World::new();
    let spec = write_spec(&second, "pg", &store);
    let (code, out, err) = cortex(
        &second,
        &["create", "--spec", spec.to_str().unwrap(), "--no-units"],
    );
    let refused = last_json(&out);
    assert_eq!(
        (code, refused["outcome"].as_str()),
        (Some(1), Some("seed-refused")),
        "{out}{err}"
    );
    let reason = refused["detail"]["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("tenant \"pg\"") && reason.contains("already holds a store"),
        "{reason}"
    );
    assert!(!second.home.join("instances/pg").exists());
    assert_eq!(
        postgres_head(&first.home.join("instances/pg"), &pg.app),
        first_head,
        "the refused create changed the first home's store"
    );
}

/// F6: a seed the store would refuse is found on a scratch SQLite store first, so nothing reaches
/// PostgreSQL; the corrected spec then creates the instance under the same name.
#[test]
fn a_refused_postgres_seed_leaves_no_lineage_and_a_corrected_create_succeeds() {
    if !container_runtime("the refused-seed case") {
        return;
    }
    let w = World::new();
    let pg = Postgres::start(&w.root);
    let spec = write_spec(&w, "pgbad", &postgres_store(&pg));
    let good = std::fs::read_to_string(&spec).unwrap();
    std::fs::write(
        w.root.join("bad.yaml"),
        "format: ekr.extraction-document/1\nontology:\n  node_types: []\n  edge_types:\n  - name: DEVELOPS\n    source_types: [Nowhere]\n    target_types: [Nothing]\n    cardinality: Many\n    properties: []\nentities: []\nfacts: []\nevidence: []\n",
    )
    .unwrap();
    std::fs::write(
        &spec,
        good.replace(
            "seed: {documents: []}",
            "seed: {schema: bad.yaml, documents: []}",
        ),
    )
    .unwrap();
    let (code, out, err) = cortex(
        &w,
        &[
            "create",
            "--spec",
            spec.to_str().unwrap(),
            "--no-units",
            "--postgres-schema-config",
            pg.owner.to_str().unwrap(),
        ],
    );
    let refused = last_json(&out);
    assert_eq!(
        (code, refused["outcome"].as_str()),
        (Some(1), Some("seed-refused")),
        "{out}{err}"
    );
    let reason = refused["detail"]["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("no PostgreSQL lineage was written"),
        "{reason}"
    );
    assert!(!w.home.join("instances/pgbad").exists());
    assert_eq!(
        tenant_head(&w.root, "pgbad", &pg.app),
        Err("ekr: the lineage has no seed".to_string()),
        "the refused seed left a PostgreSQL lineage"
    );

    std::fs::write(&spec, good).unwrap();
    let (code, out, err) = cortex(
        &w,
        &["create", "--spec", spec.to_str().unwrap(), "--no-units"],
    );
    assert_eq!(
        (code, last_json(&out)["outcome"].as_str()),
        (Some(0), Some("created")),
        "{out}{err}"
    );
    let dir = w.home.join("instances/pgbad");
    assert_eq!(postgres_head(&dir, &pg.app)["revision"], 0);
    let leftovers: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains("sqlite"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "the scratch store stayed: {leftovers:?}"
    );
}

/// F5: a labelled container a killed run left behind is removed by the next run's sweep.
#[test]
fn a_stale_labelled_test_container_is_swept() {
    if !container_runtime("the sweep case") {
        return;
    }
    let stale = format!("{CREATED}=0");
    let id = run(
        Command::new("docker").args(["create", "--label", LABEL, "--label", &stale, IMAGE]),
        "docker create",
    );
    let id = id.trim();
    sweep_stale_containers();
    let left = Command::new("docker")
        .args(["inspect", id])
        .output()
        .unwrap();
    let _ = Command::new("docker").args(["rm", "-f", id]).output();
    assert!(!left.status.success(), "the stale container {id} stayed");
}

/// Adversary pass 1, case 2, adjusted to the coordinator's F2 decision: a sqlite `value.path` is
/// refused with `seed-refused` saying it is not supported yet (the case also admitted a store at
/// that path).
#[test]
fn a_sqlite_store_path_the_spec_names_is_refused_as_not_supported_yet() {
    let w = World::new();
    let path = w.root.join("elsewhere/brain.sqlite");
    let spec = write_spec(
        &w,
        "lite",
        &format!(
            "{{backend: sqlite, value: {{path: \"{}\"}}}}",
            path.display()
        ),
    );
    let (code, out, err) = cortex(
        &w,
        &["create", "--spec", spec.to_str().unwrap(), "--no-units"],
    );
    let refused = last_json(&out);
    assert_eq!(
        (code, refused["outcome"].as_str()),
        (Some(1), Some("seed-refused")),
        "{out}{err}"
    );
    let reason = refused["detail"]["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("store.value.path") && reason.contains("not supported yet"),
        "{reason}"
    );
    assert!(!w.home.join("instances/lite").exists(), "{out}{err}");
    assert!(!path.exists());
}

/// Adversary pass 1, case 3, adjusted to the coordinator's F3 decision: the update is
/// `seed-change-refused` (update declares no `seed-refused`) with a reason naming
/// `store.value.path`, and the frozen spec is unchanged (the case asked only for "not `updated`").
#[test]
fn an_update_that_names_another_sqlite_file_is_refused() {
    let w = World::new();
    let spec = write_spec(&w, "mv", "{backend: sqlite}");
    let (code, out, err) = cortex(
        &w,
        &["create", "--spec", spec.to_str().unwrap(), "--no-units"],
    );
    assert_eq!(code, Some(0), "{out}{err}");
    let before = std::fs::read_to_string(w.home.join("instances/mv/instance.yaml")).unwrap();
    let moved = write_spec(
        &w,
        "mv",
        &format!(
            "{{backend: sqlite, value: {{path: \"{}\"}}}}",
            w.root.join("elsewhere.sqlite").display()
        ),
    );
    let (code, out, err) = cortex(
        &w,
        &[
            "update",
            "mv",
            "--spec",
            moved.to_str().unwrap(),
            "--no-units",
        ],
    );
    let refused = last_json(&out);
    assert_eq!(
        (code, refused["outcome"].as_str()),
        (Some(1), Some("seed-change-refused")),
        "{out}{err}"
    );
    let reason = refused["detail"]["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("store.value.path") && reason.contains("not supported yet"),
        "{reason}"
    );
    let frozen = std::fs::read_to_string(w.home.join("instances/mv/instance.yaml")).unwrap();
    assert_eq!(frozen, before);
}

/// Adversary pass 1, case 4, adjusted to the coordinator's F4 decision: besides not printing the
/// password, the refusal names the field `store.value.config`.
#[test]
fn a_connection_string_in_the_config_field_is_not_printed_back() {
    let w = World::new();
    let password = &["adversary", "pw", "0e1f"].join("-");
    let spec = write_spec(
        &w,
        "leak",
        &format!(
            "{{backend: postgres, value: {{config: \"postgres://ekr_app:{password}@db.example:5432/ekr\"}}}}"
        ),
    );
    let (code, out, err) = cortex(
        &w,
        &["create", "--spec", spec.to_str().unwrap(), "--no-units"],
    );
    assert_ne!(code, Some(0), "{out}{err}");
    assert!(
        !out.contains(password) && !err.contains(password),
        "the refusal prints the database password: stdout {out} stderr {err}"
    );
    assert!(out.contains("store.value.config"), "{out}{err}");
}

/// F4, the class: no store refusal, at create or at update, prints the value it refuses.
#[test]
fn no_store_refusal_prints_the_value_it_refuses() {
    const MARK: &str = "mark-7c1d";
    let forms = [
        format!("{{backend: postgres, value: {{config: \"{MARK}.json\"}}}}"),
        format!("{{backend: postgres, value: {{config: \"postgres://u:{MARK}@h/db\"}}}}"),
        format!("{{backend: sqlite, value: {{path: \"/srv/{MARK}.sqlite\"}}}}"),
        format!("{{backend: postgres, value: {{config: \"/etc/{MARK}.json\"}}}}"),
    ];
    for (i, form) in forms.iter().enumerate() {
        let w = World::new();
        let name = format!("r{i}");
        // At create: the last form is a valid spec whose store cannot be opened here.
        let spec = write_spec(&w, &name, form);
        let (code, out, err) = cortex(
            &w,
            &["create", "--spec", spec.to_str().unwrap(), "--no-units"],
        );
        assert_ne!(code, Some(0), "{form}: {out}{err}");
        assert!(
            !out.contains(MARK) && !err.contains(MARK),
            "create prints the refused value of {form}: {out}{err}"
        );
        // At update, from a SQLite instance.
        let spec = write_spec(&w, &name, "{backend: sqlite}");
        let (code, out, err) = cortex(
            &w,
            &["create", "--spec", spec.to_str().unwrap(), "--no-units"],
        );
        assert_eq!(code, Some(0), "{out}{err}");
        let spec = write_spec(&w, &name, form);
        let (code, out, err) = cortex(
            &w,
            &[
                "update",
                &name,
                "--spec",
                spec.to_str().unwrap(),
                "--no-units",
            ],
        );
        assert_eq!(code, Some(1), "{form}: {out}{err}");
        assert!(
            !out.contains(MARK) && !err.contains(MARK),
            "update prints the refused value of {form}: {out}{err}"
        );
    }
}

// ADVERSARY CASES (wave 20261005c, U1, pass 2). Each was run alone, and red, before the suite ran.

/// The tenant check (`ekr head`) and the seed are two `ekr` calls with nothing held between them.
/// Home B's `ekr` is a wrapper that lets home A create the same instance on the same schema after
/// B's `ekr head` answered "no seed" and before B seeds. `ekr seed` of an identical seed answers
/// the first seed's event with exit 0, so B must not report `created` over A's tenant.
#[test]
fn two_homes_racing_for_one_postgres_tenant_are_not_both_created() {
    if !container_runtime("the racing-homes case") {
        return;
    }
    let a = World::new();
    let pg = Postgres::start(&a.root);
    let store = postgres_store(&pg);
    run(
        Command::new(ekr())
            .args(["postgres-schema", "--config"])
            .arg(&pg.owner),
        "ekr postgres-schema",
    );
    let a_spec = write_spec(&a, "pg", &store);

    let b = World::new();
    let wrapper = b.root.join("ekr-race");
    let marker = b.root.join("raced");
    let a_log = b.root.join("a-create.out");
    common::executable(
        &wrapper,
        &format!(
            r#"if [ "$1" = head ] && [ ! -e "{marker}" ]; then
  : > "{marker}"
  out=$("{ekr}" "$@" 2>"{marker}.err"); code=$?
  env -u EKR_HOST -u EKR_BACKEND -u EKR_STORE CORTEX_HOME="{home}" CORTEX_CONNECTORS="{bin}/connectors" CORTEX_CLAUDE="{bin}/claude" CORTEX_SYSTEMCTL="{bin}/systemctl" CORTEX_UNIT_DIR="{units}" "{cortex}" create --spec "{spec}" --no-units > "{log}" 2>&1
  printf '%s' "$out"; cat "{marker}.err" >&2; exit $code
fi
exec "{ekr}" "$@"
"#,
            marker = marker.display(),
            ekr = ekr().display(),
            home = a.home.display(),
            bin = a.bin.display(),
            units = a.units.display(),
            cortex = env!("CARGO_BIN_EXE_cortex"),
            spec = a_spec.display(),
            log = a_log.display(),
        ),
    );
    let b_spec = write_spec(&b, "pg", &store);
    let text = std::fs::read_to_string(&b_spec).unwrap().replace(
        &format!("bin: \"{}\"", ekr().display()),
        &format!("bin: \"{}\"", wrapper.display()),
    );
    std::fs::write(&b_spec, text).unwrap();

    let (code, out, err) = cortex(
        &b,
        &["create", "--spec", b_spec.to_str().unwrap(), "--no-units"],
    );
    let a_out = std::fs::read_to_string(&a_log).unwrap_or_default();
    assert_eq!(
        last_json(&a_out)["outcome"].as_str(),
        Some("created"),
        "home A's create did not run inside B's window: {a_out}"
    );
    assert_ne!(
        (code, last_json(&out)["outcome"].as_str()),
        (Some(0), Some("created")),
        "both homes report `created` for tenant \"pg\" on one schema: A {a_out} B {out}{err}"
    );
}

/// `update` computes `store_refusal` and then ignores it when the frozen spec cannot be read
/// (`Err(_) => false`), so a relative PostgreSQL config that create refuses is frozen by update.
#[test]
fn an_update_does_not_freeze_a_refused_store_when_the_frozen_spec_is_unreadable() {
    let w = World::new();
    let spec = write_spec(&w, "lost", "{backend: sqlite}");
    let (code, out, err) = cortex(
        &w,
        &["create", "--spec", spec.to_str().unwrap(), "--no-units"],
    );
    assert_eq!(code, Some(0), "{out}{err}");
    let frozen = w.home.join("instances/lost/instance.yaml");
    std::fs::remove_file(&frozen).unwrap();
    let relative = write_spec(&w, "lost", "{backend: postgres, value: {config: pg.json}}");
    let (code, out, err) = cortex(
        &w,
        &[
            "update",
            "lost",
            "--spec",
            relative.to_str().unwrap(),
            "--no-units",
        ],
    );
    let now = std::fs::read_to_string(&frozen).unwrap_or_default();
    assert!(
        code != Some(0) && !now.contains("config: pg.json"),
        "update accepted and froze a store create refuses: exit {code:?} {out}{err} frozen:\n{now}"
    );
}

// Security review, wave 20261006i unit a: conformance checks of the launched-store contract that
// need no database. A stand-in `ekr` answers a `postgres` store from a SQLite file.

/// An `ekr` at `<bin>/ekr-pg` that answers a `postgres` store from `<root>/pg-as-sqlite.sqlite`
/// through the real `ekr`: `head` before that file exists answers PostgreSQL's "the lineage has no
/// seed", and `postgres-schema` answers ready. Every invocation is logged to `<root>/ekr-argv.log`
/// as `<EKR_BACKEND>|<arguments>`.
fn sqlite_backed_ekr(w: &World) -> PathBuf {
    let path = w.bin.join("ekr-pg");
    common::executable(
        &path,
        &format!(
            r#"R="{root}"; EKR="{ekr}"; DB="$R/pg-as-sqlite.sqlite"
printf '%s|%s\n' "$EKR_BACKEND" "$*" >> "$R/ekr-argv.log"
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

/// [`write_spec`] with `ekr.bin` replaced by `bin`.
fn spec_with_ekr(w: &World, name: &str, store: &str, bin: &Path) -> PathBuf {
    let spec = write_spec(w, name, store);
    let text = std::fs::read_to_string(&spec).unwrap().replace(
        &format!("bin: \"{}\"", ekr().display()),
        &format!("bin: \"{}\"", bin.display()),
    );
    std::fs::write(&spec, text).unwrap();
    spec
}

/// An `ekr.postgres/1` file at `path` whose `password_file` is `password_file`.
fn pg_config(path: &Path, password_file: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let document = serde_json::json!({
        "format": "ekr.postgres/1",
        "connection_file": "app.dsn",
        "password_file": password_file,
        "schema": "ekr",
        "database_connections": 100,
        "replicas": 1,
        "reserved_connections": 10,
    });
    std::fs::write(path, document.to_string()).unwrap();
}

/// A fake database password, assembled at run time.
fn review_password() -> String {
    [
        "review",
        "pw",
        &std::process::id().to_string(),
        &now_s().to_string(),
    ]
    .join("-")
}

fn direct_store(config: &Path) -> String {
    format!(
        "{{backend: postgres, value: {{config: \"{}\"}}}}",
        config.display()
    )
}

/// A store launched through connection `app`, with no schema connection.
fn app_launched_store(config: &Path) -> String {
    format!(
        "{{backend: postgres, value: {{config: \"{}\", connection: {{adapter: postgres, connection: app}}}}}}",
        config.display()
    )
}

/// The world's `systemd`, for calling `install_view` directly.
fn world_systemd(w: &World) -> Systemd {
    Systemd {
        systemctl: w.bin.join("systemctl"),
        unit_dir: w.units.clone(),
        home_root: w.home.clone(),
        taken_over: Default::default(),
    }
}

/// One `ExecStart=` word as systemd.syntax(7) reads it back as `value`: inside double quotes `\\`
/// and `\"` are unescaped, and in `ExecStart=` `%` starts a specifier and `$` a variable, so the
/// doubled `%%` and `$$` stand for themselves.
fn systemd_word(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('%', "%%")
        .replace('$', "$$");
    format!("\"{escaped}\"")
}

/// Invariants 2 and 6: an update that gives an existing store a `connection` leaves no `ekr` of
/// the store started without `connectors connections launch`. Runs pick the launch up; the viewer
/// unit `create` wrote must too.
#[test]
fn review_an_update_that_names_a_connection_starts_the_viewer_through_the_launch() {
    let w = World::new();
    let ekr_pg = sqlite_backed_ekr(&w);
    let config = w.root.join("pg/app.json");
    let password = review_password();
    let held = w.root.join("pg/password.txt");
    std::fs::create_dir_all(held.parent().unwrap()).unwrap();
    std::fs::write(&held, &password).unwrap();
    pg_config(&config, held.to_str().unwrap());
    let spec = spec_with_ekr(&w, "pg", &direct_store(&config), &ekr_pg);
    let (code, out, err) = cortex(
        &w,
        &["create", "--spec", spec.to_str().unwrap(), "--no-extract"],
    );
    assert_eq!(code, Some(0), "{out}{err}");
    let unit_path = w.units.join("cortex-pg-view.service");
    let before = std::fs::read_to_string(&unit_path).unwrap();
    assert!(before.contains(" view --port 18996"), "{before}");

    // The operator moves the password into Connectors and names the connection.
    let state = w.root.join("connectors-state");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(
        state.join("app.json"),
        serde_json::json!({ "password": password }).to_string(),
    )
    .unwrap();
    std::fs::remove_file(&held).unwrap();
    pg_config(&config, "/proc/self/fd/3");
    launching_connectors(&w, &state, &ekr_pg);
    let spec = spec_with_ekr(&w, "pg", &app_launched_store(&config), &ekr_pg);
    let (code, out, err) = cortex(&w, &["update", "pg", "--spec", spec.to_str().unwrap()]);
    assert_eq!(
        (code, last_json(&out)["outcome"].as_str()),
        (Some(0), Some("updated")),
        "{out}{err}"
    );
    let (code, out, err) = cortex(&w, &["run", "pg/docs"]);
    assert_eq!(code, Some(0), "{out}{err}");
    let launches = std::fs::read_to_string(w.root.join("connectors-argv.log")).unwrap_or_default();
    assert!(
        launches.contains("--connection app --consumer ekr --args [\"apply-extraction\""),
        "a run after the update did not launch ekr: {launches}"
    );

    let unit = std::fs::read_to_string(&unit_path).unwrap();
    assert!(
        unit.contains("connections launch"),
        "after the update names store.value.connection, the viewer unit still starts ekr \
         without the launch:\n{unit}"
    );
}

/// Invariant 6, back: an update that drops the `connection` leaves no password in anything cortex
/// wrote, and the viewer unit no longer goes through a connection the spec no longer names.
#[test]
fn review_an_update_that_drops_the_connection_starts_the_viewer_without_the_launch() {
    let w = World::new();
    let ekr_pg = sqlite_backed_ekr(&w);
    let config = w.root.join("pg/app.json");
    let password = review_password();
    let state = w.root.join("connectors-state");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(
        state.join("app.json"),
        serde_json::json!({ "password": password }).to_string(),
    )
    .unwrap();
    pg_config(&config, "/proc/self/fd/3");
    launching_connectors(&w, &state, &ekr_pg);
    let spec = spec_with_ekr(&w, "pg", &app_launched_store(&config), &ekr_pg);
    let (code, out, err) = cortex(
        &w,
        &["create", "--spec", spec.to_str().unwrap(), "--no-extract"],
    );
    assert_eq!(code, Some(0), "{out}{err}");
    let unit_path = w.units.join("cortex-pg-view.service");
    let before = std::fs::read_to_string(&unit_path).unwrap();
    assert!(before.contains("connections launch"), "{before}");

    // The operator takes the store back to a password file of its own.
    let held = w.root.join("pg/password.txt");
    std::fs::write(&held, &password).unwrap();
    pg_config(&config, held.to_str().unwrap());
    let spec = spec_with_ekr(&w, "pg", &direct_store(&config), &ekr_pg);
    let (code, out, err) = cortex(&w, &["update", "pg", "--spec", spec.to_str().unwrap()]);
    assert_eq!(
        (code, last_json(&out)["outcome"].as_str()),
        (Some(0), Some("updated")),
        "{out}{err}"
    );
    let logs = ["connectors-argv.log", "ekr-argv.log", "systemctl.log"].map(|log| {
        (
            log,
            std::fs::read_to_string(w.root.join(log)).unwrap_or_default(),
        )
    });
    for (place, text) in [
        ("the home", all_text(&w.home)),
        ("the units", all_text(&w.units)),
    ]
    .into_iter()
    .chain(logs)
    {
        assert!(
            !text.contains(&password),
            "the database password is in {place}"
        );
    }

    let unit = std::fs::read_to_string(&unit_path).unwrap();
    assert!(
        !unit.contains("connections launch") && unit.contains(" view --port 18996"),
        "after the update drops store.value.connection, the viewer unit still launches through \
         it:\n{unit}"
    );
}

/// Invariant 2, as the coordinator decided F3 (wave 20261006i): with `connection` set, `cortex
/// create --postgres-schema-config` starts `ekr postgres-schema` through the launch of
/// `schema_connection` when the spec names one, and without it starts it directly with the
/// operator's own schema-role file, the one direct start `operating.md` documents.
#[test]
fn review_provisioning_a_store_with_a_connection_launches_the_schema_ekr() {
    for schema_connection in [true, false] {
        let w = World::new();
        let ekr_pg = sqlite_backed_ekr(&w);
        let config = w.root.join("pg/app.json");
        let owner = w.root.join("pg/owner.json");
        let state = w.root.join("connectors-state");
        std::fs::create_dir_all(&state).unwrap();
        for connection in ["app", "owner"] {
            std::fs::write(
                state.join(format!("{connection}.json")),
                serde_json::json!({ "password": review_password() }).to_string(),
            )
            .unwrap();
        }
        pg_config(&config, "/proc/self/fd/3");
        pg_config(&owner, "/proc/self/fd/3");
        launching_connectors(&w, &state, &ekr_pg);
        let store = if schema_connection {
            launched_store(&config)
        } else {
            app_launched_store(&config)
        };
        let spec = spec_with_ekr(&w, "pg", &store, &ekr_pg);
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
        assert_eq!(code, Some(0), "{out}{err}");
        let ekrs = std::fs::read_to_string(w.root.join("ekr-argv.log")).unwrap_or_default();
        let schema_args = format!("postgres-schema --config {}", owner.display());
        assert!(ekrs.contains(&schema_args), "{ekrs}");
        let launches =
            std::fs::read_to_string(w.root.join("connectors-argv.log")).unwrap_or_default();
        let launched = format!(
            "connections launch --adapter postgres --connection owner --consumer ekr --args \
             [\"postgres-schema\",\"--config\",\"{}\"]",
            owner.display()
        );
        if schema_connection {
            assert!(
                launches.contains(&launched),
                "with store.value.schema_connection, ekr postgres-schema started without the \
                 launch.\nlaunches:\n{launches}\nekr invocations:\n{ekrs}"
            );
        } else {
            assert!(
                !launches.contains("postgres-schema"),
                "without store.value.schema_connection, ekr postgres-schema went through a \
                 launch.\nlaunches:\n{launches}"
            );
            // Started directly: no store variables, so the stand-in logs an empty backend.
            assert!(ekrs.contains(&format!("|{schema_args}")), "{ekrs}");
        }
    }
}

/// Invariant 3: the MCP line is a shell command; the launch's words in it must stay one word each
/// when the shell reads it, a `connectors` path with a space included.
#[test]
fn review_the_mcp_line_keeps_a_connectors_path_with_a_space_one_word() {
    let w = World::new();
    let config = w.root.join("pg/app.json");
    let dir = w.home.join("instances/pgl");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("instance.yaml"),
        spec_text("pgl", &app_launched_store(&config)),
    )
    .unwrap();
    let connectors = w.root.join("my tools/connectors");
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["mcp-line", "pgl"])
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", &connectors)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let line = String::from_utf8_lossy(&out.stdout).trim().to_string();

    let stub = w.root.join("mcp-bin");
    std::fs::create_dir_all(&stub).unwrap();
    common::executable(
        &stub.join("claude"),
        &format!(
            "printf '%s\\n' \"$@\" > \"{}/mcp-argv.log\"\n",
            w.root.display()
        ),
    );
    let path = format!(
        "{}:{}",
        stub.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let status = Command::new("sh")
        .arg("-c")
        .arg(&line)
        .env("PATH", path)
        .status()
        .unwrap();
    assert!(status.success(), "{line}");
    let argv: Vec<String> = std::fs::read_to_string(w.root.join("mcp-argv.log"))
        .unwrap()
        .lines()
        .map(String::from)
        .collect();
    assert!(
        argv.iter().any(|a| a == connectors.to_str().unwrap()),
        "the shell splits the connectors path of the MCP line: {argv:?}\nline: {line}"
    );
}

/// Invariant 4, a store without a connection: the viewer's `ExecStart` keeps an `ekr` path with a
/// space, a double quote and a `%` one word, read back as written.
#[test]
fn review_the_direct_viewer_exec_start_quotes_the_ekr_path() {
    let w = World::new();
    let bin = w.root.join("e k\"r%h/ekr");
    let store = cortex_cli::ekr::Store {
        bin: bin.clone(),
        host: w.root.join("host.json"),
        backend: cortex_cli::ekr::Backend::Postgres,
        store: w.root.join("pg/app.json"),
        launch: None,
    };
    world_systemd(&w).install_view("q", &store, 18996).unwrap();
    let unit = std::fs::read_to_string(w.units.join("cortex-q-view.service")).unwrap();
    let expected = format!(
        "ExecStart={} view --port 18996\n",
        systemd_word(bin.to_str().unwrap())
    );
    assert!(unit.contains(&expected), "{expected} in\n{unit}");
}

/// Invariant 4, a launched store: every word of the launch in `ExecStart` is quoted so systemd
/// reads it back as written, with spaces, quotes, `%`, `$` and `\` in it.
#[test]
fn review_the_launched_viewer_exec_start_quotes_every_word() {
    let w = World::new();
    let connectors = w.root.join("c o\"n%h$X\\y/connectors");
    let store = cortex_cli::ekr::Store {
        bin: w.root.join("ekr"),
        host: w.root.join("host.json"),
        backend: cortex_cli::ekr::Backend::Postgres,
        store: w.root.join("pg/app.json"),
        launch: Some(cortex_cli::ekr::Launch {
            connectors: connectors.clone(),
            connection: cortex_cli::ekr::Connection {
                adapter: "pg $A".into(),
                connection: "a\"b %i".into(),
            },
            schema_connection: None,
        }),
    };
    world_systemd(&w).install_view("q", &store, 18996).unwrap();
    let unit = std::fs::read_to_string(w.units.join("cortex-q-view.service")).unwrap();
    let expected = format!(
        "ExecStart={} connections launch --adapter {} --connection {} --consumer ekr --args {}\n",
        systemd_word(connectors.to_str().unwrap()),
        systemd_word("pg $A"),
        systemd_word("a\"b %i"),
        systemd_word(r#"["view","--port","18996"]"#),
    );
    assert!(unit.contains(&expected), "{expected} in\n{unit}");
}

/// The marker [`fd3_logging_ekr`] writes for a variable that is not set.
const UNSET: &str = "<unset>";

/// An `ekr` at `<bin>/ekr-fd3` that logs every invocation to `<root>/ekr-fd3.log` as
/// `<EKR_BACKEND>|<what descriptor 3 is>|<EKR_REVIEW_INHERITED>|<arguments>`, then runs
/// [`sqlite_backed_ekr`]. Descriptor 3 is named with `readlink` and never read, so a descriptor
/// the test runner left open is named and not consumed.
fn fd3_logging_ekr(w: &World) -> PathBuf {
    let inner = sqlite_backed_ekr(w);
    let path = w.bin.join("ekr-fd3");
    common::executable(
        &path,
        &format!(
            r#"L=$(/usr/bin/readlink /proc/$$/fd/3 2>/dev/null || echo none)
printf '%s|%s|%s|%s\n' "$EKR_BACKEND" "$L" "${{EKR_REVIEW_INHERITED-{UNSET}}}" "$*" >> "{root}/ekr-fd3.log"
exec "{inner}" "$@"
"#,
            root = w.root.display(),
            inner = inner.display()
        ),
    );
    path
}

/// A world whose store is launched through connection `app` (and `owner`, held for a schema
/// connection), every `ekr` of it [`fd3_logging_ekr`]: the application config, the connectors
/// state directory and that `ekr`.
fn fd3_world(w: &World) -> (PathBuf, PathBuf, PathBuf) {
    let ekr = fd3_logging_ekr(w);
    let config = w.root.join("pg/app.json");
    let state = w.root.join("connectors-state");
    std::fs::create_dir_all(&state).unwrap();
    for connection in ["app", "owner"] {
        std::fs::write(
            state.join(format!("{connection}.json")),
            serde_json::json!({ "password": review_password() }).to_string(),
        )
        .unwrap();
    }
    pg_config(&config, "/proc/self/fd/3");
    launching_connectors(w, &state, &ekr);
    (config, state, ekr)
}

/// The `postgres` invocations of `<root>/ekr-fd3.log`: (descriptor 3, inherited variable,
/// arguments).
fn store_invocations(w: &World) -> (String, Vec<(String, String, String)>) {
    let log = std::fs::read_to_string(w.root.join("ekr-fd3.log")).unwrap_or_default();
    let lines = log
        .lines()
        .filter_map(|l| {
            let mut p = l.splitn(4, '|');
            let (backend, fd3, inherited, args) = (p.next()?, p.next()?, p.next()?, p.next()?);
            (backend == "postgres").then(|| (fd3.into(), inherited.into(), args.into()))
        })
        .collect();
    (log, lines)
}

/// Invariant 4 at the binary: every `ekr` that opens a launched store, `create`'s, a run's and its
/// gate's, `quality`'s and `schema`'s, holds the connection's document on descriptor 3, which only
/// `connectors connections launch` opens there.
#[test]
fn security_every_ekr_that_opens_a_launched_store_holds_the_connection_on_descriptor_3() {
    let w = World::new();
    let (config, state, ekr) = fd3_world(&w);
    let spec = spec_with_ekr(&w, "pg", &app_launched_store(&config), &ekr);
    // A gate whose measure only `ekr quality` answers.
    let mut text = std::fs::read_to_string(&spec).unwrap();
    text.push_str("gate:\n  checks:\n    - {measure: assertions.active, min: \"0\"}\n");
    std::fs::write(&spec, text).unwrap();
    let (code, out, err) = cortex(
        &w,
        &[
            "create",
            "--spec",
            spec.to_str().unwrap(),
            "--no-units",
            "--no-extract",
        ],
    );
    assert_eq!(code, Some(0), "{out}{err}");
    let (code, out, err) = cortex(&w, &["run", "pg/docs"]);
    assert_eq!(code, Some(0), "{out}{err}");
    // The stand-in model answers neither form; only the store reads before it count here.
    let _ = cortex(&w, &["quality", "pg", "--sample", "1"]);
    let _ = cortex(&w, &["schema", "pg", "--sample", "1", "--dry-run"]);

    let (log, opened) = store_invocations(&w);
    for verb in ["head", "seed", "apply-extraction", "quality", "sample"] {
        assert!(
            opened
                .iter()
                .any(|(_, _, args)| args.split(' ').next() == Some(verb)),
            "no ekr {verb} opened the store:\n{log}"
        );
    }
    let held = state.join("app.json");
    for (fd3, _, args) in &opened {
        assert_eq!(
            Path::new(fd3),
            held.as_path(),
            "ekr {args} opened the store without the launch:\n{log}"
        );
    }
}

/// Invariant 2: only the `EKR_*` variables cortex sets reach a launched `ekr`. The stand-in
/// `connectors` passes its own `EKR_*` environment to the consumer, as `pass_env = ["EKR_"]` does,
/// so an `EKR_*` variable cortex inherited from the operator's shell must not be in what cortex
/// hands the launch. EKR 0.0.32 reads `EKR_FULL_REPLAY` besides the three cortex sets.
#[test]
fn security_an_inherited_ekr_variable_does_not_reach_a_launched_ekr() {
    let w = World::new();
    let (config, _, ekr) = fd3_world(&w);
    let spec = spec_with_ekr(&w, "pg", &app_launched_store(&config), &ekr);
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args([
            "create",
            "--spec",
            spec.to_str().unwrap(),
            "--no-units",
            "--no-extract",
        ])
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .env("EKR_REVIEW_INHERITED", "from-the-operator-shell")
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let (log, opened) = store_invocations(&w);
    assert!(!opened.is_empty(), "no ekr opened the store:\n{log}");
    for (_, inherited, args) in &opened {
        assert_eq!(
            inherited, UNSET,
            "EKR_REVIEW_INHERITED, which cortex did not set, reached the launched ekr {args}:\n{log}"
        );
    }
}

/// Invariant 5, `update`: a launched store's `ekr.postgres/1` that names the descriptor and also
/// carries a `password` is refused, and the refusal does not print it. `update` starts no `ekr`,
/// so nothing but cortex's own check can refuse it.
#[test]
fn security_update_refuses_a_launched_config_that_also_carries_a_password() {
    const MARK: &str = "pw-mark-7c41";
    let w = World::new();
    let (config, _, ekr) = fd3_world(&w);
    let spec = spec_with_ekr(&w, "pg", &app_launched_store(&config), &ekr);
    let (code, out, err) = cortex(
        &w,
        &[
            "create",
            "--spec",
            spec.to_str().unwrap(),
            "--no-units",
            "--no-extract",
        ],
    );
    assert_eq!(code, Some(0), "{out}{err}");

    // The operator writes the password into the configuration as well.
    let mut document: Value =
        serde_json::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();
    document["password"] = Value::String(MARK.into());
    std::fs::write(&config, document.to_string()).unwrap();
    let (code, out, err) = cortex(
        &w,
        &[
            "update",
            "pg",
            "--spec",
            spec.to_str().unwrap(),
            "--no-units",
        ],
    );
    assert!(
        !out.contains(MARK) && !err.contains(MARK),
        "the answer prints the password: {out}{err}"
    );
    assert_eq!(
        (code, last_json(&out)["outcome"].as_str()),
        (Some(1), Some("seed-change-refused")),
        "an ekr.postgres/1 with store.value.connection and a password was accepted: {out}{err}"
    );
}

/// Invariant 5, the schema connection: `create --postgres-schema-config <file>` with
/// `store.value.schema_connection` launches `ekr postgres-schema` with the schema connection's
/// document on descriptor 3, so a `<file>` that does not name `"password_file": "/proc/self/fd/3"`
/// is refused before anything runs, as `store.value.config` is.
#[test]
fn security_create_refuses_a_schema_config_that_takes_no_password_from_the_launch() {
    let w = World::new();
    let (config, _, ekr) = fd3_world(&w);
    // The schema role's file takes its password from a file of its own, not from the launch.
    let owner = w.root.join("pg/owner.json");
    let held = w.root.join("pg/owner-password.txt");
    std::fs::write(&held, review_password()).unwrap();
    pg_config(&owner, held.to_str().unwrap());
    let spec = spec_with_ekr(&w, "pg", &launched_store(&config), &ekr);
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
    let (log, _) = store_invocations(&w);
    assert_eq!(
        (code, last_json(&out)["outcome"].as_str()),
        (Some(1), Some("seed-refused")),
        "a schema config without password_file was launched through \
         store.value.schema_connection: {out}{err}\nekr:\n{log}"
    );
}

/// Invariant 6: a store without `connection` behaves as before. Before this unit `update`
/// installed only the source units; the viewer unit is rewritten only "when an update adds or
/// drops the connection" (`website/docs/operating.md`), and a viewer the operator stopped is left
/// stopped (`Systemd::restart_view`). An update of a SQLite instance that changes no store must not
/// enable and start its viewer.
#[test]
fn security_an_update_of_a_store_without_a_connection_leaves_the_viewer_alone() {
    let w = World::new();
    let spec = write_spec(&w, "lite", "{backend: sqlite}");
    let (code, out, err) = cortex(
        &w,
        &["create", "--spec", spec.to_str().unwrap(), "--no-extract"],
    );
    assert_eq!(code, Some(0), "{out}{err}");
    let log = w.root.join("systemctl.log");
    assert!(
        std::fs::read_to_string(&log)
            .unwrap()
            .contains("enable --now cortex-lite-view.service"),
        "create starts the viewer"
    );
    // The operator stops and disables the viewer; the stand-in only records calls.
    std::fs::write(&log, "").unwrap();
    let (code, out, err) = cortex(&w, &["update", "lite", "--spec", spec.to_str().unwrap()]);
    assert_eq!(
        (code, last_json(&out)["outcome"].as_str()),
        (Some(0), Some("updated")),
        "{out}{err}"
    );
    let calls = std::fs::read_to_string(&log).unwrap();
    assert!(
        !calls.contains("cortex-lite-view.service"),
        "an update that changes no store enabled and started the viewer:\n{calls}"
    );
}

/// Invariant 3 at the shell: the MCP line of a launched store is read back by `sh` word for word
/// with quotes, spaces, a newline, `%`, `$`, `$(…)`, a backtick, a backslash and a leading `-` in
/// the adapter alias and the connection id. A stand-in `claude` records its arguments
/// NUL-separated.
#[test]
fn security_the_mcp_line_keeps_hostile_connection_words_one_word_each() {
    let w = World::new();
    let config = w.root.join("pg/app.json");
    let dir = w.home.join("instances/pgl");
    std::fs::create_dir_all(&dir).unwrap();
    let adapter = "-a'd $HOME \\ \"q\" %i";
    let connection = "-c\nnext; $(id) `id`";
    let store = serde_json::json!({
        "backend": "postgres",
        "value": {"config": config, "connection": {"adapter": adapter, "connection": connection}},
    });
    std::fs::write(
        dir.join("instance.yaml"),
        spec_text("pgl", &store.to_string()),
    )
    .unwrap();
    let connectors = w.root.join("my tools/connectors");
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["mcp-line", "pgl"])
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", &connectors)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let line = String::from_utf8_lossy(&out.stdout)
        .trim_end_matches('\n')
        .to_string();

    let stub = w.root.join("mcp-bin");
    std::fs::create_dir_all(&stub).unwrap();
    common::executable(
        &stub.join("claude"),
        &format!(
            "printf '%s\\0' \"$@\" > \"{}/mcp-argv.bin\"\n",
            w.root.display()
        ),
    );
    let path = format!(
        "{}:{}",
        stub.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let status = Command::new("sh")
        .arg("-c")
        .arg(&line)
        .env("PATH", path)
        .status()
        .unwrap();
    assert!(status.success(), "{line}");
    let bytes = std::fs::read(w.root.join("mcp-argv.bin")).unwrap();
    let mut argv: Vec<String> = bytes
        .split(|b| *b == 0)
        .map(|a| String::from_utf8_lossy(a).into_owned())
        .collect();
    assert_eq!(argv.pop().as_deref(), Some(""), "{argv:?}");
    let env = argv
        .iter()
        .position(|a| a == "env")
        .unwrap_or_else(|| panic!("no env word in {argv:?}\nline: {line}"));
    assert_eq!(
        argv[env + 2..],
        [
            "EKR_BACKEND=postgres".to_string(),
            format!("EKR_STORE={}", config.display()),
            connectors.display().to_string(),
            "connections".into(),
            "launch".into(),
            "--adapter".into(),
            adapter.into(),
            "--connection".into(),
            connection.into(),
            "--consumer".into(),
            "ekr".into(),
            "--args".into(),
            r#"["mcp"]"#.into(),
        ],
        "line: {line}"
    );
}
