//! Adversary cases against issue #28's fix (PR #36): `cortex run <name>/seed` keeps a snapshot and
//! is held to the gate as a source run is, and is refused on an adopted instance. (A `create`
//! killed during its seed, which leaves a seeded store with no registry entry, is pre-existing and
//! not covered here.)

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{ekr, World};
use serde_json::Value;

/// As `tests/seed_partial.rs`: five seed files of about 14 700 characters, two model calls; the
/// stand-in model costs 0.01 per call and answers one refused fact per batch. `extra` is appended.
fn spec(w: &World, name: &str, budget: &str, extra: &str) -> PathBuf {
    let docs = w.root.join("docs");
    std::fs::create_dir_all(&docs).unwrap();
    for n in ["a", "b", "c", "d", "e"] {
        let text = format!(
            "# {n}\n\n{}",
            "Example Labs develops the Widget engine. ".repeat(360)
        );
        std::fs::write(docs.join(format!("{n}.md")), text).unwrap();
    }
    let path = w.root.join(format!("{name}.yaml"));
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: {name}
description: Test brain.
ekr: {{version: "0.0.30", bin: "{ekr}"}}
seed: {{documents: [docs]}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "{budget}", timeout_s: 60}}
sources: []
serve: {{view_port: 18993}}
{extra}"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    path
}

fn create(w: &World, spec: &Path) -> (i32, Value) {
    w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"])
}

fn create_partial(w: &World, spec: &Path) {
    let (code, created) = create(w, spec);
    assert_eq!(
        (code, created["outcome"].as_str()),
        (0, Some("partial")),
        "{created}"
    );
}

/// The snapshots `cortex restore` takes for instance `name`.
fn snapshots(w: &World, name: &str) -> Vec<String> {
    let mut names: Vec<String> =
        std::fs::read_dir(w.home.join(format!("instances/{name}/snapshots")))
            .map(|e| {
                e.flatten()
                    .filter_map(|e| e.file_name().into_string().ok())
                    .filter(|n| !n.starts_with('.'))
                    .filter_map(|n| n.strip_suffix(".sqlite").map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
    names.sort();
    names
}

/// How many evidence items of instance `name`'s store cite each seed file, by file name.
fn evidence_per_file(w: &World, name: &str) -> std::collections::BTreeMap<String, usize> {
    let dir = w.home.join("instances").join(name);
    let out = Command::new(ekr())
        .arg("snapshot")
        .env("EKR_HOST", dir.join("host.json"))
        .env("EKR_BACKEND", "sqlite")
        .env("EKR_STORE", dir.join("store.sqlite"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let snap: Value = serde_json::from_slice(&out.stdout).unwrap();
    let mut counts = std::collections::BTreeMap::new();
    for item in snap["graph"]["graph"]["evidence"]
        .as_object()
        .expect("evidence")
        .values()
    {
        for source in item["source"]
            .as_object()
            .into_iter()
            .flat_map(|s| s.values())
        {
            if let Some(identity) = source["identity"].as_str() {
                let file = identity.rsplit('/').next().unwrap().to_string();
                if file.ends_with(".md") {
                    *counts.entry(file).or_default() += 1;
                }
            }
        }
    }
    counts
}

/// `website/docs/operating.md`, "How a run works", step 7 and "Undoing a run": before a run's
/// first `apply-extraction` a `sqlite` store and `state/` are copied, and kept as a snapshot once
/// that apply commits. `cortex run <name>/seed` is a `cortex run` that applies to a live store,
/// and must be undoable with `cortex restore` as every other run is.
#[test]
fn adversary_run_seed_keeps_a_snapshot_before_it_applies() {
    let w = World::new();
    create_partial(&w, &spec(&w, "t", "0.01", ""));
    let before = snapshots(&w, "t");

    let (code, ran) = w.cortex(&["run", "t/seed"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    assert_eq!(
        snapshots(&w, "t").len(),
        before.len() + 1,
        "`cortex run t/seed` applied to the store and kept no snapshot to restore: before \
         {before:?}, after {:?}",
        snapshots(&w, "t")
    );
}

/// `website/docs/spec-file.md` (`gate`) and `operating.md` step 8: a run that applied something is
/// checked against the spec's `gate`, and one that fails a check fails as `apply-refused`. The
/// stand-in model answers one refused fact per batch, so `facts_refused` max 0 fails every run
/// that applies a batch.
#[test]
fn adversary_run_seed_is_held_to_the_gate() {
    let w = World::new();
    let gate = "gate: {checks: [{measure: facts_refused, max: \"0\"}]}\n";
    create_partial(&w, &spec(&w, "t", "0.01", gate));

    let (code, ran) = w.cortex(&["run", "t/seed"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("apply-refused")),
        "`cortex run t/seed` applied a batch with facts_refused 1 past a gate of max 0: {ran}"
    );
}

/// `adopt` takes an existing store with nothing seeded or extracted (`commands.md`), and its
/// `--seen` refuses `seed`, which no spec lists as a source. `run <name>/seed` on the adopted
/// instance then extracts every seed document again into a store that already holds them.
#[test]
fn adversary_run_seed_on_an_adopted_store_adds_no_second_copy() {
    let w = World::new();
    let (code, created) = create(&w, &spec(&w, "t", "1", ""));
    assert_eq!(
        (code, created["outcome"].as_str()),
        (0, Some("created")),
        "{created}"
    );
    let t = w.home.join("instances/t");
    let store = w.root.join("adopted.sqlite");
    std::fs::copy(t.join("store.sqlite"), &store).unwrap();

    let u = spec(&w, "u", "1", "");
    let text = std::fs::read_to_string(&u)
        .unwrap()
        .replace("18993", "18991");
    std::fs::write(&u, text).unwrap();
    let (code, adopted) = w.cortex(&[
        "adopt",
        "--spec",
        u.to_str().unwrap(),
        "--store",
        store.to_str().unwrap(),
        "--host",
        t.join("host.json").to_str().unwrap(),
        "--no-units",
    ]);
    assert_eq!(
        (code, adopted["outcome"].as_str()),
        (0, Some("adopted")),
        "{adopted}"
    );
    assert_eq!(
        evidence_per_file(&w, "u")
            .values()
            .copied()
            .collect::<Vec<_>>(),
        [1, 1, 1, 1, 1],
        "the adopted store does not hold the seed once"
    );

    // The seed belongs to the adopted store's earlier life: `run u/seed` is refused, saying so.
    let (code, ran) = w.cortex(&["run", "u/seed"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("fetch-failed")),
        "{ran}"
    );
    let reason = ran["detail"]["reason"].as_str().unwrap_or("");
    assert!(
        reason.contains("adopted") && reason.contains("earlier life"),
        "the refusal does not say the seed belongs to the store's earlier life: {ran}"
    );
    assert_eq!(
        evidence_per_file(&w, "u")
            .values()
            .copied()
            .collect::<Vec<_>>(),
        [1, 1, 1, 1, 1],
        "`cortex run u/seed` on an adopted store extracted the seed a second time: {ran}"
    );
}
