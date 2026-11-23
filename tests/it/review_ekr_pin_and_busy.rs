//! Findings 17 and 18 of the 0.7.2 review.
//!
//! 17: the EKR pin an instance records is the version of the binary that creates its store. A
//! binary that reports another version, named by `ekr.bin` or found in the cache `cortex setup`
//! fills, is refused before a store exists. The binaries here are `/bin/sh` stand-ins that print a
//! version and log every other call: no network and no build.
//!
//! 18: `cortex run --no-wait` answers `busy` (exit 9) at once while another command holds the
//! home's lock, and records nothing; without the flag a run waits for the lock and then runs.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::common::{executable, World};
use serde_json::Value;

/// A stand-in `ekr` at `path` that prints `ekr <version>` for `--version` and logs every other
/// call to `<dir>/ekr-calls.log`, failing it.
fn stand_in_ekr(path: &Path, version: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let log = path.parent().unwrap().join("ekr-calls.log");
    executable(
        path,
        &format!(
            r#"if [ "$1" = --version ]; then echo "ekr {version}"; exit 0; fi
echo "$*" >> "{log}"
exit 1
"#,
            log = log.display()
        ),
    );
}

fn calls(ekr: &Path) -> Vec<String> {
    std::fs::read_to_string(ekr.parent().unwrap().join("ekr-calls.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

/// A spec of instance `t` pinning EKR 0.0.34, with `ekr.bin` when `bin` is given.
fn spec(w: &World, bin: Option<&Path>) -> PathBuf {
    let ekr = match bin {
        Some(bin) => format!(r#"{{version: "0.0.34", bin: "{}"}}"#, bin.display()),
        None => r#"{version: "0.0.34"}"#.to_string(),
    };
    let path = w.root.join("t.yaml");
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: t
description: Test brain.
ekr: {ekr}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources: []
serve: {{view_port: 18999}}
"#
        ),
    )
    .unwrap();
    path
}

/// `cortex <args>` in `w` with `HOME` at `home`: its exit status, the last JSON line it printed
/// and its stderr.
fn cortex_at(w: &World, home: &Path, args: &[&str]) -> (i32, Value, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(args)
        .env("HOME", home)
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let value =
        serde_json::from_str(stdout.lines().last().unwrap_or("null")).unwrap_or(Value::Null);
    (
        out.status.code().unwrap_or(-1),
        value,
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn assert_not_created(w: &World, home: &Path, ekr: &Path) {
    assert!(
        !w.home.join("instances/t").exists(),
        "a refused create leaves no instance directory"
    );
    let listed = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .arg("list")
        .env("HOME", home)
        .env("CORTEX_HOME", &w.home)
        .output()
        .unwrap();
    assert!(listed.status.success());
    assert!(
        !String::from_utf8_lossy(&listed.stdout).contains("\"t\""),
        "the instance is not listed: {}",
        String::from_utf8_lossy(&listed.stdout)
    );
    assert_eq!(
        calls(ekr),
        Vec::<String>::new(),
        "the binary was asked its version and nothing else: no store was created"
    );
}

#[test]
fn create_refuses_an_explicit_ekr_that_reports_another_version_than_the_pin() {
    let w = World::new();
    let ekr = w.root.join("ekr-0.0.32/ekr");
    stand_in_ekr(&ekr, "0.0.32");
    let spec = spec(&w, Some(&ekr));
    let (code, created, _) = cortex_at(
        &w,
        &w.root,
        &["create", "--spec", spec.to_str().unwrap(), "--no-units"],
    );
    assert_eq!(
        (code, created["outcome"].as_str()),
        (1, Some("seed-refused")),
        "{created}"
    );
    let reason = created["detail"].to_string();
    assert!(
        reason.contains("reports version 0.0.32") && reason.contains("0.0.34"),
        "{created}"
    );
    assert_not_created(&w, &w.root, &ekr);
}

#[test]
fn create_refuses_a_cached_ekr_that_reports_another_version_than_the_pin() {
    let w = World::new();
    let home = w.root.join("user");
    let ekr = home.join(".cache/cortex/bin/0.0.34/bin/ekr");
    stand_in_ekr(&ekr, "0.0.32");
    let spec = spec(&w, None);
    let (code, created, _) = cortex_at(
        &w,
        &home,
        &["create", "--spec", spec.to_str().unwrap(), "--no-units"],
    );
    assert_eq!(
        (code, created["outcome"].as_str()),
        (1, Some("seed-refused")),
        "{created}"
    );
    assert!(
        created["detail"]
            .to_string()
            .contains("reports version 0.0.32"),
        "{created}"
    );
    assert_not_created(&w, &home, &ekr);
}

#[test]
fn setup_refuses_a_cached_ekr_of_another_version_and_keeps_one_of_the_pinned_version() {
    let w = World::new();
    let home = w.root.join("user");
    let ekr = home.join(".cache/cortex/bin/0.0.34/bin/ekr");

    stand_in_ekr(&ekr, "0.0.32");
    let (code, _, stderr) = cortex_at(&w, &home, &["setup", "--ekr-version", "0.0.34"]);
    assert_eq!(code, 2, "{stderr}");
    assert!(
        stderr.contains("reports version 0.0.32, not the pinned 0.0.34"),
        "{stderr}"
    );

    stand_in_ekr(&ekr, "0.0.34");
    let (code, out, stderr) = cortex_at(&w, &home, &["setup", "--ekr-version", "0.0.34"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(out["ekr"].as_str(), Some(ekr.to_str().unwrap()), "{out}");
    assert_eq!(calls(&ekr), Vec::<String>::new());
}

#[test]
fn adopt_refuses_an_ekr_that_reports_another_version_than_the_pin() {
    let w = World::new();
    let ekr = w.root.join("ekr-0.0.32/ekr");
    stand_in_ekr(&ekr, "0.0.32");
    let spec = spec(&w, Some(&ekr));
    // A file with SQLite's header is what `adopt` reads before it looks at the binary.
    let store = w.root.join("store.sqlite");
    let mut bytes = b"SQLite format 3\0".to_vec();
    bytes.resize(4096, 0);
    std::fs::write(&store, bytes).unwrap();
    let (code, _, stderr) = cortex_at(
        &w,
        &w.root,
        &[
            "adopt",
            "--spec",
            spec.to_str().unwrap(),
            "--store",
            store.to_str().unwrap(),
            "--no-units",
        ],
    );
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains("reports version 0.0.32"), "{stderr}");
    assert_not_created(&w, &w.root, &ekr);
}

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

fn source_row(w: &World) -> Value {
    let (code, row) = w.cortex(&["sources"]);
    assert_eq!(code, 0);
    row
}

#[test]
fn a_no_wait_run_answers_busy_at_once_while_the_lock_is_held_and_a_default_run_waits() {
    let w = World::new();
    let (code, created) = w.cortex(&[
        "create",
        "--spec",
        w.spec("t", "conn_test").to_str().unwrap(),
    ]);
    assert_eq!(
        (code, created["outcome"].as_str()),
        (0, Some("created")),
        "{created}"
    );
    let before = source_row(&w);
    let status = w.home.join("instances/t/status");
    let status_before = std::fs::read_dir(&status).map(|d| d.count()).unwrap_or(0);

    // `--no-wait`: busy at once, with its own exit status, and nothing ran or was recorded.
    let lock = hold_lock(&w.home);
    let started = Instant::now();
    let busy = finish(
        spawn(&w, &["run", "t/news", "--no-wait"]),
        Duration::from_secs(5),
    );
    let took = started.elapsed();
    let (code, busy) = busy.expect("a run with --no-wait does not wait for the lock");
    assert_eq!(
        (code, busy["outcome"].as_str()),
        (9, Some("busy")),
        "{busy}"
    );
    assert!(took < Duration::from_secs(1), "busy took {took:?}");
    assert!(
        busy["detail"]["reason"]
            .as_str()
            .is_some_and(|r| r.contains("holds the lock")),
        "{busy}"
    );
    assert!(w.lines("invocations.log").is_empty(), "nothing was fetched");
    assert!(
        w.lines("claude-calls.log").is_empty(),
        "no model was called"
    );
    assert_eq!(
        std::fs::read_dir(&status).map(|d| d.count()).unwrap_or(0),
        status_before,
        "a busy run writes no status file"
    );
    // `cortex sources` waits for the lock too: read the source once the lock is free.
    drop(lock);
    let after = source_row(&w);
    assert_eq!(after["runs"], before["runs"], "{after}");
    assert_eq!(after["last_outcome"], before["last_outcome"], "{after}");
    assert_eq!(
        after["consecutive_failures"], before["consecutive_failures"],
        "{after}"
    );

    // Without the flag the run waits for the lock, and runs once it is free.
    let lock = hold_lock(&w.home);
    let mut waiting = spawn(&w, &["run", "t/news"]);
    std::thread::sleep(Duration::from_millis(500));
    let still = waiting.try_wait().unwrap().is_none();
    drop(lock);
    if !still {
        let _ = waiting.wait();
    }
    assert!(
        still,
        "a run without --no-wait waits while the lock is held"
    );
    let (code, ran) = finish(waiting, Duration::from_secs(40))
        .expect("the waiting run ends once the lock is free");
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");

    // With the lock free, `--no-wait` runs as a default run does.
    let (code, again) = w.cortex(&["run", "t/news", "--no-wait"]);
    assert_eq!(
        (code, again["outcome"].as_str()),
        (0, Some("ran")),
        "{again}"
    );
}
