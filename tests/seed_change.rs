//! `cortex update` refuses a seed that differs from the instance's frozen copy and applies one that
//! matches it (`cortex.instance.UpdateInstance`, `seed-change-refused`), through the real binary.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::World;
use serde_json::Value;

/// The world's spec with one seed document directory, written as `seed_path`, beside it.
fn spec_with_seed(w: &World, seed_path: &str) -> PathBuf {
    let path = w.spec("t", "conn_test");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        text.replace(
            "seed: {documents: []}",
            &format!("seed: {{documents: [\"{seed_path}\"]}}"),
        ),
    )
    .unwrap();
    std::fs::create_dir_all(w.root.join("docs")).unwrap();
    std::fs::write(w.root.join("docs/a.md"), "Example Labs develops Widget.").unwrap();
    path
}

/// The same spec file with another description and model, and `seed_path` as its seed.
fn respec(path: &Path, seed_path: &str, description: &str) {
    let text = std::fs::read_to_string(path).unwrap();
    let start = text.find("seed: {documents: [").unwrap();
    let end = start + text[start..].find('\n').unwrap();
    let text = format!(
        "{}seed: {{documents: [\"{seed_path}\"]}}{}",
        &text[..start],
        &text[end..]
    )
    .replace(
        "description: Test brain.",
        &format!("description: {description}"),
    )
    .replace("model: claude-haiku-4-5-20251001", "model: another-model");
    std::fs::write(path, text).unwrap();
}

fn create(w: &World, spec: &Path) {
    let (code, created) = w.cortex(&[
        "create",
        "--spec",
        spec.to_str().unwrap(),
        "--no-extract",
        "--no-units",
    ]);
    assert_eq!(
        (code, created["outcome"].as_str()),
        (0, Some("created")),
        "{created}"
    );
}

fn update(w: &World, spec: &Path) -> (i32, Value) {
    w.cortex(&[
        "update",
        "t",
        "--spec",
        spec.to_str().unwrap(),
        "--no-units",
    ])
}

/// `cortex update` without the world's guard against exit 2: the status and stderr.
fn update_raw(w: &World, spec: &Path) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args([
            "update",
            "t",
            "--spec",
            spec.to_str().unwrap(),
            "--no-units",
        ])
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Everything an update may change or must keep: the `list` row and the frozen files.
fn observed(w: &World) -> (Value, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .arg("list")
        .env("CORTEX_HOME", &w.home)
        .output()
        .unwrap();
    let row: Value = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .find(|v| v["name"] == "t")
        .expect("t is listed");
    let dir = w.home.join("instances/t");
    let frozen = std::fs::read_to_string(dir.join("instance.yaml")).unwrap_or_default();
    let seed = std::fs::read_to_string(dir.join("docs/a.md")).unwrap_or_default();
    (row, frozen, seed)
}

fn outcome(result: &(i32, Value)) -> (i32, Option<&str>) {
    (result.0, result.1["outcome"].as_str())
}

#[test]
fn an_update_is_refused_while_its_seed_differs_from_the_frozen_copy() {
    let w = World::new();
    let spec = spec_with_seed(&w, "docs");
    create(&w, &spec);

    let same = update(&w, &spec);
    assert_eq!(outcome(&same), (0, Some("updated")), "{}", same.1);

    std::fs::write(w.root.join("docs/a.md"), "Example Labs develops Gizmo.").unwrap();
    let changed = update(&w, &spec);
    assert_eq!(
        outcome(&changed),
        (1, Some("seed-change-refused")),
        "{}",
        changed.1
    );

    std::fs::write(w.root.join("docs/a.md"), "Example Labs develops Widget.").unwrap();
    let restored = update(&w, &spec);
    assert_eq!(outcome(&restored), (0, Some("updated")), "{}", restored.1);

    // The stored seed is the frozen copy in the instance directory, not the file it came from.
    std::fs::write(w.home.join("instances/t/docs/a.md"), "Edited in place.").unwrap();
    let edited = update(&w, &spec);
    assert_eq!(
        outcome(&edited),
        (1, Some("seed-change-refused")),
        "{}",
        edited.1
    );
}

#[test]
fn a_renamed_seed_file_is_a_seed_change() {
    let w = World::new();
    let spec = spec_with_seed(&w, "docs");
    create(&w, &spec);
    std::fs::rename(w.root.join("docs/a.md"), w.root.join("docs/b.md")).unwrap();
    let renamed = update(&w, &spec);
    assert_eq!(
        outcome(&renamed),
        (1, Some("seed-change-refused")),
        "{}",
        renamed.1
    );
}

#[test]
fn a_seed_path_written_another_way_is_the_same_seed() {
    let w = World::new();
    let spec = spec_with_seed(&w, "docs");
    create(&w, &spec);
    for written in ["./docs", "docs/", "./docs/."] {
        respec(&spec, written, "Test brain.");
        let same = update(&w, &spec);
        assert_eq!(
            outcome(&same),
            (0, Some("updated")),
            "{written}: {}",
            same.1
        );
    }
}

#[test]
fn a_refused_update_leaves_every_field_of_the_instance_unchanged() {
    let w = World::new();
    let spec = spec_with_seed(&w, "docs");
    create(&w, &spec);
    let before = observed(&w);
    assert_eq!(before.0["state"], "Active", "{}", before.0);

    respec(&spec, "docs", "Another brain.");
    std::fs::write(w.root.join("docs/a.md"), "Example Labs develops Gizmo.").unwrap();
    let refused = update(&w, &spec);
    assert_eq!(
        outcome(&refused),
        (1, Some("seed-change-refused")),
        "{}",
        refused.1
    );
    assert_eq!(
        observed(&w),
        before,
        "seed-change-refused changed the instance"
    );

    let (code, removed) = w.cortex(&["remove", "t"]);
    assert_eq!(code, 0, "{removed}");
    let removed = observed(&w);
    assert_eq!(removed.0["state"], "Removed", "{}", removed.0);
    std::fs::write(w.root.join("docs/a.md"), "Example Labs develops Widget.").unwrap();
    let not_active = update(&w, &spec);
    assert_eq!(
        outcome(&not_active),
        (1, Some("not-active")),
        "{}",
        not_active.1
    );
    assert_eq!(observed(&w), removed, "not-active changed the instance");
}

#[test]
fn an_instance_whose_frozen_spec_is_missing_or_unreadable_fails_naming_the_file() {
    let w = World::new();
    let spec = spec_with_seed(&w, "docs");
    create(&w, &spec);
    let frozen = w.home.join("instances/t/instance.yaml");
    let text = std::fs::read_to_string(&frozen).unwrap();

    std::fs::write(&frozen, "format: [not a spec").unwrap();
    let (code, stderr) = update_raw(&w, &spec);
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains(frozen.to_str().unwrap()), "{stderr}");

    std::fs::remove_file(&frozen).unwrap();
    let (code, stderr) = update_raw(&w, &spec);
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains(frozen.to_str().unwrap()), "{stderr}");

    std::fs::write(&frozen, text).unwrap();
    let same = update(&w, &spec);
    assert_eq!(outcome(&same), (0, Some("updated")), "{}", same.1);
}

#[test]
fn a_removed_instance_whose_directory_was_deleted_is_not_active() {
    let w = World::new();
    let spec = spec_with_seed(&w, "docs");
    create(&w, &spec);
    let (code, removed) = w.cortex(&["remove", "t"]);
    assert_eq!(code, 0, "{removed}");
    std::fs::remove_dir_all(w.home.join("instances/t")).unwrap();
    let not_active = update(&w, &spec);
    assert_eq!(
        outcome(&not_active),
        (1, Some("not-active")),
        "{}",
        not_active.1
    );
}

/// The kind of `rel` in the instance's frozen copy, without following a symlink.
fn frozen_kind(w: &World, rel: &str) -> std::fs::FileType {
    std::fs::symlink_metadata(w.home.join("instances/t").join(rel))
        .unwrap_or_else(|e| panic!("frozen {rel}: {e}"))
        .file_type()
}

/// A seed directory that is a symlink is frozen as a regular directory holding regular files, and
/// an update with nothing changed is applied.
#[test]
fn a_symlinked_seed_directory_is_frozen_as_a_regular_directory() {
    let w = World::new();
    let spec = spec_with_seed(&w, "docs");
    std::fs::rename(w.root.join("docs"), w.root.join("real-docs")).unwrap();
    std::os::unix::fs::symlink(w.root.join("real-docs"), w.root.join("docs")).unwrap();
    create(&w, &spec);

    assert!(frozen_kind(&w, "docs").is_dir(), "docs is not a directory");
    assert!(frozen_kind(&w, "docs/a.md").is_file(), "docs/a.md");
    assert_eq!(observed(&w).2, "Example Labs develops Widget.");

    let same = update(&w, &spec);
    assert_eq!(outcome(&same), (0, Some("updated")), "{}", same.1);
}

/// A symlinked file and a symlinked subdirectory inside a seed directory are frozen as a regular
/// file and a regular directory, and an update with nothing changed is applied.
#[test]
fn symlinks_inside_a_seed_directory_are_frozen_as_regular_files_and_directories() {
    let w = World::new();
    let spec = spec_with_seed(&w, "docs");
    let elsewhere = w.root.join("elsewhere");
    std::fs::create_dir_all(elsewhere.join("sub")).unwrap();
    std::fs::rename(w.root.join("docs/a.md"), elsewhere.join("a.md")).unwrap();
    std::os::unix::fs::symlink(elsewhere.join("a.md"), w.root.join("docs/a.md")).unwrap();
    std::fs::write(elsewhere.join("sub/b.md"), "Example Labs develops Gadget.").unwrap();
    std::os::unix::fs::symlink(elsewhere.join("sub"), w.root.join("docs/sub")).unwrap();
    create(&w, &spec);

    assert!(frozen_kind(&w, "docs/a.md").is_file(), "docs/a.md");
    assert_eq!(observed(&w).2, "Example Labs develops Widget.");
    assert!(frozen_kind(&w, "docs/sub").is_dir(), "docs/sub");
    assert!(frozen_kind(&w, "docs/sub/b.md").is_file(), "docs/sub/b.md");

    let same = update(&w, &spec);
    assert_eq!(outcome(&same), (0, Some("updated")), "{}", same.1);
}

/// A dangling symlink and a symlink to an enclosing directory inside a seed directory are left
/// out of the frozen copy and of the seed digest alike, as before symlinks were followed: the
/// instance is created and an update with nothing changed is applied.
#[test]
fn a_dangling_or_looping_symlink_inside_a_seed_directory_is_left_out() {
    let w = World::new();
    let spec = spec_with_seed(&w, "docs");
    std::os::unix::fs::symlink(w.root.join("missing.md"), w.root.join("docs/dangling.md")).unwrap();
    std::os::unix::fs::symlink(w.root.join("docs"), w.root.join("docs/loop")).unwrap();
    create(&w, &spec);

    let frozen = w.home.join("instances/t/docs");
    assert!(std::fs::symlink_metadata(frozen.join("dangling.md")).is_err());
    assert!(std::fs::symlink_metadata(frozen.join("loop")).is_err());
    assert_eq!(observed(&w).2, "Example Labs develops Widget.");

    let same = update(&w, &spec);
    assert_eq!(outcome(&same), (0, Some("updated")), "{}", same.1);
}
