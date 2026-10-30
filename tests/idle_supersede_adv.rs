//! Adversarial cases for `story:idle-supersede-source-cheap`: the remembered fingerprint
//! (`SeenState::ended_listing`) says the store holds no value of this source the listing dropped.
//! It stays in the state file across runs that wrote values of this source without ending the
//! dropped ones afterwards, so a later idle run with the remembered listing skips an end that a
//! run before this change made.
//!
//! A case marked `#[ignore = "adversary: ..."]` fails on the unit's head and names the finding.

mod common;

use std::path::PathBuf;

use common::{ekr, executable, World};
use serde_json::{json, Value};

const CLAUDE: &str = r#"echo call >> "@ROOT@/claude-calls.log"
echo '{"type":"result","subtype":"error","is_error":true}'
exit 1
"#;

/// The stand-in `ekr`: logs its arguments to `ekr-argv.log`, refuses `snapshot` while the file
/// `fail-snapshot` exists (a store read that fails or is cut off, as on a large store), and runs
/// the real `ekr` otherwise.
const EKR: &str = r#"printf '%s\n' "$*" >> "@ROOT@/ekr-argv.log"
if [ "$1" = snapshot ] && [ -e "@ROOT@/fail-snapshot" ]; then
  echo "ekr: the store could not be read" >&2
  exit 1
fi
exec "@EKR@" "$@"
"#;

fn registry(w: &World, people: &[(&str, &str, &str)]) {
    let records: Vec<Value> = people
        .iter()
        .map(|(id, name, team)| json!({"id": id, "name": name, "team": team}))
        .collect();
    std::fs::write(
        w.root.join("registry/people.json"),
        json!({ "people": records }).to_string(),
    )
    .unwrap();
}

/// Writes instance `s`'s spec with `dropped`, and answers its path.
fn spec(w: &World, dropped: &str) -> PathBuf {
    let path = w.root.join("s.yaml");
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: s
description: Test brain.
ekr: {{version: "0.0.34", bin: "{bin}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
  - name: registry
    schedule: daily
    settings:
      kind: structured
      value:
        input:
          from: files
          value: {{paths: [registry], glob: "*.json"}}
        records: "$.people"
        mapping:
          node_type: Person
          id: "$.id"
          name: "$.name"
          aliases: []
          properties:
            - {{property: team, path: "$.team"}}
          relations: []
        dropped: {dropped}
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
serve: {{view_port: 18995}}
"#,
            bin = w.bin.join("ekr").display()
        ),
    )
    .unwrap();
    path
}

fn world() -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(&w.bin.join("claude"), &CLAUDE.replace("@ROOT@", &root));
    executable(
        &w.bin.join("ekr"),
        &[
            common::EKR_SESSION_SHIM,
            &EKR.replace("@ROOT@", &root)
                .replace("@EKR@", &ekr().display().to_string()),
        ]
        .concat(),
    );
    std::fs::create_dir_all(w.root.join("registry")).unwrap();
    registry(
        &w,
        &[
            ("P-1", "Ada Lovelace", "Core"),
            ("P-2", "Grace Hopper", "Core"),
        ],
    );
    let path = spec(&w, "Supersede");
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    w
}

/// The active `team` values the store holds of the person whose canonical name starts with
/// `name`, read with the real `ekr`.
fn teams(w: &World, name: &str) -> Vec<String> {
    let dir = w.home.join("instances/s");
    let out = std::process::Command::new(ekr())
        .arg("snapshot")
        .env("EKR_HOST", dir.join("host.json"))
        .env("EKR_BACKEND", "sqlite")
        .env("EKR_STORE", dir.join("store.sqlite"))
        .output()
        .unwrap();
    assert!(out.status.success());
    let snapshot: Value = serde_json::from_slice(&out.stdout).unwrap();
    let graph = &snapshot["graph"]["graph"];
    let node: Vec<&str> = graph["nodes"]
        .as_object()
        .unwrap()
        .values()
        .filter(|n| {
            n["canonical_name"]
                .as_str()
                .is_some_and(|c| c.starts_with(name))
        })
        .filter_map(|n| n["id"].as_str())
        .collect();
    graph["assertions"]
        .as_object()
        .unwrap()
        .values()
        .filter(|a| a["lifecycle"] == "Active")
        .filter(|a| {
            a["subject"]["Node"]
                .as_str()
                .is_some_and(|s| node.contains(&s))
        })
        .filter_map(|a| a["object"]["Value"]["value"].as_str().map(str::to_string))
        .collect()
}

fn run(w: &World) -> (i32, Value) {
    w.cortex(&["run", "s/registry"])
}

fn ran(w: &World) -> Value {
    let (code, ran) = run(w);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    ran
}

/// A run that applied a new record and then failed to read the store for its ends keeps the
/// fingerprint of the run before it. Once the record is gone from the source again, the listing
/// is the remembered one, and the idle run skips the end: the record's value stays active,
/// although every run before this change (and any run whose listing differs) ends it.
#[test]
fn a_record_applied_by_a_run_that_failed_before_its_ends_is_still_ended_once_dropped() {
    let w = world();
    ran(&w);

    // P-3 appears; the run applies it, then its store read for the ends fails.
    registry(
        &w,
        &[
            ("P-1", "Ada Lovelace", "Core"),
            ("P-2", "Grace Hopper", "Core"),
            ("P-3", "Edsger Dijkstra", "Core"),
        ],
    );
    std::fs::write(w.root.join("fail-snapshot"), "").unwrap();
    let (code, failed) = run(&w);
    assert_ne!(code, 0, "the run's ends could not read the store: {failed}");
    std::fs::remove_file(w.root.join("fail-snapshot")).unwrap();
    assert_eq!(teams(&w, "Edsger Dijkstra"), ["Core"], "P-3 was applied");

    // P-3 is gone from the source again: its value is dropped and ends.
    registry(
        &w,
        &[
            ("P-1", "Ada Lovelace", "Core"),
            ("P-2", "Grace Hopper", "Core"),
        ],
    );
    let ran = ran(&w);
    assert_eq!(ran["detail"]["documents_new"], 0, "{ran}");
    assert_eq!(
        ran["detail"]["retracted"], 1,
        "P-3's team, which the store holds and the source no longer lists, is retracted: {ran}"
    );
    assert!(teams(&w, "Edsger Dijkstra").is_empty(), "P-3's team ended");
}

/// The same through a spec change: while the source is `dropped: Keep`, no run computes the
/// fingerprint and none forgets it. A record added and removed again in that time leaves its
/// value in the store; back on `Supersede`, the idle run with the remembered listing skips the end.
#[test]
fn a_record_applied_while_the_source_kept_dropped_values_is_ended_back_on_supersede() {
    let w = world();
    ran(&w);

    let keep = spec(&w, "Keep");
    let (code, updated) = w.cortex(&["update", "s", "--spec", keep.to_str().unwrap()]);
    assert_eq!(code, 0, "{updated}");
    registry(
        &w,
        &[
            ("P-1", "Ada Lovelace", "Core"),
            ("P-2", "Grace Hopper", "Core"),
            ("P-3", "Edsger Dijkstra", "Core"),
        ],
    );
    let applied = ran(&w);
    assert_eq!(applied["detail"]["documents_applied"], 1, "{applied}");
    assert_eq!(teams(&w, "Edsger Dijkstra"), ["Core"], "P-3 was applied");
    registry(
        &w,
        &[
            ("P-1", "Ada Lovelace", "Core"),
            ("P-2", "Grace Hopper", "Core"),
        ],
    );
    ran(&w);

    let supersede = spec(&w, "Supersede");
    let (code, updated) = w.cortex(&["update", "s", "--spec", supersede.to_str().unwrap()]);
    assert_eq!(code, 0, "{updated}");
    let ran = ran(&w);
    assert_eq!(ran["detail"]["documents_new"], 0, "{ran}");
    assert_eq!(
        ran["detail"]["retracted"], 1,
        "P-3's team, which the source no longer lists, is retracted on the first Supersede run: \
         {ran}"
    );
}
