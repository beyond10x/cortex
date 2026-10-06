//! A run snapshots a `sqlite` store and the instance's state before it applies anything, and
//! `cortex restore` puts both back to one of those snapshots, end to end through the real binary
//! and a real `ekr` (`tests/common`). A missing `ekr` fails the test; it never skips.
//!
//! The cases checked against `website/docs/operating.md` ("Undoing a run") and
//! `website/docs/spec-file.md` (`snapshots`) include the six an adversarial review of
//! `story:run-snapshots` found red.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{answer, executable, World, PAGES};

const VIEW: &str = "cortex-t-view.service";

fn create_from(w: &World, spec: &Path) -> PathBuf {
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap()]);
    assert_eq!(
        (code, created["outcome"].as_str()),
        (0, Some("created")),
        "{created}"
    );
    w.home.join("instances/t")
}

fn create(w: &World) -> PathBuf {
    create_from(w, &w.spec("t", "conn_test"))
}

/// Every file name under `<instance>/snapshots/`, hidden ones included, sorted.
fn files(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir.join("snapshots"))
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|e| e.file_name().into_string().ok())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// The snapshots' names as `cortex restore` takes them, sorted.
fn snapshots(dir: &Path) -> Vec<String> {
    files(dir)
        .into_iter()
        .filter(|n| !n.starts_with('.'))
        .filter_map(|n| n.strip_suffix(".sqlite").map(str::to_string))
        .collect()
}

/// Exactly `N` snapshots, oldest first.
fn names<const N: usize>(dir: &Path) -> [String; N] {
    snapshots(dir)
        .try_into()
        .unwrap_or_else(|held: Vec<String>| panic!("{N} snapshot(s) expected, found {held:?}"))
}

/// Upstream answers page B in its `n`th wording, so the next run has one document to apply.
fn upstream(w: &World, n: u32) {
    std::fs::write(
        w.root.join("answer.json"),
        answer(&PAGES.replace(
            "also develops the Gadget runtime",
            &format!("now develops the Gadget runtime, release {n}"),
        )),
    )
    .unwrap();
}

fn run_applying(w: &World, n: u32) {
    upstream(w, n);
    let (code, ran) = w.cortex(&["run", "t/news"]);
    assert_eq!(
        (code, ran["detail"]["documents_applied"].as_i64()),
        (0, Some(if n == 0 { 2 } else { 1 })),
        "{ran}"
    );
}

/// `cortex run t/news` under `prlimit --fsize=<bytes>`; `ignore_xfsz` makes an oversized write fail
/// with EFBIG, as on a full disk, instead of killing the process.
fn run_limited(w: &World, bytes: u64, ignore_xfsz: bool) -> std::process::Output {
    let trap = if ignore_xfsz { "trap '' XFSZ; " } else { "" };
    Command::new("sh")
        .arg("-c")
        .arg(format!(
            "{trap}exec prlimit --fsize={bytes} -- \"$0\" run t/news"
        ))
        .arg(env!("CARGO_BIN_EXE_cortex"))
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .output()
        .unwrap()
}

/// The `systemctl` lines written since `from`.
fn systemctl_since(w: &World, from: usize) -> Vec<String> {
    w.lines("systemctl.log")[from..].to_vec()
}

fn view_restarted() -> [String; 3] {
    [
        format!("--user is-active --quiet {VIEW}"),
        format!("--user stop {VIEW}"),
        format!("--user start {VIEW}"),
    ]
}

#[test]
fn restoring_the_snapshot_taken_before_a_run_undoes_that_run() {
    let w = World::new();
    let dir = create(&w);
    let seeded = w.head("t");

    upstream(&w, 0);
    let (code, first) = w.cortex(&["run", "t/news"]);
    assert_eq!(
        (code, first["outcome"].as_str()),
        (0, Some("ran")),
        "{first}"
    );
    let after_first = w.head("t");
    assert!(after_first > seeded, "the first run committed: {first}");
    let [taken_first] = names(&dir);

    upstream(&w, 1);
    let (code, second) = w.cortex(&["run", "t/news"]);
    assert_eq!(
        (code, second["detail"]["documents_applied"].as_i64()),
        (0, Some(1)),
        "{second}"
    );
    let after_second = w.head("t");
    assert!(after_second > after_first, "the second run committed");
    // One snapshot per run that applied something.
    let [kept_first, before_second] = names(&dir);
    assert_eq!(kept_first, taken_first);
    assert!(
        second["detail"].get("snapshot").is_none(),
        "the run report is as before: {second}"
    );

    let views_before = w.lines("systemctl.log").len();
    let (code, restored) = w.cortex(&["restore", "t", &before_second]);
    assert_eq!(
        (code, restored["outcome"].as_str()),
        (0, Some("restored")),
        "{restored}"
    );
    assert_eq!(restored["detail"]["snapshot"], before_second.as_str());
    assert_eq!(
        w.head("t"),
        after_first,
        "ekr head answers the revision the first run ended at"
    );
    // The viewer, running before, was stopped before the store was replaced and started after.
    assert_eq!(systemctl_since(&w, views_before), view_restarted());

    // The restore snapshotted what it replaced, so it can be undone the same way.
    let before_restore = restored["detail"]["before_restore"]
        .as_str()
        .unwrap_or_else(|| panic!("the restore names its own snapshot: {restored}"))
        .to_string();
    assert!(
        before_restore.ends_with("-before-restore"),
        "{before_restore}"
    );
    assert!(snapshots(&dir).contains(&before_restore));
    let (code, undone) = w.cortex(&["restore", "t", &before_restore]);
    assert_eq!(
        (code, undone["outcome"].as_str()),
        (0, Some("restored")),
        "{undone}"
    );
    assert_eq!(w.head("t"), after_second, "the restore was undone");
}

#[test]
fn a_run_that_applies_nothing_takes_no_snapshot() {
    let w = World::new();
    let dir = create(&w);
    run_applying(&w, 0);
    assert_eq!(snapshots(&dir).len(), 1);

    let (code, unchanged) = w.cortex(&["run", "t/news"]);
    assert_eq!(
        (code, unchanged["detail"]["documents_new"].as_i64()),
        (0, Some(0)),
        "{unchanged}"
    );
    assert_eq!(
        snapshots(&dir).len(),
        1,
        "nothing applied, nothing snapshotted"
    );
}

#[test]
fn a_restore_is_refused_while_the_lock_or_a_writer_holds_the_store_but_not_for_a_reader() {
    let w = World::new();
    let dir = create(&w);
    let seeded = w.head("t");
    run_applying(&w, 0);
    let [snapshot] = names(&dir);
    run_applying(&w, 1);
    let head = w.head("t");

    // Another cortex command holds the home's lock.
    {
        let lock = std::fs::File::options()
            .write(true)
            .open(w.home.join("cortex.lock"))
            .unwrap();
        lock.lock().unwrap();
        let (code, busy) = w.cortex(&["restore", "t", &snapshot]);
        assert_eq!(
            (code, busy["outcome"].as_str()),
            (1, Some("busy")),
            "{busy}"
        );
        assert!(
            busy["detail"]["reason"]
                .as_str()
                .is_some_and(|r| r.contains("lock")),
            "{busy}"
        );
    }
    assert_eq!(w.head("t"), head, "nothing was restored");

    // Another connection holds the store's write lock for longer than the restore retries.
    {
        let writer = rusqlite::Connection::open(dir.join("store.sqlite")).unwrap();
        writer.execute_batch("BEGIN IMMEDIATE").unwrap();
        let views_before = w.lines("systemctl.log").len();
        let (code, busy) = w.cortex(&["restore", "t", &snapshot]);
        assert_eq!(
            (code, busy["outcome"].as_str()),
            (1, Some("busy")),
            "{busy}"
        );
        assert!(
            busy["detail"]["reason"]
                .as_str()
                .is_some_and(|r| r.contains("write lock")),
            "{busy}"
        );
        // The viewer, stopped for the restore, runs again.
        assert_eq!(systemctl_since(&w, views_before), view_restarted());
    }
    assert_eq!(w.head("t"), head, "nothing was restored");
    assert_eq!(
        snapshots(&dir).len(),
        2,
        "a refused restore snapshots nothing"
    );

    // A reader that holds the store open, as an attached `ekr mcp` does between requests, does
    // not block the restore: the snapshot is written into the live store through SQLite's online
    // backup, not swapped in, and the reader sees it on its next read.
    let reader = rusqlite::Connection::open_with_flags(
        dir.join("store.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let events = |db: &rusqlite::Connection| -> i64 {
        db.query_row("SELECT count(*) FROM ekr_events", [], |r| r.get(0))
            .unwrap()
    };
    let read_before = events(&reader);
    let (code, restored) = w.cortex(&["restore", "t", &snapshot]);
    assert_eq!(
        (code, restored["outcome"].as_str()),
        (0, Some("restored")),
        "{restored}"
    );
    assert_eq!(
        w.head("t"),
        seeded,
        "the store is back before the first run, and EKR opens it with the reader attached"
    );
    let read_after = events(&reader);
    assert!(
        read_after < read_before,
        "the reader's next read sees the restored store: {read_before} -> {read_after}"
    );
}

#[test]
fn a_viewer_that_was_not_running_is_not_started_by_a_restore() {
    let w = World::new();
    let dir = create(&w);
    run_applying(&w, 0);
    let [snapshot] = names(&dir);
    // The operator stopped the viewer: `is-active` answers inactive (3).
    executable(
        &w.bin.join("systemctl"),
        &format!(
            "echo \"$*\" >> \"{}/systemctl.log\"\ncase \"$*\" in *is-active*) exit 3 ;; esac\n",
            w.root.display()
        ),
    );
    let views_before = w.lines("systemctl.log").len();
    let (code, restored) = w.cortex(&["restore", "t", &snapshot]);
    assert_eq!(
        (code, restored["outcome"].as_str()),
        (0, Some("restored")),
        "{restored}"
    );
    assert_eq!(
        systemctl_since(&w, views_before),
        [format!("--user is-active --quiet {VIEW}")]
    );
}

#[test]
fn a_snapshot_the_operator_named_is_kept_and_can_be_restored() {
    let w = World::new();
    let dir = create(&w);
    run_applying(&w, 0);
    let after_first = w.head("t");
    let [first] = names(&dir);
    std::fs::copy(
        dir.join(format!("snapshots/{first}.sqlite")),
        dir.join("snapshots/pinned-before-incident.sqlite"),
    )
    .unwrap();
    for n in 1..=4 {
        run_applying(&w, n);
    }
    let (code, restored) = w.cortex(&["restore", "t", "pinned-before-incident"]);
    assert_eq!(
        (code, restored["outcome"].as_str()),
        (0, Some("restored")),
        "{restored}"
    );
    assert!(
        w.head("t") < after_first,
        "the store is as before the first run"
    );
    assert!(snapshots(&dir).contains(&"pinned-before-incident".to_string()));
}

#[test]
fn an_unknown_snapshot_or_instance_is_refused_and_nothing_changes() {
    let w = World::new();
    let dir = create(&w);
    run_applying(&w, 0);
    let head = w.head("t");

    for name in ["1-none", "../store", "store.sqlite", ""] {
        let (code, out) = w.cortex(&["restore", "t", name]);
        assert_eq!(
            (code, out["outcome"].as_str()),
            (1, Some("no-such-snapshot")),
            "{name:?}: {out}"
        );
        assert_eq!(out["detail"]["snapshot"], name);
    }
    assert_eq!(w.head("t"), head);
    assert!(dir.join("store.sqlite").is_file());

    let (code, out) = w.cortex(&["restore", "nobody", "1-none"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("no-such-instance")),
        "{out}"
    );
}

#[test]
fn a_postgres_instance_has_no_snapshot_to_restore() {
    let w = World::new();
    let spec = w.spec("t", "conn_test");
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    // `restore` reads the backend from the frozen spec alone; the store is never contacted.
    let frozen = w.home.join("instances/t/instance.yaml");
    let text = std::fs::read_to_string(&frozen).unwrap();
    let config = w.root.join("never-read.json");
    std::fs::write(
        &frozen,
        format!(
            "{text}store: {{backend: postgres, value: {{config: \"{}\"}}}}\n",
            config.display()
        ),
    )
    .unwrap();
    let (code, out) = w.cortex(&["restore", "t", "1-news"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("backend-unsupported")),
        "{out}"
    );
    assert_eq!(out["detail"]["name"], "t");
}

/// Does a restored store plus the seen state skip documents forever? `restore` must put the seen
/// state back with the store, so the next run applies the undone run's document again while
/// upstream keeps the same text.
#[test]
fn a_restored_store_gets_the_undone_runs_documents_again() {
    let w = World::new();
    let dir = create(&w);
    run_applying(&w, 0);
    let after_first = w.head("t");
    run_applying(&w, 1);
    let before_second = snapshots(&dir).pop().unwrap();
    let (code, restored) = w.cortex(&["restore", "t", &before_second]);
    assert_eq!((code, restored["outcome"].as_str()), (0, Some("restored")));
    assert_eq!(w.head("t"), after_first);

    // Upstream is unchanged since the undone run; the next run is the only way back.
    let (code, again) = w.cortex(&["run", "t/news"]);
    assert_eq!(code, 0, "{again}");
    assert_eq!(
        again["detail"]["documents_applied"].as_i64(),
        Some(1),
        "the document the undone run applied is never applied to the restored store again: {again}"
    );
}

/// `spec-file.md`: `keep` is "the newest snapshots kept; a run's snapshot removes the oldest
/// beyond it". A file the operator put in `snapshots/` to keep it past rotation (the only place
/// `cortex restore` reads from) is never deleted.
#[test]
fn rotation_never_deletes_a_file_cortex_did_not_take() {
    let w = World::new();
    let dir = create(&w);
    run_applying(&w, 0);
    // The operator pins the snapshot before an incident under a name of their own.
    let first = snapshots(&dir).pop().unwrap();
    std::fs::copy(
        dir.join(format!("snapshots/{first}.sqlite")),
        dir.join("snapshots/pinned-before-incident.sqlite"),
    )
    .unwrap();
    run_applying(&w, 1);
    run_applying(&w, 2);
    assert!(
        snapshots(&dir).contains(&"pinned-before-incident".to_string()),
        "rotation deleted the operator's file: {:?}",
        files(&dir)
    );
}

/// `operating.md`: the snapshot's `<time>` is the run's start in Unix milliseconds, which also
/// names its run directory and its `cortex.log` line.
#[test]
fn a_snapshot_is_named_by_its_runs_start() {
    let w = World::new();
    let dir = create(&w);
    run_applying(&w, 0);
    let runs: Vec<String> = std::fs::read_dir(dir.join("runs"))
        .unwrap()
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    assert_eq!(runs.len(), 1, "{runs:?}");
    assert_eq!(
        snapshots(&dir),
        runs,
        "the snapshot's <time> is not the run's start"
    );
}

/// `operating.md`: "A run that applies nothing takes none", tested with ekr refusing the first
/// `apply-extraction`: no snapshot is published and none is rotated out.
#[test]
fn a_run_whose_apply_fails_takes_no_snapshot() {
    let w = World::new();
    let real = common::ekr();
    let wrapper = w.bin.join("ekr-wrapper");
    executable(
        &wrapper,
        &format!(
            "if [ \"$1\" = apply-extraction ] && [ -e \"{root}/apply-fail\" ]; then echo refused >&2; exit 1; fi\nexec \"{real}\" \"$@\"\n",
            root = w.root.display(),
            real = real.display()
        ),
    );
    let spec = w.spec("t", "conn_test");
    let text = std::fs::read_to_string(&spec)
        .unwrap()
        .replace(&real.display().to_string(), &wrapper.display().to_string());
    std::fs::write(&spec, text).unwrap();
    let dir = create_from(&w, &spec);
    run_applying(&w, 0);
    let held = snapshots(&dir);
    assert_eq!(held.len(), 1);

    upstream(&w, 1);
    std::fs::write(w.root.join("apply-fail"), "").unwrap();
    let head = w.head("t");
    let (code, failed) = w.cortex(&["run", "t/news"]);
    assert_ne!(code, 0, "{failed}");
    assert_eq!(w.head("t"), head, "the failed run applied nothing");
    assert_eq!(
        snapshots(&dir),
        held,
        "a run that applied nothing took a snapshot: {failed}"
    );
}

/// A restored snapshot leaves no `-wal` or `-shm` beside it, and rotation removes a snapshot's
/// side files with it.
#[test]
fn rotation_leaves_no_side_files_of_a_restored_snapshot() {
    let w = World::new();
    let spec = w.spec("t", "conn_test");
    let mut text = std::fs::read_to_string(&spec).unwrap();
    text.push_str("snapshots: {keep: 1}\n");
    std::fs::write(&spec, text).unwrap();
    let dir = create_from(&w, &spec);
    run_applying(&w, 0);
    run_applying(&w, 1);
    let [before_second]: [String; 1] = snapshots(&dir).try_into().unwrap();
    let (code, restored) = w.cortex(&["restore", "t", &before_second]);
    assert_eq!((code, restored["outcome"].as_str()), (0, Some("restored")));
    run_applying(&w, 2);
    let [kept]: [String; 1] = snapshots(&dir).try_into().unwrap();
    assert_ne!(kept, before_second);
    assert_eq!(
        files(&dir),
        [format!("{kept}.sqlite")],
        "files of a rotated snapshot remain"
    );
}

/// A run killed while it copies the store (here by its file-size limit; in production a
/// shutdown, an OOM kill or a unit timeout) leaves `.<name>.sqlite.tmp`; the next snapshot
/// removes it.
#[test]
fn a_snapshot_cut_short_leaves_no_copy_behind() {
    let w = World::new();
    let dir = create(&w);
    upstream(&w, 0);
    let killed = run_limited(&w, 64 * 1024, false);
    assert!(!killed.status.success(), "the limit did not stop the run");
    run_applying(&w, 0);
    let hidden: Vec<String> = files(&dir)
        .into_iter()
        .filter(|n| n.starts_with('.'))
        .collect();
    assert!(hidden.is_empty(), "left in snapshots/: {hidden:?}");
}
