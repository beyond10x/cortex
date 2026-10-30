//! An unchanged `dropped: Supersede` source begins no stage and takes no snapshot
//! (`story:idle-supersede-source-cheap`): once a run has ended what its listing dropped, a later
//! run whose listing, unsettled records and held prefixes are the same selects nothing and ends
//! nothing, so it reads nothing of the store. A listing that changed is compared with the store as
//! before. A stand-in `ekr` in front of the real one logs every command; the stand-in `claude`
//! fails, and is never called.

mod common;

use std::path::PathBuf;

use common::{ekr, executable, World};
use serde_json::{json, Value};

const CLAUDE: &str = r#"echo call >> "@ROOT@/claude-calls.log"
echo '{"type":"result","subtype":"error","is_error":true}'
exit 1
"#;

/// The stand-in `ekr`: logs its arguments, one command per line, to `ekr-argv.log`, then runs the
/// real `ekr`.
const EKR: &str = r#"printf '%s\n' "$*" >> "@ROOT@/ekr-argv.log"
exec "@EKR@" "$@"
"#;

/// Writes `registry/people.json`, one record per `(id, name, team)`.
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

/// A world with instance `s`: one structured source, `registry`, over `registry/*.json`, with
/// `dropped: Supersede`, through the logging `ekr`.
fn world() -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(&w.bin.join("claude"), &CLAUDE.replace("@ROOT@", &root));
    let stand_in = w.bin.join("ekr");
    executable(
        &stand_in,
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
        dropped: Supersede
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
serve: {{view_port: 18993}}
"#,
            bin = stand_in.display()
        ),
    )
    .unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    w
}

fn argv_log(w: &World) -> PathBuf {
    w.root.join("ekr-argv.log")
}

/// `cortex run s/registry`, which must end `ran`, and the `ekr` commands it issued.
fn run(w: &World) -> (Value, Vec<String>) {
    let _ = std::fs::remove_file(argv_log(w));
    let (code, ran) = w.cortex(&["run", "s/registry"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    (ran, w.lines("ekr-argv.log"))
}

fn count(commands: &[String], prefix: &str) -> usize {
    commands.iter().filter(|c| c.starts_with(prefix)).count()
}

/// Acceptance 1: a run of a `dropped: Supersede` source whose listing did not change since its
/// last successful end, with nothing selected, makes no `stage begin` and no `snapshot` call.
#[test]
fn an_unchanged_supersede_listing_with_nothing_selected_reads_nothing_of_the_store() {
    let w = world();
    let (first, commands) = run(&w);
    assert_eq!(first["detail"]["documents_applied"], 2, "{first}");
    assert_eq!(count(&commands, "snapshot"), 1, "{commands:#?}");

    let (second, commands) = run(&w);
    assert_eq!(second["detail"]["documents_new"], 0, "{second}");
    assert_eq!(count(&commands, "snapshot"), 0, "{commands:#?}");
    assert_eq!(count(&commands, "stage begin"), 0, "{commands:#?}");
    assert!(
        w.lines("claude-calls.log").is_empty(),
        "the model was called"
    );
}

/// Acceptance 2's guard on the shortcut: a listing that dropped a record after an idle run is
/// compared with the store again, and the record's value is retracted.
#[test]
fn a_listing_that_dropped_a_record_after_an_idle_run_still_ends_its_value() {
    let w = world();
    run(&w);
    let (_, commands) = run(&w);
    assert_eq!(count(&commands, "snapshot"), 0, "{commands:#?}");

    registry(&w, &[("P-1", "Ada Lovelace", "Core")]);
    let (ran, commands) = run(&w);
    assert_eq!(ran["detail"]["documents_new"], 0, "{ran}");
    assert_eq!(count(&commands, "snapshot"), 1, "{commands:#?}");
    assert_eq!(
        ran["detail"]["retracted"], 1,
        "Grace's team is retracted: {ran}"
    );

    // The ended listing is the new one: the next run with it reads nothing again.
    let (_, commands) = run(&w);
    assert_eq!(count(&commands, "snapshot"), 0, "{commands:#?}");
}
