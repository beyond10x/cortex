//! One instance's directory: the frozen spec and seed files, the EKR host and store, the per-source
//! state, the run records and the log.

use std::io::Write;
use std::path::{Path, PathBuf};

use cortex_model::instance as m;
use serde::{Deserialize, Serialize};

use crate::ekr;
use crate::home::write_atomic;

pub struct Layout {
    pub dir: PathBuf,
}

/// What the instance needs beyond its spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub format: String,
    /// The directory the spec was read from, which relative `files` paths are read against.
    pub spec_dir: PathBuf,
    pub view_port: u16,
}

impl Layout {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }
    pub fn spec(&self) -> PathBuf {
        self.dir.join("instance.yaml")
    }
    pub fn host(&self) -> PathBuf {
        self.dir.join("host.json")
    }
    pub fn store(&self) -> PathBuf {
        self.dir.join("store.sqlite")
    }
    fn meta(&self) -> PathBuf {
        self.dir.join("meta.json")
    }
    pub fn seen(&self, source: &str) -> PathBuf {
        self.dir.join("state").join(format!("{source}.json"))
    }
    pub fn entities(&self) -> PathBuf {
        self.dir.join("state").join("entities.json")
    }
    pub fn runs(&self) -> PathBuf {
        self.dir.join("runs")
    }
    pub fn log(&self) -> PathBuf {
        self.dir.join("cortex.log")
    }

    pub fn load_meta(&self) -> Result<Meta, String> {
        let bytes = std::fs::read(self.meta())
            .map_err(|e| format!("cannot read {}: {e}", self.meta().display()))?;
        serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", self.meta().display()))
    }

    pub fn save_meta(&self, meta: &Meta) -> Result<(), String> {
        write_atomic(
            &self.meta(),
            serde_json::to_string_pretty(meta)
                .expect("plain JSON")
                .as_bytes(),
        )
    }

    /// The frozen spec.
    pub fn load_spec(&self) -> Result<m::InstanceSpec, String> {
        crate::spec::load(&self.spec()).map(|l| l.model)
    }

    /// The store on the backend the spec names, through the pinned `ekr`. A `sqlite` store, and a
    /// spec with no `store`, is `store.sqlite` in the instance directory; a `postgres` store is the
    /// spec's `ekr.postgres/1` file, `~/` expanded.
    pub fn store_handle(&self, spec: &m::InstanceSpec) -> ekr::Store {
        let (backend, store) = match &spec.store {
            Some(m::StoreSpec::Postgres(p)) => (ekr::Backend::Postgres, ekr::expand(&p.config)),
            Some(m::StoreSpec::Sqlite(_)) | None => (ekr::Backend::Sqlite, self.store()),
        };
        ekr::Store {
            bin: ekr::resolve_bin(&spec.ekr.version, spec.ekr.bin.as_deref()),
            host: self.host(),
            backend,
            store,
        }
    }

    /// Appends one JSON line to `cortex.log`.
    pub fn log_line(&self, line: &serde_json::Value) {
        let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.log())
        else {
            return;
        };
        let _ = writeln!(f, "{line}");
    }

    /// Copies the spec text and every seed file it names, keeping their relative paths.
    pub fn freeze(&self, loaded: &crate::spec::Loaded) -> Result<(), String> {
        write_atomic(&self.spec(), loaded.text.as_bytes())?;
        for rel in crate::spec::seed_files(&loaded.model) {
            let from = loaded.dir.join(&rel);
            let to = self.dir.join(&rel);
            if !to.starts_with(&self.dir) || rel.contains("..") {
                return Err(format!("seed path {rel:?} leaves the spec's directory"));
            }
            copy_tree(&from, &to)?;
        }
        Ok(())
    }
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    if from.is_dir() {
        for entry in walkdir::WalkDir::new(from) {
            let entry = entry.map_err(|e| format!("{}: {e}", from.display()))?;
            let rel = entry.path().strip_prefix(from).expect("under the root");
            let dest = to.join(rel);
            if entry.file_type().is_dir() {
                std::fs::create_dir_all(&dest).map_err(|e| format!("{}: {e}", dest.display()))?;
            } else if entry.file_type().is_file() {
                std::fs::copy(entry.path(), &dest)
                    .map_err(|e| format!("{}: {e}", entry.path().display()))?;
            }
        }
        Ok(())
    } else {
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::copy(from, to)
            .map(|_| ())
            .map_err(|e| format!("cannot copy {}: {e}", from.display()))
    }
}

/// The first free loopback port at or above `from`.
pub fn free_port(from: u16) -> Result<u16, String> {
    (from..from.saturating_add(500))
        .find(|p| std::net::TcpListener::bind(("127.0.0.1", *p)).is_ok())
        .ok_or_else(|| format!("no free port in {from}..{}", from.saturating_add(500)))
}

/// Milliseconds since the Unix epoch.
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
