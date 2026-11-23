//! `cortex run <instance>/<source> --path <dir>` reads a `files` source from a directory named per
//! run, its documents keyed by their paths relative to that directory, so a record read from two
//! run directories is one document (`story:run-path-override`).

use std::path::{Path, PathBuf};

use crate::common::{ekr, World};
use serde_json::Value;

/// One chat message per line; the source keys a record by `id`.
const EXPORT: &str = r#"{"id":"m1","user":"ana","text":"Example Labs ships the Widget engine on Friday."}
"#;

/// The same record with its text changed.
const CHANGED: &str = r#"{"id":"m1","user":"ana","text":"Example Labs ships the Widget engine on Monday."}
"#;

/// A spec with a `files` source of JSON lines (`chat`), a `files` source of whole markdown files
/// (`pages`) and a `web` source (`news`). The `files` sources name a directory that does not
/// exist: with `--path` it is never read.
fn spec(w: &World) -> PathBuf {
    let path = w.root.join("runpath.yaml");
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: runpath
description: Test brain.
ekr: {{version: "0.0.43", bin: "{ekr}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
  - name: chat
    schedule: daily
    settings:
      kind: files
      value:
        paths: [fixed]
        glob: "**/*.jsonl"
        records: {{format: JsonLines, id: id, author: user, text: ["{{text}}"], filters: []}}
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 50, max_chars_per_document: 5000}}
  - name: pages
    schedule: daily
    settings:
      kind: files
      value:
        paths: [fixed]
        glob: "**/*.md"
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 50, max_chars_per_document: 5000}}
  - name: news
    schedule: daily
    settings:
      kind: web
      value:
        connection: conn_test
        input:
          input: search
          value:
            queries: [widget engine]
            policy: {{topic: news, max_results: 3, include_domains: [], exclude_domains: []}}
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
serve: {{view_port: 18994}}
"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    path
}

fn world() -> World {
    let w = World::new();
    let path = spec(&w);
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    w
}

/// A run directory `<name>/work` holding `file` with `content`.
fn work(w: &World, name: &str, file: &str, content: &str) -> PathBuf {
    let dir = w.root.join(name).join("work");
    let path = dir.join(file);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
    dir
}

fn run(w: &World, source: &str, dir: &Path) -> Value {
    let (code, ran) = w.cortex(&[
        "run",
        &format!("runpath/{source}"),
        "--path",
        dir.to_str().unwrap(),
    ]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    ran
}

/// The document keys a source's seen state holds.
fn keys(w: &World, source: &str) -> Vec<String> {
    let path = w
        .home
        .join("instances/runpath/state")
        .join(format!("{source}.json"));
    let state: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    state["documents"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect()
}

/// Every prompt the runs of the instance showed the model.
fn prompts(w: &World) -> String {
    let mut all = String::new();
    let Ok(runs) = std::fs::read_dir(w.home.join("instances/runpath/runs")) else {
        return all;
    };
    for run in runs {
        for batch in std::fs::read_dir(run.unwrap().path()).unwrap() {
            if let Ok(p) = std::fs::read_to_string(batch.unwrap().path().join("prompt.txt")) {
                all.push_str(&p);
            }
        }
    }
    all
}

#[test]
fn a_record_read_from_two_run_directories_is_applied_once_and_again_when_it_changes() {
    let w = world();
    let first = work(&w, "run1", "slices/chat.jsonl", EXPORT);
    let ran = run(&w, "chat", &first);
    assert_eq!(ran["detail"]["documents_new"], 1, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    // The key and the evidence identity are relative to the run directory.
    assert_eq!(keys(&w, "chat"), ["slices/chat.jsonl#m1"]);
    let prompt = prompts(&w);
    assert!(
        prompt.contains("Source: file:slices/chat.jsonl#m1\n"),
        "{prompt}"
    );
    assert!(!prompt.contains("run1"), "{prompt}");

    // The same record under another run directory: counted unchanged, not extracted again.
    let second = work(&w, "run2", "slices/chat.jsonl", EXPORT);
    let (code, counted) = w.cortex(&[
        "run",
        "runpath/chat",
        "--fetch-only",
        "--path",
        second.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "{counted}");
    assert_eq!(counted["outcome"], "fetched", "{counted}");
    assert_eq!(
        (
            counted["detail"]["new"].as_i64(),
            counted["detail"]["unchanged"].as_i64()
        ),
        (Some(0), Some(1)),
        "{counted}"
    );
    let calls = w.lines("claude-calls.log").len();
    let ran = run(&w, "chat", &second);
    assert_eq!(ran["detail"]["documents_new"], 0, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 0, "{ran}");
    assert_eq!(w.lines("claude-calls.log").len(), calls, "no model call");
    assert_eq!(keys(&w, "chat"), ["slices/chat.jsonl#m1"]);

    // The record changed in a third run directory: applied again, under the same key.
    let third = work(&w, "run3", "slices/chat.jsonl", CHANGED);
    let ran = run(&w, "chat", &third);
    assert_eq!(ran["detail"]["documents_new"], 1, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    assert_eq!(keys(&w, "chat"), ["slices/chat.jsonl#m1"]);
    assert_eq!(w.lines("claude-calls.log").len(), calls + 1);
}

#[test]
fn a_whole_file_is_keyed_by_its_path_relative_to_the_run_directory() {
    let w = world();
    let first = work(
        &w,
        "a",
        "notes/widget.md",
        "Example Labs develops the Widget engine.\n",
    );
    let ran = run(&w, "pages", &first);
    assert_eq!(ran["detail"]["documents_new"], 1, "{ran}");
    assert_eq!(keys(&w, "pages"), ["notes/widget.md"]);
    assert!(prompts(&w).contains("Source: file:notes/widget.md\n"));
    let second = work(
        &w,
        "b",
        "notes/widget.md",
        "Example Labs develops the Widget engine.\n",
    );
    let ran = run(&w, "pages", &second);
    assert_eq!(ran["detail"]["documents_new"], 0, "{ran}");
}

#[test]
fn path_on_a_source_that_is_not_files_or_on_the_seed_is_refused_and_writes_nothing() {
    let w = world();
    let dir = work(&w, "run1", "slices/chat.jsonl", EXPORT);
    let dir = dir.to_str().unwrap();
    for args in [
        vec!["run", "runpath/news", "--path", dir],
        vec!["run", "runpath/news", "--record-failure", "--path", dir],
        vec!["run", "runpath/news", "--fetch-only", "--path", dir],
        vec!["run", "runpath/seed", "--path", dir],
        vec!["run", "runpath/seed", "--fetch-only", "--path", dir],
    ] {
        let (code, refused) = w.cortex(&args);
        assert_eq!(code, 1, "{args:?}: {refused}");
        assert_eq!(refused["outcome"], "path-refused", "{args:?}: {refused}");
    }
    assert!(w.lines("invocations.log").is_empty());
    assert!(w.lines("claude-calls.log").is_empty());
    let instance = w.home.join("instances/runpath");
    // No seen state, no status (where a source's last run is kept), no metric.
    assert!(!instance.join("state/news.json").exists());
    assert!(!instance.join("status/news.json").exists());
    assert!(!instance.join("metrics.prom").exists());
}

#[test]
fn a_missing_run_directory_fails_the_fetch_and_names_it() {
    let w = world();
    let missing = w.root.join("gone/work");
    let missing = missing.to_str().unwrap();
    let (code, failed) = w.cortex(&["run", "runpath/chat", "--path", missing]);
    assert_eq!(
        (code, failed["outcome"].as_str()),
        (1, Some("fetch-failed")),
        "{failed}"
    );
    let reason = failed["detail"]["reason"].as_str().unwrap();
    assert!(reason.contains(missing), "{reason}");
    let (code, failed) = w.cortex(&["run", "runpath/chat", "--fetch-only", "--path", missing]);
    assert_eq!(
        (code, failed["outcome"].as_str()),
        (1, Some("fetch-failed")),
        "{failed}"
    );
    assert!(
        failed["detail"]["reason"]
            .as_str()
            .unwrap()
            .contains(missing),
        "{failed}"
    );
    assert!(w.lines("claude-calls.log").is_empty());
}
