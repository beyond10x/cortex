//! Seed and instructions paths through `cortex update` and `cortex create`, and the seed digest
//! `cortex list` prints, through the real binary. Written as adversary cases in wave 20261005c
//! (U2, pass 2) and kept here.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::World;
use serde_json::Value;

/// The world's spec with the seed document directory `docs` and the instructions file
/// `prompt.md`, both beside it.
fn spec_with_seed_and_instructions(w: &World) -> PathBuf {
    let path = w.spec("t", "conn_test");
    let text = std::fs::read_to_string(&path).unwrap();
    let text = text
        .replace("seed: {documents: []}", "seed: {documents: [\"docs\"]}")
        .replace("timeout_s: 60}", "timeout_s: 60, instructions: prompt.md}");
    std::fs::write(&path, text).unwrap();
    std::fs::create_dir_all(w.root.join("docs")).unwrap();
    std::fs::write(w.root.join("docs/a.md"), "Example Labs develops Widget.").unwrap();
    std::fs::write(w.root.join("prompt.md"), "Extract organisations.").unwrap();
    path
}

fn replace_in(path: &Path, from: &str, to: &str) {
    let text = std::fs::read_to_string(path).unwrap();
    assert!(text.contains(from), "{from:?} is not in {}", path.display());
    std::fs::write(path, text.replace(from, to)).unwrap();
}

fn create(w: &World, spec: &Path) -> (i32, Value) {
    w.cortex(&[
        "create",
        "--spec",
        spec.to_str().unwrap(),
        "--no-extract",
        "--no-units",
    ])
}

fn created(w: &World, spec: &Path) {
    let result = create(w, spec);
    assert_eq!(
        (result.0, result.1["outcome"].as_str()),
        (0, Some("created")),
        "{}",
        result.1
    );
}

fn update(w: &World, name: &str, spec: &Path) -> (i32, Value) {
    w.cortex(&[
        "update",
        name,
        "--spec",
        spec.to_str().unwrap(),
        "--no-units",
    ])
}

fn outcome(result: &(i32, Value)) -> (i32, Option<&str>) {
    (result.0, result.1["outcome"].as_str())
}

fn list_rows(w: &World) -> Vec<Value> {
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .arg("list")
        .env("CORTEX_HOME", &w.home)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .collect()
}

/// An update respelling the instructions path as `nope/../prompt.md` must not leave the frozen
/// spec naming a path that does not resolve in the instance directory: later runs still work.
#[test]
fn an_update_respelling_the_instructions_path_through_a_missing_directory_keeps_runs_working() {
    let w = World::new();
    let spec = spec_with_seed_and_instructions(&w);
    created(&w, &spec);

    replace_in(
        &spec,
        "instructions: prompt.md",
        "instructions: nope/../prompt.md",
    );
    let _ = update(&w, "t", &spec);

    let (code, ran) = w.cortex(&["run", "t/news"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (0, Some("ran")),
        "the source no longer runs after the update: {ran}"
    );
}

/// An update making the instructions path absolute (`/prompt.md`) must not send later runs outside
/// the instance directory. `create` refuses the same spelling.
#[test]
fn an_update_making_the_instructions_path_absolute_keeps_runs_inside_the_instance() {
    let w = World::new();
    let spec = spec_with_seed_and_instructions(&w);
    created(&w, &spec);

    replace_in(&spec, "instructions: prompt.md", "instructions: /prompt.md");
    let result = update(&w, "t", &spec);

    let (code, ran) = w.cortex(&["run", "t/news"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (0, Some("ran")),
        "update {} pointed the instance's runs outside it: {ran}",
        result.1
    );
}

/// `create` refuses a seed path containing `..` ("leaves the spec's directory"); `update` must not
/// write one into the frozen spec either.
#[test]
fn update_does_not_freeze_a_seed_path_create_refuses() {
    let w = World::new();
    let spec = spec_with_seed_and_instructions(&w);

    // The same spelling, given to `create`, is refused.
    let refused = w.root.join("refused.yaml");
    std::fs::write(
        &refused,
        std::fs::read_to_string(&spec)
            .unwrap()
            .replace("name: t\n", "name: r\n")
            .replace("[\"docs\"]", "[\"nope/../docs\"]"),
    )
    .unwrap();
    let r = create(&w, &refused);
    assert_eq!(outcome(&r), (1, Some("seed-refused")), "{}", r.1);

    created(&w, &spec);
    replace_in(&spec, "[\"docs\"]", "[\"nope/../docs\"]");
    let result = update(&w, "t", &spec);
    let frozen = std::fs::read_to_string(w.home.join("instances/t/instance.yaml")).unwrap();
    assert!(
        !frozen.contains(".."),
        "update {} froze a path create refuses:\n{frozen}",
        result.1
    );
}

/// The `Instances` view (`list` on the wire) exposes `seed_digest` (spec/domains/instance.yaml,
/// website/docs/reference/ess/cortex-instance.md), read-your-writes. `cortex list` must carry it,
/// and carry the instance's seed rather than the empty text a loaded registry entry holds.
#[test]
fn the_list_row_carries_the_seed_digest_the_view_exposes() {
    let w = World::new();
    let spec = spec_with_seed_and_instructions(&w);
    created(&w, &spec);
    let rows = list_rows(&w);
    let row = rows.iter().find(|r| r["name"] == "t").expect("t is listed");
    let digest = row.get("seed_digest").and_then(Value::as_str);
    assert!(
        digest.is_some_and(|d| d.len() == 64),
        "the list row has no seed digest: {row}"
    );
}

/// An instructions file that is a symlink is frozen as a regular file (`fs::copy` follows it), and
/// the seed digest reads it as its target, so an update with nothing changed is applied.
#[test]
fn an_unchanged_symlinked_instructions_file_updates() {
    let w = World::new();
    let spec = spec_with_seed_and_instructions(&w);
    std::fs::rename(w.root.join("prompt.md"), w.root.join("real-prompt.md")).unwrap();
    std::os::unix::fs::symlink(w.root.join("real-prompt.md"), w.root.join("prompt.md")).unwrap();
    created(&w, &spec);
    assert!(w.home.join("instances/t/prompt.md").is_file());

    let same = update(&w, "t", &spec);
    assert_eq!(outcome(&same), (0, Some("updated")), "{}", same.1);
}

/// The spec reformatted (flow mapping to block mapping, comments) with the same seed updates; a
/// changed instructions file is a seed change.
#[test]
fn a_reformatted_spec_updates_and_a_changed_instructions_file_is_refused() {
    let w = World::new();
    let spec = spec_with_seed_and_instructions(&w);
    created(&w, &spec);

    replace_in(
        &spec,
        "seed: {documents: [\"docs\"]}",
        "# reformatted\nseed:\n  documents:\n    - docs\n",
    );
    let same = update(&w, "t", &spec);
    assert_eq!(outcome(&same), (0, Some("updated")), "{}", same.1);

    std::fs::write(w.root.join("prompt.md"), "Extract products.").unwrap();
    let changed = update(&w, "t", &spec);
    assert_eq!(
        outcome(&changed),
        (1, Some("seed-change-refused")),
        "{}",
        changed.1
    );
}

/// A seed path escaping the spec's directory to an identical copy of the seed is still a seed
/// change.
#[test]
fn a_seed_path_escaping_the_spec_directory_is_a_seed_change() {
    let w = World::new();
    let spec = spec_with_seed_and_instructions(&w);
    created(&w, &spec);
    let outside = w.root.parent().unwrap().join(format!(
        "{}-outside",
        w.root.file_name().unwrap().to_string_lossy()
    ));
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("a.md"), "Example Labs develops Widget.").unwrap();
    let escaping = format!("../{}", outside.file_name().unwrap().to_string_lossy());
    replace_in(&spec, "[\"docs\"]", &format!("[\"{escaping}\"]"));
    let result = update(&w, "t", &spec);
    std::fs::remove_dir_all(&outside).unwrap();
    assert_eq!(
        outcome(&result),
        (1, Some("seed-change-refused")),
        "{}",
        result.1
    );
}

/// A removed instance's name stays taken, with and without its directory.
#[test]
fn a_removed_instance_s_name_stays_taken() {
    let w = World::new();
    let spec = spec_with_seed_and_instructions(&w);
    created(&w, &spec);
    let (code, removed) = w.cortex(&["remove", "t"]);
    assert_eq!(code, 0, "{removed}");
    let again = create(&w, &spec);
    assert_eq!(outcome(&again), (1, Some("name-taken")), "{}", again.1);
    std::fs::remove_dir_all(w.home.join("instances/t")).unwrap();
    let again = create(&w, &spec);
    assert_eq!(outcome(&again), (1, Some("name-taken")), "{}", again.1);
}

/// `registry.json` never carries a seed digest, empty or not, after create and update of two
/// instances, and an update of one leaves the other's entry as it was.
#[test]
fn registry_json_carries_no_seed_digest() {
    let w = World::new();
    let spec = spec_with_seed_and_instructions(&w);
    created(&w, &spec);
    let other = w.root.join("u.yaml");
    std::fs::write(
        &other,
        std::fs::read_to_string(&spec)
            .unwrap()
            .replace("name: t\n", "name: u\n")
            .replace("view_port: 18999", "view_port: 18998"),
    )
    .unwrap();
    created(&w, &other);
    let registry = || -> Value {
        serde_json::from_slice(&std::fs::read(w.home.join("registry.json")).unwrap()).unwrap()
    };
    let before = registry();
    let u_before = before["instances"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"] == "u")
        .cloned();

    let same = update(&w, "t", &spec);
    assert_eq!(outcome(&same), (0, Some("updated")), "{}", same.1);
    std::fs::write(w.root.join("docs/a.md"), "changed").unwrap();
    let refused = update(&w, "u", &other);
    assert_eq!(
        outcome(&refused),
        (1, Some("seed-change-refused")),
        "{}",
        refused.1
    );

    let after = registry();
    let text = after.to_string();
    assert!(!text.contains("seed_digest"), "{text}");
    let u_after = after["instances"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"] == "u")
        .cloned();
    assert_eq!(u_after, u_before);
}

/// A seed or instructions path `create` would refuse (`..` or absolute) is refused by `update` as
/// `seed-change-refused`, naming the field, and nothing is frozen.
#[test]
fn an_update_naming_a_path_create_refuses_is_a_seed_change_naming_the_field() {
    let w = World::new();
    let spec = spec_with_seed_and_instructions(&w);
    created(&w, &spec);
    let frozen = || std::fs::read_to_string(w.home.join("instances/t/instance.yaml")).unwrap();
    let before = frozen();
    let original = std::fs::read_to_string(&spec).unwrap();

    for (from, to, field) in [
        (
            "instructions: prompt.md",
            "instructions: /prompt.md",
            "model.instructions",
        ),
        (
            "instructions: prompt.md",
            "instructions: nope/../prompt.md",
            "model.instructions",
        ),
        ("[\"docs\"]", "[\"nope/../docs\"]", "seed.documents[0]"),
        ("[\"docs\"]", "[\"docs\", \"/etc\"]", "seed.documents[1]"),
    ] {
        std::fs::write(&spec, original.replace(from, to)).unwrap();
        let refused = update(&w, "t", &spec);
        assert_eq!(
            outcome(&refused),
            (1, Some("seed-change-refused")),
            "{to}: {}",
            refused.1
        );
        let reason = refused.1["detail"]["reason"].as_str().unwrap_or_default();
        assert!(reason.contains(field), "{to}: {}", refused.1);
        assert_eq!(frozen(), before, "{to} was frozen");
    }
}

/// `cortex list` prints `null` for an instance whose frozen copy cannot be read, never `""`.
#[test]
fn a_list_row_without_a_readable_frozen_copy_carries_a_null_seed_digest() {
    let w = World::new();
    let spec = spec_with_seed_and_instructions(&w);
    created(&w, &spec);
    std::fs::remove_file(w.home.join("instances/t/instance.yaml")).unwrap();
    let rows = list_rows(&w);
    let row = rows.iter().find(|r| r["name"] == "t").expect("t is listed");
    assert_eq!(row.get("seed_digest"), Some(&Value::Null), "{row}");
}
