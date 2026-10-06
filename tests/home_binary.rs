//! `<home>/bin/cortex`, the copy every timer of the home runs, and the version recorded beside it
//! (`<home>/bin/cortex.version`). `create`, `update` and `adopt` replace the copy when this cortex
//! is newer or the same, and say so; an older cortex keeps it unless `--replace-binary` is given,
//! and says which versions it compared. `cortex list` shows the recorded version.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{executable, World};
use serde_json::Value;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn bin(w: &World) -> PathBuf {
    w.home.join("bin/cortex")
}

fn recorded(w: &World) -> String {
    std::fs::read_to_string(w.home.join("bin/cortex.version"))
        .unwrap()
        .trim()
        .to_string()
}

/// Makes `<home>/bin/cortex` a stand-in that prints `newer`, recorded as `version`: what a newer
/// cortex that created an instance earlier would have left.
fn newer_installed(w: &World, version: &str) {
    executable(&bin(w), "echo newer\n");
    std::fs::write(w.home.join("bin/cortex.version"), format!("{version}\n")).unwrap();
}

fn is_the_stand_in(w: &World) -> bool {
    std::fs::read_to_string(bin(w)).is_ok_and(|t| t.contains("echo newer"))
}

fn is_this_cortex(w: &World) -> bool {
    std::fs::read(bin(w)).unwrap() == std::fs::read(env!("CARGO_BIN_EXE_cortex")).unwrap()
}

fn spec(w: &World, name: &str) -> PathBuf {
    w.spec(name, "conn_test")
}

fn create(w: &World, spec: &Path, extra: &[&str]) -> Value {
    let mut args = vec!["create", "--spec", spec.to_str().unwrap(), "--no-extract"];
    args.extend_from_slice(extra);
    let (code, out) = w.cortex(&args);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("created")),
        "{out}"
    );
    out
}

fn list(w: &World) -> Vec<Value> {
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .arg("list")
        .env("CORTEX_HOME", &w.home)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[test]
fn a_create_places_this_cortex_records_its_version_and_says_so() {
    let w = World::new();
    let created = create(&w, &spec(&w, "a"), &[]);
    let binary = &created["detail"]["binary"];
    assert_eq!(binary["replaced"], true, "{created}");
    assert_eq!(
        binary["old"],
        Value::Null,
        "nothing was recorded before: {created}"
    );
    assert_eq!(binary["new"], VERSION, "{created}");
    assert!(is_this_cortex(&w));
    assert_eq!(recorded(&w), VERSION);
    let rows = list(&w);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["binary_version"], VERSION, "{rows:?}");
}

#[test]
fn the_same_version_replaces_the_binary_and_says_so() {
    let w = World::new();
    create(&w, &spec(&w, "a"), &[]);
    let created = create(&w, &spec(&w, "b"), &[]);
    let binary = &created["detail"]["binary"];
    assert_eq!(binary["replaced"], true, "{created}");
    assert_eq!(binary["old"], VERSION, "{created}");
    assert_eq!(binary["new"], VERSION, "{created}");
}

#[test]
fn an_older_recorded_version_is_replaced() {
    let w = World::new();
    create(&w, &spec(&w, "a"), &[]);
    newer_installed(&w, "0.0.1");
    let created = create(&w, &spec(&w, "b"), &[]);
    let binary = &created["detail"]["binary"];
    assert_eq!(binary["replaced"], true, "{created}");
    assert_eq!(binary["old"], "0.0.1", "{created}");
    assert!(is_this_cortex(&w));
    assert_eq!(recorded(&w), VERSION);
}

/// The defect: a `create` with an older cortex replaced the binary every timer of the home runs.
/// Now it keeps the newer one, names both versions, and the new instance's units run the newer.
#[test]
fn an_older_cortex_keeps_the_newer_binary_unless_told_to_replace_it() {
    let w = World::new();
    create(&w, &spec(&w, "a"), &[]);
    newer_installed(&w, "99.0.0");

    let created = create(&w, &spec(&w, "b"), &[]);
    assert!(is_the_stand_in(&w), "the newer binary is kept: {created}");
    let binary = &created["detail"]["binary"];
    assert_eq!(binary["replaced"], false, "{created}");
    assert_eq!(binary["old"], "99.0.0", "{created}");
    assert_eq!(binary["new"], VERSION, "{created}");
    assert!(
        binary["reason"]
            .as_str()
            .is_some_and(|r| r.contains("--replace-binary")),
        "{created}"
    );
    assert_eq!(recorded(&w), "99.0.0");
    let unit = std::fs::read_to_string(w.units.join("cortex-b-news.service")).unwrap();
    assert!(
        unit.contains(&format!("ExecStart=\"{}\"", bin(&w).display())),
        "{unit}"
    );
    assert!(list(&w).iter().all(|r| r["binary_version"] == "99.0.0"));

    // `update` refuses the same way, and replaces it with `--replace-binary`.
    let path = spec(&w, "b");
    let (code, kept) = w.cortex(&["update", "b", "--spec", path.to_str().unwrap()]);
    assert_eq!(
        (code, kept["outcome"].as_str()),
        (0, Some("updated")),
        "{kept}"
    );
    assert_eq!(kept["detail"]["binary"]["replaced"], false, "{kept}");
    assert!(is_the_stand_in(&w));
    let (code, replaced) = w.cortex(&[
        "update",
        "b",
        "--spec",
        path.to_str().unwrap(),
        "--replace-binary",
    ]);
    assert_eq!(
        (code, replaced["outcome"].as_str()),
        (0, Some("updated")),
        "{replaced}"
    );
    let binary = &replaced["detail"]["binary"];
    assert_eq!(binary["replaced"], true, "{replaced}");
    assert_eq!(binary["old"], "99.0.0", "{replaced}");
    assert_eq!(binary["new"], VERSION, "{replaced}");
    assert!(is_this_cortex(&w));
    assert_eq!(recorded(&w), VERSION);
    assert!(list(&w).iter().all(|r| r["binary_version"] == VERSION));
}

/// `--no-units` installs nothing, so it leaves the binary as it is.
#[test]
fn no_units_leaves_the_binary_alone() {
    let w = World::new();
    create(&w, &spec(&w, "a"), &[]);
    newer_installed(&w, "0.0.1");
    let created = create(&w, &spec(&w, "b"), &["--no-units"]);
    assert_eq!(created["detail"]["binary"], Value::Null, "{created}");
    assert!(is_the_stand_in(&w));
}
