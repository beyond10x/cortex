//! One PostgreSQL 17 server with TLS for every PostgreSQL test of a run, and a database of its own
//! for each test.
//!
//! A server start takes 5 to 15 s, and its removal as long again; each test started and removed one
//! of its own before. Now the first test that asks for a database starts the server in a container,
//! under a lock on a file in the target directory, and writes where it listens beside the lock; every
//! later test of the run, in any process, finds it there and only creates a database and its roles
//! (well under a second). Roles are global in PostgreSQL, so each test's role names carry the id of
//! its database.
//!
//! The server ends itself: its container runs PostgreSQL under `timeout` for [`LIFETIME_S`] and is
//! started with `--rm`, so a killed run leaves nothing for longer. A server older than
//! [`REUSE_FOR_S`] is not handed to a new test, which leaves every test that got it more than
//! enough time before the server ends. Each start appends a line to `starts.log` beside the lock,
//! which `task test` counts.
//!
//! Without a container runtime a PostgreSQL case skips with the reason, except on GitHub Actions,
//! where it fails ([`container_runtime`]).

use std::fs::OpenOptions;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

pub const IMAGE: &str = "postgres:17";
/// The label every test container carries, and the one that dates it.
pub const LABEL: &str = "cortex.test=postgres";
pub const CREATED: &str = "cortex.test.created";
/// How long a server runs before it ends itself.
pub const LIFETIME_S: u64 = 900;
/// A server older than this is not handed to a new test.
pub const REUSE_FOR_S: u64 = 600;
/// A labelled container older than this outlived its own end and is removed.
pub const STALE_AFTER_S: u64 = LIFETIME_S + 60;
/// How long a start may wait for the server to answer.
const READY_WITHIN: Duration = Duration::from_secs(50);

/// The directory of the lock, the server's state, its TLS files and `starts.log`.
pub fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cortex-test-postgres-v1")
}

pub fn now_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn run(cmd: &mut Command, what: &str) -> String {
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

/// Why a container cannot be started here, or `None` when one can.
pub fn no_container_runtime() -> Option<String> {
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

/// Whether a container can run here. Without one the case skips with the reason, except on
/// GitHub Actions, where it fails.
pub fn container_runtime(case: &str) -> bool {
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

/// Removes every labelled container older than [`STALE_AFTER_S`]: one that outlived its own end.
pub fn sweep_stale_containers() {
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
        let mut words = line.split_whitespace();
        let (Some(id), created) = (words.next(), words.next()) else {
            continue;
        };
        let created: u64 = created.and_then(|c| c.parse().ok()).unwrap_or(0);
        if now_s().saturating_sub(created) > STALE_AFTER_S {
            let _ = Command::new("docker").args(["rm", "-f", id]).output();
        }
    }
}

/// The shared server: its container, published port and CA file.
#[derive(Clone, Debug)]
pub struct Server {
    pub container: String,
    pub port: String,
    pub ca: PathBuf,
    created: u64,
}

impl Server {
    fn from_state(state: &Value) -> Option<Self> {
        Some(Self {
            container: state["container"].as_str()?.to_string(),
            port: state["port"].as_str()?.to_string(),
            ca: PathBuf::from(state["ca"].as_str()?),
            created: state["created"].as_u64()?,
        })
    }

    /// Whether a new test may use it: young enough, running, and its CA file still there.
    fn usable(&self) -> bool {
        now_s().saturating_sub(self.created) < REUSE_FOR_S
            && self.ca.is_file()
            && Command::new("docker")
                .args(["inspect", "--format", "{{.State.Running}}", &self.container])
                .output()
                .is_ok_and(|o| o.status.success() && o.stdout.starts_with(b"true"))
    }
}

/// The server of this run: the one in the state file when it is usable, else a new one.
pub fn server() -> Server {
    let dir = dir();
    std::fs::create_dir_all(&dir).unwrap();
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join("lock"))
        .unwrap();
    lock.lock().unwrap();
    let state = dir.join("server.json");
    if let Some(server) = std::fs::read(&state)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|v| Server::from_state(&v))
        .filter(Server::usable)
    {
        return server;
    }
    sweep_stale_containers();
    let server = start(&dir);
    let written = dir.join("server.json.new");
    std::fs::write(
        &written,
        json!({"container": server.container, "port": server.port, "ca": server.ca,
            "created": server.created})
        .to_string(),
    )
    .unwrap();
    std::fs::rename(&written, &state).unwrap();
    let mut starts = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("starts.log"))
        .unwrap();
    std::io::Write::write_all(
        &mut starts,
        format!("{} {}\n", server.created, server.container).as_bytes(),
    )
    .unwrap();
    server
}

/// Starts a server: TLS files, the container, and a wait until it answers on TCP.
fn start(dir: &Path) -> Server {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let container = format!("cortex-test-postgres-{}-{stamp}", std::process::id());
    let tls = dir.join(&container);
    std::fs::create_dir_all(&tls).unwrap();
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
        std::fs::set_permissions(tls.join(file), std::fs::Permissions::from_mode(0o644)).unwrap();
    }
    let mount = format!("{}:/tls:ro", tls.display());
    let created = now_s();
    let created_label = format!("{CREATED}={created}");
    let command = format!(
        "cp /tls/server.crt /tls/server.key /var/lib/postgresql/ \
         && chown postgres /var/lib/postgresql/server.* \
         && chmod 600 /var/lib/postgresql/server.key \
         && exec timeout -s INT {LIFETIME_S} docker-entrypoint.sh postgres -c ssl=on \
         -c ssl_cert_file=/var/lib/postgresql/server.crt \
         -c ssl_key_file=/var/lib/postgresql/server.key \
         -c max_connections=400 -c fsync=off -c synchronous_commit=off \
         -c full_page_writes=off"
    );
    run(
        Command::new("docker").args([
            "run",
            "-d",
            "--rm",
            "--name",
            &container,
            "--label",
            LABEL,
            "--label",
            &created_label,
            "-e",
            "POSTGRES_PASSWORD=superuser",
            "-p",
            "127.0.0.1::5432",
            "-v",
            &mount,
            IMAGE,
            "sh",
            "-c",
            &command,
        ]),
        "docker run",
    );
    // The entrypoint's initialisation server listens on no TCP port; a TCP answer is the real one.
    let deadline = Instant::now() + READY_WITHIN;
    while !Command::new("docker")
        .args([
            "exec",
            &container,
            "pg_isready",
            "-h",
            "127.0.0.1",
            "-U",
            "postgres",
        ])
        .output()
        .is_ok_and(|o| o.status.success())
    {
        assert!(Instant::now() < deadline, "PostgreSQL did not start");
        std::thread::sleep(Duration::from_millis(200));
    }
    let port = run(
        Command::new("docker").args(["port", &container, "5432/tcp"]),
        "docker port",
    );
    let port = port
        .lines()
        .next()
        .and_then(|l| l.rsplit(':').next())
        .expect("a published port")
        .trim()
        .to_string();
    Server {
        container,
        port,
        ca: tls.join("ca.crt"),
        created,
    }
}

/// A database of one test on the shared server. Its name and the names of its roles end in its id.
pub struct Database {
    pub server: Server,
    pub id: String,
    pub name: String,
}

impl Database {
    /// A new database on the shared server, after which `setup(id)` runs in it as the superuser:
    /// the roles, schemas and grants of the test, its role names ending in `_<id>`.
    pub fn create(setup: impl FnOnce(&str) -> String) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros();
        let id = format!(
            "{}_{}_{stamp}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let name = format!("t_{id}");
        let server = server();
        let sql = setup(&id);
        run(
            Command::new("docker").args([
                "exec",
                &server.container,
                "psql",
                "-v",
                "ON_ERROR_STOP=1",
                "-h",
                "127.0.0.1",
                "-U",
                "postgres",
                "-d",
                "postgres",
                "-q",
                "-c",
                &format!("CREATE DATABASE {name}"),
                "-c",
                &format!("\\connect {name}"),
                "-c",
                &sql,
            ]),
            "psql",
        );
        Self { server, id, name }
    }

    /// Runs `sql` as the superuser in this database; its rows, one per line, columns separated by
    /// `|`.
    pub fn psql(&self, sql: &str) -> String {
        run(
            Command::new("docker").args([
                "exec",
                &self.server.container,
                "psql",
                "-v",
                "ON_ERROR_STOP=1",
                "-h",
                "127.0.0.1",
                "-U",
                "postgres",
                "-d",
                &self.name,
                // Rows alone: `-q` drops the command tag (`UPDATE 1`) a RETURNING prints after.
                "-qAt",
                "-c",
                sql,
            ]),
            "psql",
        )
    }

    /// The role name `base` takes in this database's test.
    pub fn role(&self, base: &str) -> String {
        format!("{base}_{}", self.id)
    }
}
