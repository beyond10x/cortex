//! Story `update-no-wait`: `cortex update --no-wait` answers `busy` (exit 9) at once while another
//! cortex command holds the home's lock, and changes nothing; without the flag an update waits for
//! the lock and then applies. `remove` and `source enable` take the same flag.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::common::World;
use serde_json::Value;

/// The home's lock, held as another cortex command holds it.
fn hold_lock(home: &Path) -> std::fs::File {
    std::fs::create_dir_all(home).unwrap();
    let lock = std::fs::File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(home.join("cortex.lock"))
        .unwrap();
    lock.lock().unwrap();
    lock
}

/// `cortex <args>` in `w`, started and not waited for.
fn spawn(w: &World, args: &[&str]) -> Child {
    Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(args)
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .env("ANTHROPIC_API_KEY", "must-not-reach-claude")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

/// Waits at most `limit` for `child`, killing it when it runs longer: its exit status and the
/// last JSON line it printed, or `None` when it was killed.
fn finish(mut child: Child, limit: Duration) -> Option<(i32, Value)> {
    let started = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if started.elapsed() > limit {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let value =
        serde_json::from_str(stdout.lines().last().unwrap_or("null")).unwrap_or(Value::Null);
    Some((out.status.code().unwrap_or(-1), value))
}

/// `cortex <args>` with the lock held: it answers `busy` with exit 9 within 1 s.
fn assert_busy_at_once(w: &World, args: &[&str]) {
    let started = Instant::now();
    let done = finish(spawn(w, args), Duration::from_secs(5));
    let took = started.elapsed();
    let (code, busy) = done.unwrap_or_else(|| panic!("{args:?} does not wait for the lock"));
    assert_eq!(
        (code, busy["outcome"].as_str()),
        (9, Some("busy")),
        "{args:?}: {busy}"
    );
    assert!(took < Duration::from_secs(1), "{args:?} took {took:?}");
    assert!(
        busy["detail"]["reason"]
            .as_str()
            .is_some_and(|r| r.contains("holds the lock")),
        "{busy}"
    );
}

fn created(w: &World) -> PathBuf {
    let spec = w.spec("t", "conn_test");
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap()]);
    assert_eq!(
        (code, created["outcome"].as_str()),
        (0, Some("created")),
        "{created}"
    );
    spec
}

fn description(w: &World) -> String {
    let (code, row) = w.cortex(&["list"]);
    assert_eq!(code, 0);
    row["description"].as_str().unwrap_or_default().to_string()
}

#[test]
fn a_no_wait_update_answers_busy_at_once_and_a_default_update_waits_then_applies() {
    let w = World::new();
    let spec = created(&w);
    let changed = w.root.join("changed.yaml");
    let text = std::fs::read_to_string(&spec)
        .unwrap()
        .replace("Test brain.", "Changed brain.");
    std::fs::write(&changed, &text).unwrap();
    let frozen = w.home.join("instances/t/instance.yaml");
    let frozen_before = std::fs::read(&frozen).unwrap();
    let units_before = w.lines("systemctl.log");
    let args = ["update", "t", "--spec", changed.to_str().unwrap()];

    // `--no-wait`: busy at once, with its own exit status, and nothing changed.
    let lock = hold_lock(&w.home);
    assert_busy_at_once(&w, &[&args[..], &["--no-wait"]].concat());
    assert_eq!(
        std::fs::read(&frozen).unwrap(),
        frozen_before,
        "a busy update leaves the frozen spec as it was"
    );
    assert_eq!(w.lines("systemctl.log"), units_before, "nor touches a unit");
    drop(lock);
    assert_eq!(description(&w), "Test brain.", "nor the registry");

    // Without the flag the update waits for the lock, and applies once it is free.
    let lock = hold_lock(&w.home);
    let mut waiting = spawn(&w, &args);
    std::thread::sleep(Duration::from_millis(500));
    let still = waiting.try_wait().unwrap().is_none();
    drop(lock);
    if !still {
        let _ = waiting.wait();
    }
    assert!(
        still,
        "an update without --no-wait waits while the lock is held"
    );
    let (code, updated) = finish(waiting, Duration::from_secs(30))
        .expect("the waiting update ends once the lock is free");
    assert_eq!(
        (code, updated["outcome"].as_str()),
        (0, Some("updated")),
        "{updated}"
    );
    assert_eq!(std::fs::read_to_string(&frozen).unwrap(), text);
    assert_eq!(description(&w), "Changed brain.");
}

#[test]
fn remove_and_source_enable_answer_busy_at_once_with_no_wait() {
    let w = World::new();
    created(&w);

    let lock = hold_lock(&w.home);
    assert_busy_at_once(&w, &["source", "enable", "t/news", "--no-wait"]);
    assert_busy_at_once(&w, &["remove", "t", "--no-wait"]);
    drop(lock);
    let (code, row) = w.cortex(&["list"]);
    assert_eq!(code, 0);
    assert_eq!(
        row["state"], "Active",
        "a busy remove removed nothing: {row}"
    );

    // With the lock free, `--no-wait` acts as the command without it.
    let (code, enabled) = w.cortex(&["source", "enable", "t/news", "--no-wait"]);
    assert_eq!(
        (code, enabled["outcome"].as_str()),
        (1, Some("wrong-state")),
        "{enabled}"
    );
    let (code, removed) = w.cortex(&["remove", "t", "--no-wait"]);
    assert_eq!(
        (code, removed["outcome"].as_str()),
        (0, Some("removed")),
        "{removed}"
    );
}
