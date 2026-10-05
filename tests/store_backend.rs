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
        let pg = Self {
            name,
            app: cfg.join("app.json"),
            owner: cfg.join("owner.json"),
            password: format!("app-secret-{stamp}"),
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
        pg
    }
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
    let store = format!(
        "{{backend: postgres, value: {{config: \"{}\"}}}}",
        pg.app.display()
    );
    let spec = write_spec(&w, "pg", &store);

    let (code, out, err) = cortex(
        &w,
        &[
            "create",
            "--spec",
            spec.to_str().unwrap(),
            "--postgres-schema-config",
            pg.owner.to_str().unwrap(),
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

    // The viewer and the MCP line read the same store.
    let unit = std::fs::read_to_string(w.units.join("cortex-pg-view.service")).unwrap();
    assert!(
        unit.contains("Environment=\"EKR_BACKEND=postgres\""),
        "{unit}"
    );
    assert!(
        unit.contains(&format!("Environment=\"EKR_STORE={}\"", pg.app.display())),
        "{unit}"
    );
    let (code, line, err) = cortex(&w, &["mcp-line", "pg"]);
    assert_eq!(code, Some(0), "{err}");
    assert!(
        line.contains(&format!(
            "EKR_BACKEND=postgres EKR_STORE={} ",
            pg.app.display()
        )),
        "{line}"
    );

    // cortex passed file references only: the credential is nowhere it wrote.
    for (place, text) in [
        ("the instance directory", all_text(&dir)),
        ("the units", all_text(&w.units)),
        ("the MCP line", line),
    ] {
        assert!(
            !text.contains(&pg.password),
            "the database credential is in {place}"
        );
    }
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
    let password = "adversary-pw-0e1f";
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
