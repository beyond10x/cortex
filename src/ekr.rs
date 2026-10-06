//! The pinned `ekr` binary: seed, ontology, apply-extraction, and the formats cortex reads from it.

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

/// One instance's store, through its host.
pub struct Store {
    pub bin: PathBuf,
    pub host: PathBuf,
    pub backend: Backend,
    /// `EKR_STORE`: the SQLite file, or the `ekr.postgres/1` file. cortex passes the path and never
    /// reads the file, which references the database credential.
    pub store: PathBuf,
}

impl Store {
    fn cmd(&self) -> Command {
        let mut cmd = Command::new(&self.bin);
        cmd.env("EKR_HOST", &self.host)
            .env("EKR_BACKEND", self.backend.name())
            .env("EKR_STORE", &self.store);
        cmd
    }

    /// `ekr postgres-schema --config <schema_config>`: the provider's tables, created under the
    /// schema-management role that `schema_config` names. The store's own configuration names the
    /// DML-only application role, which EKR 0.0.30 refuses schema DDL; only creating an instance
    /// provisions, never a run.
    pub fn provision(&self, schema_config: &Path) -> Result<(), String> {
        let mut cmd = Command::new(&self.bin);
        cmd.args(["postgres-schema", "--config"])
            .arg(schema_config)
            .env_remove("EKR_HOST")
            .env_remove("EKR_BACKEND")
            .env_remove("EKR_STORE");
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
        let mut cmd = self.cmd();
        cmd.arg("head");
        let out = cmd
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
        let mut cmd = self.cmd();
        cmd.arg("seed").arg(seed);
        let result = json(&run(cmd, "seed")?, "seed")?;
        result["committed_at"]
            .as_i64()
            .ok_or_else(|| "ekr seed answered no committed_at".to_string())
    }

    /// The `ekr.integrate.ExtractionReport` of applying `doc`.
    pub fn apply(&self, doc: &Path) -> Result<Value, String> {
        let mut cmd = self.cmd();
        cmd.arg("apply-extraction").arg(doc);
        json(&run(cmd, "apply-extraction")?, "apply-extraction")
    }

    pub fn ontology(&self) -> Result<Value, String> {
        let mut cmd = self.cmd();
        cmd.arg("ontology");
        json(&run(cmd, "ontology")?, "ontology")
    }

    /// The graph at the head (`ekr snapshot`): `graph.graph` holds `nodes`, `edges`, `assertions`
    /// and `evidence`, each a map keyed by id, every assertion with its `lifecycle`, retracted and
    /// superseded ones included (EKR 0.0.31 `docs/cli.md`, `ekr snapshot`).
    pub fn snapshot(&self) -> Result<Value, String> {
        let mut cmd = self.cmd();
        cmd.arg("snapshot");
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
        let mut mint = self.cmd();
        mint.args(["mint", "transaction"]);
        let minted = json(&run(mint, "mint")?, "mint")?;
        let id = minted["id"]
            .as_str()
            .ok_or("ekr mint answered no id")?
            .to_string();
        std::fs::write(path, transaction_document(&id, proposer, operations))
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let mut propose = self.cmd();
        propose.arg("propose").arg(path);
        run(propose, "propose")?;
        let mut validate = self.cmd();
        validate.args(["validate", &id]);
        let validated = json(&run(validate, "validate")?, "validate")?;
        if validated["kind"] != "Validated" {
            let issues: Vec<String> = validated["issues"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|i| {
                    format!(
                        "{}: {}",
                        i["code"].as_str().unwrap_or("?"),
                        i["message"].as_str().unwrap_or_default()
                    )
                })
                .collect();
            return Err(format!(
                "ekr validate rejected transaction {id}: {}",
                issues.join("; ")
            ));
        }
        let mut commit = self.cmd();
        commit.args(["commit", &id]);
        let committed = json(&run(commit, "commit")?, "commit")?;
        match committed["kind"].as_str() {
            Some("Committed") => committed["result"]["revision"]
                .as_i64()
                .ok_or_else(|| "ekr commit answered no revision".to_string()),
            other => Err(format!(
                "ekr commit of transaction {id} answered {}",
                other.unwrap_or("nothing")
            )),
        }
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
        };
        Yaml::Tagged(Box::new(TaggedValue {
            tag: Tag::new(tag),
            value,
        }))
    }
}

/// The `ekr.transaction-document/2` text of transaction `id`: `operations` in order, proposed by
/// `proposer`, citing no evidence (none of them adds an assertion). The `format:` line comes
/// first, on its own, as EKR reads the byte cap from it.
fn transaction_document(id: &str, proposer: &str, operations: &[Operation]) -> String {
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
    use std::ffi::OsStr;
    use std::path::PathBuf;

    use super::{transaction_document, Backend, Operation, Store};

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
        );
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

    fn env(store: &Store) -> Vec<(String, String)> {
        let cmd = store.cmd();
        let mut out: Vec<_> = cmd
            .get_envs()
            .map(|(k, v)| {
                (
                    k.to_string_lossy().into_owned(),
                    v.map(OsStr::to_string_lossy)
                        .unwrap_or_default()
                        .into_owned(),
                )
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
        }
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
