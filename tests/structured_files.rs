//! A `structured` source reads its records from JSON files, and with `dropped: Supersede` a value
//! the source no longer lists stops being active (`story:structured-from-files-and-drops`): a
//! changed value is superseded by the new one, and a value with no replacement — a record the
//! source no longer lists — is retracted, since EKR 0.0.31 supersedes an assertion only by a
//! replacement. `Keep`, the default, leaves every earlier value active. The stand-in `claude`
//! fails when called, and is never called.

mod common;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

use common::{ekr, executable, World};
use serde_json::{json, Value};

/// A stand-in `claude` that logs the call and fails.
const CLAUDE: &str = r#"echo call >> "@ROOT@/claude-calls.log"
echo '{"type":"result","subtype":"error","is_error":true}'
exit 1
"#;

fn world() -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(&w.bin.join("claude"), &CLAUDE.replace("@ROOT@", &root));
    std::fs::create_dir_all(w.root.join("registry")).unwrap();
    w
}

/// Writes the registry file `registry/people.json`: `{"people": [...]}`, one record per
/// `(id, name, team)`.
fn registry(w: &World, people: &[(&str, &str, &str)]) {
    let records: Vec<Value> = people
        .iter()
        .map(|(id, name, team)| json!({"id": id, "name": name, "team": team}))
        .collect();
    std::fs::write(
        w.root.join("registry/people.json"),
        json!({"people": records}).to_string(),
    )
    .unwrap();
}

/// Creates instance `name` with one structured source, `registry`, reading `registry/*.json`, and
/// `dropped` as given (absent when `None`).
fn create(w: &World, name: &str, dropped: Option<&str>) {
    create_with(w, name, dropped, "[]");
}

/// As `create`, with `relations` as the mapping's relations.
fn create_with(w: &World, name: &str, dropped: Option<&str>, relations: &str) {
    let dropped = dropped.map_or(String::new(), |d| format!("        dropped: {d}\n"));
    let path = w.root.join(format!("{name}.yaml"));
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: {name}
description: Test brain.
ekr: {{version: "0.0.32", bin: "{ekr}"}}
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
          relations: {relations}
{dropped}    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
serve: {{view_port: 18994}}
"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
}

fn instance(w: &World, name: &str) -> PathBuf {
    w.home.join("instances").join(name)
}

fn read(w: &World, name: &str, args: &[&str]) -> Value {
    let dir = instance(w, name);
    let out = Command::new(ekr())
        .args(args)
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
    serde_json::from_slice(&out.stdout).unwrap()
}

/// One `team` assertion of the store, as `ekr snapshot` holds it.
#[derive(Debug, Clone)]
struct Team {
    id: String,
    value: String,
    /// `Active`, `Superseded` or `Retracted`.
    lifecycle: String,
    /// The assertion that superseded it.
    by: Option<String>,
}

/// Every `team` assertion of the store, by its subject node's name.
fn teams(w: &World, name: &str) -> BTreeMap<String, Vec<Team>> {
    let snapshot = read(w, name, &["snapshot"]);
    let ontology = read(w, name, &["ontology"]);
    let team = ontology["node_types"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|t| t["properties"].as_array().cloned().unwrap_or_default())
        .find(|p| p["name"] == "team")
        .expect("the ontology declares team")["id"]
        .clone();
    let graph = &snapshot["graph"]["graph"];
    let mut out: BTreeMap<String, Vec<Team>> = BTreeMap::new();
    for a in graph["assertions"].as_object().unwrap().values() {
        if a["predicate"]["Property"] != team {
            continue;
        }
        let node = a["subject"]["Node"].as_str().unwrap();
        let subject = graph["nodes"][node]["canonical_name"]
            .as_str()
            .unwrap()
            .to_string();
        let (lifecycle, by) = match &a["lifecycle"] {
            Value::String(s) => (s.clone(), None),
            Value::Object(o) => {
                let (kind, body) = o.iter().next().unwrap();
                (kind.clone(), body["by"].as_str().map(str::to_string))
            }
            other => panic!("lifecycle {other}"),
        };
        out.entry(subject).or_default().push(Team {
            id: a["id"].as_str().unwrap().to_string(),
            value: a["object"]["Value"]["value"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            lifecycle,
            by,
        });
    }
    out
}

fn active(teams: &[Team]) -> Vec<&str> {
    teams
        .iter()
        .filter(|t| t.lifecycle == "Active")
        .map(|t| t.value.as_str())
        .collect()
}

const ADA: &str = "Ada Lovelace (files:registry:*.json:P-1)";
const GRACE: &str = "Grace Hopper (files:registry:*.json:P-2)";
const LINUS: &str = "Linus Example (files:registry:*.json:P-3)";

/// The first registry, and the second: Ada moved from Core to Ops, Grace is gone, Linus is as he
/// was.
fn before(w: &World) {
    registry(
        w,
        &[
            ("P-1", "Ada Lovelace", "Core"),
            ("P-2", "Grace Hopper", "Core"),
            ("P-3", "Linus Example", "Ops"),
        ],
    );
}

fn after(w: &World) {
    registry(
        w,
        &[
            ("P-1", "Ada Lovelace", "Ops"),
            ("P-3", "Linus Example", "Ops"),
        ],
    );
}

fn run(w: &World, name: &str) -> Value {
    let (code, ran) = w.cortex(&["run", &format!("{name}/registry")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    ran
}

#[test]
fn a_registry_file_imports_and_a_second_run_supersedes_a_changed_team_and_ends_a_removed_person() {
    let w = world();
    before(&w);
    create(&w, "s", Some("Supersede"));

    let ran = run(&w, "s");
    assert_eq!(ran["detail"]["documents_applied"], 3, "{ran}");
    assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");
    let first = teams(&w, "s");
    assert_eq!(first.len(), 3, "three people: {first:?}");
    assert_eq!(active(&first[ADA]), ["Core"], "{first:?}");
    assert_eq!(active(&first[GRACE]), ["Core"], "{first:?}");
    assert_eq!(active(&first[LINUS]), ["Ops"], "{first:?}");

    after(&w);
    let ran = run(&w, "s");
    assert_eq!(
        ran["detail"]["documents_applied"], 1,
        "only Ada changed: {ran}"
    );
    assert!(ran["detail"]["stopped"].is_null(), "{ran}");
    let second = teams(&w, "s");

    // Ada: the new team is the active assertion, and the old one is superseded by it.
    let ada = &second[ADA];
    assert_eq!(active(ada), ["Ops"], "{ada:?}");
    let ops = ada.iter().find(|t| t.value == "Ops").unwrap();
    let core = ada.iter().find(|t| t.value == "Core").unwrap();
    assert_eq!(core.lifecycle, "Superseded", "{ada:?}");
    assert_eq!(core.by.as_deref(), Some(ops.id.as_str()), "{ada:?}");

    // Grace: no longer listed, so nothing of hers is active; with no replacement, her value is
    // retracted.
    let grace = &second[GRACE];
    assert!(active(grace).is_empty(), "{grace:?}");
    assert!(
        grace.iter().all(|t| t.lifecycle == "Retracted"),
        "{grace:?}"
    );

    // Linus: unchanged, his one assertion still active.
    assert_eq!(second[LINUS].len(), 1, "{second:?}");
    assert_eq!(active(&second[LINUS]), ["Ops"], "{second:?}");

    // A third run with nothing changed ends nothing more.
    let ran = run(&w, "s");
    assert_eq!(ran["detail"]["documents_applied"], 0, "{ran}");
    let third = teams(&w, "s");
    for who in [ADA, GRACE, LINUS] {
        let lifecycles = |t: &BTreeMap<String, Vec<Team>>| -> Vec<String> {
            t[who].iter().map(|a| a.lifecycle.clone()).collect()
        };
        assert_eq!(lifecycles(&third), lifecycles(&second), "{who}");
    }
    assert!(
        w.lines("claude-calls.log").is_empty(),
        "the model was called"
    );
}

#[test]
fn without_dropped_every_earlier_value_stays_active() {
    let w = world();
    before(&w);
    create(&w, "k", None);
    run(&w, "k");
    after(&w);
    run(&w, "k");
    let teams = teams(&w, "k");
    let mut ada = active(&teams[ADA]);
    ada.sort();
    assert_eq!(ada, ["Core", "Ops"], "{teams:?}");
    assert_eq!(active(&teams[GRACE]), ["Core"], "{teams:?}");
    assert_eq!(active(&teams[LINUS]), ["Ops"], "{teams:?}");
}

#[test]
fn a_registry_file_that_is_not_json_fails_the_run_and_ends_nothing() {
    let w = world();
    before(&w);
    create(&w, "b", Some("Supersede"));
    run(&w, "b");
    let first = teams(&w, "b");
    std::fs::write(w.root.join("registry/people.json"), "{\"people\": [").unwrap();
    let (code, ran) = w.cortex(&["run", "b/registry"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("fetch-failed")),
        "{ran}"
    );
    assert!(
        ran["detail"]["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("people.json"),
        "{ran}"
    );
    let after = teams(&w, "b");
    for who in [ADA, GRACE, LINUS] {
        assert_eq!(active(&after[who]), active(&first[who]), "{who}");
    }
}

/// The `MEMBER_OF` relations of the store: `(person, team, lifecycle)` per assertion, and
/// `(person, team)` per edge.
#[allow(clippy::type_complexity)]
fn memberships(w: &World, name: &str) -> (Vec<(String, String, String)>, Vec<(String, String)>) {
    let snapshot = read(w, name, &["snapshot"]);
    let ontology = read(w, name, &["ontology"]);
    let member_of = ontology["edge_types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "MEMBER_OF")
        .expect("the ontology declares MEMBER_OF")["id"]
        .clone();
    let graph = &snapshot["graph"]["graph"];
    let named = |id: &Value| {
        graph["nodes"][id.as_str().unwrap()]["canonical_name"]
            .as_str()
            .unwrap()
            .to_string()
    };
    let mut assertions = Vec::new();
    for a in graph["assertions"].as_object().unwrap().values() {
        if a["predicate"]["Relation"] != member_of {
            continue;
        }
        let lifecycle = match &a["lifecycle"] {
            Value::String(s) => s.clone(),
            Value::Object(o) => o.keys().next().unwrap().clone(),
            other => panic!("lifecycle {other}"),
        };
        assertions.push((
            named(&a["subject"]["Node"]),
            named(&a["object"]["Node"]),
            lifecycle,
        ));
    }
    let mut edges: Vec<(String, String)> = graph["edges"]
        .as_object()
        .unwrap()
        .values()
        .filter(|e| e["type_id"] == member_of)
        .map(|e| (named(&e["source"]), named(&e["target"])))
        .collect();
    assertions.sort();
    edges.sort();
    (assertions, edges)
}

#[test]
fn a_relation_the_source_no_longer_lists_is_retracted_and_its_edge_removed() {
    let w = world();
    before(&w);
    create_with(
        &w,
        "r",
        Some("Supersede"),
        "\n            - {relation: MEMBER_OF, target_type: Team, target_name: \"$.team\"}",
    );
    run(&w, "r");
    let (_, edges) = memberships(&w, "r");
    assert_eq!(edges.len(), 3, "{edges:?}");

    after(&w);
    let ran = run(&w, "r");
    let (assertions, edges) = memberships(&w, "r");
    let s = |t: &str| t.to_string();
    assert_eq!(
        assertions,
        [
            (s(ADA), s("Core"), s("Retracted")),
            (s(ADA), s("Ops"), s("Active")),
            (s(GRACE), s("Core"), s("Retracted")),
            (s(LINUS), s("Ops"), s("Active")),
        ],
        "{ran}"
    );
    assert_eq!(edges, [(s(ADA), s("Ops")), (s(LINUS), s("Ops"))], "{ran}");
    // Ada's team is superseded; Ada's and Grace's memberships of Core, and Grace's team, retracted.
    assert_eq!(ran["detail"]["superseded"], 1, "{ran}");
    assert_eq!(ran["detail"]["retracted"], 3, "{ran}");
}

// ---- adversary, wave 20261006f unit a, pass 1 ----

/// The `Active` team values of every person, by name.
fn active_teams(w: &World, name: &str) -> BTreeMap<String, Vec<String>> {
    teams(w, name)
        .into_iter()
        .map(|(who, t)| {
            let mut v: Vec<String> = active(&t).into_iter().map(str::to_string).collect();
            v.sort();
            (who, v)
        })
        .collect()
}

/// The registry file is moved away (an export renamed, a mount not yet there, a glob edited): the
/// source's glob matches no file at all. A fetch that read nothing is not a source that lists
/// nobody, so no value is ended.
#[test]
fn adv_a_a_glob_that_matches_no_file_ends_nothing() {
    let w = world();
    before(&w);
    create(&w, "g", Some("Supersede"));
    run(&w, "g");
    let first = active_teams(&w, "g");
    std::fs::rename(
        w.root.join("registry/people.json"),
        w.root.join("registry/people.json.moved"),
    )
    .unwrap();
    let (code, ran) = w.cortex(&["run", "g/registry"]);
    let after = active_teams(&w, "g");
    assert_eq!(after, first, "exit {code}: {ran}");
    // Correction pass 1: the fetch fails, naming the root and the glob.
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("fetch-failed")),
        "{ran}"
    );
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("registry") && reason.contains("no file matches glob \"*.json\""),
        "{ran}"
    );
}

/// Correction pass 1: a file whose list is empty lists no record, which is read as a list that
/// failed: a source cannot drop its last record, and nothing is ended.
#[test]
fn an_empty_list_of_records_ends_nothing() {
    let w = world();
    before(&w);
    create(&w, "e", Some("Supersede"));
    run(&w, "e");
    let first = active_teams(&w, "e");
    registry(&w, &[]);
    let ran = run(&w, "e");
    assert!(ran["detail"]["retracted"].is_null(), "{ran}");
    assert_eq!(active_teams(&w, "e"), first, "{ran}");
}

/// Correction pass 1: two `files` sources with one glob under different roots are two record
/// sets, so each keeps what the other lists, and the same id under each root is two nodes.
#[test]
fn two_file_sources_with_one_glob_under_different_roots_do_not_end_each_others_values() {
    let w = world();
    for (dir, name, team) in [
        ("east", "Ada Lovelace", "Core"),
        ("west", "Bob Example", "Ops"),
    ] {
        std::fs::create_dir_all(w.root.join(dir)).unwrap();
        std::fs::write(
            w.root.join(dir).join("people.json"),
            json!({"people": [{"id": "P-1", "name": name, "team": team}]}).to_string(),
        )
        .unwrap();
    }
    let source = |name: &str| {
        format!(
            r#"  - name: {name}
    schedule: daily
    settings:
      kind: structured
      value:
        input:
          from: files
          value: {{paths: [{name}], glob: "*.json"}}
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
"#
        )
    };
    let path = w.root.join("two.yaml");
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: two
description: Test brain.
ekr: {{version: "0.0.32", bin: "{ekr}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
{east}{west}serve: {{view_port: 18993}}
"#,
            ekr = ekr().display(),
            east = source("east"),
            west = source("west"),
        ),
    )
    .unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    for run in ["two/east", "two/west", "two/east", "two/west"] {
        let (code, ran) = w.cortex(&["run", run]);
        assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
        assert!(ran["detail"]["retracted"].is_null(), "{run}: {ran}");
    }
    let teams = active_teams(&w, "two");
    assert_eq!(
        teams,
        BTreeMap::from([
            (
                "Ada Lovelace (files:east:*.json:P-1)".to_string(),
                vec!["Core".to_string()]
            ),
            (
                "Bob Example (files:west:*.json:P-1)".to_string(),
                vec!["Ops".to_string()]
            ),
        ])
    );
}

/// Ada moves Core -> Ops -> Core: after the third run her team is Core, active.
#[test]
fn adv_a_a_value_that_changes_back_is_active_again() {
    let w = world();
    before(&w);
    create(&w, "f", Some("Supersede"));
    run(&w, "f");
    after(&w);
    run(&w, "f");
    registry(
        &w,
        &[
            ("P-1", "Ada Lovelace", "Core"),
            ("P-3", "Linus Example", "Ops"),
        ],
    );
    let ran = run(&w, "f");
    let t = teams(&w, "f");
    assert_eq!(active(&t[ADA]), ["Core"], "{ran} {t:?}");
}

/// The spec: "a record whose current text the run did not apply (held back ..., beyond
/// `max_documents_per_run`, or rejected) keeps its values". A record whose mapped values are over
/// the evidence payload bound is not applied (it is named in `skipped`), so its earlier values
/// stay.
#[test]
fn adv_a_a_record_not_applied_as_too_large_keeps_its_values() {
    let w = world();
    before(&w);
    create(&w, "l", Some("Supersede"));
    run(&w, "l");
    let huge = "X".repeat(20_000);
    registry(
        &w,
        &[
            ("P-1", "Ada Lovelace", &huge),
            ("P-2", "Grace Hopper", "Core"),
            ("P-3", "Linus Example", "Ops"),
        ],
    );
    let ran = run(&w, "l");
    assert_eq!(ran["detail"]["documents_applied"], 0, "{ran}");
    let t = teams(&w, "l");
    assert_eq!(active(&t[ADA]), ["Core"], "{ran} {t:?}");
    // Correction pass 1: the record is now seen, and still too large; a later run keeps its
    // values too.
    let ran = run(&w, "l");
    let t = teams(&w, "l");
    assert_eq!(active(&t[ADA]), ["Core"], "{ran} {t:?}");
}

/// A team written with surrounding space is a value the source lists: it stays active through a
/// run that changes nothing.
#[test]
fn adv_a_a_value_with_surrounding_space_stays_active() {
    let w = world();
    registry(&w, &[("P-1", "Ada Lovelace", " Core ")]);
    create(&w, "sp", Some("Supersede"));
    let ran1 = run(&w, "sp");
    let first = active_teams(&w, "sp");
    let ran2 = run(&w, "sp");
    let second = active_teams(&w, "sp");
    assert_eq!(first[ADA].len(), 1, "{ran1} {first:?}");
    assert_eq!(second, first, "{ran2}");
}

/// A `kind: connectors` source reading the same adapter and operation keys its documents
/// `<adapter>:<operation>:<id>` and cites them `record:<key>` (`src/sources.rs` `fetch_records`,
/// `evidence::identity`), exactly as a structured source does, so a `dropped: Supersede`
/// structured source would claim and end what the model extracted from those records.
/// Correction pass 1: `create` refuses that spec, naming both sources, and creates nothing.
#[test]
fn adv_a_a_supersede_beside_a_connectors_source_on_the_same_operation_is_refused() {
    let w = world();
    let path = w.root.join("both.yaml");
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: both
description: Test brain.
ekr: {{version: "0.0.32", bin: "{ekr}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
  - name: notes
    schedule: daily
    settings:
      kind: connectors
      value: {{adapter: directory, connection: conn_test, operation: people.list, inputs: [{{}}], records: "$.people", id: "$.id", text: ["{{name}}"]}}
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
  - name: people
    schedule: daily
    settings:
      kind: structured
      value:
        input:
          from: connectors
          value: {{adapter: directory, connection: conn_test, operation: people.list, inputs: [{{}}]}}
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
serve: {{view_port: 18992}}
"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["create", "--spec", path.to_str().unwrap(), "--no-units"])
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(2), "{stderr}");
    for part in [
        "sources.people: dropped: Supersede",
        "directory:people.list",
        "\"notes\"",
    ] {
        assert!(stderr.contains(part), "{part} in {stderr}");
    }
    assert!(!instance(&w, "both").exists(), "{stderr}");

    // With `dropped: Keep` the same pair is accepted.
    executable(
        &w.bin.join("connectors"),
        r#"case "$*" in
  *"connections list"*) echo '{"ok":true,"result":{"connections":[{"adapter":"directory","connection":"conn_test","state":"ready","revision":"rev1"}]}}' ;;
  *"operations describe"*) echo '{"ok":true,"result":{"schema":"s","revision":"r"}}' ;;
  *) echo '{"ok":false}'; exit 2 ;;
esac
"#,
    );
    let keep = std::fs::read_to_string(&path)
        .unwrap()
        .replace("dropped: Supersede", "dropped: Keep");
    std::fs::write(&path, keep).unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
}
