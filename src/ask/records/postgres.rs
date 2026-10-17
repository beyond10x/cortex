//! The PostgreSQL backend of the answer records (`cortex.ask.PostgresRecords`): the tables
//! `answers`, `votes`, `reviews` and `schema_version` in a schema of their own, never EKR's.
//!
//! The service never holds the database password. It starts its records process,
//! `cortex ask records --url <url> --schema <schema> [--ca-file <file>]`, through
//! `connectors connections launch --consumer cortex-records` ([`CONSUMER`]), which hands that
//! process the connection's `{"password": …}` document on descriptor 3. The records process
//! ([`serve_process`]) connects, creates the schema when it is missing, migrates it to
//! [`SCHEMA_VERSION`](super::SCHEMA_VERSION), and then runs the statements the service sends it
//! on its standard input, one JSON line each, answering one JSON line each ([`Remote`]).
//! [`PgRecords`] is the [`RecordStore`] over either: the records process, or a connection of this
//! process ([`PgRecords::connect`], for tests).

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cortex_model::ask as ma;
use cortex_model::instance as m;
use serde_json::{json, Value};

use super::{
    cause_name, cause_of, ms_of, outcome_name, outcome_of, review_state_name, review_state_of,
    stamp, tools_json, tools_of, value_name, value_of, Purge, RecordStore, NO_SUCH_ANSWER,
    REVIEW_NOT_OPEN, SCHEMA_VERSION,
};

/// The name of the `[consumers]` entry in the operator's Connectors configuration that pins the
/// `cortex` binary the records process runs. It needs no variable passed (`pass_env = []`).
pub const CONSUMER: &str = "cortex-records";

/// The schema of the records, one step per version as for `ask.sqlite`: step `i` takes version
/// `i` to `i + 1`, and the one row of `schema_version` holds the version a schema is at.
const MIGRATIONS: &[&str] = &["
    CREATE TABLE answers (
        answer_id TEXT PRIMARY KEY,
        instance_name TEXT NOT NULL,
        identity TEXT NOT NULL,
        question TEXT NOT NULL,
        answer TEXT,
        reason TEXT,
        outcome TEXT NOT NULL,
        tool_calls TEXT NOT NULL,
        latency_ms BIGINT NOT NULL,
        input_tokens BIGINT NOT NULL,
        output_tokens BIGINT NOT NULL,
        cached_tokens BIGINT NOT NULL,
        context_tokens BIGINT NOT NULL,
        steps BIGINT NOT NULL,
        at_ms BIGINT NOT NULL
    );
    CREATE INDEX answers_at ON answers (at_ms);
    CREATE TABLE votes (
        vote_id TEXT PRIMARY KEY,
        answer_id TEXT NOT NULL REFERENCES answers (answer_id) ON DELETE CASCADE,
        identity TEXT NOT NULL,
        value TEXT NOT NULL,
        comment TEXT,
        at_ms BIGINT NOT NULL
    );
    CREATE INDEX votes_answer ON votes (answer_id);
    CREATE TABLE reviews (
        review_id TEXT PRIMARY KEY,
        answer_id TEXT NOT NULL REFERENCES answers (answer_id) ON DELETE CASCADE,
        cause TEXT NOT NULL,
        state TEXT NOT NULL,
        opened_ms BIGINT NOT NULL,
        close_reason TEXT,
        closed_ms BIGINT
    );
    CREATE INDEX reviews_answer ON reviews (answer_id);
    CREATE INDEX reviews_state ON reviews (state, opened_ms);
"];

const _: () = assert!(MIGRATIONS.len() as i64 == SCHEMA_VERSION);

/// Why `schema` cannot hold the records: it is no lowercase PostgreSQL identifier of at most 63
/// bytes, or it starts with `pg_`, which PostgreSQL reserves. Names the field, never the value.
pub fn schema_refusal(schema: &str) -> Option<String> {
    let identifier = !schema.is_empty()
        && schema.len() <= 63
        && schema
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        && !schema.as_bytes()[0].is_ascii_digit();
    if !identifier {
        return Some(
            "ask.records.schema: not a lowercase PostgreSQL identifier (letters a-z, digits and \
             `_`, not starting with a digit, at most 63 bytes)"
                .into(),
        );
    }
    schema.starts_with("pg_").then(|| {
        "ask.records.schema: a schema whose name starts with `pg_` is PostgreSQL's own".into()
    })
}

/// Why `url` cannot reach the records: it is no `postgres://` or `postgresql://` URL, names no
/// role, or carries a password, which only the Connectors connection holds. Names the field,
/// never the value.
pub fn url_refusal(url: &str) -> Option<String> {
    if !(url.starts_with("postgres://") || url.starts_with("postgresql://")) {
        return Some("ask.records.url: not a postgres:// or postgresql:// URL".into());
    }
    let Ok(config) = postgres::Config::from_str(url) else {
        return Some("ask.records.url: not a PostgreSQL connection URL".into());
    };
    if config.get_password().is_some() {
        return Some(
            "ask.records.url: carries a password; the password belongs to the Connectors \
             connection ask.records.connection names, never to the spec"
                .into(),
        );
    }
    if config.get_user().is_none() {
        return Some("ask.records.url: names no role (`postgres://<role>@<host>/<db>`)".into());
    }
    None
}

/// The connection settings of the records: the URL, TLS required unless it says
/// `sslmode=disable`, verified against `ca_file` or the public web roots.
fn config_of(url: &str, password: &str) -> Result<postgres::Config, String> {
    if let Some(refused) = url_refusal(url) {
        return Err(refused);
    }
    let mut config = postgres::Config::from_str(url)
        .map_err(|_| "ask.records.url: not a PostgreSQL connection URL".to_string())?;
    if !url.contains("sslmode=") {
        config.ssl_mode(postgres::config::SslMode::Require);
    }
    config
        .password(password)
        .connect_timeout(Duration::from_secs(30))
        .application_name("cortex ask records");
    Ok(config)
}

fn tls(ca_file: Option<&Path>) -> Result<tokio_postgres_rustls::MakeRustlsConnect, String> {
    use rustls::pki_types::pem::PemObject;
    use rustls::pki_types::CertificateDer;
    let mut roots = rustls::RootCertStore::empty();
    match ca_file {
        Some(file) => {
            let certs = CertificateDer::pem_file_iter(file)
                .and_then(|certs| certs.collect::<Result<Vec<_>, _>>())
                .map_err(|e| format!("ask.records.ca_file: {}: {e}", file.display()))?;
            if certs.is_empty() {
                return Err(format!(
                    "ask.records.ca_file: {}: holds no certificate",
                    file.display()
                ));
            }
            for cert in certs {
                roots
                    .add(cert)
                    .map_err(|e| format!("ask.records.ca_file: {}: {e}", file.display()))?;
            }
        }
        None => roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned()),
    }
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|e| e.to_string())?
    .with_root_certificates(roots)
    .with_no_client_auth();
    Ok(tokio_postgres_rustls::MakeRustlsConnect::new(config))
}

/// A parameter of a statement: text or a 64-bit integer, either of which may be null.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Param {
    Text(Option<String>),
    Int(Option<i64>),
}

impl Param {
    fn text(s: impl Into<String>) -> Self {
        Self::Text(Some(s.into()))
    }

    fn to_json(&self) -> Value {
        match self {
            Self::Text(t) => json!({ "t": t }),
            Self::Int(i) => json!({ "i": i }),
        }
    }

    fn from_json(v: &Value) -> Result<Self, String> {
        if let Some(t) = v.get("t") {
            return Ok(Self::Text(t.as_str().map(str::to_string)));
        }
        if let Some(i) = v.get("i") {
            return Ok(Self::Int(i.as_i64()));
        }
        Err(format!("not a parameter: {v}"))
    }
}

/// A failure of a statement: PostgreSQL's SQLSTATE when it gave one, and the message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SqlError {
    pub code: Option<String>,
    pub message: String,
}

impl From<String> for SqlError {
    fn from(message: String) -> Self {
        Self {
            code: None,
            message,
        }
    }
}

/// SQLSTATE `foreign_key_violation`: a vote or a review item whose answer is not kept.
const FOREIGN_KEY_VIOLATION: &str = "23503";

/// Where the statements run: a connection of this process, or the records process. A row is its
/// cells as JSON: a string, an integer, a boolean or null.
pub trait Sql: Send + Sync {
    fn query(&self, sql: &str, params: &[Param]) -> Result<Vec<Vec<Value>>, SqlError>;
    fn execute(&self, sql: &str, params: &[Param]) -> Result<u64, SqlError>;
    /// The statements in one transaction: each one's count of rows, or nothing written.
    fn batch(&self, statements: &[(&str, Vec<Param>)]) -> Result<Vec<u64>, SqlError>;
}

/// A connection of this process to the records schema, connected again when it closed.
pub struct Local {
    client: Mutex<postgres::Client>,
    config: postgres::Config,
    tls: tokio_postgres_rustls::MakeRustlsConnect,
    schema: String,
}

fn db_error(e: postgres::Error) -> SqlError {
    SqlError {
        code: e.code().map(|c| c.code().to_string()),
        message: crate::mask::mask(&e.to_string()).0,
    }
}

impl Local {
    /// Connects to `url` as its role with `password`, creates `schema` when it is missing and
    /// migrates it to [`SCHEMA_VERSION`]; a schema at a newer version is refused, unchanged.
    pub fn connect(
        url: &str,
        schema: &str,
        ca_file: Option<&Path>,
        password: &str,
    ) -> Result<Self, String> {
        if let Some(refused) = schema_refusal(schema) {
            return Err(refused);
        }
        let config = config_of(url, password)?;
        let tls = tls(ca_file)?;
        let mut client = Self::open(&config, &tls, schema)?;
        migrate(&mut client, schema)?;
        Ok(Self {
            client: Mutex::new(client),
            config,
            tls,
            schema: schema.to_string(),
        })
    }

    fn open(
        config: &postgres::Config,
        tls: &tokio_postgres_rustls::MakeRustlsConnect,
        schema: &str,
    ) -> Result<postgres::Client, String> {
        let mut client = config
            .connect(tls.clone())
            .map_err(|e| format!("ask.records: cannot connect: {}", db_error(e).message))?;
        // Every statement after this names its tables unqualified, and they resolve in the
        // records schema alone: a missing schema is an error, never another schema.
        client
            .batch_execute(&format!("SET search_path TO \"{schema}\""))
            .map_err(|e| db_error(e).message)?;
        Ok(client)
    }

    fn with<T>(
        &self,
        f: impl FnOnce(&mut postgres::Client) -> Result<T, postgres::Error>,
    ) -> Result<T, SqlError> {
        let mut client = self
            .client
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if client.is_closed() {
            *client = Self::open(&self.config, &self.tls, &self.schema)?;
        }
        f(&mut client).map_err(db_error)
    }
}

/// Creates `schema` when it is missing and takes it to [`SCHEMA_VERSION`], in one transaction
/// under an advisory lock, so two services starting at once migrate it once.
fn migrate(client: &mut postgres::Client, schema: &str) -> Result<(), String> {
    let err = |e: postgres::Error| format!("ask.records: schema {schema}: {}", db_error(e).message);
    let mut tx = client.transaction().map_err(err)?;
    tx.execute(
        "SELECT pg_advisory_xact_lock(hashtext('cortex.ask.records'), hashtext($1::text))",
        &[&schema],
    )
    .map_err(err)?;
    let exists = tx
        .query_opt(
            "SELECT 1 FROM pg_catalog.pg_namespace WHERE nspname = $1::text",
            &[&schema],
        )
        .map_err(err)?
        .is_some();
    if !exists {
        tx.batch_execute(&format!("CREATE SCHEMA \"{schema}\""))
            .map_err(err)?;
    }
    tx.batch_execute(
        "CREATE TABLE IF NOT EXISTS schema_version (version BIGINT NOT NULL); \
         INSERT INTO schema_version (version) SELECT 0 \
         WHERE NOT EXISTS (SELECT 1 FROM schema_version)",
    )
    .map_err(err)?;
    let version: i64 = tx
        .query_one("SELECT max(version) FROM schema_version", &[])
        .map_err(err)?
        .get(0);
    if version > SCHEMA_VERSION {
        return Err(format!(
            "ask.records: schema {schema}: schema version {version} is newer than this \
             cortex's {SCHEMA_VERSION}; a newer cortex wrote it"
        ));
    }
    for step in &MIGRATIONS[version.max(0) as usize..] {
        tx.batch_execute(step).map_err(err)?;
    }
    tx.execute(
        "UPDATE schema_version SET version = $1::bigint",
        &[&SCHEMA_VERSION],
    )
    .map_err(err)?;
    tx.commit().map_err(err)
}

fn bound(params: &[Param]) -> Vec<Box<dyn postgres::types::ToSql + Sync>> {
    params
        .iter()
        .map(|p| -> Box<dyn postgres::types::ToSql + Sync> {
            match p {
                Param::Text(t) => Box::new(t.clone()),
                Param::Int(i) => Box::new(*i),
            }
        })
        .collect()
}

fn cells(row: &postgres::Row) -> Result<Vec<Value>, postgres::Error> {
    use postgres::types::Type;
    let mut out = Vec::with_capacity(row.len());
    for (i, column) in row.columns().iter().enumerate() {
        let ty = column.type_();
        let cell = if *ty == Type::INT8 {
            row.try_get::<_, Option<i64>>(i)?.map(Value::from)
        } else if *ty == Type::INT4 {
            row.try_get::<_, Option<i32>>(i)?.map(Value::from)
        } else if *ty == Type::BOOL {
            row.try_get::<_, Option<bool>>(i)?.map(Value::from)
        } else {
            row.try_get::<_, Option<String>>(i)?.map(Value::from)
        };
        out.push(cell.unwrap_or(Value::Null));
    }
    Ok(out)
}

impl Sql for Local {
    fn query(&self, sql: &str, params: &[Param]) -> Result<Vec<Vec<Value>>, SqlError> {
        let params = bound(params);
        let refs: Vec<&(dyn postgres::types::ToSql + Sync)> =
            params.iter().map(|p| p.as_ref()).collect();
        self.with(|c| c.query(sql, &refs)?.iter().map(cells).collect())
    }

    fn execute(&self, sql: &str, params: &[Param]) -> Result<u64, SqlError> {
        let params = bound(params);
        let refs: Vec<&(dyn postgres::types::ToSql + Sync)> =
            params.iter().map(|p| p.as_ref()).collect();
        self.with(|c| c.execute(sql, &refs))
    }

    fn batch(&self, statements: &[(&str, Vec<Param>)]) -> Result<Vec<u64>, SqlError> {
        self.with(|c| {
            let mut tx = c.transaction()?;
            let mut counts = Vec::with_capacity(statements.len());
            for (sql, params) in statements {
                let params = bound(params);
                let refs: Vec<&(dyn postgres::types::ToSql + Sync)> =
                    params.iter().map(|p| p.as_ref()).collect();
                counts.push(tx.execute(*sql, &refs)?);
            }
            tx.commit()?;
            Ok(counts)
        })
    }
}

/// How to start the records process: `connectors connections launch` of the pinned consumer
/// [`CONSUMER`] through the records connection, with the arguments of `cortex ask records`.
#[derive(Clone, Debug)]
pub struct Launch {
    pub connectors: PathBuf,
    pub adapter: String,
    /// The connection's id, a profile already resolved.
    pub connection: String,
    pub url: String,
    pub schema: String,
    pub ca_file: Option<String>,
    /// Variables the records process must not inherit: the gateway's token among them.
    pub remove: Vec<String>,
}

impl Launch {
    /// The arguments of `cortex ask records`, as the launch's `--args` carries them.
    pub fn args(&self) -> Vec<String> {
        let mut args = vec![
            "ask".to_string(),
            "records".into(),
            "--url".into(),
            self.url.clone(),
            "--schema".into(),
            self.schema.clone(),
        ];
        if let Some(ca) = &self.ca_file {
            args.extend(["--ca-file".into(), ca.clone()]);
        }
        args
    }

    fn command(&self) -> Command {
        let mut cmd = Command::new(&self.connectors);
        cmd.args([
            "connections",
            "launch",
            "--adapter",
            &self.adapter,
            "--connection",
            &self.connection,
            "--consumer",
            CONSUMER,
            "--args",
        ])
        .arg(Value::from(self.args()).to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
        for key in &self.remove {
            cmd.env_remove(key);
        }
        cmd
    }
}

struct Proc {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Drop for Proc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The records process, started through Connectors: one statement at a time, one JSON line each
/// way. A process that ended, or answered no line, is started again for the next statement.
pub struct Remote {
    launch: Launch,
    process: Mutex<Option<Proc>>,
}

impl Remote {
    /// Starts the records process and waits for its ready line, after its migration: its refusal
    /// (a schema at a newer version, a connection that fails) is the `Err`.
    pub fn start(launch: Launch) -> Result<Self, String> {
        let process = Self::spawn(&launch)?;
        Ok(Self {
            launch,
            process: Mutex::new(Some(process)),
        })
    }

    fn spawn(launch: &Launch) -> Result<Proc, String> {
        let mut child = launch.command().spawn().map_err(|e| {
            format!(
                "ask.records: cannot run {}: {e}",
                launch.connectors.display()
            )
        })?;
        let stdin = child.stdin.take().expect("piped");
        let mut stdout = BufReader::new(child.stdout.take().expect("piped"));
        let mut line = String::new();
        let read = stdout.read_line(&mut line);
        let mut process = Proc {
            child,
            stdin,
            stdout,
        };
        let answer: Value = match read {
            Ok(n) if n > 0 => serde_json::from_str(line.trim()).unwrap_or(Value::Null),
            _ => Value::Null,
        };
        if answer.get("ready").is_some() {
            return Ok(process);
        }
        let status = process.child.wait().ok();
        Err(match answer.get("err").and_then(Value::as_str) {
            Some(err) => err.to_string(),
            None => format!(
                "ask.records: the records process launched through {}:{} ended without \
                 starting ({}); is the consumer `{CONSUMER}` pinned in the Connectors \
                 configuration?",
                launch.adapter,
                launch.connection,
                status.map_or("killed".into(), |s| s.to_string())
            ),
        })
    }

    fn call(&self, request: Value) -> Result<Value, SqlError> {
        let mut process = self
            .process
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if process.is_none() {
            *process = Some(Self::spawn(&self.launch)?);
        }
        let p = process.as_mut().expect("started");
        let mut line = String::new();
        let exchanged = writeln!(p.stdin, "{request}")
            .and_then(|()| p.stdin.flush())
            .and_then(|()| p.stdout.read_line(&mut line));
        let answer = match exchanged {
            Ok(n) if n > 0 => serde_json::from_str::<Value>(line.trim()).ok(),
            _ => None,
        };
        let Some(answer) = answer else {
            // The process is gone or out of step: the next statement starts another.
            *process = None;
            return Err("ask.records: the records process ended".to_string().into());
        };
        if let Some(err) = answer.get("err") {
            return Err(SqlError {
                code: answer
                    .get("code")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                message: err.as_str().unwrap_or_default().to_string(),
            });
        }
        Ok(answer)
    }
}

fn params_json(params: &[Param]) -> Value {
    Value::Array(params.iter().map(Param::to_json).collect())
}

fn params_of(v: &Value) -> Result<Vec<Param>, String> {
    v.as_array()
        .ok_or("params: not an array")?
        .iter()
        .map(Param::from_json)
        .collect()
}

fn protocol(what: &str) -> SqlError {
    format!("ask.records: the records process answered no {what}").into()
}

impl Sql for Remote {
    fn query(&self, sql: &str, params: &[Param]) -> Result<Vec<Vec<Value>>, SqlError> {
        let answer =
            self.call(json!({"op": "query", "sql": sql, "params": params_json(params)}))?;
        answer["rows"]
            .as_array()
            .ok_or_else(|| protocol("rows"))?
            .iter()
            .map(|row| row.as_array().cloned().ok_or_else(|| protocol("row")))
            .collect()
    }

    fn execute(&self, sql: &str, params: &[Param]) -> Result<u64, SqlError> {
        let answer =
            self.call(json!({"op": "execute", "sql": sql, "params": params_json(params)}))?;
        answer["count"].as_u64().ok_or_else(|| protocol("count"))
    }

    fn batch(&self, statements: &[(&str, Vec<Param>)]) -> Result<Vec<u64>, SqlError> {
        let statements: Vec<Value> = statements
            .iter()
            .map(|(sql, params)| json!({"sql": sql, "params": params_json(params)}))
            .collect();
        let answer = self.call(json!({"op": "batch", "statements": statements}))?;
        answer["counts"]
            .as_array()
            .ok_or_else(|| protocol("counts"))?
            .iter()
            .map(|c| c.as_u64().ok_or_else(|| protocol("count")))
            .collect()
    }
}

/// One request of the service to the records process, answered by `sql`.
fn answer(sql: &dyn Sql, request: &Value) -> Value {
    let result = (|| -> Result<Value, SqlError> {
        let text = |v: &Value| -> Result<String, SqlError> {
            Ok(v.as_str().ok_or_else(|| protocol("sql"))?.to_string())
        };
        match request["op"].as_str() {
            Some("query") => {
                let rows = sql.query(&text(&request["sql"])?, &params_of(&request["params"])?)?;
                Ok(json!({ "rows": rows }))
            }
            Some("execute") => {
                let count =
                    sql.execute(&text(&request["sql"])?, &params_of(&request["params"])?)?;
                Ok(json!({ "count": count }))
            }
            Some("batch") => {
                let mut statements = Vec::new();
                for s in request["statements"].as_array().into_iter().flatten() {
                    statements.push((text(&s["sql"])?, params_of(&s["params"])?));
                }
                let borrowed: Vec<(&str, Vec<Param>)> = statements
                    .iter()
                    .map(|(s, p)| (s.as_str(), p.clone()))
                    .collect();
                Ok(json!({ "counts": sql.batch(&borrowed)? }))
            }
            _ => Err("ask.records: unknown request".to_string().into()),
        }
    })();
    result.unwrap_or_else(|e| json!({"err": e.message, "code": e.code}))
}

/// The password of the `{"password": …}` document at `file`: descriptor 3 of a process
/// `connectors connections launch` started.
fn password_of(file: &Path) -> Result<String, String> {
    let missing = || {
        format!(
            "ask.records: no password document on {}; the records process starts through \
             `connectors connections launch --consumer {CONSUMER}`",
            file.display()
        )
    };
    let text = std::fs::read(file).map_err(|_| missing())?;
    let doc: Value = serde_json::from_slice(&text).map_err(|_| missing())?;
    doc.get("password")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(missing)
}

/// `cortex ask records`: the records process. Reads the password from `password_file`, connects
/// and migrates, prints `{"ready": {"schema_version": N}}`, then answers each JSON line of its
/// standard input until it ends. A failure to start prints `{"err": …}` and is the `Err`. The
/// password is held by the connection alone; nothing prints it.
pub fn serve_process(
    url: &str,
    schema: &str,
    ca_file: Option<&Path>,
    password_file: &Path,
) -> Result<(), String> {
    let mut stdout = std::io::stdout().lock();
    let started =
        password_of(password_file).and_then(|pw| Local::connect(url, schema, ca_file, &pw));
    let local = match started {
        Ok(local) => local,
        Err(e) => {
            let _ = writeln!(stdout, "{}", json!({ "err": e }));
            return Err(e);
        }
    };
    writeln!(
        stdout,
        "{}",
        json!({"ready": {"schema_version": SCHEMA_VERSION}})
    )
    .and_then(|()| stdout.flush())
    .map_err(|e| e.to_string())?;
    for line in std::io::stdin().lock().lines() {
        let line = line.map_err(|e| e.to_string())?;
        let request: Value = serde_json::from_str(&line).unwrap_or(Value::Null);
        writeln!(stdout, "{}", answer(&local, &request))
            .and_then(|()| stdout.flush())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// The records in a PostgreSQL schema of their own: [`RecordStore`] over [`Sql`].
pub struct PgRecords {
    sql: Arc<dyn Sql>,
}

impl PgRecords {
    /// Over a connection of this process (tests): see [`Local::connect`].
    pub fn connect(
        url: &str,
        schema: &str,
        ca_file: Option<&Path>,
        password: &str,
    ) -> Result<Self, String> {
        Ok(Self {
            sql: Arc::new(Local::connect(url, schema, ca_file, password)?),
        })
    }

    /// Over the records process `launch` starts: see [`Remote::start`].
    pub fn launched(launch: Launch) -> Result<Self, String> {
        Ok(Self {
            sql: Arc::new(Remote::start(launch)?),
        })
    }

    /// Over any [`Sql`].
    pub fn over(sql: Arc<dyn Sql>) -> Self {
        Self { sql }
    }

    fn rows(&self, sql: &str, params: &[Param]) -> Result<Vec<Vec<Value>>, String> {
        self.sql.query(sql, params).map_err(|e| e.message)
    }

    fn exec(&self, sql: &str, params: &[Param]) -> Result<u64, String> {
        self.sql.execute(sql, params).map_err(|e| e.message)
    }
}

/// The launch of the records of `spec`, an instance on a `postgres` store with `ask.records`
/// (`ask::check` holds both): its connection resolved and made ready.
pub fn launch_of(spec: &m::InstanceSpec, records: &ma::PostgresRecords) -> Result<Launch, String> {
    let connectors = crate::connectors::Connectors::new(crate::connectors::bin());
    let adapter = records.connection.adapter.clone();
    let reference = crate::connectors::Reference::of(
        records.connection.connection.as_ref(),
        records.connection.profile.as_ref(),
    );
    let connection = connectors
        .resolve(&adapter, &reference)
        .map_err(|e| format!("ask.records.connection: {e}"))?;
    connectors
        .ensure_ready(&adapter, &connection)
        .map_err(|e| format!("ask.records.connection: {adapter}:{connection}: {e}"))?;
    let mut remove = vec!["ANTHROPIC_API_KEY".to_string()];
    if let Some(g) = &spec.model.gateway {
        remove.push(g.token_env.clone());
    }
    Ok(Launch {
        connectors: crate::connectors::bin(),
        adapter,
        connection,
        url: records.url.clone(),
        schema: records.schema.clone(),
        ca_file: records.ca_file.clone(),
        remove,
    })
}

fn text_at(row: &[Value], i: usize) -> Result<String, String> {
    row.get(i)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("column {i}: not text"))
}

fn opt_text_at(row: &[Value], i: usize) -> Result<Option<String>, String> {
    match row.get(i) {
        Some(Value::Null) => Ok(None),
        _ => text_at(row, i).map(Some),
    }
}

fn int_at(row: &[Value], i: usize) -> Result<i64, String> {
    row.get(i)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("column {i}: not an integer"))
}

fn opt_int_at(row: &[Value], i: usize) -> Result<Option<i64>, String> {
    match row.get(i) {
        Some(Value::Null) => Ok(None),
        _ => int_at(row, i).map(Some),
    }
}

const ANSWER: &str = "SELECT answer_id, instance_name, identity, question, answer, reason, \
    outcome, tool_calls, latency_ms, input_tokens, output_tokens, cached_tokens, context_tokens, \
    steps, at_ms FROM answers";
const VOTE: &str = "SELECT vote_id, answer_id, identity, value, comment, at_ms FROM votes";
const REVIEW: &str = "SELECT review_id, answer_id, cause, state, opened_ms, close_reason, \
    closed_ms FROM reviews";

fn answer_row(row: &[Value]) -> Result<ma::AnswerRecordData, String> {
    let id = text_at(row, 0)?;
    let decoded = (|| {
        let outcome = text_at(row, 6)?;
        Ok(ma::AnswerRecordData {
            answer_id: ma::AnswerId(id.clone()),
            instance_name: m::InstanceName(text_at(row, 1)?),
            identity: text_at(row, 2)?,
            question: text_at(row, 3)?,
            answer: opt_text_at(row, 4)?,
            reason: opt_text_at(row, 5)?,
            outcome: outcome_of(&outcome).ok_or(format!("outcome {outcome:?}"))?,
            tool_calls: tools_of(&text_at(row, 7)?).ok_or("tool_calls")?,
            latency_ms: int_at(row, 8)?,
            input_tokens: int_at(row, 9)?,
            output_tokens: int_at(row, 10)?,
            cached_tokens: int_at(row, 11)?,
            context_tokens: int_at(row, 12)?,
            steps: int_at(row, 13)?,
            at: stamp(int_at(row, 14)?),
        })
    })();
    decoded.map_err(|e: String| format!("answer record {id}: {e}"))
}

fn vote_row(row: &[Value]) -> Result<ma::VoteSnapshot, String> {
    let id = text_at(row, 0)?;
    let value = text_at(row, 3)?;
    let value = value_of(&value).ok_or(format!("vote {id}: value {value:?}"))?;
    Ok(ma::VoteSnapshot {
        state: ma::VoteState::Cast,
        data: ma::VoteData {
            vote_id: ma::VoteId(id),
            answer_id: ma::AnswerId(text_at(row, 1)?),
            identity: text_at(row, 2)?,
            value,
            comment: opt_text_at(row, 4)?,
            at: stamp(int_at(row, 5)?),
        },
    })
}

fn review_row(row: &[Value]) -> Result<ma::ReviewItemSnapshot, String> {
    let id = text_at(row, 0)?;
    let (cause, state) = (text_at(row, 2)?, text_at(row, 3)?);
    let (Some(cause), Some(state)) = (cause_of(&cause), review_state_of(&state)) else {
        return Err(format!(
            "review item {id}: cause {cause:?}, state {state:?}"
        ));
    };
    Ok(ma::ReviewItemSnapshot {
        state,
        data: ma::ReviewItemData {
            review_id: ma::ReviewId(id),
            answer_id: ma::AnswerId(text_at(row, 1)?),
            cause,
            opened_at: stamp(int_at(row, 4)?),
            close_reason: opt_text_at(row, 5)?,
            closed_at: opt_int_at(row, 6)?.map(stamp),
        },
    })
}

/// A write refused by the answer's foreign key: the answer is not kept.
fn kept_answer(result: Result<u64, SqlError>) -> Result<u64, String> {
    match result {
        Err(e) if e.code.as_deref() == Some(FOREIGN_KEY_VIOLATION) => Err(NO_SUCH_ANSWER.into()),
        other => other.map_err(|e| e.message),
    }
}

impl RecordStore for PgRecords {
    fn answer(&self, id: &ma::AnswerId) -> Result<Option<ma::AnswerRecordSnapshot>, String> {
        let rows = self.rows(
            &format!("{ANSWER} WHERE answer_id = $1::text"),
            &[Param::text(&id.0)],
        )?;
        rows.first()
            .map(|row| {
                answer_row(row).map(|data| ma::AnswerRecordSnapshot {
                    state: ma::AnswerRecordState::Kept,
                    data,
                })
            })
            .transpose()
    }

    fn put_answer(&self, record: &ma::AnswerRecordSnapshot) -> Result<(), String> {
        let d = &record.data;
        let at = ms_of(&d.at)?;
        self.exec(
            "INSERT INTO answers (answer_id, instance_name, identity, question, answer, reason, \
             outcome, tool_calls, latency_ms, input_tokens, output_tokens, cached_tokens, \
             context_tokens, steps, at_ms) VALUES ($1::text, $2::text, $3::text, $4::text, \
             $5::text, $6::text, $7::text, $8::text, $9::bigint, $10::bigint, $11::bigint, \
             $12::bigint, $13::bigint, $14::bigint, $15::bigint) \
             ON CONFLICT (answer_id) DO UPDATE SET instance_name = EXCLUDED.instance_name, \
             identity = EXCLUDED.identity, question = EXCLUDED.question, \
             answer = EXCLUDED.answer, reason = EXCLUDED.reason, outcome = EXCLUDED.outcome, \
             tool_calls = EXCLUDED.tool_calls, latency_ms = EXCLUDED.latency_ms, \
             input_tokens = EXCLUDED.input_tokens, output_tokens = EXCLUDED.output_tokens, \
             cached_tokens = EXCLUDED.cached_tokens, context_tokens = EXCLUDED.context_tokens, \
             steps = EXCLUDED.steps, at_ms = EXCLUDED.at_ms",
            &[
                Param::text(&d.answer_id.0),
                Param::text(&d.instance_name.0),
                Param::text(&d.identity),
                Param::text(&d.question),
                Param::Text(d.answer.clone()),
                Param::Text(d.reason.clone()),
                Param::text(outcome_name(d.outcome)),
                Param::text(tools_json(&d.tool_calls)),
                Param::Int(Some(d.latency_ms)),
                Param::Int(Some(d.input_tokens)),
                Param::Int(Some(d.output_tokens)),
                Param::Int(Some(d.cached_tokens)),
                Param::Int(Some(d.context_tokens)),
                Param::Int(Some(d.steps)),
                Param::Int(Some(at)),
            ],
        )
        .map(drop)
    }

    fn delete_answer(&self, id: &ma::AnswerId) -> Result<(), String> {
        self.exec(
            "DELETE FROM answers WHERE answer_id = $1::text",
            &[Param::text(&id.0)],
        )
        .map(drop)
    }

    fn vote(&self, id: &ma::VoteId) -> Result<Option<ma::VoteSnapshot>, String> {
        let rows = self.rows(
            &format!("{VOTE} WHERE vote_id = $1::text"),
            &[Param::text(&id.0)],
        )?;
        rows.first().map(|row| vote_row(row)).transpose()
    }

    fn put_vote(&self, vote: &ma::VoteSnapshot) -> Result<(), String> {
        let d = &vote.data;
        let at = ms_of(&d.at)?;
        // The answer's presence is checked in the statement that writes the vote, and held by
        // the foreign key against a purge in another transaction.
        let written = kept_answer(self.sql.execute(
            "INSERT INTO votes (vote_id, answer_id, identity, value, comment, at_ms) \
             SELECT $1::text, $2::text, $3::text, $4::text, $5::text, $6::bigint \
             WHERE EXISTS (SELECT 1 FROM answers WHERE answer_id = $2::text) \
             ON CONFLICT (vote_id) DO UPDATE SET answer_id = EXCLUDED.answer_id, \
             identity = EXCLUDED.identity, value = EXCLUDED.value, \
             comment = EXCLUDED.comment, at_ms = EXCLUDED.at_ms",
            &[
                Param::text(&d.vote_id.0),
                Param::text(&d.answer_id.0),
                Param::text(&d.identity),
                Param::text(value_name(d.value)),
                Param::Text(d.comment.clone()),
                Param::Int(Some(at)),
            ],
        ))?;
        if written == 0 {
            return Err(NO_SUCH_ANSWER.into());
        }
        Ok(())
    }

    fn delete_vote(&self, id: &ma::VoteId) -> Result<(), String> {
        self.exec(
            "DELETE FROM votes WHERE vote_id = $1::text",
            &[Param::text(&id.0)],
        )
        .map(drop)
    }

    fn review(&self, id: &ma::ReviewId) -> Result<Option<ma::ReviewItemSnapshot>, String> {
        let rows = self.rows(
            &format!("{REVIEW} WHERE review_id = $1::text"),
            &[Param::text(&id.0)],
        )?;
        rows.first().map(|row| review_row(row)).transpose()
    }

    fn put_review(&self, item: &ma::ReviewItemSnapshot) -> Result<(), String> {
        let d = &item.data;
        let opened = ms_of(&d.opened_at)?;
        let closed = d.closed_at.as_ref().map(ms_of).transpose()?;
        match item.state {
            ma::ReviewItemState::Open => {
                let written = kept_answer(self.sql.execute(
                    "INSERT INTO reviews (review_id, answer_id, cause, state, opened_ms, \
                     close_reason, closed_ms) SELECT $1::text, $2::text, $3::text, $4::text, \
                     $5::bigint, $6::text, $7::bigint \
                     WHERE EXISTS (SELECT 1 FROM answers WHERE answer_id = $2::text) \
                     ON CONFLICT (review_id) DO NOTHING",
                    &[
                        Param::text(&d.review_id.0),
                        Param::text(&d.answer_id.0),
                        Param::text(cause_name(d.cause)),
                        Param::text(review_state_name(item.state)),
                        Param::Int(Some(opened)),
                        Param::Text(d.close_reason.clone()),
                        Param::Int(closed),
                    ],
                ))?;
                let kept = !self
                    .rows(
                        "SELECT 1 FROM answers WHERE answer_id = $1::text",
                        &[Param::text(&d.answer_id.0)],
                    )?
                    .is_empty();
                // An item opened at once by another request is the one this open meant.
                if written == 0 && !kept {
                    return Err(NO_SUCH_ANSWER.into());
                }
                Ok(())
            }
            ma::ReviewItemState::Closed => {
                let written = self.exec(
                    "UPDATE reviews SET state = $2::text, close_reason = $3::text, \
                     closed_ms = $4::bigint WHERE review_id = $1::text AND state = $5::text",
                    &[
                        Param::text(&d.review_id.0),
                        Param::text(review_state_name(item.state)),
                        Param::Text(d.close_reason.clone()),
                        Param::Int(closed),
                        Param::text(review_state_name(ma::ReviewItemState::Open)),
                    ],
                )?;
                if written == 0 {
                    return Err(REVIEW_NOT_OPEN.into());
                }
                Ok(())
            }
        }
    }

    fn delete_review(&self, id: &ma::ReviewId) -> Result<(), String> {
        self.exec(
            "DELETE FROM reviews WHERE review_id = $1::text",
            &[Param::text(&id.0)],
        )
        .map(drop)
    }

    fn answers(&self) -> Result<Vec<ma::AnswerRecordData>, String> {
        self.rows(&format!("{ANSWER} ORDER BY at_ms, answer_id"), &[])?
            .iter()
            .map(|row| answer_row(row))
            .collect()
    }

    fn votes(&self, answer_id: &str) -> Result<Vec<ma::VoteData>, String> {
        self.rows(
            &format!("{VOTE} WHERE answer_id = $1::text ORDER BY at_ms, vote_id"),
            &[Param::text(answer_id)],
        )?
        .iter()
        .map(|row| vote_row(row).map(|v| v.data))
        .collect()
    }

    fn reviews(&self, answer_id: &str) -> Result<Vec<ma::ReviewItemSnapshot>, String> {
        self.rows(
            &format!("{REVIEW} WHERE answer_id = $1::text ORDER BY opened_ms, review_id"),
            &[Param::text(answer_id)],
        )?
        .iter()
        .map(|row| review_row(row))
        .collect()
    }

    fn open_reviews(&self) -> Result<Vec<ma::OpenReviews>, String> {
        self.rows(
            &format!("{REVIEW} WHERE state = $1::text ORDER BY opened_ms, review_id"),
            &[Param::text(review_state_name(ma::ReviewItemState::Open))],
        )?
        .iter()
        .map(|row| {
            review_row(row).map(|item| ma::OpenReviews {
                review_id: item.data.review_id,
                answer_id: item.data.answer_id,
                cause: item.data.cause,
                opened_at: item.data.opened_at,
            })
        })
        .collect()
    }

    fn purge(&self, before_ms: i64) -> Result<Purge, String> {
        // A vote or an item whose answer is gone has no meaning: it goes too, whatever left it.
        let expired = "answer_id IN (SELECT answer_id FROM answers WHERE at_ms < $1::bigint) \
            OR answer_id NOT IN (SELECT answer_id FROM answers)";
        let votes = format!("DELETE FROM votes WHERE {expired}");
        let reviews = format!("DELETE FROM reviews WHERE {expired}");
        let before = vec![Param::Int(Some(before_ms))];
        let counts = self
            .sql
            .batch(&[
                (votes.as_str(), before.clone()),
                (reviews.as_str(), before.clone()),
                ("DELETE FROM answers WHERE at_ms < $1::bigint", before),
            ])
            .map_err(|e| e.message)?;
        let count = |i: usize| counts.get(i).copied().unwrap_or(0) as i64;
        Ok(Purge {
            records: count(2),
            votes: count(0),
            reviews: count(1),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_records_schema_is_a_lowercase_identifier_not_postgresqls_own() {
        for good in ["cortex_ask", "_a", "a1", &"a".repeat(63)] {
            assert_eq!(schema_refusal(good), None, "{good}");
        }
        for bad in [
            "",
            "Cortex",
            "a-b",
            "1a",
            "a b",
            "a\"b",
            "pg_x",
            &"a".repeat(64),
        ] {
            let refused = schema_refusal(bad).expect(bad);
            assert!(refused.starts_with("ask.records.schema"), "{refused}");
        }
    }

    #[test]
    fn a_records_url_names_a_role_and_carries_no_password() {
        assert_eq!(url_refusal("postgres://cortex@db.example:5432/brain"), None);
        assert_eq!(
            url_refusal("postgresql://cortex@db.example/brain?sslmode=disable"),
            None
        );
        for (bad, secret) in [
            ("postgres://cortex:hunter2-x@db/brain", "hunter2-x"),
            ("postgres://cortex@db/brain?password=hunter2-y", "hunter2-y"),
            ("host=db user=cortex password=hunter2-z", "hunter2-z"),
            ("postgres://db/brain", "db/brain"),
            ("mysql://cortex@db/brain", "mysql"),
        ] {
            let refused = url_refusal(bad).expect(bad);
            assert!(refused.starts_with("ask.records.url"), "{refused}");
            assert!(!refused.contains(secret), "{refused}");
        }
    }

    /// The statements and their parameters reach the records process as they were given, and
    /// its answers, failures with their SQLSTATE included, come back as they were made.
    #[test]
    fn a_request_and_its_answer_cross_the_records_process_unchanged() {
        struct Echo;
        impl Sql for Echo {
            fn query(&self, sql: &str, params: &[Param]) -> Result<Vec<Vec<Value>>, SqlError> {
                Ok(vec![vec![
                    Value::from(sql),
                    params_json(params),
                    Value::Null,
                    Value::from(-5i64),
                ]])
            }
            fn execute(&self, _: &str, params: &[Param]) -> Result<u64, SqlError> {
                if params.is_empty() {
                    return Err(SqlError {
                        code: Some(FOREIGN_KEY_VIOLATION.into()),
                        message: "fk".into(),
                    });
                }
                Ok(params.len() as u64)
            }
            fn batch(&self, statements: &[(&str, Vec<Param>)]) -> Result<Vec<u64>, SqlError> {
                Ok(statements.iter().map(|(_, p)| p.len() as u64).collect())
            }
        }
        let params = vec![
            Param::text("a\"b\n"),
            Param::Text(None),
            Param::Int(Some(i64::MIN)),
            Param::Int(None),
        ];
        let round =
            |request: Value| answer(&Echo, &serde_json::from_str(&request.to_string()).unwrap());
        let rows = round(json!({"op": "query", "sql": "SELECT 1", "params": params_json(&params)}));
        let row = rows["rows"][0].as_array().unwrap();
        assert_eq!(row[0], "SELECT 1");
        assert_eq!(params_of(&row[1]).unwrap(), params);
        assert_eq!(row[2], Value::Null);
        assert_eq!(row[3], -5);
        let count = round(json!({"op": "execute", "sql": "x", "params": params_json(&params)}));
        assert_eq!(count["count"], 4);
        let failed = round(json!({"op": "execute", "sql": "x", "params": []}));
        assert_eq!(failed["code"], FOREIGN_KEY_VIOLATION);
        assert_eq!(
            kept_answer(Err(SqlError {
                code: failed["code"].as_str().map(str::to_string),
                message: "fk".into()
            })),
            Err(NO_SUCH_ANSWER.to_string())
        );
        let counts = round(json!({"op": "batch", "statements": [
            {"sql": "a", "params": params_json(&params)}, {"sql": "b", "params": []}]}));
        assert_eq!(counts["counts"], json!([4, 0]));
        assert!(round(json!({"op": "drop"}))["err"].is_string());
    }

    #[test]
    fn the_records_process_without_a_password_document_says_how_to_start_it() {
        let dir = tempfile::tempdir().unwrap();
        let refused = password_of(&dir.path().join("missing")).unwrap_err();
        assert!(refused.contains(CONSUMER), "{refused}");
        let file = dir.path().join("doc");
        std::fs::write(&file, r#"{"password": "pw"}"#).unwrap();
        assert_eq!(password_of(&file).unwrap(), "pw");
    }

    #[test]
    fn the_launch_runs_the_pinned_consumer_through_the_records_connection() {
        let launch = Launch {
            connectors: "connectors".into(),
            adapter: "postgres".into(),
            connection: "conn_1".into(),
            url: "postgres://cortex@db/brain".into(),
            schema: "cortex_ask".into(),
            ca_file: Some("/etc/ca.pem".into()),
            remove: vec!["GW_TOKEN".into()],
        };
        let cmd = launch.command();
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            [
                "connections",
                "launch",
                "--adapter",
                "postgres",
                "--connection",
                "conn_1",
                "--consumer",
                CONSUMER,
                "--args",
                r#"["ask","records","--url","postgres://cortex@db/brain","--schema","cortex_ask","--ca-file","/etc/ca.pem"]"#,
            ]
        );
        assert!(cmd.get_envs().any(|(k, v)| k == "GW_TOKEN" && v.is_none()));
    }
}
