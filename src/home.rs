//! Where instances live (`$CORTEX_HOME`), and the registry of instances and sources: the storage
//! the generated behaviours read and write.

use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};

use cortex_model::instance as m;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub struct Home {
    pub root: PathBuf,
}

impl Home {
    /// `root` made absolute, so every path derived from it, the ones the systemd units carry
    /// included, names the same file from any directory.
    pub fn new(root: PathBuf) -> Self {
        let root = std::path::absolute(&root).unwrap_or(root);
        Self { root }
    }

    /// `$HOME/.local/share/cortex`.
    pub fn default_root() -> PathBuf {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default();
        home.join(".local/share/cortex")
    }

    pub fn instance_dir(&self, name: &str) -> PathBuf {
        self.root.join("instances").join(name)
    }

    fn registry_path(&self) -> PathBuf {
        self.root.join("registry.json")
    }

    /// Takes the exclusive lock every writing command holds, so two runs of one home never
    /// interleave their writes to the registry or to a store.
    pub fn lock(&self) -> std::io::Result<File> {
        std::fs::create_dir_all(&self.root)?;
        let file = File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.root.join("cortex.lock"))?;
        file.lock()?;
        Ok(file)
    }

    /// The lock [`Home::lock`] takes, without waiting: `None` while another command holds it.
    pub fn try_lock(&self) -> std::io::Result<Option<File>> {
        std::fs::create_dir_all(&self.root)?;
        let file = File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.root.join("cortex.lock"))?;
        match file.try_lock() {
            Ok(()) => Ok(Some(file)),
            Err(std::fs::TryLockError::WouldBlock) => Ok(None),
            Err(std::fs::TryLockError::Error(e)) => Err(e),
        }
    }

    pub fn load_registry(&self) -> Result<Registry, String> {
        let path = self.registry_path();
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Registry::default()),
            Err(e) => return Err(format!("cannot read {}: {e}", path.display())),
        };
        let value: Value = serde_json::from_str(&text)
            .map_err(|e| format!("{} is not JSON: {e}", path.display()))?;
        Registry::from_json(&value).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The seed an instance holds: the digest of its frozen copy. An error names the frozen spec
    /// file that is missing or cannot be parsed.
    pub fn frozen_seed_digest(&self, name: &str) -> Result<String, String> {
        let dir = self.instance_dir(name);
        crate::spec::load(&dir.join("instance.yaml")).map(|frozen| seed_digest(&dir, &frozen.model))
    }

    pub fn save_registry(&self, registry: &Registry) -> Result<(), String> {
        let text = serde_json::to_string_pretty(&registry.to_json()).expect("plain JSON");
        write_atomic(&self.registry_path(), text.as_bytes())
    }
}

/// Writes `bytes` to `path` through a sibling temporary file and a rename.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let dir = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let tmp = dir.join(format!(
        ".{}.{}.tmp",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("file"),
        std::process::id()
    ));
    std::fs::write(&tmp, bytes).map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("cannot replace {}: {e}", path.display()))
}

/// `rel` normalised lexically: `.` components and repeated or trailing separators dropped, so
/// `./docs`, `docs/` and `docs` name one path. A leading `/` is kept and `..` is never folded:
/// [`seed_path_refusal`] refuses both before a digest is compared.
pub fn normalise(rel: &str) -> String {
    let parts: Vec<&str> = rel
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect();
    let joined = parts.join("/");
    if rel.starts_with('/') {
        format!("/{joined}")
    } else {
        joined
    }
}

/// Why a spec's seed or instructions path cannot be frozen, naming the field: the rule
/// `Layout::freeze` applies at create, a path that is absolute or contains `..`.
pub fn seed_path_refusal(spec: &m::InstanceSpec) -> Option<String> {
    let mut paths: Vec<(String, &String)> = Vec::new();
    paths.extend(
        spec.seed
            .schema
            .iter()
            .map(|p| ("seed.schema".to_string(), p)),
    );
    paths.extend(
        spec.seed
            .ekr_seed
            .iter()
            .map(|p| ("seed.ekr_seed".to_string(), p)),
    );
    paths.extend(
        spec.seed
            .documents
            .iter()
            .enumerate()
            .map(|(i, p)| (format!("seed.documents[{i}]"), p)),
    );
    paths.extend(
        spec.model
            .instructions
            .iter()
            .map(|p| ("model.instructions".to_string(), p)),
    );
    paths
        .into_iter()
        .find(|(_, path)| Path::new(path.as_str()).is_absolute() || path.contains(".."))
        .map(|(field, path)| format!("{field} {path:?} leaves the spec's directory"))
}

/// The digest of a spec's seed: its `seed` paths, then, for every seed file under `dir`
/// (`spec::seed_files`, which counts the model instructions file too), the file's path relative
/// to `dir`, its length and its bytes. Paths are compared after [`normalise`]. Two specs have one
/// digest exactly when their normalised `seed` values, the set of seed file paths and each file's
/// bytes are equal, so a renamed file, or bytes moved from one file to another, change it. A
/// symlinked file or directory, the walk's root included, is read as its target.
pub fn seed_digest(dir: &Path, spec: &m::InstanceSpec) -> String {
    fn part(h: &mut Sha256, bytes: &[u8]) {
        h.update((bytes.len() as u64).to_be_bytes());
        h.update(bytes);
    }
    fn optional(h: &mut Sha256, value: Option<&String>) {
        match value {
            Some(v) => {
                h.update([1]);
                part(h, normalise(v).as_bytes());
            }
            None => h.update([0]),
        }
    }
    let mut h = Sha256::new();
    h.update(b"cortex.seed/2");
    optional(&mut h, spec.seed.schema.as_ref());
    optional(&mut h, spec.seed.ekr_seed.as_ref());
    h.update((spec.seed.documents.len() as u64).to_be_bytes());
    for document in &spec.seed.documents {
        part(&mut h, normalise(document).as_bytes());
    }
    for rel in crate::spec::seed_files(spec) {
        let rel = normalise(&rel);
        part(&mut h, rel.as_bytes());
        let root = dir.join(&rel);
        let files: Vec<_> = walkdir::WalkDir::new(&root)
            .follow_links(true)
            .sort_by_file_name()
            .into_iter()
            .flatten()
            .filter(|entry| entry.file_type().is_file())
            .collect();
        h.update((files.len() as u64).to_be_bytes());
        for entry in files {
            let under = entry.path().strip_prefix(dir).unwrap_or(entry.path());
            part(&mut h, normalise(&under.to_string_lossy()).as_bytes());
            part(&mut h, &std::fs::read(entry.path()).unwrap_or_default());
        }
    }
    crate::state::hex(&h.finalize())
}

/// Every instance and source, as the generated storage ports see them.
#[derive(Default)]
pub struct Registry {
    pub instances: BTreeMap<String, m::InstanceSnapshot>,
    pub sources: BTreeMap<String, m::SourceSnapshot>,
}

fn text(v: &Value, key: &str) -> Result<String, String> {
    v.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("missing text {key:?}"))
}

fn integer(v: &Value, key: &str) -> Result<i64, String> {
    v.get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("missing integer {key:?}"))
}

pub fn kind_name(kind: m::SourceKind) -> &'static str {
    match kind {
        m::SourceKind::Web => "Web",
        m::SourceKind::Connectors => "Connectors",
        m::SourceKind::Files => "Files",
        m::SourceKind::Structured => "Structured",
    }
}

pub fn instance_state_name(state: m::InstanceState) -> &'static str {
    match state {
        m::InstanceState::Active => "Active",
        m::InstanceState::Removed => "Removed",
    }
}

pub fn source_state_name(state: m::SourceState) -> &'static str {
    match state {
        m::SourceState::Enabled => "Enabled",
        m::SourceState::Disabled => "Disabled",
    }
}

impl Registry {
    /// `seed_digest` is not a key of `registry.json` and is not read here: an instance's seed is
    /// its frozen copy in the instance directory, which only `update` reads
    /// ([`Home::frozen_seed_digest`]). Until then the field holds the empty text.
    fn from_json(v: &Value) -> Result<Self, String> {
        let mut registry = Registry::default();
        for i in v
            .get("instances")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let snapshot = m::InstanceSnapshot {
                state: match text(i, "state")?.as_str() {
                    "Active" => m::InstanceState::Active,
                    "Removed" => m::InstanceState::Removed,
                    other => return Err(format!("unknown instance state {other:?}")),
                },
                data: m::InstanceData {
                    name: m::InstanceName(text(i, "name")?),
                    description: text(i, "description")?,
                    model: text(i, "model")?,
                    ekr_version: text(i, "ekr_version")?,
                    seed_digest: String::new(),
                },
            };
            registry
                .instances
                .insert(snapshot.data.name.0.clone(), snapshot);
        }
        for s in v
            .get("sources")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let snapshot = m::SourceSnapshot {
                state: match text(s, "state")?.as_str() {
                    "Enabled" => m::SourceState::Enabled,
                    "Disabled" => m::SourceState::Disabled,
                    other => return Err(format!("unknown source state {other:?}")),
                },
                data: m::SourceData {
                    source_id: m::SourceId(text(s, "source_id")?),
                    instance_name: m::InstanceName(text(s, "instance_name")?),
                    name: text(s, "name")?,
                    kind: match text(s, "kind")?.as_str() {
                        "Web" => m::SourceKind::Web,
                        "Connectors" => m::SourceKind::Connectors,
                        "Files" => m::SourceKind::Files,
                        "Structured" => m::SourceKind::Structured,
                        other => return Err(format!("unknown source kind {other:?}")),
                    },
                    schedule: text(s, "schedule")?,
                    runs: integer(s, "runs")?,
                    consecutive_failures: integer(s, "consecutive_failures")?,
                },
            };
            registry
                .sources
                .insert(snapshot.data.source_id.0.clone(), snapshot);
        }
        Ok(registry)
    }

    fn to_json(&self) -> Value {
        json!({
            "format": "cortex.registry/1",
            "instances": self.instances.values().map(|i| json!({
                "name": i.data.name.0,
                "description": i.data.description,
                "model": i.data.model,
                "ekr_version": i.data.ekr_version,
                "state": instance_state_name(i.state),
            })).collect::<Vec<_>>(),
            "sources": self.sources.values().map(|s| json!({
                "source_id": s.data.source_id.0,
                "instance_name": s.data.instance_name.0,
                "name": s.data.name,
                "kind": kind_name(s.data.kind),
                "schedule": s.data.schedule,
                "runs": s.data.runs,
                "consecutive_failures": s.data.consecutive_failures,
                "state": source_state_name(s.state),
            })).collect::<Vec<_>>(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: &str = r#"format: cortex.instance/1
name: t
description: Test brain.
ekr: {version: "0.0.30"}
seed: {documents: [docs]}
model: {model: m, budget_usd: "1", timeout_s: 60, instructions: prompt.md}
sources: []
serve: {}
"#;

    const REGISTRY: &str = r#"{"format": "cortex.registry/1", "instances": [{"name": "t",
        "description": "Test brain.", "model": "m", "ekr_version": "0.0.30", "state": "Active"}],
        "sources": []}"#;

    fn home_with_frozen_instance() -> (tempfile::TempDir, Home) {
        let tmp = tempfile::tempdir().unwrap();
        let home = Home::new(tmp.path().join("home"));
        let dir = home.instance_dir("t");
        std::fs::create_dir_all(dir.join("docs")).unwrap();
        std::fs::write(dir.join("instance.yaml"), SPEC).unwrap();
        std::fs::write(dir.join("docs/a.md"), "A").unwrap();
        std::fs::write(dir.join("prompt.md"), "P").unwrap();
        std::fs::write(home.root.join("registry.json"), REGISTRY).unwrap();
        (tmp, home)
    }

    fn spec_with_documents(path: &str) -> m::InstanceSpec {
        crate::spec::parse(&SPEC.replace("[docs]", &format!("[\"{path}\"]"))).unwrap()
    }

    #[test]
    fn an_instance_s_seed_is_its_frozen_copy() {
        let (_tmp, home) = home_with_frozen_instance();
        let dir = home.instance_dir("t");
        let frozen = crate::spec::load(&dir.join("instance.yaml")).unwrap().model;

        let stored = home.frozen_seed_digest("t").unwrap();
        assert_eq!(stored, seed_digest(&dir, &frozen));

        std::fs::write(dir.join("docs/a.md"), "A, changed").unwrap();
        assert_ne!(home.frozen_seed_digest("t").unwrap(), stored);
        std::fs::write(dir.join("docs/a.md"), "A").unwrap();
        assert_eq!(home.frozen_seed_digest("t").unwrap(), stored);

        std::fs::write(dir.join("prompt.md"), "P, changed").unwrap();
        assert_ne!(
            home.frozen_seed_digest("t").unwrap(),
            stored,
            "the model instructions file counts as seed, as the frozen-copy comparison did"
        );
    }

    #[test]
    fn a_missing_or_unparseable_frozen_spec_is_an_error_naming_the_file() {
        let (_tmp, home) = home_with_frozen_instance();
        let frozen = home.instance_dir("t").join("instance.yaml");
        std::fs::write(&frozen, "format: [not a spec").unwrap();
        let unparseable = home.frozen_seed_digest("t").unwrap_err();
        assert!(
            unparseable.contains(frozen.to_str().unwrap()),
            "{unparseable}"
        );
        std::fs::remove_file(&frozen).unwrap();
        let missing = home.frozen_seed_digest("t").unwrap_err();
        assert!(missing.contains(frozen.to_str().unwrap()), "{missing}");
    }

    #[test]
    fn a_renamed_file_or_bytes_moved_between_files_change_the_seed() {
        let (_tmp, home) = home_with_frozen_instance();
        let dir = home.instance_dir("t");
        let spec = spec_with_documents("docs");
        let digest = || seed_digest(&dir, &spec);

        std::fs::write(dir.join("docs/a.md"), "AB").unwrap();
        std::fs::write(dir.join("docs/b.md"), "").unwrap();
        let before = digest();
        std::fs::write(dir.join("docs/a.md"), "A").unwrap();
        std::fs::write(dir.join("docs/b.md"), "B").unwrap();
        assert_ne!(digest(), before, "bytes moved from a.md to b.md");

        std::fs::remove_file(dir.join("docs/b.md")).unwrap();
        let one = digest();
        std::fs::rename(dir.join("docs/a.md"), dir.join("docs/c.md")).unwrap();
        assert_ne!(digest(), one, "a.md renamed to c.md");
    }

    #[test]
    fn a_seed_path_written_another_way_is_the_same_seed() {
        let (_tmp, home) = home_with_frozen_instance();
        let dir = home.instance_dir("t");
        let plain = seed_digest(&dir, &spec_with_documents("docs"));
        for written in ["./docs", "docs/", "./docs/.", ".//docs//"] {
            assert_eq!(
                seed_digest(&dir, &spec_with_documents(written)),
                plain,
                "{written}"
            );
        }
        assert_ne!(seed_digest(&dir, &spec_with_documents("prompt.md")), plain);
    }

    #[test]
    fn normalising_keeps_a_leading_separator_and_never_folds_a_parent() {
        assert_eq!(normalise("./docs/./a.md"), "docs/a.md");
        assert_eq!(normalise("docs//a.md/"), "docs/a.md");
        assert_eq!(normalise("/prompt.md"), "/prompt.md");
        assert_eq!(normalise("//etc//"), "/etc");
        assert_eq!(normalise("nope/../prompt.md"), "nope/../prompt.md");
        assert_eq!(normalise("./../x"), "../x");
    }

    #[test]
    fn a_path_create_would_refuse_is_named_by_its_field() {
        let refusal = |from: &str, to: &str| {
            seed_path_refusal(&crate::spec::parse(&SPEC.replace(from, to)).unwrap())
        };
        assert_eq!(refusal("prompt.md", "prompt.md"), None);
        for (from, to, field) in [
            (
                "instructions: prompt.md",
                "instructions: /prompt.md",
                "model.instructions",
            ),
            (
                "instructions: prompt.md",
                "instructions: a/../prompt.md",
                "model.instructions",
            ),
            ("[docs]", "[docs, ../x]", "seed.documents[1]"),
            ("seed: {", "seed: {schema: /s.yaml, ", "seed.schema"),
            (
                "seed: {",
                "seed: {ekr_seed: ../seed.yaml, ",
                "seed.ekr_seed",
            ),
        ] {
            let reason = refusal(from, to).unwrap_or_else(|| panic!("{to} is admitted"));
            assert!(reason.contains(field), "{to}: {reason}");
        }
    }

    #[test]
    fn the_digest_reads_a_symlinked_file_or_directory_as_its_target() {
        let (tmp, home) = home_with_frozen_instance();
        let dir = home.instance_dir("t");
        let spec = spec_with_documents("docs");
        let regular = seed_digest(&dir, &spec);

        let elsewhere = tmp.path().join("elsewhere");
        std::fs::create_dir_all(&elsewhere).unwrap();
        std::fs::rename(dir.join("docs/a.md"), elsewhere.join("a.md")).unwrap();
        std::os::unix::fs::symlink(elsewhere.join("a.md"), dir.join("docs/a.md")).unwrap();
        assert_eq!(
            seed_digest(&dir, &spec),
            regular,
            "a symlinked file inside docs"
        );

        std::fs::remove_file(dir.join("docs/a.md")).unwrap();
        std::fs::remove_dir(dir.join("docs")).unwrap();
        std::os::unix::fs::symlink(&elsewhere, dir.join("docs")).unwrap();
        assert_eq!(seed_digest(&dir, &spec), regular, "docs itself a symlink");

        std::fs::rename(dir.join("prompt.md"), tmp.path().join("prompt-real.md")).unwrap();
        std::os::unix::fs::symlink(tmp.path().join("prompt-real.md"), dir.join("prompt.md"))
            .unwrap();
        assert_eq!(
            seed_digest(&dir, &spec),
            regular,
            "a symlinked instructions file"
        );
    }

    #[test]
    fn loading_the_registry_reads_no_seed_and_writes_none_back() {
        let (_tmp, home) = home_with_frozen_instance();
        // A frozen spec that cannot be read does not stop the registry from loading.
        std::fs::remove_file(home.instance_dir("t").join("instance.yaml")).unwrap();
        let registry = home.load_registry().unwrap();
        home.save_registry(&registry).unwrap();
        let written: Value =
            serde_json::from_slice(&std::fs::read(home.root.join("registry.json")).unwrap())
                .unwrap();
        let reread: Value = serde_json::from_str(REGISTRY).unwrap();
        assert_eq!(written, reread);
    }
}
