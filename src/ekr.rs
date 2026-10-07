//! The pinned `ekr` binary: seed, ontology, apply-extraction, and the formats cortex reads from it.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

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

fn run(mut cmd: Command, what: &str) -> Result<Vec<u8>, String> {
    let out = cmd
        .output()
        .map_err(|e| format!("cannot run ekr for {what}: {e}"))?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err(format!("ekr {what} failed: {}", err.trim()))
    }
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
        cmd.args(["schema", "ekr.extraction-document/1"]);
        let mut schema = json(&run(cmd, "schema")?, "schema")?;
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

/// A saved Connectors connection: its adapter alias and id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    pub adapter: String,
    pub connection: String,
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
pub struct Store {
    pub bin: PathBuf,
    pub host: PathBuf,
    pub backend: Backend,
    /// `EKR_STORE`: the SQLite file, or the `ekr.postgres/1` file. cortex passes the path; for a
    /// launched store it reads only the file's `password_file` (`instance::launch_refusal`).
    pub store: PathBuf,
    /// For a PostgreSQL store that names a connection: every `ekr` starts through the launch.
    pub launch: Option<Launch>,
}

impl Launch {
    /// `connectors connections launch --adapter A --connection C --consumer ekr --args '<JSON>'`:
    /// the pinned `ekr` with `args`, as one JSON array of strings, after the consumer's pinned
    /// argument prefix. Built and not run.
    fn command(&self, connection: &Connection, args: &[&OsStr]) -> Result<Command, String> {
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
            &connection.connection,
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
        crate::connectors::Connectors {
            bin: self.connectors.clone(),
        }
        .ensure_ready(&connection.adapter, &connection.connection)
        .map_err(|e| format!("{}:{}: {e}", connection.adapter, connection.connection))
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
    /// variables (`pass_env`).
    fn cmd(&self, args: &[&OsStr]) -> Result<Command, String> {
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
            .env("EKR_STORE", &self.store);
        Ok(cmd)
    }

    /// [`Store::cmd`], after a launched store's connection is ready.
    pub fn command(&self, args: &[&OsStr]) -> Result<Command, String> {
        if let Some(launch) = &self.launch {
            launch.ready(&launch.connection)?;
        }
        self.cmd(args)
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

    /// Whether the store's lineage holds a seed: `ekr head` answers a head, or says "the lineage
    /// has no seed" (EKR 0.0.30, measured on PostgreSQL). Any other answer is an error.
    pub fn seeded(&self) -> Result<bool, String> {
        let out = self
            .command(&[OsStr::new("head")])?
            .output()
            .map_err(|e| format!("cannot run ekr for head: {e}"))?;
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
        let cmd = self.command(&[OsStr::new("seed"), seed.as_os_str()])?;
        let result = json(&run(cmd, "seed")?, "seed")?;
        result["committed_at"]
            .as_i64()
            .ok_or_else(|| "ekr seed answered no committed_at".to_string())
    }

    /// The `ekr.integrate.ExtractionReport` of applying `doc`.
    pub fn apply(&self, doc: &Path) -> Result<Value, String> {
        let cmd = self.command(&[OsStr::new("apply-extraction"), doc.as_os_str()])?;
        json(&run(cmd, "apply-extraction")?, "apply-extraction")
    }

    pub fn ontology(&self) -> Result<Value, String> {
        let cmd = self.command(&[OsStr::new("ontology")])?;
        json(&run(cmd, "ontology")?, "ontology")
    }

    /// The graph at the head (`ekr snapshot`): `graph.graph` holds `nodes`, `edges`, `assertions`
    /// and `evidence`, each a map keyed by id, every assertion with its `lifecycle`, retracted and
    /// superseded ones included (EKR 0.0.31 `docs/cli.md`, `ekr snapshot`).
    pub fn snapshot(&self) -> Result<Value, String> {
        let cmd = self.command(&[OsStr::new("snapshot")])?;
        json(&run(cmd, "snapshot")?, "snapshot")
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
        match self.submit(path, proposer, operations, None)? {
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

    /// A fresh id of `kind` (`ekr mint <kind>`): `transaction`, `type`, `property`,
    /// `schema-version`, ….
    pub fn mint(&self, kind: &str) -> Result<String, String> {
        let mint = self.command(&[OsStr::new("mint"), OsStr::new(kind)])?;
        let minted = json(&run(mint, "mint")?, "mint")?;
        minted["id"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| "ekr mint answered no id".to_string())
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
        let id = self.mint("transaction")?;
        std::fs::write(
            path,
            transaction_document(&id, proposer, operations, schema_version),
        )
        .map_err(|e| format!("{}: {e}", path.display()))?;
        let propose = self.command(&[OsStr::new("propose"), path.as_os_str()])?;
        run(propose, "propose")?;
        let validate = self.command(&[OsStr::new("validate"), OsStr::new(&id)])?;
        let validated = json(&run(validate, "validate")?, "validate")?;
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
        let commit = self.command(&[OsStr::new("commit"), OsStr::new(&id)])?;
        let committed = json(&run(commit, "commit")?, "commit")?;
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
        remove_inherited_ekr, transaction_document, Backend, Connection, Launch, Operation,
        PropertyDecl, Store,
    };

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
