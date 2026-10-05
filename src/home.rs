//! Where instances live (`$CORTEX_HOME`), and the registry of instances and sources: the storage
//! the generated behaviours read and write.

use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};

use cortex_model::instance as m;
use serde_json::{json, Value};

pub struct Home {
    pub root: PathBuf,
}

impl Home {
    pub fn new(root: PathBuf) -> Self {
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
