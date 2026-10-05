//! Snapshots of a `sqlite` store and of the instance's `state/`: one taken before a run's first
//! `apply-extraction` and kept once that apply commits, the newest `snapshots.keep` (default
//! [`DEFAULT_KEEP`]) of those cortex named kept, and the restore that puts both back.
//!
//! A snapshot `<name>` is `<instance>/snapshots/<name>.sqlite`, the store, and
//! `<instance>/snapshots-state/<name>.json`, every file of `state/` (each source's seen state and
//! the known entity names). cortex names a snapshot `<unix ms>-<label>`: a run's by its start and
//! source, a restore's own by the time it starts and `before-restore`.

use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use cortex_model::instance as m;
use rusqlite::backup::{Backup, StepResult};
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};

use crate::ekr;
use crate::instance::Layout;
use crate::schedule::Systemd;

/// How many snapshots an instance keeps when its spec names no `snapshots.keep`.
pub const DEFAULT_KEEP: usize = 3;

/// The files SQLite may keep beside a database.
const SIDE: [&str; 3] = ["-wal", "-shm", "-journal"];

/// `<instance>/snapshots`: each snapshot's copy of the store.
pub fn dir(layout: &Layout) -> PathBuf {
    layout.dir.join("snapshots")
}

/// `<instance>/snapshots-state`: each snapshot's copy of `state/`.
pub fn state_dir(layout: &Layout) -> PathBuf {
    layout.dir.join("snapshots-state")
}

fn live_state(layout: &Layout) -> PathBuf {
    layout.dir.join("state")
}

/// The spec's `snapshots.keep`, [`DEFAULT_KEEP`] without one; `0` (or less) takes none.
pub fn keep(spec: &m::InstanceSpec) -> usize {
    spec.snapshots
        .as_ref()
        .map_or(DEFAULT_KEEP, |p| usize::try_from(p.keep).unwrap_or(0))
}

/// Every snapshot `cortex restore` takes, oldest first: each `<name>.sqlite` in `snapshots/`
/// that is not hidden, whoever put it there.
pub fn list(layout: &Layout) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir(layout)) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| !n.starts_with('.'))
        .filter_map(|n| n.strip_suffix(".sqlite").map(str::to_string))
        .collect();
    names.sort_by_key(|n| (cortex_named(n).then(|| taken_at(n)), n.clone()));
    names
}

/// Whether `name` is one cortex gives a snapshot, `<digits>-<label>`; only those are rotated.
fn cortex_named(name: &str) -> bool {
    let Some((ms, label)) = name.split_once('-') else {
        return false;
    };
    !ms.is_empty()
        && ms.bytes().all(|b| b.is_ascii_digit())
        && !label.is_empty()
        && label.chars().all(label_char)
}

fn label_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-')
}

/// The `<unix ms>` a name cortex gave starts with.
fn taken_at(name: &str) -> u64 {
    name.split('-')
        .next()
        .and_then(|ms| ms.parse().ok())
        .unwrap_or(0)
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Removes the oldest snapshots cortex named beyond `keep`, each with its side files and its copy
/// of `state/`. A file of any other name is never removed and counts toward nothing.
fn rotate(layout: &Layout, keep: usize) {
    let named: Vec<String> = list(layout)
        .into_iter()
        .filter(|n| cortex_named(n))
        .collect();
    for old in &named[..named.len().saturating_sub(keep)] {
        let store = dir(layout).join(format!("{old}.sqlite"));
        let _ = std::fs::remove_file(&store);
        for suffix in SIDE {
            let _ = std::fs::remove_file(with_suffix(&store, suffix));
        }
        let _ = std::fs::remove_file(state_dir(layout).join(format!("{old}.json")));
    }
}

/// Removes the hidden temporary copies a killed snapshot left behind, with their side files,
/// unless a live process holds one open.
fn sweep(layout: &Layout) {
    for dir in [dir(layout), state_dir(layout)] {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            let temporary = name.ends_with(".tmp")
                || SIDE
                    .iter()
                    .any(|suffix| name.ends_with(&format!(".tmp{suffix}")));
            if name.starts_with('.') && temporary && holder(&entry.path()).is_none() {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

/// A snapshot copied to hidden temporary names. [`Taken::publish`] gives it its name; dropped
/// unpublished, its temporary files are removed.
struct Taken {
    name: String,
    store_tmp: PathBuf,
    state_tmp: PathBuf,
    store: PathBuf,
    state: PathBuf,
    published: bool,
}

impl Taken {
    /// Copies `store` and `state/` under the first free name `<ms>-<label>` (`ms` counted up when
    /// it is taken), after sweeping what a killed snapshot left.
    fn copy(layout: &Layout, store: &Path, ms: i64, label: &str) -> Result<Self, String> {
        let (dir, state_dir) = (dir(layout), state_dir(layout));
        for d in [&dir, &state_dir] {
            std::fs::create_dir_all(d)
                .map_err(|e| format!("cannot create {}: {e}", d.display()))?;
        }
        sweep(layout);
        let label: String = label
            .chars()
            .map(|c| if label_char(c) { c } else { '_' })
            .collect();
        let mut ms = ms.max(0);
        let name = loop {
            let name = format!("{ms}-{label}");
            let taken = [
                dir.join(format!("{name}.sqlite")),
                dir.join(format!(".{name}.sqlite.tmp")),
            ];
            if !taken.iter().any(|p| p.exists()) {
                break name;
            }
            ms += 1;
        };
        let taken = Self {
            store_tmp: dir.join(format!(".{name}.sqlite.tmp")),
            state_tmp: state_dir.join(format!(".{name}.json.tmp")),
            store: dir.join(format!("{name}.sqlite")),
            state: state_dir.join(format!("{name}.json")),
            name,
            published: false,
        };
        backup_to_new(store, &taken.store_tmp)
            .map_err(|e| format!("cannot snapshot {}: {e}", store.display()))?;
        let state = read_state(&live_state(layout))?;
        std::fs::write(&taken.state_tmp, state)
            .map_err(|e| format!("cannot write {}: {e}", taken.state_tmp.display()))?;
        Ok(taken)
    }

    /// Moves the copy to its name, the state first, so a listed snapshot always has its state.
    fn publish(mut self) -> Result<String, String> {
        std::fs::rename(&self.state_tmp, &self.state)
            .map_err(|e| format!("cannot place {}: {e}", self.state.display()))?;
        if let Err(e) = std::fs::rename(&self.store_tmp, &self.store) {
            let _ = std::fs::remove_file(&self.state);
            return Err(format!("cannot place {}: {e}", self.store.display()));
        }
        self.published = true;
        Ok(std::mem::take(&mut self.name))
    }
}

impl Drop for Taken {
    fn drop(&mut self) {
        if self.published {
            return;
        }
        let _ = std::fs::remove_file(&self.store_tmp);
        for suffix in SIDE {
            let _ = std::fs::remove_file(with_suffix(&self.store_tmp, suffix));
        }
        let _ = std::fs::remove_file(&self.state_tmp);
    }
}

/// Every regular file of `state/` that is not hidden, by name, as one JSON document. A missing
/// `state/` is an empty one.
fn read_state(state: &Path) -> Result<String, String> {
    let mut files = serde_json::Map::new();
    if let Ok(entries) = std::fs::read_dir(state) {
        for entry in entries.flatten() {
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            if name.starts_with('.') || !entry.file_type().is_ok_and(|t| t.is_file()) {
                continue;
            }
            let text = std::fs::read_to_string(entry.path())
                .map_err(|e| format!("cannot read {}: {e}", entry.path().display()))?;
            files.insert(name, Value::String(text));
        }
    }
    Ok(
        serde_json::to_string_pretty(&json!({"format": "cortex.state-snapshot/1", "files": files}))
            .expect("plain JSON"),
    )
}

/// Puts `state/` back to the copy at `from`: each file it holds is written, and every other file
/// of `state/` removed.
fn write_state(layout: &Layout, from: &Path) -> Result<(), String> {
    let text = std::fs::read_to_string(from)
        .map_err(|e| format!("cannot read {}: {e}", from.display()))?;
    let doc: Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", from.display()))?;
    let files = doc["files"]
        .as_object()
        .ok_or_else(|| format!("{} holds no files", from.display()))?;
    let state = live_state(layout);
    for (name, text) in files {
        let text = text
            .as_str()
            .ok_or_else(|| format!("{}: {name} is not text", from.display()))?;
        if name.contains('/') || name.starts_with('.') {
            return Err(format!("{}: {name:?} is not a state file", from.display()));
        }
        crate::home::write_atomic(&state.join(name), text.as_bytes())?;
    }
    if let Ok(entries) = std::fs::read_dir(&state) {
        for entry in entries.flatten() {
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            let regular = entry.file_type().is_ok_and(|t| t.is_file());
            if regular && !name.starts_with('.') && !files.contains_key(&name) {
                std::fs::remove_file(entry.path())
                    .map_err(|e| format!("cannot remove {}: {e}", entry.path().display()))?;
            }
        }
    }
    Ok(())
}

/// The one snapshot a run takes: copied before its first `apply-extraction`
/// ([`BeforeApply::take`]) and kept only once that apply commits ([`BeforeApply::publish`]).
/// Dropped before then, the copy is removed, so a run that applies nothing leaves none.
pub struct BeforeApply {
    instance: PathBuf,
    store: PathBuf,
    started: i64,
    label: String,
    keep: usize,
    stage: Stage,
}

enum Stage {
    Off,
    Due,
    Taken(Taken),
    Done,
}

impl BeforeApply {
    /// A run of `label` started at `started` over `store`; a `postgres` store, whose snapshot is
    /// the operator's database backup, and a `keep` of `0` take none.
    pub fn new(
        layout: &Layout,
        store: &ekr::Store,
        label: &str,
        started: i64,
        keep: usize,
    ) -> Self {
        let due = store.backend == ekr::Backend::Sqlite && keep > 0;
        Self {
            instance: layout.dir.clone(),
            store: store.store.clone(),
            started,
            label: label.to_string(),
            keep,
            stage: if due { Stage::Due } else { Stage::Off },
        }
    }

    /// Takes no snapshot.
    pub fn none() -> Self {
        Self {
            instance: PathBuf::new(),
            store: PathBuf::new(),
            started: 0,
            label: String::new(),
            keep: 0,
            stage: Stage::Off,
        }
    }

    /// Copies the store and `state/` to hidden temporary names, the first time it is asked.
    pub fn take(&mut self) -> Result<(), String> {
        if matches!(self.stage, Stage::Due) {
            let layout = Layout::new(self.instance.clone());
            let taken = Taken::copy(&layout, &self.store, self.started, &self.label)?;
            self.stage = Stage::Taken(taken);
        }
        Ok(())
    }

    /// After the first apply committed: gives the copy its name and removes the oldest beyond
    /// `keep`. Answers the name the first time, `None` every later time.
    pub fn publish(&mut self) -> Result<Option<String>, String> {
        match std::mem::replace(&mut self.stage, Stage::Done) {
            Stage::Taken(taken) => {
                let name = taken.publish()?;
                rotate(&Layout::new(self.instance.clone()), self.keep);
                Ok(Some(name))
            }
            Stage::Off => {
                self.stage = Stage::Off;
                Ok(None)
            }
            Stage::Due | Stage::Done => Ok(None),
        }
    }
}

/// `store` copied to the new file `to`; both connections are closed when it answers.
fn backup_to_new(store: &Path, to: &Path) -> Result<(), String> {
    let from = open(store, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut to = open(
        to,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
    )?;
    copy(&from, &mut to)
}

fn open(path: &Path, flags: OpenFlags) -> Result<Connection, String> {
    Connection::open_with_flags(path, flags).map_err(|e| format!("{}: {e}", path.display()))
}

/// A snapshot opened read-only and `immutable=1`, so SQLite writes no `-wal` or `-shm` beside it.
fn open_snapshot(path: &Path) -> Result<Connection, String> {
    let path = std::path::absolute(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut uri = String::from("file:");
    for &b in path.as_os_str().as_bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/".contains(&b) {
            uri.push(char::from(b));
        } else {
            uri.push_str(&format!("%{b:02X}"));
        }
    }
    uri.push_str("?immutable=1");
    Connection::open_with_flags(
        &uri,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|e| format!("{}: {e}", path.display()))
}

/// Every page of `from` into `to` in one step of SQLite's online backup, so the copy is one
/// consistent state; a store that stays locked for 5 s fails it.
fn copy(from: &Connection, to: &mut Connection) -> Result<(), String> {
    let backup = Backup::new(from, to).map_err(|e| e.to_string())?;
    for _ in 0..50 {
        match backup.step(-1).map_err(|e| e.to_string())? {
            StepResult::Done => return Ok(()),
            StepResult::More => {}
            _ => std::thread::sleep(Duration::from_millis(100)),
        }
    }
    Err("the store stayed locked for 5 s".into())
}

/// A process other than this one holding `path`, or its `-wal`, `-shm` or `-journal` file,
/// open, read from `/proc/<pid>/fd`. A process of another user, or one `/proc` hides, is not
/// seen, and a process may open the file after the check.
fn holder(path: &Path) -> Option<u32> {
    let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let files: Vec<PathBuf> = std::iter::once(path.clone())
        .chain(SIDE.iter().map(|suffix| with_suffix(&path, suffix)))
        .collect();
    let me = std::process::id();
    let processes = std::fs::read_dir("/proc").ok()?;
    for process in processes.flatten() {
        let Some(pid) = process
            .file_name()
            .to_str()
            .and_then(|p| p.parse::<u32>().ok())
        else {
            continue;
        };
        if pid == me {
            continue;
        }
        let Ok(fds) = std::fs::read_dir(process.path().join("fd")) else {
            continue;
        };
        for fd in fds.flatten() {
            if std::fs::read_link(fd.path()).is_ok_and(|target| files.contains(&target)) {
                return Some(pid);
            }
        }
    }
    None
}

/// Why a restore did not happen, each a declared outcome of `cortex.instance.RestoreSnapshot`.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The store is on the `postgres` backend.
    Unsupported,
    /// Another cortex command holds the home's lock, or another process holds the store open.
    Busy(String),
    NoSuchSnapshot,
}

#[derive(Debug)]
pub enum RestoreError {
    Refused(Refusal),
    /// The restore could not be attempted or failed; the message says which.
    Failed(String),
}

/// A finished restore.
#[derive(Debug)]
pub struct Restored {
    /// The snapshot of the store and state the restore replaced, `<ms>-before-restore`; `None`
    /// when `snapshots.keep` is `0`.
    pub before_restore: Option<String>,
    /// Whether `state/` was put back: a snapshot with no copy of it leaves `state/` as it is.
    pub state_restored: bool,
    /// Why the viewer did not start again, when it did not.
    pub viewer_failed: Option<String>,
}

/// Puts the store and `state/` of the instance `name` back to `snapshot`, for a caller that
/// already holds the home's lock (`cortex restore` takes it; a run holds it). A running viewer is
/// stopped first and started again after, whatever the restore answered. What the restore
/// replaces is snapshotted first as `<ms>-before-restore`, which counts toward `keep`.
pub fn restore_held(
    layout: &Layout,
    name: &str,
    snapshot: &str,
    systemd: &Systemd,
) -> Result<Restored, RestoreError> {
    let spec = layout.load_spec().map_err(RestoreError::Failed)?;
    let store = layout.store_handle(&spec);
    if store.backend != ekr::Backend::Sqlite {
        return Err(RestoreError::Refused(Refusal::Unsupported));
    }
    // Only a name `list` answers is a snapshot: no path, no hidden temporary file.
    if !list(layout).iter().any(|held| held == snapshot) {
        return Err(RestoreError::Refused(Refusal::NoSuchSnapshot));
    }
    let keep = keep(&spec);
    let from = dir(layout).join(format!("{snapshot}.sqlite"));
    let state_from = state_dir(layout).join(format!("{snapshot}.json"));
    let (restored, viewer_failed) = systemd
        .restart_view(name, || {
            if let Some(pid) = holder(&store.store) {
                return Err(RestoreError::Refused(Refusal::Busy(format!(
                    "process {pid} holds {} open",
                    store.store.display()
                ))));
            }
            let before_restore = match keep {
                0 => None,
                _ => Some(
                    Taken::copy(
                        layout,
                        &store.store,
                        crate::instance::now_ms(),
                        "before-restore",
                    )
                    .and_then(Taken::publish)
                    .map_err(RestoreError::Failed)?,
                ),
            };
            let source = open_snapshot(&from).map_err(RestoreError::Failed)?;
            let mut target = open(&store.store, OpenFlags::SQLITE_OPEN_READ_WRITE)
                .map_err(RestoreError::Failed)?;
            copy(&source, &mut target).map_err(|e| {
                RestoreError::Failed(format!("cannot restore {}: {e}", store.store.display()))
            })?;
            drop((source, target));
            let state_restored = state_from.is_file();
            if state_restored {
                write_state(layout, &state_from).map_err(RestoreError::Failed)?;
            }
            rotate(layout, keep);
            Ok((before_restore, state_restored))
        })
        .map_err(RestoreError::Failed)?;
    let (before_restore, state_restored) = restored?;
    Ok(Restored {
        before_restore,
        state_restored,
        viewer_failed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_with(dir: &Path, rows: &[&str]) -> PathBuf {
        let path = dir.join("store.sqlite");
        let db = Connection::open(&path).unwrap();
        db.execute_batch("CREATE TABLE IF NOT EXISTS t (v TEXT); DELETE FROM t;")
            .unwrap();
        for row in rows {
            db.execute("INSERT INTO t (v) VALUES (?1)", [row]).unwrap();
        }
        path
    }

    fn rows(path: &Path) -> Vec<String> {
        let db = open_snapshot(path).unwrap();
        let mut stmt = db.prepare("SELECT v FROM t ORDER BY v").unwrap();
        stmt.query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    fn sqlite(layout: &Layout) -> ekr::Store {
        ekr::Store {
            bin: PathBuf::from("ekr-unused"),
            host: layout.host(),
            backend: ekr::Backend::Sqlite,
            store: layout.store(),
        }
    }

    fn seen(layout: &Layout, text: &str) {
        std::fs::create_dir_all(live_state(layout)).unwrap();
        std::fs::write(live_state(layout).join("news.json"), text).unwrap();
    }

    fn entries(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .map(|e| {
                e.flatten()
                    .filter_map(|e| e.file_name().into_string().ok())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }

    /// A run over `layout` started at `started`, whose first apply commits.
    fn run(layout: &Layout, started: i64, keep: usize) -> String {
        let mut before = BeforeApply::new(layout, &sqlite(layout), "news", started, keep);
        before.take().unwrap();
        before.take().unwrap();
        let name = before.publish().unwrap().expect("a snapshot");
        assert_eq!(before.publish().unwrap(), None, "one snapshot per run");
        name
    }

    #[test]
    fn keep_2_leaves_the_newest_2_after_3_snapshots() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path().to_path_buf());
        let mut taken = Vec::new();
        for (started, state) in [(1000, "a"), (2000, "b"), (3000, "c")] {
            store_with(&layout.dir, &[state]);
            seen(&layout, state);
            taken.push(run(&layout, started, 2));
        }
        assert_eq!(taken, ["1000-news", "2000-news", "3000-news"]);
        assert_eq!(list(&layout), taken[1..], "the oldest went");
        assert_eq!(
            entries(&dir(&layout)),
            ["2000-news.sqlite", "3000-news.sqlite"]
        );
        assert_eq!(
            entries(&state_dir(&layout)),
            ["2000-news.json", "3000-news.json"],
            "its copy of state/ went with it"
        );
        let newest = dir(&layout).join("3000-news.sqlite");
        assert_eq!(rows(&newest), ["c"], "a snapshot holds the store as it was");
    }

    #[test]
    fn a_postgres_store_or_keep_0_takes_no_snapshot() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path().to_path_buf());
        store_with(&layout.dir, &["a"]);
        let mut postgres = sqlite(&layout);
        postgres.backend = ekr::Backend::Postgres;
        for (store, keep) in [(postgres, 3), (sqlite(&layout), 0)] {
            let mut before = BeforeApply::new(&layout, &store, "news", 1, keep);
            before.take().unwrap();
            assert_eq!(before.publish().unwrap(), None);
        }
        assert!(list(&layout).is_empty());
    }

    #[test]
    fn a_copy_never_published_leaves_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path().to_path_buf());
        store_with(&layout.dir, &["a"]);
        seen(&layout, "a");
        let mut before = BeforeApply::new(&layout, &sqlite(&layout), "news", 1, 3);
        before.take().unwrap();
        assert!(!entries(&dir(&layout)).is_empty(), "the copy exists");
        drop(before);
        assert!(entries(&dir(&layout)).is_empty());
        assert!(entries(&state_dir(&layout)).is_empty());
    }

    #[test]
    fn rotation_counts_and_removes_only_names_cortex_gave() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path().to_path_buf());
        store_with(&layout.dir, &["a"]);
        std::fs::create_dir_all(dir(&layout)).unwrap();
        for other in [
            "pinned.sqlite",
            "1-.sqlite",
            "x-1.sqlite",
            "12a-news.sqlite",
        ] {
            std::fs::write(dir(&layout).join(other), "").unwrap();
        }
        run(&layout, 1000, 1);
        run(&layout, 2000, 1);
        assert_eq!(
            entries(&dir(&layout)),
            [
                "1-.sqlite",
                "12a-news.sqlite",
                "2000-news.sqlite",
                "pinned.sqlite",
                "x-1.sqlite"
            ]
        );
    }

    #[test]
    fn a_snapshot_names_the_runs_start_and_counts_up_on_a_clash() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path().to_path_buf());
        store_with(&layout.dir, &["a"]);
        assert_eq!(run(&layout, 1000, 3), "1000-news");
        assert_eq!(run(&layout, 1000, 3), "1001-news");
    }

    #[test]
    fn a_stale_copy_no_process_holds_is_swept_by_the_next_snapshot() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path().to_path_buf());
        store_with(&layout.dir, &["a"]);
        std::fs::create_dir_all(dir(&layout)).unwrap();
        std::fs::create_dir_all(state_dir(&layout)).unwrap();
        for stale in [".5-news.sqlite.tmp", ".5-news.sqlite.tmp-journal"] {
            std::fs::write(dir(&layout).join(stale), "").unwrap();
        }
        std::fs::write(state_dir(&layout).join(".5-news.json.tmp"), "").unwrap();
        run(&layout, 1000, 3);
        assert_eq!(entries(&dir(&layout)), ["1000-news.sqlite"]);
        assert_eq!(entries(&state_dir(&layout)), ["1000-news.json"]);
    }

    #[test]
    fn a_restore_puts_the_store_and_state_back_after_snapshotting_them() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path().to_path_buf());
        std::fs::write(
            layout.spec(),
            "format: cortex.instance/1\nname: t\ndescription: d\nekr: {version: \"0.0.30\"}\n\
             seed: {documents: []}\nmodel: {model: m, budget_usd: \"1\", timeout_s: 60}\n\
             sources: []\nserve: {}\n",
        )
        .unwrap();
        let store = store_with(&layout.dir, &["a"]);
        seen(&layout, "seen a");
        let name = run(&layout, 1000, 3);
        store_with(&layout.dir, &["b", "c"]);
        seen(&layout, "seen b");
        std::fs::write(live_state(&layout).join("other.json"), "later").unwrap();
        let systemd = Systemd {
            systemctl: PathBuf::from("systemctl-unused"),
            unit_dir: tmp.path().join("no-units"),
            home_root: tmp.path().to_path_buf(),
        };
        let restored = restore_held(&layout, "t", &name, &systemd).unwrap();
        assert!(restored.viewer_failed.is_none());
        assert!(restored.state_restored);
        assert_eq!(rows(&store), ["a"]);
        assert_eq!(entries(&live_state(&layout)), ["news.json"]);
        assert_eq!(
            std::fs::read_to_string(live_state(&layout).join("news.json")).unwrap(),
            "seen a"
        );
        let before = restored.before_restore.expect("the replaced state is kept");
        assert!(before.ends_with("-before-restore"), "{before}");
        assert_eq!(list(&layout), [name.clone(), before.clone()]);
        assert_eq!(
            entries(&dir(&layout)),
            [format!("{name}.sqlite"), format!("{before}.sqlite")],
            "restoring leaves no side files"
        );

        // The restore is undone from its own snapshot.
        restore_held(&layout, "t", &before, &systemd).unwrap();
        assert_eq!(rows(&store), ["b", "c"]);
        assert_eq!(entries(&live_state(&layout)), ["news.json", "other.json"]);

        for bad in ["", "..", "../store", "x/y", "store.sqlite"] {
            assert!(
                matches!(
                    restore_held(&layout, "t", bad, &systemd),
                    Err(RestoreError::Refused(Refusal::NoSuchSnapshot))
                ),
                "{bad:?}"
            );
        }
    }
}
