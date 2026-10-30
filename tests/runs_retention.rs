//! `story:runs-retention`: with `runs.keep_days`, a run removes the run directories under
//! `runs/` older than that many days at its end; it never removes the current run's directory, nor
//! one whose name is not `<unix ms>-<source>`. Without the setting nothing is removed.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use common::World;

const DAY_MS: u128 = 86_400_000;

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

/// The spec of instance `name`, with `extra` appended at the top level.
fn spec(w: &World, name: &str, extra: &str) -> PathBuf {
    let path = w.spec(name, "conn_test");
    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str(extra);
    std::fs::write(&path, text).unwrap();
    path
}

fn create(w: &World, name: &str, extra: &str) -> PathBuf {
    let path = spec(w, name, extra);
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    w.home.join("instances").join(name).join("runs")
}

/// Plants the directory `runs/<name>` with one file in it.
fn plant(runs: &Path, name: &str) {
    let dir = runs.join(name).join("batch-1");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("prompt.txt"), "planted").unwrap();
}

fn names(runs: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(runs)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// Plants two run directories two days old, one an hour old, and three whose names cortex did
/// not give; answers them in that order.
fn planted(runs: &Path) -> [String; 6] {
    let now = now_ms();
    let names = [
        format!("{}-news", now - 2 * DAY_MS),
        format!("{}-seed", now - 2 * DAY_MS - 5),
        format!("{}-news", now - DAY_MS / 24),
        "notes".to_string(),
        format!("{}-", now - 2 * DAY_MS),
        format!("x{}-news", now - 2 * DAY_MS),
    ];
    for name in &names {
        plant(runs, name);
    }
    names
}

fn ran(w: &World, name: &str) {
    let (code, ran) = w.cortex(&["run", &format!("{name}/news")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert!(
        ran["detail"]["documents_applied"].as_i64() > Some(0),
        "{ran}"
    );
}

/// The acceptance: with `runs.keep_days: 1` a run removes the run directories older than one day
/// and keeps the current one, the one an hour old, and every name cortex did not give.
#[test]
fn with_keep_days_a_run_removes_run_directories_older_than_that_and_keeps_the_current_one() {
    let w = World::new();
    let runs = create(&w, "r", "runs: {keep_days: 1}\n");
    let [old, old_seed, recent, notes, bare, prefixed] = planted(&runs);
    let started = now_ms();
    ran(&w, "r");

    let left = names(&runs);
    assert!(!left.contains(&old), "{left:?}");
    assert!(!left.contains(&old_seed), "{left:?}");
    for kept in [&recent, &notes, &bare, &prefixed] {
        assert!(left.contains(kept), "{kept} was removed: {left:?}");
    }
    let current: Vec<&String> = left
        .iter()
        .filter(|n| {
            n.strip_suffix("-news")
                .and_then(|ms| ms.parse::<u128>().ok())
                .is_some_and(|ms| ms >= started)
        })
        .collect();
    assert_eq!(current.len(), 1, "the current run's directory: {left:?}");
    assert!(runs.join(current[0]).is_dir());
}

/// Without a `runs` section nothing under `runs/` is removed.
#[test]
fn without_the_setting_no_run_directory_is_removed() {
    let w = World::new();
    let runs = create(&w, "k", "");
    let planted = planted(&runs);
    ran(&w, "k");
    let left = names(&runs);
    for name in &planted {
        assert!(left.contains(name), "{name} was removed: {left:?}");
    }
    assert_eq!(left.len(), planted.len() + 1, "{left:?}");
}

/// `runs.keep_days` below 1 is refused when the spec is checked, naming the field.
#[test]
fn keep_days_below_one_is_refused() {
    let w = World::new();
    let path = spec(&w, "z", "runs: {keep_days: 0}\n");
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["create", "--spec", path.to_str().unwrap(), "--no-units"])
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .output()
        .unwrap();
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.status.code(), Some(2), "{said}");
    assert!(said.contains("runs.keep_days"), "{said}");
}
