//! The pinned `ekr` binary: seed, ontology, apply-extraction, and the formats cortex reads from it.
//!
//! A run sends its reads and transactions through one `ekr session` ([`Session`]); every other
//! caller, and a run's `apply-extraction` and `ekr stage` verbs, start one `ekr` per verb.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};

use serde::de::{DeserializeOwned, IgnoredAny};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

/// An empty ontology and an empty graph at revision 0: what `apply-extraction` grows a store from.
pub const MINIMAL_SEED: &str = "\
format: ekr-seed/2
ontology:
  version:
    id: 00000000-0000-4000-8000-000000000001
    number: 0
    parent: null
    created_at: 0
  node_types: []
  edge_types: []
graph:
  format: ekr.graph-document/2
  graph:
    root:
      id: 00000000-0000-4000-8000-000000000002
      space: Canonical
      schema_version_id: 00000000-0000-4000-8000-000000000001
      parent: null
      created_at: 0
    revision: 0
    nodes: {}
    edges: {}
    assertions: {}
    evidence: {}
evidence_payloads: {}
";

/// Where the pinned `ekr` lives when the spec names no `bin`.
pub fn default_bin(version: &str) -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    home.join(".cache/cortex/bin").join(version).join("bin/ekr")
}

pub fn resolve_bin(version: &str, bin: Option<&str>) -> PathBuf {
    match bin {
        Some(bin) => expand(bin),
        None => default_bin(version),
    }
}

/// `~/` at the start of a path, expanded.
pub fn expand(path: &str) -> PathBuf {
    match path.strip_prefix("~/") {
        Some(rest) => std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default()
            .join(rest),
        None => PathBuf::from(path),
    }
}

/// Installs `ekr` at `version` with `cargo install`, as its release tag.
pub fn install(version: &str) -> Result<PathBuf, String> {
    let bin = default_bin(version);
    if bin.is_file() {
        return Ok(bin);
    }
    let root = bin
        .parent()
        .and_then(Path::parent)
        .expect("bin/ekr under a root");
    let status = Command::new("nice")
        .args(["-n", "19", "cargo", "install", "--locked", "--git"])
        .arg("https://github.com/beyond10x/epistemic-knowledge-runtime")
        .args(["--tag", version, "--bin", "ekr", "--root"])
        .arg(root)
        .arg("ekr")
        .env("CARGO_BUILD_JOBS", "4")
        .status()
        .map_err(|e| format!("cannot run cargo: {e}"))?;
    if !status.success() || !bin.is_file() {
        return Err(format!("installing ekr {version} failed"));
    }
    Ok(bin)
}

fn run(cmd: Command, what: &str) -> Result<Vec<u8>, String> {
    finished(output(cmd, what)?, what)
}

/// What `cmd` exited with, printed and wrote to stderr.
fn output(mut cmd: Command, what: &str) -> Result<std::process::Output, String> {
    cmd.output()
        .map_err(|e| format!("cannot run ekr for {what}: {e}"))
}

/// The stdout of `out`, or `ekr <what> failed: <its stderr>` when it exited other than 0.
fn finished(out: std::process::Output, what: &str) -> Result<Vec<u8>, String> {
    if out.status.success() {
        Ok(out.stdout)
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err(format!("ekr {what} failed: {}", err.trim()))
    }
}

/// What EKR prints when it cannot say whether a publication took effect (`StoreError::UnknownCommit`
/// in EKR 0.0.34, `crates/ekr-store/src/lib.rs`). EKR gives it no refusal name and no exit status
/// of its own (a fault: `ekr: <message>`, exit 1), so the message is what tells it apart.
const OUTCOME_UNKNOWN: &str = "publication outcome is unknown";

/// Whether `error`, as a store verb answered it, says the verb's outcome is unknown: it may or may
/// not have taken effect, and the store, not the caller, is at fault.
pub fn outcome_unknown(error: &str) -> bool {
    error.contains(OUTCOME_UNKNOWN)
}

fn json(bytes: &[u8], what: &str) -> Result<Value, String> {
    serde_json::from_slice(bytes).map_err(|e| format!("ekr {what} printed no JSON: {e}"))
}

/// The pinned binary, before any store exists.
pub struct Binary(pub PathBuf);

impl Binary {
    /// `ekr example ekr.cli-host/1`, with this instance's tenant and the validation profile that
    /// admits schema changes (`ekr.p2-apply/1`), which `apply-extraction` needs to grow the ontology.
    pub fn host_json(&self, tenant: &str) -> Result<String, String> {
        let mut cmd = Command::new(&self.0);
        cmd.args(["example", "ekr.cli-host/1"]);
        let mut host = json(&run(cmd, "example ekr.cli-host/1")?, "example")?;
        host["tenant"] = Value::String(tenant.to_string());
        let profile = &mut host["authority"]["validation_profile"];
        profile["application"] = Value::String("ekr.p2-apply/1".into());
        profile["ruleset"] = Value::String("ekr.p2-deterministic/1".into());
        Ok(serde_json::to_string_pretty(&host).expect("plain JSON"))
    }

    /// `ekr schema ekr.extraction-document/1` without the `evidence` section, which cortex fills,
    /// and without `$schema`, which `claude --json-schema` refuses for draft 2020-12.
    pub fn model_schema(&self) -> Result<Value, String> {
        let mut cmd = Command::new(&self.0);
        cmd.args(MODEL_SCHEMA);
        model_schema(json(&run(cmd, "schema")?, "schema")?)
    }
}

/// The argument list of `ekr schema` for the document the model answers in.
const MODEL_SCHEMA: [&str; 2] = ["schema", "ekr.extraction-document/1"];

/// [`Binary::model_schema`] of `schema`, the document `ekr schema ekr.extraction-document/1` prints.
fn model_schema(mut schema: Value) -> Result<Value, String> {
    let obj = schema
        .as_object_mut()
        .ok_or("ekr schema is not an object")?;
    obj.remove("$schema");
    if let Some(props) = obj.get_mut("properties").and_then(Value::as_object_mut) {
        props.remove("evidence");
    }
    if let Some(req) = obj.get_mut("required").and_then(Value::as_array_mut) {
        req.retain(|r| r != "evidence");
    }
    if let Some(defs) = obj.get_mut("$defs").and_then(Value::as_object_mut) {
        defs.remove("ExtractionEvidence");
    }
    Ok(schema)
}

/// The EKR provider a store lives on (`EKR_BACKEND`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// One SQLite file.
    Sqlite,
    /// Hosted PostgreSQL, configured by an `ekr.postgres/1` file.
    Postgres,
}

impl Backend {
    pub fn name(self) -> &'static str {
        match self {
            Backend::Sqlite => "sqlite",
            Backend::Postgres => "postgres",
        }
    }
}

/// The name of the `[consumers]` entry in the operator's Connectors configuration that pins the
/// `ekr` binary a launch runs, with `pass_env = ["EKR_"]`.
pub const CONSUMER: &str = "ekr";

/// A saved Connectors connection: its adapter alias, and its id or a profile of the adapter, which
/// [`Launch::resolve`] turns into the id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    pub adapter: String,
    pub connection: crate::connectors::Reference,
}

/// How a PostgreSQL store's `ekr` gets the database password: `connectors connections launch`
/// starts the `ekr` the operator pinned as consumer [`CONSUMER`], with the connection's saved
/// `{"password": …}` document on descriptor 3, which the `ekr.postgres/1` file names as its
/// `password_file`. cortex never reads the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    /// The `connectors` binary.
    pub connectors: PathBuf,
    /// The application role's connection, for every `ekr` that opens the store.
    pub connection: Connection,
    /// The schema-management role's connection, for [`Store::provision`].
    pub schema_connection: Option<Connection>,
}

/// One instance's store, through its host.
#[derive(Clone)]
pub struct Store {
    pub bin: PathBuf,
    pub host: PathBuf,
    pub backend: Backend,
    /// `EKR_STORE`: the SQLite file, or the `ekr.postgres/1` file. cortex passes the path; for a
    /// launched store it reads only the file's `password_file` (`instance::launch_refusal`).
    pub store: PathBuf,
    /// For a PostgreSQL store that names a connection: every `ekr` starts through the launch.
    pub launch: Option<Launch>,
    /// `EKR_STAGE`: the stage every `ekr` of a staged run joins ([`Store::begin_stage`]), so its
    /// reads see the run's own commits and its writes reach the store only when the stage is
    /// published. None outside a staged run.
    pub stage: Option<String>,
    /// Whether a launched store's connection was found ready, shared by the handle's clones (its
    /// stage's and its session's), so a run checks it once ([`Store::command`]).
    pub ready: Readiness,
}

/// Whether a launched store's connection was found ready ([`Store::ready`]): checked before the
/// first `ekr` a store handle launches, and again only after a launch was refused because the
/// connection's validation evidence lapsed meanwhile ([`lapsed`]).
#[derive(Debug, Clone, Default)]
pub struct Readiness(Arc<std::sync::atomic::AtomicBool>);

impl Readiness {
    fn held(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn set(&self, ready: bool) {
        self.0.store(ready, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Whether `stderr`, what a `connectors connections launch` wrote when it failed, is its refusal
/// of a connection whose validation evidence lapsed: `unavailable` at `readiness` (Connectors
/// `crates/connectors-host/src/local/runtime/launch/tests.rs`, "Lapsed evidence"). The consumer
/// was not started, so the launch can be made again once the connection is ready.
fn lapsed(stderr: &[u8]) -> bool {
    let refusal = |v: Value| {
        let data = &v["error"]["data"];
        data["code"] == "unavailable" && data["stage"] == "readiness"
    };
    serde_json::from_slice::<Value>(stderr).is_ok_and(refusal)
        || stderr
            .split(|b| *b == b'\n')
            .any(|line| serde_json::from_slice::<Value>(line).is_ok_and(refusal))
}

impl Launch {
    /// `connectors connections launch --adapter A --connection C --consumer ekr --args '<JSON>'`:
    /// the pinned `ekr` with `args`, as one JSON array of strings, after the consumer's pinned
    /// argument prefix. Built and not run.
    fn command(&self, connection: &Connection, args: &[&OsStr]) -> Result<Command, String> {
        let id = self.resolve(connection)?;
        let args = args
            .iter()
            .map(|a| {
                a.to_str()
                    .map(str::to_string)
                    .ok_or_else(|| format!("an ekr argument is not UTF-8: {}", a.to_string_lossy()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut cmd = Command::new(&self.connectors);
        remove_inherited_ekr(&mut cmd, std::env::vars_os().map(|(key, _)| key));
        cmd.args([
            "connections",
            "launch",
            "--adapter",
            &connection.adapter,
            "--connection",
            &id,
            "--consumer",
            CONSUMER,
            "--args",
        ])
        .arg(serde_json::to_string(&args).expect("plain JSON"));
        Ok(cmd)
    }

    /// Renews `connection`'s validation evidence when it lapsed: a launch with lapsed evidence is
    /// refused (`unavailable` at `readiness`).
    fn ready(&self, connection: &Connection) -> Result<(), String> {
        let id = self.resolve(connection)?;
        crate::connectors::Connectors::new(self.connectors.clone())
            .ensure_ready(&connection.adapter, &id)
            .map_err(|e| format!("{}:{id}: {e}", connection.adapter))
    }

    /// The id `connection` names: an id as given, a profile resolved through `connections list`
    /// once per process ([`crate::connectors::Connectors::resolve`]). A refusal names the adapter,
    /// the profile, the count and every matching id.
    pub fn resolve(&self, connection: &Connection) -> Result<String, String> {
        crate::connectors::Connectors::new(self.connectors.clone())
            .resolve(&connection.adapter, &connection.connection)
            .map_err(|e| e.to_string())
    }
}

/// Removes from `cmd` every `EKR_*` variable of `inherited`, the names of the environment it would
/// inherit. A launch hands its consumer every `EKR_*` variable it gets (`pass_env = ["EKR_"]`), so
/// only the ones the caller sets afterwards reach the launched `ekr`, none this process inherited
/// (such as `EKR_FULL_REPLAY` from the operator's shell).
fn remove_inherited_ekr(cmd: &mut Command, inherited: impl IntoIterator<Item = OsString>) {
    for key in inherited {
        if key.as_encoded_bytes().starts_with(b"EKR_") {
            cmd.env_remove(&key);
        }
    }
}

impl Store {
    /// The connection every `ekr` that opens this store launches through, the viewer's included;
    /// none for a store `ekr` opens directly. The schema connection is provisioning's alone.
    pub fn launched_through(&self) -> Option<&Connection> {
        self.launch.as_ref().map(|l| &l.connection)
    }

    /// The `ekr` invocation of `args` on this store, built and not run: the pinned `ekr` itself, or
    /// for a launched store `connectors connections launch`, whose consumer gets the `EKR_*`
    /// variables (`pass_env`), `EKR_STAGE` in a staged run included. Outside one, an `EKR_STAGE`
    /// this process inherited is removed.
    fn cmd(&self, args: &[&OsStr]) -> Result<Command, String> {
        let mut cmd = self.store_cmd(args)?;
        match &self.stage {
            Some(stage) => cmd.env("EKR_STAGE", stage),
            None => cmd.env_remove("EKR_STAGE"),
        };
        Ok(cmd)
    }

    /// [`Store::cmd`] on the store itself, in no stage: the `ekr stage` verbs, which refuse
    /// `--stage`.
    fn store_cmd(&self, args: &[&OsStr]) -> Result<Command, String> {
        let mut cmd = match &self.launch {
            Some(launch) => launch.command(&launch.connection, args)?,
            None => {
                let mut cmd = Command::new(&self.bin);
                cmd.args(args);
                cmd
            }
        };
        cmd.env("EKR_HOST", &self.host)
            .env("EKR_BACKEND", self.backend.name())
            .env("EKR_STORE", &self.store)
            .env_remove("EKR_STAGE");
        Ok(cmd)
    }

    /// [`Store::cmd`], after a launched store's connection is ready.
    pub fn command(&self, args: &[&OsStr]) -> Result<Command, String> {
        self.ensure_ready()?;
        self.cmd(args)
    }

    /// Makes a launched store's connection ready, unless this handle (or a clone of it) already
    /// did: one `connections list` per run, not one per `ekr` it launches.
    fn ensure_ready(&self) -> Result<(), String> {
        if let Some(launch) = self.launch.as_ref().filter(|_| !self.ready.held()) {
            launch.ready(&launch.connection)?;
            self.ready.set(true);
        }
        Ok(())
    }

    /// What the `ekr` `build` makes exited with, printed and wrote to stderr. A launch refused
    /// because the connection's validation evidence lapsed since it was checked ([`lapsed`]) is
    /// made again once, after the connection is made ready again.
    fn launched_output(
        &self,
        build: impl Fn() -> Result<Command, String>,
        what: &str,
    ) -> Result<std::process::Output, String> {
        let out = output(build()?, what)?;
        if out.status.success() || self.launch.is_none() || !lapsed(&out.stderr) {
            return Ok(out);
        }
        self.ready.set(false);
        output(build()?, what)
    }

    /// [`Store::launched_output`], answering the stdout of an `ekr` that exited 0.
    fn run_verb(
        &self,
        build: impl Fn() -> Result<Command, String>,
        what: &str,
    ) -> Result<Vec<u8>, String> {
        finished(self.launched_output(build, what)?, what)
    }

    /// `ekr stage <args>` on the store, built and not run.
    fn stage_cmd(&self, args: &[&str]) -> Result<Command, String> {
        let args: Vec<&OsStr> = std::iter::once("stage")
            .chain(args.iter().copied())
            .map(OsStr::new)
            .collect();
        self.store_cmd(&args)
    }

    /// [`Store::stage_cmd`], after a launched store's connection is ready.
    fn stage_command(&self, args: &[&str]) -> Result<Command, String> {
        self.ensure_ready()?;
        self.stage_cmd(args)
    }

    /// `ekr stage <args>`, answering its stdout ([`Store::run_verb`]).
    fn stage_verb(&self, args: &[&str], what: &str) -> Result<Vec<u8>, String> {
        self.run_verb(|| self.stage_command(args), what)
    }

    /// `ekr stage begin`: a stage forked from the store at its head, answered by its id and its
    /// base, the head it was forked at (what `ekr stage publish --expect-head` names). The store's
    /// head and history do not move.
    pub fn begin_stage(&self) -> Result<(String, i64), String> {
        let begun = json(&self.stage_verb(&["begin"], "stage begin")?, "stage begin")?;
        match (begun["stage_id"].as_str(), begun["base"].as_i64()) {
            (Some(stage), Some(base)) => Ok((stage.to_string(), base)),
            _ => Err(format!(
                "ekr stage begin answered no stage_id and base: {begun}"
            )),
        }
    }

    /// The ids of this store's stages that are still `Begun` (`ekr stage list`, whose entries
    /// name their store, the host's `tenant`): stages a run began and neither published nor
    /// abandoned, each holding a copy of the store.
    pub fn begun_stages(&self) -> Result<Vec<String>, String> {
        let host = std::fs::read_to_string(&self.host)
            .map_err(|e| format!("cannot read {}: {e}", self.host.display()))?;
        let host: Value = serde_json::from_str(&host).map_err(|e| format!("host.json: {e}"))?;
        let tenant = host["tenant"]
            .as_str()
            .ok_or_else(|| "host.json names no tenant".to_string())?;
        let listed = json(&self.stage_verb(&["list"], "stage list")?, "stage list")?;
        let listed = listed
            .as_array()
            .ok_or_else(|| format!("ekr stage list answered no list: {listed}"))?;
        Ok(listed
            .iter()
            .filter(|s| s["state"] == "Begun" && s["store"] == tenant)
            .filter_map(|s| s["stage_id"].as_str().map(str::to_string))
            .collect())
    }

    /// `ekr stage publish <stage> --expect-head <head>`: every revision the stage committed lands
    /// in the store at once, or, when the store's head is no longer `head` (`stage-head-moved`),
    /// none does and the answer is the refusal.
    pub fn publish_stage(&self, stage: &str, head: i64) -> Result<(), String> {
        let head = head.to_string();
        self.stage_verb(&["publish", stage, "--expect-head", &head], "stage publish")
            .map(drop)
    }

    /// `ekr stage abandon <stage>`: the stage is forgotten; the store's head and history do not
    /// move. A stage already published is refused (`stage-already-published`).
    pub fn abandon_stage(&self, stage: &str) -> Result<(), String> {
        self.stage_verb(&["abandon", stage], "stage abandon")
            .map(drop)
    }

    /// The connection [`Store::provision`] launches through: the schema connection, when the
    /// store names one.
    fn schema_launch(&self) -> Option<(&Launch, &Connection)> {
        self.launch
            .as_ref()
            .and_then(|l| l.schema_connection.as_ref().map(|c| (l, c)))
    }

    /// `ekr postgres-schema --config <schema_config>`, built and not run, with no store variables:
    /// through the schema connection's launch, or the pinned `ekr` itself.
    fn schema_cmd(&self, schema_config: &Path) -> Result<Command, String> {
        let args = [
            OsStr::new("postgres-schema"),
            OsStr::new("--config"),
            schema_config.as_os_str(),
        ];
        let mut cmd = match self.schema_launch() {
            Some((launch, connection)) => launch.command(connection, &args)?,
            None => {
                let mut cmd = Command::new(&self.bin);
                cmd.args(args);
                cmd
            }
        };
        cmd.env_remove("EKR_HOST")
            .env_remove("EKR_BACKEND")
            .env_remove("EKR_STORE");
        Ok(cmd)
    }

    /// `ekr postgres-schema --config <schema_config>`: the provider's tables, created under the
    /// schema-management role that `schema_config` names. The store's own configuration names the
    /// DML-only application role, which EKR 0.0.30 refuses schema DDL; only creating an instance
    /// provisions, never a run.
    pub fn provision(&self, schema_config: &Path) -> Result<(), String> {
        if let Some((launch, connection)) = self.schema_launch() {
            launch.ready(connection)?;
        }
        let cmd = self.schema_cmd(schema_config)?;
        let receipt = json(&run(cmd, "postgres-schema")?, "postgres-schema")?;
        if receipt["format"] == "ekr.postgres-schema/1" && receipt["ready"] == true {
            Ok(())
        } else {
            Err(format!(
                "ekr postgres-schema reported no ready schema: {receipt}"
            ))
        }
    }

    /// The revision `ekr head` answers: the store's, or its stage's when it is in one.
    pub fn head(&self) -> Result<i64, String> {
        head(self)
    }

    /// One `ekr session` on this store, in its stage when it is in one: started as every verb of
    /// the store is ([`Store::command`]), so a launched store's connection is made ready once, for
    /// the session, and not again for its requests. It runs in a process group of its own, so a
    /// run sent SIGTERM while it waits for an answer ends it with what it started
    /// ([`crate::terminate::end`]).
    pub fn session(&self) -> Result<Session, String> {
        use std::os::unix::process::CommandExt;
        let mut cmd = self.command(&[OsStr::new("session")])?;
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0);
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("cannot run ekr session: {e}"))?;
        let stdin = child.stdin.take().expect("piped");
        let stdout = BufReader::new(Answers::of(child.stdout.take().expect("piped")));
        let mut from = child.stderr.take().expect("piped");
        // A request's own diagnostics travel in its answer; what the session itself writes to
        // stderr (why it could not start, or ended) is read as it comes, its end kept.
        let stderr = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&stderr);
        let drain = std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = from.read(&mut buf) {
                if n == 0 {
                    break;
                }
                let mut kept = kept.lock().expect("not poisoned");
                kept.extend_from_slice(&buf[..n]);
                let over = kept.len().saturating_sub(STDERR_KEPT);
                kept.drain(..over);
            }
        });
        Ok(Session {
            store: self.clone(),
            terminated: std::cell::Cell::new(false),
            pipe: RefCell::new(Some(Pipe {
                child,
                stdin,
                stdout,
                stderr,
                drain: Some(drain),
            })),
        })
    }

    /// Whether the store's lineage holds a seed: `ekr head` answers a head, or says "the lineage
    /// has no seed" (EKR 0.0.30, measured on PostgreSQL). Any other answer is an error.
    pub fn seeded(&self) -> Result<bool, String> {
        let out = self.launched_output(|| self.command(&[OsStr::new("head")]), "head")?;
        if out.status.success() {
            return Ok(true);
        }
        let err = String::from_utf8_lossy(&out.stderr);
        if err.contains("the lineage has no seed") {
            Ok(false)
        } else {
            Err(format!("ekr head failed: {}", err.trim()))
        }
    }

    /// Seeds the store and answers the `committed_at` (Unix milliseconds) of its
    /// `ekr.seed-result/1`. Seeding an identical seed again exits 0 with the first seed's result,
    /// so a `committed_at` from before this call means another caller wrote the seed.
    pub fn seed(&self, seed: &Path) -> Result<i64, String> {
        let args = [OsStr::new("seed"), seed.as_os_str()];
        let result = json(&self.run_verb(|| self.command(&args), "seed")?, "seed")?;
        result["committed_at"]
            .as_i64()
            .ok_or_else(|| "ekr seed answered no committed_at".to_string())
    }

    /// The `ekr.integrate.ExtractionReport` of applying `doc`.
    pub fn apply(&self, doc: &Path) -> Result<Value, String> {
        let args = [OsStr::new("apply-extraction"), doc.as_os_str()];
        json(
            &self.run_verb(|| self.command(&args), "apply-extraction")?,
            "apply-extraction",
        )
    }

    pub fn ontology(&self) -> Result<Value, String> {
        self.json(&[OsStr::new("ontology")], "ontology")
    }

    /// Writes one `ekr.transaction-document/2` of `operations`, proposed by `proposer` (the host
    /// operator), to `path`, and proposes, validates and commits it. Answers the committed
    /// revision; a transaction validation rejects, or one a commit in between left stale, is an
    /// error naming the issues, with nothing applied.
    pub fn transact(
        &self,
        path: &Path,
        proposer: &str,
        operations: &[Operation],
    ) -> Result<i64, String> {
        transact(self, path, proposer, operations)
    }

    /// A fresh id of `kind` (`ekr mint <kind>`): `transaction`, `type`, `property`,
    /// `schema-version`, ….
    pub fn mint(&self, kind: &str) -> Result<String, String> {
        mint(self, kind)
    }

    /// [`Store::transact`], answering a transaction validation rejects as its issues instead of an
    /// error. With `schema_version`, the transaction is a schema change producing that version.
    pub fn submit(
        &self,
        path: &Path,
        proposer: &str,
        operations: &[Operation],
        schema_version: Option<&str>,
    ) -> Result<Result<i64, Rejected>, String> {
        submit(self, path, proposer, operations, schema_version)
    }
}

/// What answers a store verb: one `ekr` per verb ([`Store`]), or a request to a run's
/// [`Session`]. Either answers a verb that failed as `ekr <what> failed: <what it wrote to
/// stderr>`, so a caller tells an unknown outcome ([`outcome_unknown`]) the same way from both.
trait Verbs {
    /// The JSON document the verb `args` prints.
    fn json(&self, args: &[&OsStr], what: &str) -> Result<Value, String>;

    /// The verb `args`, whatever it prints.
    fn ok(&self, args: &[&OsStr], what: &str) -> Result<(), String>;
}

impl Verbs for Store {
    fn json(&self, args: &[&OsStr], what: &str) -> Result<Value, String> {
        json(&self.run_verb(|| self.command(args), what)?, what)
    }

    fn ok(&self, args: &[&OsStr], what: &str) -> Result<(), String> {
        self.run_verb(|| self.command(args), what).map(drop)
    }
}

/// [`Store::head`] through `verbs`.
fn head(verbs: &impl Verbs) -> Result<i64, String> {
    let head = verbs.json(&[OsStr::new("head")], "head")?;
    head["revision"]
        .as_i64()
        .ok_or_else(|| format!("ekr head answered no revision: {head}"))
}

/// [`Store::mint`] through `verbs`.
fn mint(verbs: &impl Verbs, kind: &str) -> Result<String, String> {
    let minted = verbs.json(&[OsStr::new("mint"), OsStr::new(kind)], "mint")?;
    minted["id"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "ekr mint answered no id".to_string())
}

/// [`Store::transact`] through `verbs`.
fn transact(
    verbs: &impl Verbs,
    path: &Path,
    proposer: &str,
    operations: &[Operation],
) -> Result<i64, String> {
    match submit(verbs, path, proposer, operations, None)? {
        Ok(revision) => Ok(revision),
        Err(rejected) => {
            let issues: Vec<String> = rejected
                .issues
                .iter()
                .map(|i| format!("{}: {}", i.code, i.message))
                .collect();
            Err(format!(
                "ekr validate rejected transaction {}: {}",
                rejected.transaction,
                issues.join("; ")
            ))
        }
    }
}

/// [`Store::submit`] through `verbs`.
fn submit(
    verbs: &impl Verbs,
    path: &Path,
    proposer: &str,
    operations: &[Operation],
    schema_version: Option<&str>,
) -> Result<Result<i64, Rejected>, String> {
    let id = mint(verbs, "transaction")?;
    std::fs::write(
        path,
        transaction_document(&id, proposer, operations, schema_version),
    )
    .map_err(|e| format!("{}: {e}", path.display()))?;
    verbs.ok(&[OsStr::new("propose"), path.as_os_str()], "propose")?;
    let validated = verbs.json(&[OsStr::new("validate"), OsStr::new(&id)], "validate")?;
    if validated["kind"] != "Validated" {
        let issues = validated["issues"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|i| Issue {
                code: i["code"].as_str().unwrap_or("?").to_string(),
                message: i["message"].as_str().unwrap_or_default().to_string(),
            })
            .collect();
        return Ok(Err(Rejected {
            transaction: id,
            issues,
        }));
    }
    let committed = verbs.json(&[OsStr::new("commit"), OsStr::new(&id)], "commit")?;
    match committed["kind"].as_str() {
        Some("Committed") => committed["result"]["revision"]
            .as_i64()
            .map(Ok)
            .ok_or_else(|| "ekr commit answered no revision".to_string()),
        other => Err(format!(
            "ekr commit of transaction {id} answered {}",
            other.unwrap_or("nothing")
        )),
    }
}

/// How much of what an `ekr session` writes to its own stderr is kept, from its end.
const STDERR_KEPT: usize = 16 * 1024;

/// One `ekr session` on a store ([`Store::session`]): one process, opened once, that answers
/// `ontology`, `snapshot`, `schema`, `head`, `mint`, `propose`, `validate`, `commit` and `quality`,
/// one request at a time, each read against the store (or its stage) as it stands when the request
/// is read, so it sees what a separate `apply-extraction` committed before it (EKR 0.0.34
/// `docs/cli.md`, `ekr session`). The session does not serve `apply-extraction` or the `ekr stage`
/// verbs (`session-verb-refused`): [`Session::apply`] runs one `ekr` of its own.
///
/// A run holds one session from the moment its stage, if any, is begun, and closes it before the
/// stage is published or abandoned and before a snapshot is restored over a SQLite store, which
/// EKR asks of every process holding the store. Dropping it closes it.
pub struct Session {
    store: Store,
    /// The running process; none once it was closed or ended.
    pipe: RefCell<Option<Pipe>>,
    /// Whether it was ended because the run was sent SIGTERM while it waited for an answer.
    terminated: std::cell::Cell<bool>,
}

/// The start of what a session request that the run's SIGTERM ended fails with
/// ([`is_terminated`]).
pub const SESSION_TERMINATED: &str = "the run was sent SIGTERM while it waited for its ekr session";

/// Whether `error`, what a [`Session`] verb failed with, says the run's SIGTERM ended the wait.
pub fn is_terminated(error: &str) -> bool {
    error.contains(SESSION_TERMINATED)
}

/// The verbs a session answers by reading: a wait for one of them ends on the run's SIGTERM. A
/// `mint`, `propose`, `validate` or `commit` in flight is waited for, as a one-shot
/// `apply-extraction` is, so the run never stops not knowing whether a write took effect.
const READS: [&str; 5] = ["ontology", "snapshot", "schema", "quality", "head"];

/// How long a wait for a session's output sleeps between looks at the run's SIGTERM.
const LOOK: std::time::Duration = std::time::Duration::from_millis(20);

/// A session's stdout, read on a thread of its own into a short queue of chunks, so a wait for an
/// answer can end on the run's SIGTERM ([`crate::terminate::requested`]) while the answer is still
/// read from the pipe as it comes, a few chunks at a time.
struct Answers {
    chunks: std::sync::mpsc::Receiver<Vec<u8>>,
    chunk: Vec<u8>,
    at: usize,
    /// Whether the run's SIGTERM ends the current wait ([`READS`]).
    interruptible: bool,
    /// Whether it did.
    stopped: bool,
}

impl Answers {
    /// The chunks of what the session writes to `out`, at most 4 of 64 KiB held unread.
    fn of(mut out: ChildStdout) -> Self {
        let (tx, chunks) = std::sync::mpsc::sync_channel(4);
        std::thread::spawn(move || loop {
            let mut chunk = vec![0u8; 64 * 1024];
            match out.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    chunk.truncate(n);
                    if tx.send(chunk).is_err() {
                        break;
                    }
                }
            }
        });
        Answers {
            chunks,
            chunk: Vec::new(),
            at: 0,
            interruptible: false,
            stopped: false,
        }
    }
}

impl Read for Answers {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        use std::sync::mpsc::RecvTimeoutError;
        while self.at == self.chunk.len() {
            match self.chunks.recv_timeout(LOOK) {
                Ok(chunk) => {
                    self.chunk = chunk;
                    self.at = 0;
                }
                Err(RecvTimeoutError::Timeout) => {
                    if self.interruptible && crate::terminate::requested() {
                        self.stopped = true;
                        return Err(std::io::Error::other(SESSION_TERMINATED));
                    }
                }
                Err(RecvTimeoutError::Disconnected) => return Ok(0),
            }
        }
        let n = buf.len().min(self.chunk.len() - self.at);
        buf[..n].copy_from_slice(&self.chunk[self.at..self.at + n]);
        self.at += n;
        Ok(n)
    }
}

struct Pipe {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<Answers>,
    /// The end of what the session wrote to its own stderr ([`STDERR_KEPT`]).
    stderr: Arc<Mutex<Vec<u8>>>,
    drain: Option<std::thread::JoinHandle<()>>,
}

impl Pipe {
    /// Ends the session and what it started at once, its input first: SIGTERM to its process
    /// group, then SIGKILL after [`crate::terminate::GRACE`]. Its output is not waited for.
    fn kill(self) {
        let Pipe {
            mut child, stdin, ..
        } = self;
        drop(stdin);
        crate::terminate::end(&mut child);
    }

    /// Ends the input, so the session writes its replay checkpoint and exits, and waits for it.
    fn end(self) -> Result<(), String> {
        self.finish().1
    }

    /// [`Pipe::end`], with the exit status it waited for, when it could.
    fn finish(self) -> (Option<std::process::ExitStatus>, Result<(), String>) {
        let Pipe {
            mut child,
            stdin,
            stdout,
            stderr,
            drain,
        } = self;
        drop(stdin);
        drop(stdout);
        let status = child.wait();
        if let Some(drain) = drain {
            let _ = drain.join();
        }
        let said = String::from_utf8_lossy(&stderr.lock().expect("not poisoned"))
            .trim()
            .to_string();
        match status {
            Ok(status) if status.success() => (Some(status), Ok(())),
            Ok(status) => (
                Some(status),
                Err(format!("ekr session ended with {status}: {said}")),
            ),
            Err(e) => (None, Err(format!("cannot wait for ekr session: {e}"))),
        }
    }
}

/// How long a session that a signal ended may precede the run's own SIGTERM: a stop of the whole
/// unit (`systemctl stop`) signals the run and its session at once, and the session's end can be
/// read before the run's signal handler ran.
const SIGNAL_SKEW: std::time::Duration = std::time::Duration::from_millis(500);

/// Whether a session that ended with `status` ended because the run was sent SIGTERM: the run was
/// sent it, as `src/terminate.rs` holds for any child that ends after it, also when the session's
/// end, by a signal, is seen up to [`SIGNAL_SKEW`] before the run's own SIGTERM is.
fn ended_by_sigterm(status: Option<std::process::ExitStatus>) -> bool {
    use std::os::unix::process::ExitStatusExt;
    if crate::terminate::requested() {
        return true;
    }
    if status.and_then(|s| s.signal()).is_none() {
        return false;
    }
    let until = std::time::Instant::now() + SIGNAL_SKEW;
    while std::time::Instant::now() < until {
        if crate::terminate::requested() {
            return true;
        }
        std::thread::sleep(LOOK);
    }
    crate::terminate::requested()
}

/// One answer of an `ekr session`: what the one-shot verb exits with, prints (as its JSON
/// document, read into `T`) and writes to stderr.
#[derive(Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
struct Answer<T> {
    exit: i32,
    #[serde(default)]
    stdout: Option<T>,
    #[serde(default)]
    stderr: String,
}

/// Why no answer was read.
#[derive(Debug, PartialEq)]
enum Unread {
    /// The session's output ended or failed: it is gone.
    Ended(String),
    /// A line that is no answer, or whose document is not a `T`; the session reads on.
    Malformed(String),
}

/// The next answer on `reader`, its document read into `T` from the pipe as it comes, without
/// holding the line or a [`Value`] of it; then the rest of the line, its newline.
fn read_answer<T: DeserializeOwned>(reader: &mut impl BufRead) -> Result<Answer<T>, Unread> {
    let parsed = Answer::<T>::deserialize(&mut serde_json::Deserializer::from_reader(&mut *reader));
    let mut rest = Vec::new();
    match parsed {
        Ok(answer) => {
            reader
                .read_until(b'\n', &mut rest)
                .map_err(|e| Unread::Ended(e.to_string()))?;
            Ok(answer)
        }
        Err(e) if e.is_eof() => Err(Unread::Ended("it closed its output".into())),
        Err(e) if e.is_io() => Err(Unread::Ended(e.to_string())),
        Err(e) => match reader.read_until(b'\n', &mut rest) {
            Ok(0) | Err(_) => Err(Unread::Ended(format!("its output ended: {e}"))),
            Ok(_) => Err(Unread::Malformed(e.to_string())),
        },
    }
}

impl Session {
    /// The store the session is open on, for the verbs it does not serve.
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// The verb `args` as one request, answering the document it printed (none when it printed
    /// nothing), or `ekr <what> failed: <its stderr>` when it exited other than 0. A session that
    /// ended is closed, and every request after it fails.
    fn request<T: DeserializeOwned>(
        &self,
        args: &[&OsStr],
        what: &str,
    ) -> Result<Option<T>, String> {
        let argv = args
            .iter()
            .map(|a| {
                a.to_str()
                    .ok_or_else(|| format!("an ekr argument is not UTF-8: {}", a.to_string_lossy()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut line =
            serde_json::to_vec(&serde_json::json!({ "argv": argv })).expect("plain JSON");
        line.push(b'\n');
        let mut slot = self.pipe.borrow_mut();
        let Some(pipe) = slot.as_mut() else {
            if self.terminated.get() || crate::terminate::requested() {
                self.terminated.set(true);
                return Err(format!(
                    "{SESSION_TERMINATED} to answer `{what}`; the session had ended"
                ));
            }
            return Err(format!(
                "ekr {what} failed: the run's ekr session has ended"
            ));
        };
        pipe.stdout.get_mut().interruptible = argv.first().is_some_and(|verb| READS.contains(verb));
        let answer = match pipe
            .stdin
            .write_all(&line)
            .and_then(|()| pipe.stdin.flush())
        {
            Ok(()) => read_answer::<T>(&mut pipe.stdout),
            Err(e) => Err(Unread::Ended(e.to_string())),
        };
        if pipe.stdout.get_ref().stopped {
            slot.take().expect("matched above").kill();
            self.terminated.set(true);
            return Err(format!(
                "{SESSION_TERMINATED} to answer `{what}`; the session was ended"
            ));
        }
        match answer {
            Ok(answer) if answer.exit == 0 => Ok(answer.stdout),
            Ok(answer) => Err(format!("ekr {what} failed: {}", answer.stderr.trim())),
            Err(Unread::Malformed(e)) => Err(format!("ekr {what} printed no JSON: {e}")),
            // A session whose output ended (end of file, a failed read or write, or a process a
            // signal ended) while the run was sent SIGTERM ended because of it, a stop of the
            // whole unit (`systemctl stop`): the request, a read or a write, fails as the run's
            // SIGTERM ([`SESSION_TERMINATED`]), as one the run ended itself does.
            Err(Unread::Ended(e)) => {
                let (status, ended) = slot.take().expect("matched above").finish();
                if ended_by_sigterm(status) {
                    self.terminated.set(true);
                    return Err(format!(
                        "{SESSION_TERMINATED} to answer `{what}`; the session ended with it ({e})"
                    ));
                }
                let ended = ended.err();
                Err(format!(
                    "ekr {what} failed: the run's ekr session ended ({e}){}",
                    ended.map(|why| format!(": {why}")).unwrap_or_default()
                ))
            }
        }
    }

    /// Whether the session is gone: it ended, was closed, or was ended on the run's SIGTERM. A
    /// request that failed on a session that is not gone was refused by the verb itself.
    pub fn ended(&self) -> bool {
        self.pipe.borrow().is_none()
    }

    /// Whether the run's SIGTERM ended a wait for its answer, and the session with it.
    pub fn terminated(&self) -> bool {
        self.terminated.get()
    }

    /// Ends the session: its input is closed, so it writes its replay checkpoint and exits. A
    /// session that already ended is closed.
    pub fn close(&self) -> Result<(), String> {
        match self.pipe.borrow_mut().take() {
            Some(pipe) => pipe.end(),
            None => Ok(()),
        }
    }

    /// [`Store::head`], as a request.
    pub fn head(&self) -> Result<i64, String> {
        head(self)
    }

    /// [`Store::ontology`], as a request.
    pub fn ontology(&self) -> Result<Value, String> {
        self.json(&[OsStr::new("ontology")], "ontology")
    }

    /// The graph at the head (`ekr snapshot`), read into a [`Snapshot`] as the answer arrives.
    pub fn snapshot(&self) -> Result<Snapshot, String> {
        self.request(&[OsStr::new("snapshot")], "snapshot")?
            .ok_or_else(|| "ekr snapshot printed no JSON".to_string())
    }

    /// [`Binary::model_schema`], as a request.
    pub fn model_schema(&self) -> Result<Value, String> {
        let args = MODEL_SCHEMA.map(OsStr::new);
        model_schema(self.json(&args, "schema")?)
    }

    /// `ekr quality`, the `ekr.store-quality/1` document of the store at its newest revision.
    pub fn quality(&self) -> Result<Value, String> {
        self.json(&[OsStr::new("quality")], "quality")
    }

    /// [`Store::mint`], as a request.
    pub fn mint(&self, kind: &str) -> Result<String, String> {
        mint(self, kind)
    }

    /// [`Store::transact`], as requests.
    pub fn transact(
        &self,
        path: &Path,
        proposer: &str,
        operations: &[Operation],
    ) -> Result<i64, String> {
        transact(self, path, proposer, operations)
    }

    /// [`Store::submit`], as requests.
    pub fn submit(
        &self,
        path: &Path,
        proposer: &str,
        operations: &[Operation],
        schema_version: Option<&str>,
    ) -> Result<Result<i64, Rejected>, String> {
        submit(self, path, proposer, operations, schema_version)
    }

    /// [`Store::apply`]: one `ekr apply-extraction` of its own, which a session does not serve,
    /// in the session's stage.
    pub fn apply(&self, doc: &Path) -> Result<Value, String> {
        self.store.apply(doc)
    }
}

impl Verbs for Session {
    fn json(&self, args: &[&OsStr], what: &str) -> Result<Value, String> {
        self.request(args, what)?
            .ok_or_else(|| format!("ekr {what} printed no JSON"))
    }

    fn ok(&self, args: &[&OsStr], what: &str) -> Result<(), String> {
        self.request::<IgnoredAny>(args, what).map(drop)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Some(pipe) = self.pipe.get_mut().take() {
            let _ = pipe.end();
        }
    }
}

/// The graph `ekr snapshot` prints, read into the fields cortex uses and no others: each node's
/// type and names, each edge's ends, each assertion's lifecycle, cited evidence, subject,
/// predicate, object and valid time, and each evidence item's source identity. The rest (payloads,
/// properties, roots) is skipped as it is read. A field of another shape than EKR writes reads as
/// absent, as it did from a [`Value`].
#[derive(Debug, Default, Deserialize)]
pub struct Snapshot {
    #[serde(default)]
    pub graph: SnapshotGraph,
}

/// `graph` of a [`Snapshot`]: the `ekr.graph-document/2`.
#[derive(Debug, Default, Deserialize)]
pub struct SnapshotGraph {
    #[serde(default)]
    pub graph: Graph,
}

/// The graph itself, each map keyed by id.
#[derive(Debug, Default, Deserialize)]
pub struct Graph {
    #[serde(default)]
    pub nodes: BTreeMap<String, Node>,
    #[serde(default)]
    pub edges: BTreeMap<String, Edge>,
    #[serde(default)]
    pub assertions: BTreeMap<String, Assertion>,
    #[serde(default)]
    pub evidence: BTreeMap<String, Evidence>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Node {
    #[serde(default, deserialize_with = "lenient")]
    pub canonical_name: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    pub type_id: Option<String>,
    /// Its aliases as EKR holds them, in its order.
    #[serde(default, deserialize_with = "strings")]
    pub aliases: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Edge {
    #[serde(default, deserialize_with = "lenient")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    pub source: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    pub type_id: Option<String>,
    #[serde(default, deserialize_with = "lenient")]
    pub target: Option<String>,
}

/// An assertion. Its subject, predicate, object and valid time are small tagged documents, kept as
/// EKR writes them.
#[derive(Debug, Default, Deserialize)]
pub struct Assertion {
    #[serde(default, deserialize_with = "lenient")]
    pub id: Option<String>,
    /// `Active`, or how it ended.
    #[serde(default, deserialize_with = "lenient")]
    pub lifecycle: Option<String>,
    /// The ids of the evidence items it cites.
    #[serde(default, deserialize_with = "strings")]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub subject: Value,
    #[serde(default)]
    pub predicate: Value,
    #[serde(default)]
    pub object: Value,
    #[serde(default)]
    pub valid_time: Value,
}

/// An evidence item: the identity of its `HumanStatement` source, the only kind cortex writes.
#[derive(Debug, Default, Deserialize)]
pub struct Evidence {
    #[serde(default, deserialize_with = "lenient")]
    pub source: Option<EvidenceSource>,
}

#[derive(Debug, Default, Deserialize)]
pub struct EvidenceSource {
    #[serde(rename = "HumanStatement", default, deserialize_with = "lenient")]
    pub human_statement: Option<Statement>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Statement {
    #[serde(default, deserialize_with = "lenient")]
    pub identity: Option<String>,
}

impl Evidence {
    /// The identity of its `HumanStatement` source, if it has one.
    pub fn identity(&self) -> Option<&str> {
        self.source
            .as_ref()?
            .human_statement
            .as_ref()?
            .identity
            .as_deref()
    }
}

impl Snapshot {
    /// The snapshot `value` holds, an `ekr snapshot` document already read whole; one of another
    /// shape holds nothing.
    pub fn of(value: &Value) -> Self {
        Self::deserialize(value).unwrap_or_default()
    }
}

/// A `T`, or `T::default()` when the document holds another shape there.
fn lenient<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Either<T> {
        Is(T),
        Other(IgnoredAny),
    }
    Ok(match Either::<T>::deserialize(d)? {
        Either::Is(t) => t,
        Either::Other(_) => T::default(),
    })
}

/// The strings of a list, in order, its other items left out; none when it is no list.
fn strings<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Item {
        Text(String),
        Other(IgnoredAny),
    }
    let items: Vec<Item> = lenient(d)?;
    Ok(items
        .into_iter()
        .filter_map(|i| match i {
            Item::Text(t) => Some(t),
            Item::Other(_) => None,
        })
        .collect())
}

/// One issue of a rejected transaction, as `ekr validate` names it.
#[derive(Debug, Clone, PartialEq)]
pub struct Issue {
    pub code: String,
    pub message: String,
}

/// A transaction validation rejected: nothing of it was applied.
#[derive(Debug, Clone, PartialEq)]
pub struct Rejected {
    pub transaction: String,
    pub issues: Vec<Issue>,
}

/// A property as a schema change declares it (EKR `docs/cli.md`, "Property definitions").
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyDecl {
    pub id: String,
    pub name: String,
    /// `{value_kind: …}`, with `parameters` for a compound kind, as `ekr ontology` prints it.
    pub value_type: Value,
    /// `One` or `Many`.
    pub cardinality: String,
    pub required: bool,
    pub constraints: Vec<String>,
}

impl PropertyDecl {
    /// The declaration as `ekr ontology` prints it.
    pub fn json(&self) -> Value {
        serde_json::json!({
            "id": self.id,
            "name": self.name,
            "value_type": self.value_type,
            "cardinality": self.cardinality,
            "required": self.required,
            "constraints": self.constraints,
        })
    }

    fn yaml(&self) -> serde_yaml_ng::Value {
        use serde_yaml_ng::{Mapping, Value as Yaml};
        let s = |t: &str| Yaml::String(t.to_string());
        let mut m = Mapping::new();
        m.insert(s("id"), s(&self.id));
        m.insert(s("name"), s(&self.name));
        m.insert(
            s("value_type"),
            serde_yaml_ng::to_value(&self.value_type).expect("plain JSON"),
        );
        m.insert(s("cardinality"), s(&self.cardinality));
        m.insert(s("required"), Yaml::Bool(self.required));
        m.insert(
            s("constraints"),
            Yaml::Sequence(self.constraints.iter().map(|c| s(c)).collect()),
        );
        Yaml::Mapping(m)
    }
}

/// One operation of a transaction cortex writes itself.
#[derive(Debug, Clone, PartialEq)]
pub enum Operation {
    /// `!SupersedeAssertion`: `assertion` ends where `by`, its active replacement, begins.
    Supersede {
        assertion: String,
        by: String,
        effective_from: i64,
    },
    /// `!RetractAssertion`: `assertion` is withdrawn, with `reason`.
    Retract { assertion: String, reason: String },
    /// `!DeleteEdge`: the edge is removed.
    DeleteEdge(String),
    /// `!DefineNodeType`, a schema change: a node type with no parents and `properties`.
    DefineNodeType {
        id: String,
        name: String,
        properties: Vec<PropertyDecl>,
    },
    /// `!DefineEdgeType`, a schema change: an edge type between the node type ids given, `Many`,
    /// with no properties.
    DefineEdgeType {
        id: String,
        name: String,
        source_types: Vec<String>,
        target_types: Vec<String>,
    },
    /// `!ModifyProperty`, a schema change: `property` added to the node or edge type `owner`, or
    /// redeclared when `owner` declares its id.
    ModifyProperty {
        owner: String,
        property: PropertyDecl,
    },
    /// `!WidenEdgeType`, a schema change: the ends of the edge type `edge_type`, each written whole
    /// (every node type id it holds now and the ones added), since EKR refuses an end that loses
    /// a type.
    WidenEdgeType {
        edge_type: String,
        source_types: Vec<String>,
        target_types: Vec<String>,
    },
}

impl Operation {
    fn yaml(&self) -> serde_yaml_ng::Value {
        use serde_yaml_ng::value::{Tag, TaggedValue};
        use serde_yaml_ng::{Mapping, Value as Yaml};
        let s = |t: &str| Yaml::String(t.to_string());
        let (tag, value) = match self {
            Operation::Supersede {
                assertion,
                by,
                effective_from,
            } => {
                let mut m = Mapping::new();
                m.insert(s("assertion"), s(assertion));
                m.insert(s("by"), s(by));
                m.insert(s("effective_from"), Yaml::Number((*effective_from).into()));
                ("SupersedeAssertion", Yaml::Mapping(m))
            }
            Operation::Retract { assertion, reason } => {
                let mut m = Mapping::new();
                m.insert(s("assertion"), s(assertion));
                m.insert(s("reason"), s(reason));
                ("RetractAssertion", Yaml::Mapping(m))
            }
            Operation::DeleteEdge(edge) => ("DeleteEdge", s(edge)),
            Operation::DefineNodeType {
                id,
                name,
                properties,
            } => {
                let mut props = Mapping::new();
                for p in properties {
                    props.insert(s(&p.id), p.yaml());
                }
                let mut m = Mapping::new();
                m.insert(s("id"), s(id));
                m.insert(s("name"), s(name));
                m.insert(s("parents"), Yaml::Sequence(Vec::new()));
                m.insert(s("properties"), Yaml::Mapping(props));
                m.insert(s("abstract_type"), Yaml::Bool(false));
                m.insert(s("lifecycle"), Yaml::Null);
                m.insert(s("operations"), Yaml::Mapping(Mapping::new()));
                ("DefineNodeType", Yaml::Mapping(m))
            }
            Operation::DefineEdgeType {
                id,
                name,
                source_types,
                target_types,
            } => {
                let ids = |v: &[String]| Yaml::Sequence(v.iter().map(|t| s(t)).collect());
                let mut m = Mapping::new();
                m.insert(s("id"), s(id));
                m.insert(s("name"), s(name));
                m.insert(s("source_types"), ids(source_types));
                m.insert(s("target_types"), ids(target_types));
                m.insert(s("cardinality"), s("Many"));
                m.insert(s("properties"), Yaml::Mapping(Mapping::new()));
                m.insert(s("inverse"), Yaml::Null);
                m.insert(s("symmetric"), Yaml::Bool(false));
                m.insert(s("transitive"), Yaml::Bool(false));
                ("DefineEdgeType", Yaml::Mapping(m))
            }
            Operation::ModifyProperty { owner, property } => {
                let mut m = Mapping::new();
                m.insert(s("owner"), s(owner));
                m.insert(s("property"), property.yaml());
                ("ModifyProperty", Yaml::Mapping(m))
            }
            Operation::WidenEdgeType {
                edge_type,
                source_types,
                target_types,
            } => {
                let ids = |v: &[String]| Yaml::Sequence(v.iter().map(|t| s(t)).collect());
                let mut m = Mapping::new();
                m.insert(s("edge_type"), s(edge_type));
                m.insert(s("source_types"), ids(source_types));
                m.insert(s("target_types"), ids(target_types));
                ("WidenEdgeType", Yaml::Mapping(m))
            }
        };
        Yaml::Tagged(Box::new(TaggedValue {
            tag: Tag::new(tag),
            value,
        }))
    }
}

/// The `ekr.transaction-document/2` text of transaction `id`: `operations` in order, proposed by
/// `proposer`, citing no evidence (none of them adds an assertion, and a schema change cites none:
/// EKR holds a transaction's evidence set to its assertions, `evidence-set-mismatch`), and naming
/// the `schema_version` a schema change produces. The `format:` line comes first, on its own, as
/// EKR reads the byte cap from it.
fn transaction_document(
    id: &str,
    proposer: &str,
    operations: &[Operation],
    schema_version: Option<&str>,
) -> String {
    use serde_yaml_ng::{Mapping, Value as Yaml};
    let s = |t: &str| Yaml::String(t.to_string());
    let mut tx = Mapping::new();
    tx.insert(s("id"), s(id));
    tx.insert(s("proposer"), s(proposer));
    tx.insert(
        s("operations"),
        Yaml::Sequence(operations.iter().map(Operation::yaml).collect()),
    );
    tx.insert(s("evidence"), Yaml::Sequence(Vec::new()));
    if let Some(version) = schema_version {
        tx.insert(s("schema_version"), s(version));
    }
    let mut doc = Mapping::new();
    doc.insert(s("format"), s("ekr.transaction-document/2"));
    doc.insert(s("transaction"), Yaml::Mapping(tx));
    serde_yaml_ng::to_string(&Yaml::Mapping(doc)).expect("YAML")
}

/// The operator agent of a host document, which every evidence item names as `extracted_by`.
pub fn operator(host_json: &str) -> Result<String, String> {
    let host: Value = serde_json::from_str(host_json).map_err(|e| format!("host.json: {e}"))?;
    host["context"]["operator"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "host.json names no context.operator".to_string())
}

/// Commits and refusals in an apply report.
pub struct Applied {
    pub rejected: usize,
}

pub fn applied(report: &Value) -> Applied {
    let count = |key: &str| report[key].as_array().map_or(0, Vec::len);
    Applied {
        rejected: count("rejected"),
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::{OsStr, OsString};
    use std::path::PathBuf;

    use super::{
        outcome_unknown, read_answer, remove_inherited_ekr, transaction_document, Backend,
        Connection, Launch, Operation, PropertyDecl, Snapshot, Store, Unread,
    };

    /// Acceptance 3 of `story:ekr-session-per-run`: a session's `snapshot` answer is read into a
    /// typed [`Snapshot`] straight from the pipe. Only the fields cortex uses are kept; payloads
    /// and the rest are skipped as read, and the reader stops at the answer's newline, so the next
    /// answer on the same pipe is read whole.
    #[test]
    fn a_snapshot_answer_is_read_typed_from_the_pipe_and_the_next_answer_follows() {
        let payload: String = "x".repeat(100_000);
        let snapshot = serde_json::json!({
            "graph": {"format": "ekr.graph-document/2", "graph": {
                "revision": 3,
                "nodes": {"n-1": {"canonical_name": "Acme", "type_id": "t-1",
                    "aliases": ["Acme", 7, "acme inc"], "properties": {"p": {"big": payload}}}},
                "edges": {"e-1": {"id": "e-1", "source": "n-1", "type_id": "et-1",
                    "target": "n-2"}},
                "assertions": {"a-1": {"id": "a-1", "lifecycle": "Active", "evidence": ["ev-1"],
                    "subject": {"Node": "n-1"}, "predicate": {"Property": "p-1"},
                    "object": {"Value": {"value": 12345678901234567890u64}},
                    "valid_time": {"from": 5, "to": null}}},
                "evidence": {"ev-1": {"source": {"HumanStatement": {"identity": "record:r-1"}},
                    "payload": payload}}}},
            "evidence_payloads": {"ev-1": payload},
            "root": {"knowledge_root": "abc"},
        });
        let first = serde_json::json!({"exit": 0, "stdout": snapshot, "stderr": ""});
        let pipe = format!(
            "{first}\n{}\n",
            r#"{"exit":0,"stdout":{"revision":3},"stderr":""}"#
        );
        let mut reader = std::io::BufReader::new(pipe.as_bytes());

        let answer = read_answer::<Snapshot>(&mut reader).expect("an answer");
        assert_eq!(answer.exit, 0);
        let graph = answer.stdout.expect("a snapshot").graph.graph;
        let node = &graph.nodes["n-1"];
        assert_eq!(node.canonical_name.as_deref(), Some("Acme"));
        assert_eq!(node.type_id.as_deref(), Some("t-1"));
        assert_eq!(
            node.aliases,
            ["Acme", "acme inc"],
            "a non-string alias is left out"
        );
        assert_eq!(graph.edges["e-1"].target.as_deref(), Some("n-2"));
        let a = &graph.assertions["a-1"];
        assert_eq!(a.lifecycle.as_deref(), Some("Active"));
        assert_eq!(a.evidence, ["ev-1"]);
        assert_eq!(a.valid_time["from"].as_i64(), Some(5));
        assert_eq!(
            a.object["Value"]["value"].to_string(),
            "12345678901234567890",
            "a number keeps its digits"
        );
        assert_eq!(graph.evidence["ev-1"].identity(), Some("record:r-1"));

        let next = read_answer::<serde_json::Value>(&mut reader).expect("the next answer");
        assert_eq!(next.stdout.unwrap()["revision"], 3);
        assert!(matches!(
            read_answer::<serde_json::Value>(&mut reader),
            Err(Unread::Ended(_))
        ));
    }

    /// A refused request answers no document and its refusal; a line that is no answer is said as
    /// such and the line after it is read; a session that closed its output has ended.
    #[test]
    fn a_refusal_a_malformed_line_and_an_ended_session_are_told_apart() {
        let pipe = concat!(
            r#"{"exit":2,"stdout":null,"stderr":"ekr: session-verb-refused: no\n"}"#,
            "\n",
            "not json at all\n",
            r#"{"exit":0,"stdout":{"graph":{"graph":{"nodes":"no map"}}},"stderr":""}"#,
            "\n",
            r#"{"exit":0,"stdout":{"id":"m-1"},"stderr":""}"#,
            "\n",
            r#"{"exit":0,"stdout":{"#,
        );
        let mut reader = std::io::BufReader::new(pipe.as_bytes());
        let refused = read_answer::<Snapshot>(&mut reader).expect("an answer");
        assert_eq!(refused.exit, 2);
        assert!(refused.stdout.is_none());
        assert!(refused.stderr.contains("session-verb-refused"));
        assert!(matches!(
            read_answer::<Snapshot>(&mut reader),
            Err(Unread::Malformed(_))
        ));
        // A document of another shape than a snapshot is malformed too, and the pipe reads on.
        assert!(matches!(
            read_answer::<Snapshot>(&mut reader),
            Err(Unread::Malformed(_))
        ));
        let minted = read_answer::<serde_json::Value>(&mut reader).expect("an answer");
        assert_eq!(minted.stdout.unwrap()["id"], "m-1");
        assert!(matches!(
            read_answer::<serde_json::Value>(&mut reader),
            Err(Unread::Ended(_))
        ));
    }

    /// The snapshot a caller read whole, as a [`serde_json::Value`], reads as the same typed
    /// [`Snapshot`]; one of another shape holds nothing.
    #[test]
    fn a_snapshot_read_whole_reads_as_the_typed_one() {
        let value = serde_json::json!({"graph": {"graph": {"nodes": {
            "n-1": {"canonical_name": "Acme", "type_id": "t-1", "aliases": ["acme"]},
            "n-2": {"canonical_name": null, "aliases": null}}}}});
        let snapshot = Snapshot::of(&value);
        assert_eq!(snapshot.graph.graph.nodes.len(), 2);
        assert_eq!(snapshot.graph.graph.nodes["n-1"].aliases, ["acme"]);
        assert_eq!(snapshot.graph.graph.nodes["n-2"].canonical_name, None);
        assert!(Snapshot::of(&serde_json::json!({}))
            .graph
            .graph
            .nodes
            .is_empty());
        assert!(Snapshot::of(&serde_json::json!([1, 2]))
            .graph
            .graph
            .nodes
            .is_empty());
    }

    /// EKR 0.0.34 has no refusal name or exit status of its own for an unknown publication outcome
    /// (`StoreError::UnknownCommit` is a fault, exit 1): its message is what tells it apart.
    #[test]
    fn an_unknown_publication_outcome_is_told_from_a_refusal_by_its_message() {
        let unknown = "ekr stage begin failed: ekr: publication outcome is unknown; resolve the \
                       exact retained occurrence";
        assert!(outcome_unknown(unknown));
        for other in [
            "ekr stage publish failed: ekr: stage-head-moved: the head is 9, not 7",
            "ekr stage begin failed: ekr: the store is unavailable: connection refused",
            "ekr stage abandon failed: ekr: stage-already-published: st-1",
        ] {
            assert!(!outcome_unknown(other), "{other}");
        }
    }

    #[test]
    fn a_transaction_document_names_its_format_first_and_tags_each_operation() {
        let text = transaction_document(
            "t-1",
            "op-1",
            &[
                Operation::Supersede {
                    assertion: "a-1".into(),
                    by: "a-2".into(),
                    effective_from: 7,
                },
                Operation::Retract {
                    assertion: "a-3".into(),
                    reason: "P-2: gone".into(),
                },
                Operation::DeleteEdge("e-1".into()),
            ],
            None,
        );
        assert!(!text.contains("schema_version"), "{text}");
        assert!(
            text.starts_with("format: ekr.transaction-document/2\n"),
            "{text}"
        );
        for part in [
            "id: t-1",
            "proposer: op-1",
            "- !SupersedeAssertion",
            "effective_from: 7",
            "- !RetractAssertion",
            "reason: 'P-2: gone'",
            "- !DeleteEdge e-1",
            "evidence: []",
        ] {
            assert!(text.contains(part), "{part} in {text}");
        }
    }

    #[test]
    fn a_schema_change_names_its_version_and_writes_each_kind_as_ekr_declares_it() {
        let prop = |id: &str| PropertyDecl {
            id: id.into(),
            name: "founded".into(),
            value_type: serde_json::json!({"value_kind": "Integer"}),
            cardinality: "One".into(),
            required: id == "p-2",
            constraints: Vec::new(),
        };
        let text = transaction_document(
            "t-1",
            "op-1",
            &[
                Operation::DefineNodeType {
                    id: "n-1".into(),
                    name: "Maker".into(),
                    properties: vec![prop("p-1")],
                },
                Operation::DefineEdgeType {
                    id: "e-1".into(),
                    name: "MADE_BY".into(),
                    source_types: vec!["n-0".into()],
                    target_types: vec!["n-1".into()],
                },
                Operation::ModifyProperty {
                    owner: "n-0".into(),
                    property: prop("p-2"),
                },
                Operation::WidenEdgeType {
                    edge_type: "e-1".into(),
                    source_types: vec!["n-0".into(), "n-2".into()],
                    target_types: vec!["n-1".into()],
                },
            ],
            Some("v-1"),
        );
        let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).unwrap();
        let tx = &doc["transaction"];
        assert_eq!(tx["schema_version"].as_str(), Some("v-1"), "{text}");
        assert_eq!(
            tx["evidence"].as_sequence().map(Vec::len),
            Some(0),
            "{text}"
        );
        for part in [
            "- !DefineNodeType",
            "parents: []",
            "abstract_type: false",
            "lifecycle: null",
            "- !DefineEdgeType",
            "cardinality: Many",
            "symmetric: false",
            "- !ModifyProperty",
            "owner: n-0",
            "value_kind: Integer",
            "required: false",
            "required: true",
            "constraints: []",
            "- !WidenEdgeType",
            "edge_type: e-1",
        ] {
            assert!(text.contains(part), "{part} in {text}");
        }
    }

    /// The variables the store's command sets. A launch also removes each `EKR_*` variable the
    /// test runner inherited, which is no variable it sets.
    fn env(store: &Store) -> Vec<(String, String)> {
        let cmd = store.cmd(&[OsStr::new("head")]).unwrap();
        let mut out: Vec<_> = cmd
            .get_envs()
            .filter_map(|(k, v)| {
                Some((
                    k.to_string_lossy().into_owned(),
                    v?.to_string_lossy().into_owned(),
                ))
            })
            .collect();
        out.sort();
        out
    }

    fn store(backend: Backend, store: &str) -> Store {
        Store {
            bin: PathBuf::from("ekr"),
            host: PathBuf::from("/i/host.json"),
            backend,
            store: PathBuf::from(store),
            launch: None,
            stage: None,
            ready: Default::default(),
        }
    }

    fn launched(schema: bool) -> Store {
        let connection = |id: &str| Connection {
            adapter: "pg".into(),
            connection: id.into(),
        };
        Store {
            bin: PathBuf::from("/opt/ekr/bin/ekr"),
            launch: Some(Launch {
                connectors: PathBuf::from("/opt/connectors"),
                connection: connection("app"),
                schema_connection: schema.then(|| connection("owner")),
            }),
            ..store(Backend::Postgres, "/etc/brain/pg.json")
        }
    }

    fn argv(cmd: &std::process::Command) -> (String, Vec<String>) {
        (
            cmd.get_program().to_string_lossy().into_owned(),
            cmd.get_args()
                .map(|a| a.to_string_lossy().into_owned())
                .collect(),
        )
    }

    #[test]
    fn a_launched_store_starts_ekr_through_connectors_connections_launch() {
        let store = launched(false);
        let cmd = store
            .cmd(&[OsStr::new("seed"), OsStr::new("/i/seed \"x\".yaml")])
            .unwrap();
        assert_eq!(
            argv(&cmd),
            (
                "/opt/connectors".to_string(),
                [
                    "connections",
                    "launch",
                    "--adapter",
                    "pg",
                    "--connection",
                    "app",
                    "--consumer",
                    "ekr",
                    "--args",
                    r#"["seed","/i/seed \"x\".yaml"]"#,
                ]
                .map(String::from)
                .to_vec()
            )
        );
        // The operator's consumer entry pins the `ekr`; cortex names no binary of its own.
        assert!(
            !cmd.get_args()
                .any(|a| a == store.bin.as_os_str() || a == "--"),
            "{cmd:?}"
        );
        // `pass_env = ["EKR_"]` hands these to the launched `ekr`.
        assert_eq!(
            env(&store),
            [
                ("EKR_BACKEND".into(), "postgres".into()),
                ("EKR_HOST".into(), "/i/host.json".into()),
                ("EKR_STORE".into(), "/etc/brain/pg.json".into()),
            ]
        );
    }

    #[test]
    fn a_launch_removes_every_inherited_ekr_variable_and_keeps_the_rest() {
        let mut cmd = std::process::Command::new("/opt/connectors");
        remove_inherited_ekr(
            &mut cmd,
            [
                "EKR_FULL_REPLAY",
                "EKR_HOST",
                "EKR_",
                "PATH",
                "CONNECTORS_HOME",
                "ekr_lower",
                "XEKR_",
            ]
            .map(OsString::from),
        );
        cmd.env("EKR_HOST", "/i/host.json");
        let mut envs: Vec<_> = cmd
            .get_envs()
            .map(|(k, v)| {
                (
                    k.to_string_lossy().into_owned(),
                    v.map(|v| v.to_string_lossy().into_owned()),
                )
            })
            .collect();
        envs.sort();
        assert_eq!(
            envs,
            [
                ("EKR_".to_string(), None),
                ("EKR_FULL_REPLAY".to_string(), None),
                ("EKR_HOST".to_string(), Some("/i/host.json".to_string())),
            ]
        );
    }

    /// What the viewer launches through: the application connection, never the schema
    /// connection, which only provisioning uses.
    #[test]
    fn a_store_is_launched_through_its_connection_alone() {
        let app = launched(false);
        assert_eq!(
            app.launched_through(),
            Some(&Connection {
                adapter: "pg".into(),
                connection: "app".into(),
            })
        );
        assert_eq!(launched(true).launched_through(), app.launched_through());
        assert_eq!(
            store(Backend::Postgres, "/etc/brain/pg.json").launched_through(),
            None
        );
    }

    #[test]
    fn a_store_without_a_connection_starts_ekr_itself() {
        let cmd = store(Backend::Postgres, "/etc/brain/pg.json")
            .cmd(&[OsStr::new("head")])
            .unwrap();
        assert_eq!(argv(&cmd), ("ekr".to_string(), vec!["head".to_string()]));
    }

    #[test]
    fn provisioning_launches_through_the_schema_connection_with_no_store_variables() {
        let schema = PathBuf::from("/etc/brain/owner.json");
        let cmd = launched(true).schema_cmd(&schema).unwrap();
        let (program, args) = argv(&cmd);
        assert_eq!(program, "/opt/connectors");
        assert_eq!(
            args[..8],
            [
                "connections",
                "launch",
                "--adapter",
                "pg",
                "--connection",
                "owner",
                "--consumer",
                "ekr"
            ]
        );
        assert_eq!(
            args[8..],
            [
                "--args".to_string(),
                r#"["postgres-schema","--config","/etc/brain/owner.json"]"#.to_string()
            ]
        );
        let removed: Vec<_> = cmd
            .get_envs()
            .map(|(k, v)| (k.to_string_lossy().into_owned(), v.is_none()))
            .collect();
        for key in ["EKR_HOST", "EKR_BACKEND", "EKR_STORE"] {
            assert!(removed.contains(&(key.to_string(), true)), "{removed:?}");
        }
        // Without a schema connection the schema role's own file is used by `ekr` directly.
        let direct = launched(false).schema_cmd(&schema).unwrap();
        assert_eq!(argv(&direct).0, "/opt/ekr/bin/ekr");
    }

    /// Whether `cmd` removes `key` from the environment it inherits.
    fn removes(cmd: &std::process::Command, key: &str) -> bool {
        cmd.get_envs().any(|(k, v)| k == key && v.is_none())
    }

    /// A staged run's `ekr` joins its stage with `EKR_STAGE`, through a launch too, whose
    /// consumer gets every `EKR_*` variable cortex sets; the `ekr stage` verbs, which refuse a
    /// stage, run on the store; and outside a stage an inherited `EKR_STAGE` is removed.
    #[test]
    fn a_staged_store_names_its_stage_to_every_verb_but_the_stage_verbs() {
        for base in [
            launched(false),
            store(Backend::Postgres, "/etc/brain/pg.json"),
        ] {
            let staged = Store {
                stage: Some("st-1".into()),
                ..base.clone()
            };
            assert!(
                env(&staged).contains(&("EKR_STAGE".into(), "st-1".into())),
                "{:?}",
                env(&staged)
            );
            let verb = staged.store_cmd(&[OsStr::new("stage"), OsStr::new("begin")]);
            let verb = verb.unwrap();
            assert!(removes(&verb, "EKR_STAGE"), "{verb:?}");
            assert!(!env(&base).iter().any(|(k, _)| k == "EKR_STAGE"));
            assert!(removes(
                &base.cmd(&[OsStr::new("head")]).unwrap(),
                "EKR_STAGE"
            ));
        }
        let publish = launched(false)
            .stage_cmd(&["publish", "st-1", "--expect-head", "7"])
            .unwrap();
        assert_eq!(
            argv(&publish).1.last().map(String::as_str),
            Some(r#"["stage","publish","st-1","--expect-head","7"]"#)
        );
    }

    #[test]
    fn a_store_verb_names_the_backend_and_its_location() {
        assert_eq!(
            env(&store(Backend::Sqlite, "/i/store.sqlite")),
            [
                ("EKR_BACKEND".into(), "sqlite".into()),
                ("EKR_HOST".into(), "/i/host.json".into()),
                ("EKR_STORE".into(), "/i/store.sqlite".into()),
            ]
        );
        assert_eq!(
            env(&store(Backend::Postgres, "/etc/brain/pg.json")),
            [
                ("EKR_BACKEND".into(), "postgres".into()),
                ("EKR_HOST".into(), "/i/host.json".into()),
                ("EKR_STORE".into(), "/etc/brain/pg.json".into()),
            ]
        );
    }
}
