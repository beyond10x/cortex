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
    /// For a `postgres` store, the lineage the instance grows, as `create` or `adopt` resolved it.
    /// Absent for a `sqlite` store, and in a meta file written before it was recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineage: Option<Lineage>,
}

impl Layout {
    /// The PostgreSQL lineage the instance grows: the one its meta file records, or for an
    /// instance whose meta file predates that, the one its frozen spec and host name.
    pub fn lineage(&self) -> Option<Lineage> {
        if let Some(lineage) = self.load_meta().ok().and_then(|m| m.lineage) {
            return Some(lineage);
        }
        let spec = self.load_spec().ok()?;
        let host = std::fs::read_to_string(self.host()).ok()?;
        Lineage::of(&spec, &host)
    }

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

/// Copies `from` to `to`, following symlinks: a symlinked root, file or subdirectory is copied as
/// a regular file or directory holding its target's contents, the way [`crate::home::seed_digest`]
/// reads it. A symlink below the root that cannot be followed (dangling, or a loop back to an
/// enclosing directory) is left out, as the digest leaves it out.
fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    if from.is_dir() {
        for entry in walkdir::WalkDir::new(from).follow_links(true) {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) if e.depth() > 0 && e.path().is_some_and(is_symlink) => continue,
                Err(e) => return Err(format!("{}: {e}", from.display())),
            };
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

fn is_symlink(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink())
}

/// Why `cortex adopt` takes no instance from a store (`cortex.instance.AdoptInstance`).
#[derive(Debug, PartialEq, Eq)]
pub enum AdoptRefusal {
    /// `--store` is not a store on the backend the spec names (`backend-mismatch`).
    BackendMismatch(String),
    /// `--store` cannot be read, or `ekr` cannot open it with the instance's host
    /// (`store-unreadable`).
    StoreUnreadable(String),
}

/// The first bytes of every SQLite database file.
const SQLITE_HEADER: &[u8; 16] = b"SQLite format 3\0";

/// The backend `store` is adopted from, checked against the spec's `store`. A `sqlite` spec (and a
/// spec with no `store`) takes a SQLite database file, which is copied into the instance; only its
/// first 16 bytes are read here. A `postgres` spec takes the very `ekr.postgres/1` file its
/// `store.value.config` names, which is never read: cortex passes it to `ekr`. A reason names
/// fields, never the spec's `config` value, which may be a connection string written in the wrong
/// place.
pub fn adopt_backend(spec: &m::InstanceSpec, store: &Path) -> Result<ekr::Backend, AdoptRefusal> {
    let unreadable =
        |e: std::io::Error| AdoptRefusal::StoreUnreadable(format!("cannot read --store: {e}"));
    match &spec.store {
        Some(m::StoreSpec::Postgres(p)) => {
            let given = std::fs::canonicalize(store).map_err(unreadable)?;
            match std::fs::canonicalize(ekr::expand(&p.config)) {
                Ok(config) if config == given => Ok(ekr::Backend::Postgres),
                _ => Err(AdoptRefusal::BackendMismatch(
                    "the spec's store is postgres: --store must name the ekr.postgres/1 file \
                     store.value.config names"
                        .into(),
                )),
            }
        }
        Some(m::StoreSpec::Sqlite(_)) | None => {
            let mut header = [0u8; 16];
            let mut file = std::fs::File::open(store).map_err(unreadable)?;
            let read = std::io::Read::read(&mut file, &mut header).map_err(unreadable)?;
            if read == header.len() && &header == SQLITE_HEADER {
                Ok(ekr::Backend::Sqlite)
            } else {
                Err(AdoptRefusal::BackendMismatch(
                    "the spec's store is sqlite: --store is no SQLite database".into(),
                ))
            }
        }
    }
}

/// Why a store was not copied.
#[derive(Debug, PartialEq, Eq)]
pub enum CopyError {
    /// The store cannot be opened or read (`store-unreadable`).
    Source(String),
    /// The store was read, but the copy could not be written: a full disk, a file-size limit.
    Destination(String),
}

/// `path` with `suffix` after its file name: `store.sqlite` and `-wal` give `store.sqlite-wal`.
fn beside(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Copies the SQLite store `from` to the new file `to` through SQLite's online backup, from a
/// read-only connection on `from` resolved to its file, so the copy is one consistent state of
/// every revision, the ones still in a `-wal` included (a store another process holds open keeps
/// its newest revisions there). The database and its `-wal` are not written. SQLite itself creates
/// or updates the `-shm` beside a WAL store any connection opens, and a read-only connection on a
/// store at rest also leaves an empty `-wal` there. A store another process keeps locked fails the
/// copy.
pub fn copy_store(from: &Path, to: &Path) -> Result<(), CopyError> {
    use rusqlite::{Connection, OpenFlags};
    let from = std::fs::canonicalize(from)
        .map_err(|e| CopyError::Source(format!("cannot read --store: {e}")))?;
    let source = Connection::open_with_flags(&from, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .and_then(|c| {
            c.query_row("select count(*) from sqlite_master", [], |r| {
                r.get::<_, i64>(0)
            })?;
            Ok(c)
        })
        .map_err(|e| CopyError::Source(format!("cannot read --store: {e}")))?;
    let destination = |e: rusqlite::Error| {
        CopyError::Destination(format!("cannot copy --store into {}: {e}", to.display()))
    };
    let mut target = Connection::open(to).map_err(destination)?;
    let backup = rusqlite::backup::Backup::new(&source, &mut target).map_err(destination)?;
    match backup.step(-1).map_err(destination)? {
        rusqlite::backup::StepResult::Done => Ok(()),
        _ => Err(CopyError::Source(
            "cannot read --store: another process keeps it locked".into(),
        )),
    }
}

/// The bytes copying the SQLite store `from` needs: the database and its `-wal`, plus
/// [`COPY_MARGIN`] for the copy's own journal and the instance's first runs.
pub fn copy_needs(from: &Path) -> std::io::Result<u64> {
    let from = std::fs::canonicalize(from)?;
    let wal = std::fs::metadata(beside(&from, "-wal")).map_or(0, |m| m.len());
    Ok(std::fs::metadata(&from)?.len() + wal + COPY_MARGIN)
}

/// What a copy needs beyond the store's own bytes: 64 MiB.
pub const COPY_MARGIN: u64 = 64 * 1024 * 1024;

/// The bytes an unprivileged process may still write on the filesystem holding `dir` (statvfs:
/// available blocks times the fragment size).
pub fn free_bytes(dir: &Path) -> std::io::Result<u64> {
    let stat = rustix::fs::statvfs(dir)?;
    Ok(stat.f_bavail.saturating_mul(stat.f_frsize))
}

/// Why a copy of `need` bytes does not fit `free` bytes under `dir`, or nothing when it fits.
pub fn space_refusal(dir: &Path, need: u64, free: u64) -> Option<String> {
    (need > free).then(|| {
        format!(
            "not enough space under {}: the store's copy needs {need} bytes (the store, its -wal \
             and a {COPY_MARGIN}-byte margin), {free} are free",
            dir.display()
        )
    })
}

/// The `identity` of every evidence item the store holds at its head (`ekr snapshot`).
pub fn evidence_identities(
    store: &ekr::Store,
) -> Result<std::collections::BTreeSet<String>, String> {
    let out = std::process::Command::new(&store.bin)
        .arg("snapshot")
        .env("EKR_HOST", &store.host)
        .env("EKR_BACKEND", store.backend.name())
        .env("EKR_STORE", &store.store)
        .output()
        .map_err(|e| format!("cannot run ekr for snapshot: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "ekr snapshot failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let snapshot: serde_json::Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("ekr snapshot printed no JSON: {e}"))?;
    Ok(snapshot["graph"]["graph"]["evidence"]
        .as_object()
        .into_iter()
        .flat_map(|items| items.values())
        .filter_map(|item| item["source"].as_object())
        .flat_map(|source| source.values())
        .filter_map(|kind| kind["identity"].as_str())
        .map(str::to_string)
        .collect())
}

/// Whether the store holds evidence of the seen document `key`: an identity cortex gives a
/// document with that key (its URL, `file:<key>` or `record:<key>`).
pub fn has_evidence(key: &str, identities: &std::collections::BTreeSet<String>) -> bool {
    [
        key.to_string(),
        format!("file:{key}"),
        format!("record:{key}"),
    ]
    .iter()
    .any(|id| identities.contains(id))
}

/// The PostgreSQL lineage an instance grows: its `ekr.postgres/1` file, resolved, and the host's
/// tenant. Two instances on one lineage would each keep their own seen state on one store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lineage {
    pub config: PathBuf,
    pub tenant: String,
}

impl Lineage {
    /// The lineage of a `postgres` spec under `host` (an `ekr.cli-host/1` document); none for a
    /// `sqlite` spec, whose store is the instance's own file. The config is resolved when it
    /// exists, else kept as written with `~/` expanded.
    pub fn of(spec: &m::InstanceSpec, host: &str) -> Option<Self> {
        let host = serde_json::from_str::<serde_json::Value>(host).ok()?;
        Self::for_tenant(spec, host["tenant"].as_str()?)
    }

    /// The lineage of a `postgres` spec under tenant `tenant`; none for a `sqlite` spec.
    pub fn for_tenant(spec: &m::InstanceSpec, tenant: &str) -> Option<Self> {
        let Some(m::StoreSpec::Postgres(p)) = &spec.store else {
            return None;
        };
        let config = ekr::expand(&p.config);
        Some(Self {
            config: std::fs::canonicalize(&config).unwrap_or(config),
            tenant: tenant.to_string(),
        })
    }
}

/// The node and edge type names the spec's seed declares, in file order without repeats: the
/// `ontology` of `seed.ekr_seed` (an `ekr-seed/2` file) and of `seed.schema` (an
/// `ekr.extraction-document/1`), both read under `dir`.
pub fn seed_types(dir: &Path, spec: &m::InstanceSpec) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for rel in spec.seed.ekr_seed.iter().chain(spec.seed.schema.iter()) {
        let path = dir.join(rel);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text)
            .map_err(|e| format!("{}: not YAML: {e}", path.display()))?;
        for kind in ["node_types", "edge_types"] {
            let types = doc
                .get("ontology")
                .and_then(|o| o.get(kind))
                .and_then(serde_yaml_ng::Value::as_sequence);
            for name in types
                .into_iter()
                .flatten()
                .filter_map(|t| t.get("name").and_then(serde_yaml_ng::Value::as_str))
            {
                if !out.iter().any(|n| n == name) {
                    out.push(name.to_string());
                }
            }
        }
    }
    Ok(out)
}

/// The names in `types` that no node or edge type of `ontology` (the `ekr ontology` document)
/// carries, in the order given.
pub fn missing_types(types: &[String], ontology: &serde_json::Value) -> Vec<String> {
    let held: Vec<&str> = ["node_types", "edge_types"]
        .iter()
        .flat_map(|kind| ontology[kind].as_array().into_iter().flatten())
        .filter_map(|t| t["name"].as_str())
        .collect();
    types
        .iter()
        .filter(|t| !held.contains(&t.as_str()))
        .cloned()
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(store: &str) -> m::InstanceSpec {
        crate::spec::parse(&format!(
            "format: cortex.instance/1\nname: t\ndescription: d\nekr: {{version: \"0.0.30\"}}\n\
             seed: {{schema: schema.yaml, ekr_seed: seed.yaml, documents: []}}\n\
             model: {{model: m, budget_usd: \"1\", timeout_s: 60}}\nsources: []\nserve: {{}}\n{store}"
        ))
        .unwrap()
    }

    #[test]
    fn a_sqlite_spec_adopts_a_sqlite_database_and_nothing_else() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("store.sqlite");
        rusqlite::Connection::open(&db)
            .unwrap()
            .execute_batch("create table t (x)")
            .unwrap();
        let config = dir.path().join("pg.json");
        std::fs::write(&config, r#"{"format": "ekr.postgres/1"}"#).unwrap();
        for store in ["", "store: {backend: sqlite}"] {
            let spec = spec(store);
            assert_eq!(adopt_backend(&spec, &db), Ok(ekr::Backend::Sqlite));
            assert!(matches!(
                adopt_backend(&spec, &config),
                Err(AdoptRefusal::BackendMismatch(_))
            ));
            assert!(matches!(
                adopt_backend(&spec, &dir.path().join("none.sqlite")),
                Err(AdoptRefusal::StoreUnreadable(_))
            ));
        }
    }

    #[test]
    fn a_postgres_spec_adopts_only_the_configuration_it_names_and_never_names_its_value() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("pg.json");
        let other = dir.path().join("other.json");
        for f in [&config, &other] {
            std::fs::write(f, "{}").unwrap();
        }
        let spec = spec(&format!(
            "store: {{backend: postgres, value: {{config: \"{}\"}}}}",
            config.display()
        ));
        assert_eq!(adopt_backend(&spec, &config), Ok(ekr::Backend::Postgres));
        match adopt_backend(&spec, &other) {
            Err(AdoptRefusal::BackendMismatch(reason)) => {
                assert!(!reason.contains(config.to_str().unwrap()), "{reason}");
                assert!(reason.contains("store.value.config"), "{reason}");
            }
            other => panic!("{other:?}"),
        }
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut out: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        out.sort();
        out
    }

    fn rows(path: &Path) -> Vec<i64> {
        let db = rusqlite::Connection::open(path).unwrap();
        let mut q = db.prepare("select x from t order by x").unwrap();
        q.query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    /// A WAL store at rest is copied with its database unchanged; beside it only the side files
    /// SQLite's read-only connection writes appear (a `-shm` and an empty `-wal`). One a connection
    /// holds open is copied, through a symlink too, with the rows still in its `-wal`.
    #[test]
    fn a_store_is_copied_whole_and_only_sqlite_side_files_appear_beside_it() {
        let dir = tempfile::tempdir().unwrap();
        let from_dir = dir.path().join("from");
        std::fs::create_dir_all(&from_dir).unwrap();
        let from = from_dir.join("store with space.sqlite");
        {
            let db = rusqlite::Connection::open(&from).unwrap();
            db.pragma_update(None, "journal_mode", "wal").unwrap();
            db.execute_batch("create table t (x); insert into t values (1)")
                .unwrap();
        }
        assert_eq!(names(&from_dir), ["store with space.sqlite"]);
        let bytes = std::fs::read(&from).unwrap();
        let at_rest = dir.path().join("at-rest.sqlite");
        copy_store(&from, &at_rest).unwrap();
        assert_eq!(std::fs::read(&from).unwrap(), bytes);
        for name in names(&from_dir) {
            assert!(
                name == "store with space.sqlite"
                    || name.ends_with("-shm")
                    || (name.ends_with("-wal")
                        && std::fs::metadata(from_dir.join(&name)).unwrap().len() == 0),
                "{name} appeared beside the store"
            );
        }
        assert_eq!(rows(&at_rest), [1]);

        let writer = rusqlite::Connection::open(&from).unwrap();
        writer.pragma_update(None, "wal_autocheckpoint", 0).unwrap();
        writer.execute("insert into t values (2)", []).unwrap();
        assert!(from_dir.join("store with space.sqlite-wal").exists());
        let link = dir.path().join("link.sqlite");
        std::os::unix::fs::symlink(&from, &link).unwrap();
        let live = dir.path().join("live.sqlite");
        copy_store(&link, &live).unwrap();
        assert_eq!(rows(&live), [1, 2]);
    }

    /// A store that cannot be read is the source's failure; a copy that cannot be written is the
    /// destination's.
    #[test]
    fn a_copy_failure_names_the_side_that_failed() {
        let dir = tempfile::tempdir().unwrap();
        let from = dir.path().join("store.sqlite");
        rusqlite::Connection::open(&from)
            .unwrap()
            .execute_batch("create table t (x)")
            .unwrap();
        assert!(matches!(
            copy_store(
                &dir.path().join("none.sqlite"),
                &dir.path().join("a.sqlite")
            ),
            Err(CopyError::Source(_))
        ));
        let nowhere = dir.path().join("no-such-dir/copy.sqlite");
        assert!(matches!(
            copy_store(&from, &nowhere),
            Err(CopyError::Destination(_))
        ));
    }

    #[test]
    fn a_copy_larger_than_the_free_space_is_refused_with_both_numbers() {
        let dir = Path::new("/srv/cortex");
        assert_eq!(space_refusal(dir, 100, 100), None);
        let refusal = space_refusal(dir, 101, 100).unwrap();
        assert!(
            refusal.contains("needs 101 bytes") && refusal.contains("100 are free"),
            "{refusal}"
        );
        let tmp = tempfile::tempdir().unwrap();
        let store = tmp.path().join("s.sqlite");
        std::fs::write(&store, vec![0u8; 1000]).unwrap();
        std::fs::write(tmp.path().join("s.sqlite-wal"), vec![0u8; 24]).unwrap();
        assert_eq!(copy_needs(&store).unwrap(), 1024 + COPY_MARGIN);
        assert!(free_bytes(tmp.path()).unwrap() > 0);
    }

    #[test]
    fn a_seen_key_has_evidence_under_any_identity_cortex_gives_it() {
        let ids: std::collections::BTreeSet<String> =
            ["https://example.org/a", "file:/docs/b.md", "record:t:op:7"]
                .map(String::from)
                .into();
        for key in ["https://example.org/a", "/docs/b.md", "t:op:7"] {
            assert!(has_evidence(key, &ids), "{key}");
        }
        assert!(!has_evidence("/docs/c.md", &ids));
    }

    #[test]
    fn a_postgres_lineage_is_the_resolved_config_and_the_host_tenant() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("pg.json");
        std::fs::write(&config, "{}").unwrap();
        let link = dir.path().join("link.json");
        std::os::unix::fs::symlink(&config, &link).unwrap();
        let pg = |path: &Path| {
            spec(&format!(
                "store: {{backend: postgres, value: {{config: \"{}\"}}}}",
                path.display()
            ))
        };
        let host = r#"{"tenant": "legacy"}"#;
        let lineage = Lineage::of(&pg(&config), host).unwrap();
        assert_eq!(Lineage::of(&pg(&link), host), Some(lineage.clone()));
        assert_eq!(lineage.tenant, "legacy");
        assert_ne!(Lineage::for_tenant(&pg(&config), "other"), Some(lineage));
        assert_eq!(Lineage::of(&spec(""), host), None);
    }

    #[test]
    fn the_seed_types_are_the_ontology_names_of_the_seed_and_the_schema() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("seed.yaml"),
            "format: ekr-seed/2\nontology:\n  node_types:\n  - {id: a, name: Person}\n  \
             edge_types:\n  - {id: b, name: KNOWS}\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("schema.yaml"),
            "format: ekr.extraction-document/1\nontology:\n  node_types:\n  - {name: Person}\n  \
             - {name: Team}\n  edge_types: []\nfacts:\n- !Property {property: p}\n",
        )
        .unwrap();
        let types = seed_types(dir.path(), &spec("")).unwrap();
        assert_eq!(types, ["Person", "KNOWS", "Team"]);

        let ontology = serde_json::json!({
            "node_types": [{"name": "Person"}],
            "edge_types": [{"name": "KNOWS"}],
        });
        assert_eq!(missing_types(&types, &ontology), ["Team"]);

        std::fs::remove_file(dir.path().join("schema.yaml")).unwrap();
        let missing = seed_types(dir.path(), &spec("")).unwrap_err();
        assert!(missing.contains("schema.yaml"), "{missing}");
    }
}
