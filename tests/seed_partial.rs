//! A seed extraction that stops before every seed document is extracted (issue #28): `create`
//! answers `partial` with the seed's counts and why it stopped, the documents it did not extract are
//! not recorded as seen, and `cortex run <name>/seed` extracts them. A seed that finishes still
//! answers `created`.

mod common;

use std::path::{Path, PathBuf};

use common::{ekr, executable, World};
use serde_json::Value;

/// Five seed files of about 14 700 characters: four fit one 60 000-character batch, the fifth goes
/// into a second, so the seed asks the model twice. The stand-in model costs 0.01 per call.
fn spec(w: &World, name: &str, budget: &str) -> PathBuf {
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
serve: {{view_port: 18995}}
"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    path
}

fn create(w: &World, spec: &Path) -> (i32, Value) {
    w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"])
}

/// The seed documents recorded as seen, by file name.
fn seen(w: &World, name: &str) -> Vec<String> {
    let path = w.home.join(format!("instances/{name}/state/seed.json"));
    let state: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let mut out: Vec<String> = state["documents"]
        .as_object()
        .unwrap()
        .keys()
        .map(|k| k.rsplit('/').next().unwrap().to_string())
        .collect();
    out.sort();
    out
}

#[test]
fn a_seed_stopped_by_its_budget_answers_partial_and_a_rerun_extracts_the_rest() {
    let w = World::new();
    let (code, created) = create(&w, &spec(&w, "t", "0.01"));
    assert_eq!(
        (code, created["outcome"].as_str()),
        (0, Some("partial")),
        "{created}"
    );
    let seed = &created["detail"]["seed"];
    assert_eq!(seed["documents_new"], 5, "{created}");
    assert_eq!(seed["documents_applied"], 4, "{created}");
    assert!(
        seed["stopped"].as_str().unwrap_or("").contains("budget"),
        "{created}"
    );
    assert_eq!(seen(&w, "t"), ["a.md", "b.md", "c.md", "d.md"]);

    let (code, ran) = w.cortex(&["run", "t/seed"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["source_id"], "t/seed", "{ran}");
    assert_eq!(ran["detail"]["documents_new"], 1, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    assert_eq!(seen(&w, "t"), ["a.md", "b.md", "c.md", "d.md", "e.md"]);

    // Everything is extracted: a further run of the seed has nothing new.
    let (code, ran) = w.cortex(&["run", "t/seed"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_new"], 0, "{ran}");
}

#[test]
fn a_seed_whose_model_call_fails_answers_partial_and_a_rerun_extracts_it() {
    let w = World::new();
    let claude = w.bin.join("claude");
    let working = std::fs::read_to_string(&claude).unwrap();
    executable(&claude, "echo 'the model is unavailable' >&2\nexit 1\n");
    let (code, created) = create(&w, &spec(&w, "t", "1"));
    assert_eq!(
        (code, created["outcome"].as_str()),
        (0, Some("partial")),
        "{created}"
    );
    let seed = &created["detail"]["seed"];
    assert!(
        seed["failed"]
            .as_str()
            .unwrap_or("")
            .contains("the model is unavailable"),
        "{created}"
    );
    assert!(
        !w.home.join("instances/t/state/seed.json").exists() || seen(&w, "t").is_empty(),
        "a failed seed recorded documents as seen"
    );

    executable(&claude, &working);
    let (code, ran) = w.cortex(&["run", "t/seed"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 5, "{ran}");
    assert_eq!(seen(&w, "t"), ["a.md", "b.md", "c.md", "d.md", "e.md"]);
}

#[test]
fn a_seed_that_finishes_still_answers_created() {
    let w = World::new();
    let (code, created) = create(&w, &spec(&w, "t", "1"));
    assert_eq!(
        (code, created["outcome"].as_str()),
        (0, Some("created")),
        "{created}"
    );
    assert_eq!(
        created["detail"]["seed"]["documents_applied"], 5,
        "{created}"
    );
    assert_eq!(seen(&w, "t"), ["a.md", "b.md", "c.md", "d.md", "e.md"]);
}
