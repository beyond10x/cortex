//! `story:answer-records-and-review` on a PostgreSQL store: the answer records, votes and review
//! items of an instance on a `postgres` store live in the schema `ask.records` names, never in
//! EKR's, reached through a Connectors connection of their own (`cortex.ask.PostgresRecords`).
//!
//! The case starts a disposable PostgreSQL 17 container with TLS, provisions EKR's schema in it
//! with the real `ekr postgres-schema`, and gives the records a role of their own that could
//! create tables in EKR's schema too. The service's records process is launched by a stand-in
//! `connectors` that hands it the role's `{"password": …}` document on descriptor 3, as
//! `connectors connections launch --consumer cortex-records` does. It skips with a named reason
//! when no container runtime answers, except on GitHub Actions, where it fails.

mod common;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use common::{ekr, executable};
use cortex_cli::ask::records::{postgres::PgRecords, RecordStore, SCHEMA_VERSION};
use cortex_model::ask as ma;
use serde_json::{json, Value};

const IMAGE: &str = "postgres:17";
const TOKEN_ENV: &str = "ASK_PG_GATEWAY_TOKEN";
const HEADER: &str = "x-asker";
const LIMITS: &str =
    "{max_steps: 4, max_result_bytes: 4096, max_seconds: 30, per_identity_per_minute: 60}";
const WAIT: Duration = Duration::from_secs(60);

/// The label every container of this file carries, and the one that dates it.
const LABEL: &str = "cortex.test=answer-records-postgres";
const CREATED: &str = "cortex.test.created";
const STALE_AFTER_S: u64 = 600;

fn now_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
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

/// Whether a container can run here. Without one the case skips with the reason, except on
/// GitHub Actions, where it fails.
fn container_runtime(case: &str) -> bool {
    let reason = match Command::new("docker")
        .args(["info", "--format", "{{.ServerVersion}}"])
        .output()
    {
        Ok(out) if out.status.success() => return true,
        Ok(out) => format!(
            "`docker info` failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ),
        Err(e) => format!("cannot run `docker`: {e}"),
    };
    assert_ne!(
        std::env::var("GITHUB_ACTIONS").as_deref(),
        Ok("true"),
        "the PostgreSQL case must run on GitHub Actions: {reason}"
    );
    eprintln!("SKIP {case}: no container runtime ({reason})");
    false
}

/// Removes every labelled container older than [`STALE_AFTER_S`], left by a killed run.
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

/// A disposable PostgreSQL container with TLS: EKR's schema `ekr`, provisioned by `ekr`, and a
/// records role `cortex_ask` that may create schemas in the database and tables in `ekr`.
struct Postgres {
    name: String,
    port: String,
    ca: PathBuf,
    /// The records role's password, which must appear nowhere cortex writes or prints.
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
        let name = format!("cortex-records-pg-{}-{stamp}", std::process::id());
        let tls = dir.join("tls");
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
            std::fs::set_permissions(tls.join(file), std::fs::Permissions::from_mode(0o644))
                .unwrap();
        }
        sweep_stale_containers();
        let mount = format!("{}:/tls:ro", tls.display());
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
            port: String::new(),
            ca: tls.join("ca.crt"),
            password: format!("records-secret-{stamp}"),
        };
        let deadline = std::time::Instant::now() + Duration::from_secs(120);
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
            std::thread::sleep(Duration::from_millis(500));
        }
        pg.psql(&format!(
            "CREATE ROLE ekr_owner LOGIN PASSWORD 'owner-secret'; \
             CREATE SCHEMA ekr AUTHORIZATION ekr_owner; \
             CREATE ROLE cortex_ask LOGIN PASSWORD '{}'; \
             GRANT CREATE ON DATABASE ekr TO cortex_ask; \
             GRANT USAGE, CREATE ON SCHEMA ekr TO cortex_ask;",
            pg.password
        ));
        let port = run(
            Command::new("docker").args(["port", &pg.name, "5432/tcp"]),
            "docker port",
        );
        pg.port = port
            .lines()
            .next()
            .and_then(|l| l.rsplit(':').next())
            .expect("a published port")
            .trim()
            .to_string();
        pg
    }

    /// Runs `sql` as the superuser; its rows, one per line, columns separated by `|`.
    fn psql(&self, sql: &str) -> String {
        run(
            Command::new("docker").args([
                "exec",
                &self.name,
                "psql",
                "-v",
                "ON_ERROR_STOP=1",
                "-h",
                "127.0.0.1",
                "-U",
                "postgres",
                "-d",
                "ekr",
                // Rows alone: `-q` drops the command tag (`UPDATE 1`) a RETURNING prints after.
                "-qAt",
                "-c",
                sql,
            ]),
            "psql",
        )
    }

    /// The tables of `schema`, sorted.
    fn tables(&self, schema: &str) -> Vec<String> {
        self.psql(&format!(
            "SELECT table_name FROM information_schema.tables WHERE table_schema = '{schema}' \
             ORDER BY table_name"
        ))
        .lines()
        .map(str::to_string)
        .collect()
    }

    /// Provisions EKR's tables in schema `ekr` with `ekr postgres-schema`, as the owner role, and
    /// writes the store's `ekr.postgres/1` file, launched through connection `app`.
    fn provision_ekr(&self, dir: &Path) -> PathBuf {
        let owner_dsn = dir.join("owner.connection");
        std::fs::write(
            &owner_dsn,
            format!(
                "postgres://ekr_owner:owner-secret@localhost:{}/ekr\n",
                self.port
            ),
        )
        .unwrap();
        let owner = dir.join("owner.json");
        let document = |connection: &str, launched: bool| {
            let mut d = json!({
                "format": "ekr.postgres/1",
                "connection_file": connection,
                "ca_file": self.ca,
                "schema": "ekr",
                "database_connections": 100,
                "replicas": 1,
                "reserved_connections": 10,
            });
            if launched {
                d["password_file"] = json!("/proc/self/fd/3");
            }
            d.to_string()
        };
        std::fs::write(&owner, document("owner.connection", false)).unwrap();
        run(
            Command::new(ekr())
                .args(["postgres-schema", "--config"])
                .arg(&owner),
            "ekr postgres-schema",
        );
        std::fs::write(
            dir.join("app.connection"),
            format!("postgres://ekr_owner@localhost:{}/ekr\n", self.port),
        )
        .unwrap();
        let app = dir.join("app.json");
        std::fs::write(&app, document("app.connection", true)).unwrap();
        app
    }

    fn url(&self) -> String {
        format!("postgres://cortex_ask@localhost:{}/ekr", self.port)
    }

    /// The records backend, opened in this process with the role's password.
    fn records(&self, schema: &str) -> PgRecords {
        PgRecords::connect(&self.url(), schema, Some(&self.ca), &self.password)
            .expect("the records schema opens")
    }
}

/// A stand-in model gateway: the n-th request is answered with the n-th scripted reply (the last
/// again once the script is spent), after that reply's delay.
fn gateway(script: Vec<(u64, Value)>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for (n, stream) in listener.incoming().enumerate() {
            let Ok(mut stream) = stream else { return };
            read_request(&stream);
            let (delay, reply) = script[n.min(script.len() - 1)].clone();
            std::thread::sleep(Duration::from_millis(delay));
            let reply = reply.to_string();
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}",
                reply.len()
            );
        }
    });
    url
}

fn read_request(stream: &TcpStream) {
    let mut reader = BufReader::new(stream);
    let mut length = 0;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                length = value.trim().parse().unwrap_or(0);
            }
        }
    }
    let mut body = vec![0; length];
    let _ = reader.read_exact(&mut body);
}

fn tool_reply(usage: (i64, i64, i64)) -> Value {
    json!({
        "choices": [{"index": 0, "finish_reason": "tool_calls", "message": {
            "role": "assistant", "content": "Looking.",
            "tool_calls": [{"id": "call-1", "type": "function",
                "function": {"name": "search", "arguments": "{\"query\":\"widget\"}"}}]}}],
        "usage": {"prompt_tokens": usage.0, "completion_tokens": usage.1,
            "prompt_tokens_details": {"cached_tokens": usage.2}}
    })
}

fn text_reply(text: &str, usage: (i64, i64, i64)) -> Value {
    json!({
        "choices": [{"index": 0, "finish_reason": "stop",
            "message": {"role": "assistant", "content": text}}],
        "usage": {"prompt_tokens": usage.0, "completion_tokens": usage.1,
            "prompt_tokens_details": {"cached_tokens": usage.2}}
    })
}

fn broken_reply() -> Value {
    json!({"choices": []})
}

/// A temporary root: the stand-in `ekr` and `connectors`, the connections' documents, the spec
/// and the home.
struct World {
    _tmp: tempfile::TempDir,
    root: PathBuf,
}

impl World {
    fn new() -> Self {
        let tmp = tempfile::Builder::new()
            .prefix("cortex-records-pg")
            .tempdir_in(
                std::env::var_os("CARGO_TARGET_TMPDIR")
                    .map(PathBuf::from)
                    .unwrap_or_else(std::env::temp_dir),
            )
            .unwrap();
        let root = tmp.path().to_path_buf();
        std::fs::create_dir_all(root.join("bin")).unwrap();
        std::fs::create_dir_all(root.join("state")).unwrap();
        Self { _tmp: tmp, root }
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    /// The stand-in `connectors`: `connections list` lists every `state/<id>.json` as a ready
    /// `postgres` connection; `connections launch` opens `state/<connection>.json` on descriptor
    /// 3 and runs the consumer with the `--args` JSON array as its arguments and an empty
    /// environment: consumer `ekr` is the stand-in `ekr`, consumer `cortex-records` the cortex
    /// binary. Every argument list is logged to `connectors-argv.log`.
    fn connectors(&self, password: &str) {
        std::fs::write(
            self.root.join("state/app.json"),
            json!({"password": "owner-secret"}).to_string(),
        )
        .unwrap();
        std::fs::write(
            self.root.join("state/records.json"),
            json!({"password": password}).to_string(),
        )
        .unwrap();
        executable(
            &self.root.join("bin/ekr"),
            r#"[ "$1" = mcp ] || exit 2
while IFS= read -r line; do
  id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9][0-9]*\).*/\1/p')
  case "$line" in
    *'"method":"initialize"'*) printf '{"jsonrpc":"2.0","id":%s,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"ekr","version":"0"}}}\n' "$id" ;;
    *'"method":"tools/list"'*) printf '{"jsonrpc":"2.0","id":%s,"result":{"tools":[{"name":"search","description":"Search","inputSchema":{"type":"object"}}]}}\n' "$id" ;;
    *'"method":"tools/call"'*) printf '{"jsonrpc":"2.0","id":%s,"result":{"content":[{"type":"text","text":"Widget engine, made by Example Labs"}]}}\n' "$id" ;;
  esac
done
"#,
        );
        executable(
            &self.root.join("bin/connectors"),
            &format!(
                r#"R="{root}"; STATE="$R/state"
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
    case "$CONSUMER" in
      ekr) BIN="$R/bin/ekr" ;;
      cortex-records) BIN="{cortex}" ;;
      *) echo "no consumer $CONSUMER" >&2; exit 2 ;;
    esac
    [ -e "$STATE/$C.json" ] || {{ echo "no connection $C" >&2; exit 2; }}
    case "$ARGS" in *\\*) echo "escapes are not handled here: $ARGS" >&2; exit 2 ;; esac
    exec 3<"$STATE/$C.json"
    L=$(printf '%s' "$ARGS" | sed -e 's/^\["//' -e 's/"\]$//' -e 's/","/\n/g')
    set -- "$BIN"
    if [ "$ARGS" != "[]" ]; then
      while IFS= read -r a; do set -- "$@" "$a"; done <<EOF
$L
EOF
    fi
    exec env -i "$@" ;;
  *) echo '{{"ok":false}}'; exit 2 ;;
esac
"#,
                root = self.root.display(),
                cortex = env!("CARGO_BIN_EXE_cortex"),
            ),
        );
    }

    /// The spec of the instance `asker` on the store `config`, its gateway at `url`, its records
    /// in `schema` through connection `records`, with `extra` lines under `ask:`.
    fn spec(&self, url: &str, config: &Path, pg: &Postgres, schema: &str, extra: &str) -> PathBuf {
        let spec = self.root.join("asker.yaml");
        std::fs::write(
            &spec,
            format!(
                r#"format: cortex.instance/1
name: asker
description: Test brain.
ekr: {{version: "0.0.34", bin: "{ekr}"}}
seed: {{documents: []}}
model:
  model: test-model
  budget_tokens: 100000
  timeout_s: 60
  gateway: {{base_url: "{url}", token_env: {TOKEN_ENV}}}
sources: []
serve: {{view_port: 18997}}
store: {{backend: postgres, value: {{config: "{config}", connection: {{adapter: postgres, connection: app}}}}}}
ask:
  listen: "127.0.0.1:0"
  identity_header: {HEADER}
  limits: {LIMITS}
  records:
    url: "{records_url}"
    schema: {schema}
    ca_file: "{ca}"
    connection: {{adapter: postgres, connection: records}}
{extra}
"#,
                ekr = self.root.join("bin/ekr").display(),
                config = config.display(),
                records_url = pg.url(),
                ca = pg.ca.display(),
            ),
        )
        .unwrap();
        spec
    }

    fn command(&self, spec: &Path) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_cortex"));
        command
            .args(["ask", "serve", "--spec"])
            .arg(spec)
            .env("CORTEX_HOME", self.home())
            .env("CORTEX_CONNECTORS", self.root.join("bin/connectors"))
            .env(TOKEN_ENV, "stand-in-token");
        command
    }
}

/// A running `cortex ask serve`, stopped on drop; what it printed is kept in `log`.
struct Service {
    child: Child,
    addr: String,
    client: reqwest::blocking::Client,
    log: std::sync::Arc<std::sync::Mutex<String>>,
}

impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn serve(world: &World, spec: &Path) -> Service {
    let mut child = world
        .command(spec)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let log = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let stderr = child.stderr.take().unwrap();
    let kept = log.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            kept.lock().unwrap().push_str(&line);
            kept.lock().unwrap().push('\n');
        }
    });
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    let kept = log.clone();
    std::thread::spawn(move || {
        let mut lines = BufReader::new(stdout).lines().map_while(Result::ok);
        if let Some(first) = lines.next() {
            let _ = tx.send(first);
        }
        for line in lines {
            kept.lock().unwrap().push_str(&line);
            kept.lock().unwrap().push('\n');
        }
    });
    let Ok(line) = rx.recv_timeout(WAIT) else {
        let _ = child.kill();
        let _ = child.wait();
        panic!(
            "`cortex ask serve` printed no ready line within {WAIT:?}: {}",
            log.lock().unwrap()
        );
    };
    log.lock().unwrap().push_str(&line);
    let ready: Value = serde_json::from_str(line.trim()).expect("a ready line");
    Service {
        child,
        addr: ready["listening"].as_str().expect("listening").to_string(),
        client: reqwest::blocking::Client::builder()
            .timeout(WAIT)
            .build()
            .unwrap(),
        log,
    }
}

impl Service {
    fn ask(&self, identity: &str, question: &str) -> (String, Value) {
        let text = self
            .client
            .post(format!("http://{}/ask", self.addr))
            .header(HEADER, identity)
            .body(json!({"question": question}).to_string())
            .send()
            .unwrap()
            .text()
            .unwrap();
        let block = text
            .split("\n\n")
            .filter(|b| !b.trim().is_empty())
            .last()
            .expect("an event");
        let data: String = block
            .lines()
            .filter_map(|l| l.strip_prefix("data:"))
            .map(str::trim)
            .collect();
        let decoded: cortex_spec_types::CortexAskAskEvent =
            serde_json::from_str(&data).unwrap_or_else(|e| panic!("not an AskEvent ({e}): {data}"));
        let wire = serde_json::to_value(decoded).unwrap();
        (
            wire["event"].as_str().unwrap().to_string(),
            wire["value"].clone(),
        )
    }

    fn answered(&self, identity: &str, question: &str) -> (String, Value) {
        let (name, done) = self.ask(identity, question);
        assert_eq!(name, "done", "{done}");
        (done["answer_id"].as_str().unwrap().to_string(), done)
    }

    fn call(&self, method: &str, path: &str, body: Option<Value>) -> (u16, Value) {
        let url = format!("http://{}{path}", self.addr);
        let mut request = match method {
            "GET" => self.client.get(url),
            _ => self.client.post(url),
        };
        request = request.header(HEADER, "reviewer");
        if let Some(body) = body {
            request = request
                .header("content-type", "application/json")
                .body(body.to_string());
        }
        let response = request.send().unwrap();
        let status = response.status().as_u16();
        let text = response.text().unwrap();
        (status, serde_json::from_str(&text).unwrap_or(Value::Null))
    }

    fn vote(&self, id: &str, identity: &str, value: &str) -> u16 {
        let url = format!("http://{}/ask/{id}/vote", self.addr);
        self.client
            .post(url)
            .header(HEADER, identity)
            .header("content-type", "application/json")
            .body(json!({"value": value}).to_string())
            .send()
            .unwrap()
            .status()
            .as_u16()
    }

    fn reviews(&self) -> Vec<Value> {
        let (status, body) = self.call("GET", "/review", None);
        assert_eq!(status, 200, "{body}");
        body["reviews"].as_array().expect("reviews").clone()
    }

    fn printed(&self) -> String {
        self.log.lock().unwrap().clone()
    }
}

fn causes_of(reviews: &[Value], answer_id: &str) -> Vec<String> {
    let mut causes: Vec<String> = reviews
        .iter()
        .filter(|r| r["answer_id"] == answer_id)
        .map(|r| r["cause"].as_str().unwrap().to_string())
        .collect();
    causes.sort();
    causes
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

/// The story's acceptance on a PostgreSQL store: one answer is one record with the `done` event's
/// token counts; a vote changed from `down` to `up` stays one vote with one review item; a `down`
/// vote, a failure and a slow answer each open one item with their cause, which `GET /review`
/// lists and a close removes; a record two days old is purged with its votes and items on the next
/// start while today's stays; the records schema holds its tables at this cortex's schema version,
/// and EKR's schema holds the same tables after as before. The records role's password reaches
/// only the records process: the service's output, its home and the launch arguments hold none.
#[test]
fn answer_records_live_in_a_postgres_schema_of_their_own_and_never_in_ekrs() {
    if !container_runtime("answer_records_live_in_a_postgres_schema_of_their_own_and_never_in_ekrs")
    {
        return;
    }
    let world = World::new();
    let pg = Postgres::start(&world.root);
    let config = pg.provision_ekr(&world.root);
    world.connectors(&pg.password);
    let ekr_before = pg.tables("ekr");
    assert!(!ekr_before.is_empty(), "ekr postgres-schema made no tables");

    let url = gateway(vec![
        (0, tool_reply((100, 10, 40))),
        (0, text_reply("Example Labs [node:n-1].", (200, 20, 60))),
        (0, text_reply("Fast answer.", (5, 2, 0))),
        (0, broken_reply()),
        (2500, text_reply("Slow answer.", (5, 2, 0))),
        (0, text_reply("Another answer.", (5, 2, 0))),
    ]);
    let spec = world.spec(
        &url,
        &config,
        &pg,
        "cortex_ask",
        "  slow_after_ms: 1500\n  retention_days: 1",
    );
    let mut printed = String::new();
    let (first, voted, failed, slow, old) = {
        let service = serve(&world, &spec);

        // One answer, one record, the `done` event's token counts.
        let (first, done) = service.answered("alice", "Who makes the Widget engine?");
        let records = pg.records("cortex_ask").answers().unwrap();
        assert_eq!(records.len(), 1, "{records:?}");
        let record = &records[0];
        assert_eq!(record.answer_id.0, first);
        assert_eq!(record.instance_name.0, "asker");
        assert_eq!(record.identity, "alice");
        assert_eq!(record.outcome, ma::AnswerOutcome::Answered);
        assert_eq!(record.answer.as_deref(), done["answer"].as_str());
        for (kept, event) in [
            (record.input_tokens, "input_tokens"),
            (record.output_tokens, "output_tokens"),
            (record.cached_tokens, "cached_tokens"),
            (record.context_tokens, "context_tokens"),
            (record.steps, "steps"),
        ] {
            assert_eq!(kept, done[event].as_i64().unwrap(), "{event}");
        }
        assert_eq!((record.input_tokens, record.output_tokens), (300, 30));
        assert_eq!(record.tool_calls.len(), 1, "{:?}", record.tool_calls);

        // Down, then up: one vote, `up`, and the one item the first vote opened.
        assert_eq!(service.vote(&first, "bob", "down"), 200);
        assert_eq!(service.vote(&first, "bob", "up"), 200);
        let votes = pg.records("cortex_ask").votes(&first).unwrap();
        assert_eq!(votes.len(), 1, "{votes:?}");
        assert_eq!(votes[0].value, ma::VoteValue::Up);
        assert_eq!(causes_of(&service.reviews(), &first), ["voted-down"]);
        assert_eq!(pg.records("cortex_ask").reviews(&first).unwrap().len(), 1);

        // A down vote, a failure and a slow answer: one item each, with its cause.
        let (voted, _) = service.answered("alice", "fast?");
        let (name, error) = service.ask("alice", "broken?");
        assert_eq!(name, "error", "{error}");
        let (slow, _) = service.answered("alice", "slow?");
        assert_eq!(service.vote(&voted, "carol", "down"), 200);
        let failed = pg
            .records("cortex_ask")
            .answers()
            .unwrap()
            .into_iter()
            .find(|r| r.question == "broken?")
            .expect("the failed answer is recorded")
            .answer_id
            .0;
        let reviews = service.reviews();
        assert_eq!(causes_of(&reviews, &voted), ["voted-down"]);
        assert_eq!(causes_of(&reviews, &failed), ["failed"]);
        assert_eq!(causes_of(&reviews, &slow), ["slow"]);
        let review_id = format!("{failed}:failed");
        let (status, body) = service.call(
            "POST",
            &format!("/review/{review_id}/close"),
            Some(json!({"reason": "the gateway was down"})),
        );
        assert_eq!(status, 200, "{body}");
        assert!(causes_of(&service.reviews(), &failed).is_empty());
        let (status, _) = service.call(
            "POST",
            &format!("/review/{review_id}/close"),
            Some(json!({"reason": "again"})),
        );
        assert_eq!(status, 409);

        let (old, _) = service.answered("alice", "old?");
        assert_eq!(service.vote(&old, "bob", "down"), 200);
        printed.push_str(&service.printed());
        (first, voted, failed, slow, old)
    };

    // Two days old: gone after the purge of the next start, with its vote and item.
    let two_days_ago = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
        - 2 * 86_400_000;
    assert_eq!(
        pg.psql(&format!(
            "UPDATE cortex_ask.answers SET at_ms = {two_days_ago} WHERE answer_id = '{old}' \
             RETURNING answer_id"
        ))
        .trim(),
        old
    );
    let service = serve(&world, &spec);
    let records = pg.records("cortex_ask");
    let kept: Vec<String> = records
        .answers()
        .unwrap()
        .into_iter()
        .map(|r| r.answer_id.0)
        .collect();
    assert!(!kept.contains(&old), "{kept:?}");
    for id in [&first, &voted, &failed, &slow] {
        assert!(kept.contains(id), "{id} in {kept:?}");
    }
    assert!(records.votes(&old).unwrap().is_empty());
    assert!(records.reviews(&old).unwrap().is_empty());
    assert_eq!(service.vote(&old, "bob", "up"), 404);
    printed.push_str(&service.printed());
    drop(service);

    // The records schema holds the records' tables at this cortex's version; EKR's is unchanged.
    assert_eq!(
        pg.tables("cortex_ask"),
        ["answers", "reviews", "schema_version", "votes"]
    );
    assert_eq!(
        pg.psql("SELECT version FROM cortex_ask.schema_version")
            .trim(),
        SCHEMA_VERSION.to_string()
    );
    assert_eq!(pg.tables("ekr"), ekr_before);

    // The records process was launched through the records connection, and the password stayed
    // with it.
    let launches = std::fs::read_to_string(world.root.join("connectors-argv.log")).unwrap();
    assert!(
        launches.contains(
            "connections launch --adapter postgres --connection records --consumer cortex-records --args [\"ask\",\"records\""
        ),
        "{launches}"
    );
    assert!(!launches.contains(&pg.password), "{launches}");
    assert!(!printed.contains(&pg.password), "{printed}");
    assert!(!all_text(&world.home()).contains(&pg.password));
    assert!(!world.home().join("instances/asker/ask.sqlite").exists());
}

/// A records schema at a version newer than this cortex's is refused when the service starts
/// (exit 1, naming the version), and is left as it was.
#[test]
fn a_records_schema_from_a_newer_cortex_is_refused() {
    if !container_runtime("a_records_schema_from_a_newer_cortex_is_refused") {
        return;
    }
    let world = World::new();
    let pg = Postgres::start(&world.root);
    let config = pg.provision_ekr(&world.root);
    world.connectors(&pg.password);
    drop(pg.records("cortex_ask"));
    let newer = SCHEMA_VERSION + 1;
    pg.psql(&format!(
        "UPDATE cortex_ask.schema_version SET version = {newer}"
    ));
    let spec = world.spec("http://127.0.0.1:9", &config, &pg, "cortex_ask", "");
    let out = world
        .command(&spec)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert!(text.contains("newer"), "{text}");
    assert!(!text.contains("listening"), "{text}");
    assert_eq!(
        pg.psql("SELECT version FROM cortex_ask.schema_version")
            .trim(),
        newer.to_string()
    );
}
