//! Adversary pass 1 on `story:structured-children`: a structured source whose children are called
//! once per parent record. Each case drives the `cortex` binary against a real `ekr` with a
//! stand-in `connectors` and asserts what the documentation (`website/docs/spec-file.md`, `kind:
//! structured`, **Children**) or the story's acceptance says.

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::{ekr, executable, World};
use serde_json::{json, Value};

/// A stand-in `connectors` with one ready `forge` connection. `projects.list` answers
/// `projects.json`; `tags.list` and `events.list` answer `tags-<project>.json` and
/// `events-<project>.json`, the project taken from the input's `"project"`. An operation answers
/// `forbidden` for a project whose file `forbidden-<operation>-<project>` exists.
const CONNECTORS: &str = r#"R="@ROOT@"
P=$(printf '%s\n' "$*" | sed -n 's/.*"project":"\([^"]*\)".*/\1/p')
answer() { printf '{"ok":true,"result":{"adapter":"forge","operation":"%s","revision":"r","result":%s}}\n' "$1" "$(cat "$2")"; }
child() {
  echo "$1 $P" >> "$R/invocations.log"
  if [ -e "$R/forbidden-$1-$P" ]; then echo '{"ok":false,"error":{"code":"failure","data":{"code":"forbidden","stage":"execution"}}}'; exit 1; fi
  answer "$1" "$R/$2-$P.json"
}
case "$*" in
  *"connections list"*) echo '{"ok":true,"result":{"connections":[{"adapter":"forge","connection":"conn_test","state":"ready","revision":"rev1"}]}}' ;;
  *"connections revalidate"*) echo '{"ok":true,"result":{}}' ;;
  *"operations describe"*) echo '{"ok":true,"result":{"schema":"s","revision":"r"}}' ;;
  *"--operation projects.list"*) echo "projects.list" >> "$R/invocations.log"; answer projects.list "$R/projects.json" ;;
  *"--operation tags.list"*) child tags.list tags ;;
  *"--operation events.list"*) child events.list events ;;
  *) echo '{"ok":false}'; exit 2 ;;
esac
"#;

/// A stand-in `claude` that logs the call and fails: a structured source never calls it.
const CLAUDE: &str = r#"echo call >> "@ROOT@/claude-calls.log"
echo '{"type":"result","subtype":"error","is_error":true}'
exit 1
"#;

/// 2026-01-02T00:00:00Z in milliseconds, and one day.
const JAN_2: i64 = 1_767_312_000_000;
const DAY: i64 = 86_400_000;

const PROJECT: &str = "forge:projects.list:";
const TAG: &str = "forge:projects.list/tags.list:";
const EVENT: &str = "forge:projects.list/events.list:";

fn projects(list: &[(&str, &str)]) -> String {
    let records: Vec<Value> = list
        .iter()
        .map(|(id, name)| json!({"id": id, "name": name, "path": format!("group/{name}")}))
        .collect();
    json!({ "projects": records }).to_string()
}

fn tags(names: &[&str]) -> String {
    let tags: Vec<Value> = names
        .iter()
        .map(|n| json!({"name": n, "commit": format!("c-{n}")}))
        .collect();
    json!({ "tags": tags }).to_string()
}

/// Four events, ids `<base>01` to `<base>04`, created on 2026-01-02 to 2026-01-05.
fn events(base: i64) -> Vec<Value> {
    (0..4)
        .map(|n| {
            json!({"id": base * 100 + n + 1, "action": "pushed",
                "created_at": format!("2026-01-0{}T00:00:00Z", n + 2)})
        })
        .collect()
}

fn write(w: &World, file: &str, text: &str) {
    std::fs::write(w.root.join(file), text).unwrap();
}

fn write_events(w: &World, project: &str, events: &[Value]) {
    write(
        w,
        &format!("events-{project}.json"),
        &json!({ "events": events }).to_string(),
    );
}

/// A world whose stand-in answers `list` (`(id, name)` of each project), each project with tags
/// `v1.0`, `v1.1` and `v2.0` and four events.
fn world(list: &[(&str, &str)]) -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(
        &w.bin.join("connectors"),
        &CONNECTORS.replace("@ROOT@", &root),
    );
    executable(&w.bin.join("claude"), &CLAUDE.replace("@ROOT@", &root));
    write(&w, "projects.json", &projects(list));
    for (n, (id, _)) in list.iter().enumerate() {
        write(
            &w,
            &format!("tags-{id}.json"),
            &tags(&["v1.0", "v1.1", "v2.0"]),
        );
        write_events(&w, id, &events(n as i64 + 1));
    }
    w
}

const TAGS_CHILD: &str = r#"              - operation: tags.list
                input: {project: "{id}"}
                records: "$.tags"
                parent: IN_PROJECT
                mapping:
                  node_type: Tag
                  id: "$.name"
                  name: "$.name"
                  aliases: []
                  properties: [{property: commit, path: "$.commit"}]
                  relations: []
"#;

const EVENTS_CHILD: &str = r#"              - operation: events.list
                input: {project: "{id}"}
                records: "$.events"
                time: "$.created_at"
                parent: IN_PROJECT
                mapping:
                  node_type: Event
                  id: "$.id"
                  name: "$.action"
                  aliases: []
                  properties: [{property: action, path: "$.action"}]
                  relations: []
"#;

/// The spec of instance `name`: one structured source `projects` with `children` (the YAML of
/// each child entry) and `dropped` as given.
fn spec_text(name: &str, dropped: &str, children: &[&str]) -> String {
    format!(
        r#"format: cortex.instance/1
name: {name}
description: Test brain.
ekr: {{version: "0.0.32", bin: "{ekr}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
  - name: projects
    schedule: daily
    settings:
      kind: structured
      value:
        input:
          from: connectors
          value:
            adapter: forge
            connection: conn_test
            operation: projects.list
            inputs: [{{}}]
            children:
{children}        records: "$.projects"
        mapping:
          node_type: Project
          id: "$.id"
          name: "$.name"
          aliases: ["$.path"]
          properties: [{{property: path, path: "$.path"}}]
          relations: []
{dropped}
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 100, max_chars_per_document: 5000}}
serve: {{view_port: 18993}}
"#,
        ekr = ekr().display(),
        children = children.concat(),
    )
}

const SUPERSEDE: &str = "        dropped: Supersede";

fn create_with(w: &World, name: &str, dropped: &str, children: &[&str]) {
    let path = w.root.join(format!("{name}.yaml"));
    std::fs::write(&path, spec_text(name, dropped, children)).unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
}

fn create(w: &World, name: &str, dropped: &str) {
    create_with(w, name, dropped, &[TAGS_CHILD, EVENTS_CHILD]);
}

fn run(w: &World, name: &str) -> Value {
    let (code, ran) = w.cortex(&["run", &format!("{name}/projects")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert!(
        w.lines("claude-calls.log").is_empty(),
        "the model was called"
    );
    ran
}

fn instance(w: &World, name: &str) -> PathBuf {
    w.home.join("instances").join(name)
}

fn ekr_json(w: &World, name: &str, args: &[&str]) -> Value {
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

#[derive(Debug, Clone)]
struct Node {
    id: String,
    aliases: Vec<String>,
    identity: String,
}

/// One assertion: its subject node, its predicate's name (a property's or an edge type's), its
/// lifecycle, its value (a property's), and when it is valid from.
#[derive(Debug, Clone)]
struct Assertion {
    subject: String,
    predicate: String,
    lifecycle: String,
    value: Option<String>,
    from: Option<i64>,
}

struct Store {
    nodes: Vec<Node>,
    edges: Vec<(String, String, String)>,
    assertions: Vec<Assertion>,
}

fn store(w: &World, name: &str) -> Store {
    let snapshot = ekr_json(w, name, &["snapshot"]);
    let ontology = ekr_json(w, name, &["ontology"]);
    let mut names: BTreeMap<String, String> = BTreeMap::new();
    for t in ontology["edge_types"].as_array().unwrap() {
        names.insert(
            t["id"].as_str().unwrap().into(),
            t["name"].as_str().unwrap().into(),
        );
    }
    for t in ontology["node_types"].as_array().unwrap() {
        for p in t["properties"].as_array().into_iter().flatten() {
            names.insert(
                p["id"].as_str().unwrap().into(),
                p["name"].as_str().unwrap().into(),
            );
        }
    }
    let graph = &snapshot["graph"]["graph"];
    let nodes = graph["nodes"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(id, n)| {
            let aliases: Vec<String> = n["aliases"]
                .as_array()
                .unwrap()
                .iter()
                .map(|a| a.as_str().unwrap().to_string())
                .collect();
            let identity = aliases
                .iter()
                .find(|a| a.starts_with("forge:"))
                .unwrap_or_else(|| panic!("node {id} has no identity among {aliases:?}"))
                .clone();
            Node {
                id: id.clone(),
                aliases,
                identity,
            }
        })
        .collect();
    let edges = graph["edges"]
        .as_object()
        .unwrap()
        .values()
        .map(|e| {
            (
                e["source"].as_str().unwrap().to_string(),
                names[e["type_id"].as_str().unwrap()].clone(),
                e["target"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    let assertions = graph["assertions"]
        .as_object()
        .unwrap()
        .values()
        .map(|a| {
            let lifecycle = match &a["lifecycle"] {
                Value::String(s) => s.clone(),
                Value::Object(o) => o.keys().next().unwrap().clone(),
                other => panic!("lifecycle {other}"),
            };
            let predicate = a["predicate"]["Property"]
                .as_str()
                .or_else(|| a["predicate"]["Relation"].as_str())
                .map(|id| names.get(id).cloned().unwrap_or_else(|| id.to_string()))
                .unwrap_or_default();
            Assertion {
                subject: a["subject"]["Node"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                predicate,
                lifecycle,
                value: match &a["object"]["Value"]["value"] {
                    Value::String(s) => Some(s.clone()),
                    Value::Null => None,
                    other => Some(other.to_string()),
                },
                from: a["valid_time"]["from"].as_i64(),
            }
        })
        .collect();
    Store {
        nodes,
        edges,
        assertions,
    }
}

impl Store {
    fn of(&self, prefix: &str) -> Vec<&Node> {
        self.nodes
            .iter()
            .filter(|n| n.identity.starts_with(prefix))
            .collect()
    }

    fn node(&self, identity: &str) -> &Node {
        self.nodes
            .iter()
            .find(|n| n.identity == identity)
            .unwrap_or_else(|| panic!("no node {identity}: {:?}", self.nodes))
    }

    fn edges_from(&self, id: &str) -> Vec<(&str, &str)> {
        self.edges
            .iter()
            .filter(|(s, _, _)| s == id)
            .map(|(_, t, target)| (t.as_str(), target.as_str()))
            .collect()
    }

    fn active(&self, id: &str) -> Vec<&Assertion> {
        self.assertions
            .iter()
            .filter(|a| a.subject == id && a.lifecycle == "Active")
            .collect()
    }
}

/// Every child under `prefix` of the project whose identity is `project` has exactly one edge,
/// `IN_PROJECT` to that project's node. A child's identity holds the project's part with `%` and
/// `:` escaped.
fn assert_linked(s: &Store, prefix: &str, project: &str) {
    let parent = s.node(project);
    let part = project
        .strip_prefix(PROJECT)
        .unwrap()
        .replace('%', "%25")
        .replace(':', "%3A");
    let children = s.of(&format!("{prefix}{part}:"));
    assert!(!children.is_empty(), "children of {project} under {prefix}");
    for child in children {
        assert_eq!(
            s.edges_from(&child.id),
            [("IN_PROJECT", parent.id.as_str())],
            "{child:?} has one edge, to {project}"
        );
    }
}

fn skipped(ran: &Value) -> Vec<String> {
    ran["detail"]["skipped"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

/// Every file under `dir`, recursively.
fn files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(files(&path));
        } else {
            out.push(path);
        }
    }
    out
}

/// A child record's identity never holds what masking would change (spec-file.md: "a child's
/// identity never holds an id masking or a rule would change"), so the seen state keeps it and an
/// unchanged child is not applied again. A parent whose id ends in a credential's name
/// (`top-secret`) is followed by `:` and the child's id in the child's identity: masking reads
/// that as an assignment, the identity falls back to the id's digest, and `secret:<digest>` is
/// still an assignment. The seen state drops such a key on load (`SeenState::load`), so the run
/// applies the unchanged tags again, every run.
#[test]
fn adv_children_of_a_parent_whose_id_ends_in_a_credential_name_are_applied_once() {
    let w = world(&[("top-secret", "alpha")]);
    write(&w, "tags-top-secret.json", &tags(&["v1.0.0", "v2.0.0"]));
    create(&w, "m", "");
    let first = run(&w, "m");
    assert_eq!(first["detail"]["documents_applied"], 1 + 2 + 4, "{first}");

    let again = run(&w, "m");
    assert_eq!(
        again["detail"]["documents_new"], 0,
        "nothing changed, so nothing is new: {again}"
    );
    let s = store(&w, "m");
    for n in &s.nodes {
        assert_eq!(
            cortex_cli::mask::mask_key(&n.identity).1,
            0,
            "an identity masking would change: {}",
            n.identity
        );
    }
}

/// An event's `time` dates it, and a changed event keeps its time: its new value is valid from
/// the same instant as the old one. With `dropped: Supersede` the store then holds the new value
/// alone, valid from the event's time, and the event keeps its one edge.
#[test]
fn adv_an_event_changed_at_its_fixed_time_holds_its_new_value_alone() {
    let w = world(&[("1", "alpha")]);
    create(&w, "c", SUPERSEDE);
    run(&w, "c");

    let mut changed = events(1);
    changed[0]["action"] = json!("merged");
    write_events(&w, "1", &changed);
    let ran = run(&w, "c");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");

    let s = store(&w, "c");
    let event = s.node(&format!("{EVENT}1:101"));
    let actions: Vec<(Option<&str>, Option<i64>)> = s
        .active(&event.id)
        .into_iter()
        .filter(|a| a.predicate == "action")
        .map(|a| (a.value.as_deref(), a.from))
        .collect();
    assert_eq!(actions, [(Some("merged"), Some(JAN_2))], "{ran}");
    assert_linked(&s, EVENT, "forge:projects.list:1");
}

/// A parent the source no longer lists ends its children, as the documentation says: every child
/// of a parent the source no longer lists has its values ended, its relation to the parent among
/// them, and the edge goes.
#[test]
fn adv_a_parent_no_longer_listed_ends_every_child_and_its_edge() {
    let w = world(&[("1", "alpha"), ("2", "beta")]);
    create(&w, "p", SUPERSEDE);
    run(&w, "p");

    write(&w, "projects.json", &projects(&[("1", "alpha")]));
    let ran = run(&w, "p");
    // Project 2's path; three tags' commit and edge; four events' action and edge.
    assert_eq!(ran["detail"]["retracted"], 1 + 6 + 8, "{ran}");
    let s = store(&w, "p");
    for child in s
        .of(&format!("{TAG}2:"))
        .into_iter()
        .chain(s.of(&format!("{EVENT}2:")))
    {
        assert!(s.active(&child.id).is_empty(), "{child:?}");
        assert!(s.edges_from(&child.id).is_empty(), "{child:?}");
    }
    assert_linked(&s, TAG, "forge:projects.list:1");
    assert_linked(&s, EVENT, "forge:projects.list:1");
}

/// A child call refused for one parent keeps that parent's earlier records of the operation; once
/// the call succeeds again, what the parent no longer lists is ended.
#[test]
fn adv_a_failed_child_call_holds_its_records_until_it_succeeds_again() {
    let w = world(&[("1", "alpha"), ("2", "beta")]);
    create(&w, "h", SUPERSEDE);
    run(&w, "h");

    write(&w, "forbidden-events.list-2", "");
    write_events(&w, "2", &events(2)[..3]);
    let held = run(&w, "h");
    assert_eq!(held["detail"]["retracted"], Value::Null, "{held}");
    let s = store(&w, "h");
    for event in s.of(&format!("{EVENT}2:")) {
        assert_eq!(s.active(&event.id).len(), 2, "{event:?}");
    }

    std::fs::remove_file(w.root.join("forbidden-events.list-2")).unwrap();
    let back = run(&w, "h");
    assert!(skipped(&back).is_empty(), "{back}");
    assert_eq!(back["detail"]["retracted"], 2, "{back}");
    let s = store(&w, "h");
    let gone = s.node(&format!("{EVENT}2:204"));
    assert!(s.active(&gone.id).is_empty(), "{gone:?}");
    assert!(s.edges_from(&gone.id).is_empty(), "{gone:?}");
}

/// A failed child call holds exactly its parent's records: parent `1`'s refused call does not
/// hold the records of parent `12`, whose identities start with the same characters, so an event
/// project 12 no longer lists is ended in that run.
#[test]
fn adv_a_failed_child_call_holds_no_parent_whose_id_its_id_begins() {
    let w = world(&[("1", "alpha"), ("12", "beta")]);
    create(&w, "g", SUPERSEDE);
    run(&w, "g");

    write(&w, "forbidden-events.list-1", "");
    write_events(&w, "12", &events(2)[..3]);
    let ran = run(&w, "g");
    assert_eq!(skipped(&ran).len(), 1, "{ran}");
    assert_eq!(ran["detail"]["retracted"], 2, "{ran}");
    let s = store(&w, "g");
    let gone = s.node(&format!("{EVENT}12:204"));
    assert!(s.active(&gone.id).is_empty(), "{gone:?}");
    for event in s.of(&format!("{EVENT}1:")) {
        assert_eq!(s.active(&event.id).len(), 2, "{event:?}");
    }
}

/// A child call that fails for every parent holds every parent's records of that operation and no
/// other: a tag a parent no longer lists is still ended in the same run.
#[test]
fn adv_a_child_call_failing_for_every_parent_holds_only_that_operation() {
    let w = world(&[("1", "alpha"), ("2", "beta")]);
    create(&w, "e", SUPERSEDE);
    run(&w, "e");

    write(&w, "forbidden-events.list-1", "");
    write(&w, "forbidden-events.list-2", "");
    write(&w, "tags-1.json", &tags(&["v1.0", "v1.1"]));
    let ran = run(&w, "e");
    let lines = skipped(&ran);
    assert_eq!(lines.len(), 2, "{ran}");
    for id in ["1", "2"] {
        assert!(
            lines
                .iter()
                .any(|l| l.contains("events.list") && l.contains(&format!("{PROJECT}{id}:"))),
            "{ran}"
        );
    }
    // Tag v2.0 of project 1: its commit and its edge.
    assert_eq!(ran["detail"]["retracted"], 2, "{ran}");
    let s = store(&w, "e");
    for event in s.of(EVENT) {
        assert_eq!(s.active(&event.id).len(), 2, "{event:?}");
    }
    let gone = s.node(&format!("{TAG}1:v2.0"));
    assert!(s.active(&gone.id).is_empty(), "{gone:?}");
}

/// A child call that fails for another reason than `forbidden` (an answer with no array at
/// `records`, a result that is not JSON) does not fail the run either: the parent and its other
/// children are applied and `skipped` names the parent and the operation.
#[test]
fn adv_a_child_answer_without_records_or_not_json_is_skipped_not_fatal() {
    let w = world(&[("1", "alpha"), ("2", "beta")]);
    write(&w, "tags-1.json", r#"{"items": []}"#);
    write(&w, "tags-2.json", r#""not json""#);
    create(&w, "j", "");
    let ran = run(&w, "j");
    assert_eq!(ran["detail"]["documents_applied"], 2 + 8, "{ran}");
    let lines = skipped(&ran);
    assert_eq!(lines.len(), 2, "{ran}");
    assert!(lines.iter().any(|l| l.contains("tags.list")
        && l.contains(&format!("{PROJECT}1:"))
        && l.contains("no array")));
    assert!(lines.iter().any(|l| l.contains("tags.list")
        && l.contains(&format!("{PROJECT}2:"))
        && l.contains("not JSON")));
    let s = store(&w, "j");
    assert!(s.of(TAG).is_empty());
    assert_linked(&s, EVENT, "forge:projects.list:1");
    assert_linked(&s, EVENT, "forge:projects.list:2");
}

/// The child's time field as operating.md reads a record's `time`: missing, empty, epoch
/// milliseconds, an unparseable text and an instant after the run are observed at the run's
/// start; epoch seconds as a JSON number and a bare date date the event.
#[test]
fn adv_an_event_time_the_reader_cannot_use_is_the_runs_and_a_usable_one_is_the_events() {
    let w = world(&[("1", "alpha")]);
    let shapes = [
        json!({"id": 1, "action": "a"}),
        json!({"id": 2, "action": "a", "created_at": ""}),
        json!({"id": 3, "action": "a", "created_at": JAN_2 + DAY}),
        json!({"id": 4, "action": "a", "created_at": "yesterday"}),
        json!({"id": 5, "action": "a", "created_at": "2999-01-01"}),
        json!({"id": 6, "action": "a", "created_at": JAN_2 / 1000}),
        json!({"id": 7, "action": "a", "created_at": "2026-01-03"}),
    ];
    write_events(&w, "1", &shapes);
    create(&w, "t", "");
    let started = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    run(&w, "t");
    let s = store(&w, "t");
    for id in 1..=5 {
        let node = s.node(&format!("{EVENT}1:{id}"));
        let froms: Vec<Option<i64>> = s.active(&node.id).iter().map(|a| a.from).collect();
        assert_eq!(froms.len(), 2, "{node:?}");
        assert!(
            froms.iter().all(|f| f.is_some_and(|f| f >= started)),
            "event {id}: {froms:?} is the run's time"
        );
    }
    for (id, at) in [(6, JAN_2), (7, JAN_2 + DAY)] {
        let node = s.node(&format!("{EVENT}1:{id}"));
        let froms: Vec<Option<i64>> = s.active(&node.id).iter().map(|a| a.from).collect();
        assert_eq!(froms, [Some(at); 2], "event {id}");
    }
}

/// Parent ids holding `:` and `%` stay apart from each other after escaping, a child id equal to
/// its parent's id is a node of its own, and every child is linked to its own parent.
#[test]
fn adv_parent_ids_with_colon_and_percent_keep_their_children_apart() {
    let w = world(&[("a:b", "alpha"), ("a%3Ab", "beta")]);
    write(&w, "tags-a:b.json", &tags(&["v1.0", "a:b"]));
    write(&w, "tags-a%3Ab.json", &tags(&["v1.0", "a%3Ab"]));
    create(&w, "x", "");
    let ran = run(&w, "x");
    assert_eq!(ran["detail"]["documents_applied"], 2 + 4 + 8, "{ran}");
    assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");
    let s = store(&w, "x");
    assert_eq!(s.nodes.len(), 2 + 4 + 8, "{:?}", s.nodes);
    s.node(&format!("{TAG}a%3Ab:v1.0"));
    s.node(&format!("{TAG}a%253Ab:v1.0"));
    s.node(&format!("{TAG}a%3Ab:a:b"));
    for parent in ["forge:projects.list:a:b", "forge:projects.list:a%3Ab"] {
        let p = s.node(parent);
        assert!(s.edges_from(&p.id).is_empty(), "{p:?}");
    }
    assert_linked(&s, TAG, "forge:projects.list:a:b");
    assert_linked(&s, EVENT, "forge:projects.list:a:b");
    // `assert_linked` reads children by the parent's raw part; the escaped one is `a%253Ab`.
    let beta = s.node("forge:projects.list:a%3Ab");
    for child in s
        .of(&format!("{TAG}a%253Ab:"))
        .into_iter()
        .chain(s.of(&format!("{EVENT}a%253Ab:")))
    {
        assert_eq!(s.edges_from(&child.id), [("IN_PROJECT", beta.id.as_str())]);
    }
}

/// Children with one `parent` name share one edge type (spec-file.md). An operator who adds a
/// second child with the same `parent` name to a source that already ran (`cortex update`) gets
/// its records linked to their parents as the first child's are.
#[test]
fn adv_a_child_added_later_with_the_same_parent_name_is_linked() {
    let w = world(&[("1", "alpha")]);
    create_with(&w, "u", "", &[TAGS_CHILD]);
    run(&w, "u");

    let path = w.root.join("u2.yaml");
    std::fs::write(&path, spec_text("u", "", &[TAGS_CHILD, EVENTS_CHILD])).unwrap();
    let (code, updated) = w.cortex(&[
        "update",
        "u",
        "--spec",
        path.to_str().unwrap(),
        "--no-units",
    ]);
    assert_eq!(code, 0, "{updated}");
    let ran = run(&w, "u");
    assert_eq!(ran["detail"]["documents_applied"], 4, "{ran}");
    assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");
    let s = store(&w, "u");
    assert_eq!(s.of(&format!("{EVENT}1:")).len(), 4);
    assert_linked(&s, EVENT, "forge:projects.list:1");
    assert_linked(&s, TAG, "forge:projects.list:1");
}

/// No credential reaches a file of the instance: a child record's id and an unmapped field that
/// are credentials, and a parent whose id is one and whose child call is refused (its identity is
/// in `skipped`, `cortex.log` and the evidence).
#[test]
fn adv_no_credential_of_a_child_or_its_parent_reaches_the_instance() {
    let token = format!("glpat-{}", "q".repeat(24));
    let assigned = format!("Zr4{}", "p0".repeat(8));
    let w = world(&[("1", "alpha"), (token.as_str(), "beta")]);
    write(&w, "tags-1.json", &tags(&["v1.0", token.as_str()]));
    let mut with_note = events(1);
    with_note[0]["note"] = json!(format!("password={assigned}"));
    write_events(&w, "1", &with_note);
    write(&w, &format!("forbidden-events.list-{token}"), "");
    create(&w, "k", SUPERSEDE);
    let ran = run(&w, "k");
    assert_eq!(ran["detail"]["documents_applied"], 2 + 2 + 4 + 3, "{ran}");
    let lines = skipped(&ran);
    assert_eq!(lines.len(), 1, "{ran}");
    assert!(!ran.to_string().contains(&token), "{ran}");
    let s = store(&w, "k");
    let beta = s
        .of(PROJECT)
        .into_iter()
        .find(|p| p.aliases.iter().any(|a| a.starts_with("beta (")))
        .expect("project beta")
        .identity
        .clone();
    assert_linked(&s, TAG, &beta);
    for file in files(&instance(&w, "k")) {
        let bytes = std::fs::read(&file).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(!text.contains(&token), "{} holds the token", file.display());
        assert!(
            !text.contains(&assigned),
            "{} holds the value",
            file.display()
        );
    }
}
