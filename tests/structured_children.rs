//! A `structured` source with child operations (`story:structured-children`): the stand-in
//! `connectors` answers projects, and per project its tags and its events. Each child record is a
//! node of its own, linked to its project's node by one edge the child mapping names (`parent:`),
//! and an event is dated by the record field its mapping names as `time:`. No model is called.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::Command;

use common::{ekr, executable, World};
use serde_json::Value;

/// A stand-in `connectors` with one ready `forge` connection. `projects.list` answers the file
/// `projects.json`; `tags.list` and `events.list` answer `tags-<project>.json` and
/// `events-<project>.json`, the project taken from the input's `"project"`. `events.list` answers
/// `forbidden` for a project whose file `forbidden-events-<project>` exists. Every invocation is
/// logged to `invocations.log`.
const CONNECTORS: &str = r#"R="@ROOT@"
P=$(printf '%s\n' "$*" | sed -n 's/.*"project":"\([^"]*\)".*/\1/p')
answer() { printf '{"ok":true,"result":{"adapter":"forge","operation":"%s","revision":"r","result":%s}}\n' "$1" "$(cat "$2")"; }
case "$*" in
  *"connections list"*) echo '{"ok":true,"result":{"connections":[{"adapter":"forge","connection":"conn_test","state":"ready","revision":"rev1"}]}}' ;;
  *"connections revalidate"*) echo '{"ok":true,"result":{}}' ;;
  *"operations describe"*) echo '{"ok":true,"result":{"schema":"s","revision":"r"}}' ;;
  *"--operation projects.list"*) echo "projects.list" >> "$R/invocations.log"; answer projects.list "$R/projects.json" ;;
  *"--operation tags.list"*) echo "tags.list $P" >> "$R/invocations.log"; answer tags.list "$R/tags-$P.json" ;;
  *"--operation events.list"*)
    echo "events.list $P" >> "$R/invocations.log"
    if [ -e "$R/forbidden-events-$P" ]; then echo '{"ok":false,"error":{"code":"failure","data":{"code":"forbidden","stage":"execution"}}}'; exit 1; fi
    answer events.list "$R/events-$P.json" ;;
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

/// Two projects. Their tags have the same names, so only the parent tells them apart.
fn projects(alpha_path: &str) -> String {
    format!(
        r#"{{"projects": [
 {{"id": 1, "name": "alpha", "path": "{alpha_path}"}},
 {{"id": 2, "name": "beta", "path": "group/beta"}}
]}}"#
    )
}

fn tags(names: &[&str]) -> String {
    let tags: Vec<Value> = names
        .iter()
        .map(|n| serde_json::json!({"name": n, "commit": format!("c-{n}")}))
        .collect();
    serde_json::json!({ "tags": tags }).to_string()
}

/// Four events of project `project`, ids `<project>01` to `<project>04`, created on 2026-01-02 to
/// 2026-01-05 in four spellings of a time: RFC 3339 in UTC, with an offset, a bare date, and epoch
/// seconds.
fn events(project: i64) -> Vec<Value> {
    let created = [
        "2026-01-02T00:00:00Z".to_string(),
        "2026-01-03T02:00:00+02:00".to_string(),
        "2026-01-04".to_string(),
        ((JAN_2 + 3 * DAY) / 1000).to_string(),
    ];
    created
        .iter()
        .enumerate()
        .map(|(n, at)| {
            serde_json::json!({"id": project * 100 + n as i64 + 1, "action": "pushed", "created_at": at})
        })
        .collect()
}

fn write_events(w: &World, project: i64, events: &[Value]) {
    std::fs::write(
        w.root.join(format!("events-{project}.json")),
        serde_json::json!({ "events": events }).to_string(),
    )
    .unwrap();
}

/// A world whose stand-in answers both projects with three tags and four events each.
fn world() -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(
        &w.bin.join("connectors"),
        &CONNECTORS.replace("@ROOT@", &root),
    );
    executable(&w.bin.join("claude"), &CLAUDE.replace("@ROOT@", &root));
    std::fs::write(w.root.join("projects.json"), projects("group/alpha")).unwrap();
    for project in [1, 2] {
        std::fs::write(
            w.root.join(format!("tags-{project}.json")),
            tags(&["v1.0", "v1.1", "v2.0"]),
        )
        .unwrap();
        write_events(&w, project, &events(project));
    }
    w
}

/// Creates instance `name` with one structured source, `projects`, whose records are the stand-in's
/// projects, each with its tags and its events as children, and `dropped` as given.
fn create(w: &World, name: &str, dropped: &str) {
    let path = w.root.join(format!("{name}.yaml"));
    std::fs::write(&path, spec_text(name, dropped)).unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
}

/// The spec file [`create`] writes.
fn spec_text(name: &str, dropped: &str) -> String {
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
              - operation: tags.list
                input: {{project: "{{id}}"}}
                records: "$.tags"
                parent: IN_PROJECT
                mapping:
                  node_type: Tag
                  id: "$.name"
                  name: "$.name"
                  aliases: []
                  properties: [{{property: commit, path: "$.commit"}}]
                  relations: []
              - operation: events.list
                input: {{project: "{{id}}"}}
                records: "$.events"
                time: "$.created_at"
                parent: IN_PROJECT
                mapping:
                  node_type: Event
                  id: "$.id"
                  name: "$.action"
                  aliases: []
                  properties: [{{property: action, path: "$.action"}}]
                  relations: []
        records: "$.projects"
        mapping:
          node_type: Project
          id: "$.id"
          name: "$.name"
          aliases: ["$.path"]
          properties: [{{property: path, path: "$.path"}}, {{property: notes, path: "$.notes"}}]
          relations: []
{dropped}
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 100, max_chars_per_document: 5000}}
serve: {{view_port: 18992}}
"#,
        ekr = ekr().display()
    )
}

fn run(w: &World, name: &str) -> Value {
    let (code, ran) = w.cortex(&["run", &format!("{name}/projects")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert!(
        w.lines("claude-calls.log").is_empty(),
        "the stand-in model was called"
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

/// The identities a child record's node is known by start with these: `<adapter>:<parent
/// operation>/<child operation>:<parent id>:`.
const TAG: &str = "forge:projects.list/tags.list:";
const EVENT: &str = "forge:projects.list/events.list:";
const PROJECT: &str = "forge:projects.list:";

/// One node of the store, as `ekr snapshot` and `ekr ontology` hold it.
#[derive(Debug, Clone)]
struct Node {
    id: String,
    /// Its canonical name.
    name: String,
    aliases: Vec<String>,
    /// The identity among its aliases: `<prefix><id>`.
    identity: String,
}

/// What a store holds: its nodes; its edges as `(source, edge type name, target)` node ids; and
/// its assertions as `(subject node, lifecycle, valid from)`.
struct Store {
    nodes: Vec<Node>,
    edges: Vec<(String, String, String)>,
    assertions: Vec<(String, String, Option<i64>)>,
}

fn store(w: &World, name: &str) -> Store {
    let snapshot = ekr_json(w, name, &["snapshot"]);
    let ontology = ekr_json(w, name, &["ontology"]);
    let edge_types: BTreeMap<String, String> = ontology["edge_types"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            (
                t["id"].as_str().unwrap().to_string(),
                t["name"].as_str().unwrap().to_string(),
            )
        })
        .collect();
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
                name: n["canonical_name"].as_str().unwrap().to_string(),
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
                edge_types[e["type_id"].as_str().unwrap()].clone(),
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
            (
                a["subject"]["Node"].as_str().unwrap().to_string(),
                lifecycle,
                a["valid_time"]["from"].as_i64(),
            )
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

    fn project(&self, id: i64) -> &Node {
        let found = self.of(&format!("{PROJECT}{id}"));
        assert_eq!(found.len(), 1, "one node for project {id}: {found:?}");
        found[0]
    }

    /// The children of project `parent` of one child prefix (`TAG` or `EVENT`).
    fn children(&self, prefix: &str, parent: i64) -> Vec<&Node> {
        self.of(&format!("{prefix}{parent}:"))
    }

    /// The edges leaving node `id`, as `(edge type name, target node)`.
    fn edges_from(&self, id: &str) -> Vec<(&str, &str)> {
        self.edges
            .iter()
            .filter(|(s, _, _)| s == id)
            .map(|(_, t, target)| (t.as_str(), target.as_str()))
            .collect()
    }

    fn active_about(&self, id: &str) -> Vec<Option<i64>> {
        self.assertions
            .iter()
            .filter(|(s, l, _)| s == id && l == "Active")
            .map(|(_, _, from)| *from)
            .collect()
    }
}

/// Every child of `parent` has exactly one edge, `IN_PROJECT` to `parent`'s node.
fn assert_linked(s: &Store, prefix: &str, parent: i64) {
    let project = s.project(parent);
    for child in s.children(prefix, parent) {
        assert_eq!(
            s.edges_from(&child.id),
            [("IN_PROJECT", project.id.as_str())],
            "{child:?} has one edge, to project {parent}"
        );
    }
}

/// Acceptance 1: two projects with three tags and four events each import as 2 + 14 nodes, and
/// every child node has one edge to its parent. The tags of both projects have the same names:
/// a child's identity holds its parent's id, so they are six nodes, not three.
#[test]
fn two_projects_with_their_tags_and_events_import_as_sixteen_nodes_each_child_linked_to_its_project(
) {
    let w = world();
    create(&w, "c", "");
    let ran = run(&w, "c");
    assert_eq!(ran["detail"]["documents_applied"], 16, "{ran}");
    assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");
    assert!(ran["detail"].get("skipped").is_none(), "{ran}");
    assert_eq!(
        w.lines("invocations.log"),
        [
            "projects.list",
            "tags.list 1",
            "events.list 1",
            "tags.list 2",
            "events.list 2"
        ]
    );

    let s = store(&w, "c");
    assert_eq!(s.nodes.len(), 2 + 14, "{:?}", s.nodes);
    assert_eq!(s.of(PROJECT).len(), 2);
    for project in [1, 2] {
        assert_eq!(s.children(TAG, project).len(), 3, "tags of {project}");
        assert_eq!(s.children(EVENT, project).len(), 4, "events of {project}");
        assert_linked(&s, TAG, project);
        assert_linked(&s, EVENT, project);
        // A project has no edge of its own.
        assert!(s.edges_from(&s.project(project).id).is_empty());
    }
    assert_eq!(s.edges.len(), 14, "{:?}", s.edges);
    let tag = |project: i64| -> BTreeSet<&str> {
        s.children(TAG, project)
            .iter()
            .map(|n| n.identity.as_str())
            .collect()
    };
    assert_eq!(
        tag(1),
        BTreeSet::from([
            "forge:projects.list/tags.list:1:v1.0",
            "forge:projects.list/tags.list:1:v1.1",
            "forge:projects.list/tags.list:1:v2.0"
        ])
    );
    assert!(tag(1).is_disjoint(&tag(2)));
    // A node is named by its record's name and its identity, a project too, though its children
    // name it as well.
    let named = |identity: &str| {
        s.nodes
            .iter()
            .find(|n| n.identity == identity)
            .unwrap_or_else(|| panic!("no node {identity}"))
            .name
            .clone()
    };
    assert_eq!(
        named("forge:projects.list/tags.list:2:v1.0"),
        "v1.0 (forge:projects.list/tags.list:2:v1.0)"
    );
    assert_eq!(
        named("forge:projects.list/events.list:1:101"),
        "pushed (forge:projects.list/events.list:1:101)"
    );
    assert_eq!(
        named("forge:projects.list:1"),
        "alpha (forge:projects.list:1)"
    );

    // Unchanged records: nothing new is applied, and the children are still asked for.
    let again = run(&w, "c");
    assert_eq!(again["detail"]["documents_new"], 0, "{again}");
    assert_eq!(w.lines("invocations.log").len(), 10);
}

/// Acceptance 2: each event node's facts are valid from the time its record names at `time:`, and
/// its evidence is observed then; a tag, whose mapping names no time, is dated by the run.
#[test]
fn an_event_is_valid_from_the_time_its_record_names() {
    let w = world();
    create(&w, "t", "");
    let started = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    run(&w, "t");
    let s = store(&w, "t");
    for project in [1, 2] {
        for n in 0..4 {
            let identity = format!("{EVENT}{project}:{}", project * 100 + n + 1);
            let node = s
                .nodes
                .iter()
                .find(|x| x.identity == identity)
                .unwrap_or_else(|| panic!("no node {identity}"));
            let froms = s.active_about(&node.id);
            // The `action` property and the `IN_PROJECT` relation.
            assert_eq!(
                froms,
                vec![Some(JAN_2 + n * DAY); 2],
                "{identity} is valid from its created_at"
            );
        }
        for tag in s.children(TAG, project) {
            for from in s.active_about(&tag.id) {
                assert!(
                    from.is_some_and(|f| f >= started),
                    "{tag:?}: {from:?} is the run's time"
                );
            }
        }
    }
}

/// Acceptance 3: run 1 imports project `alpha` at path `group/alpha`; run 2 answers the same
/// project id at path `group/alpha-renamed`. The store holds one node for that id, with
/// `group/alpha` among its aliases: EKR adds no alias to a node an extraction matched, so the new
/// path is not added. Its children, a tag new in run 2 among them, stay linked to that node.
#[test]
fn a_renamed_project_keeps_its_one_node_its_old_path_and_its_children() {
    let w = world();
    create(&w, "r", "");
    run(&w, "r");
    let first = store(&w, "r");
    let alpha = first.project(1).clone();
    assert!(
        alpha.aliases.contains(&"group/alpha".to_string()),
        "{alpha:?}"
    );

    std::fs::write(
        w.root.join("projects.json"),
        projects("group/alpha-renamed"),
    )
    .unwrap();
    std::fs::write(
        w.root.join("tags-1.json"),
        tags(&["v1.0", "v1.1", "v2.0", "v3.0"]),
    )
    .unwrap();
    let ran = run(&w, "r");
    assert_eq!(
        ran["detail"]["documents_applied"], 2,
        "the project and its new tag: {ran}"
    );
    assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");

    let s = store(&w, "r");
    let renamed = s.project(1);
    assert_eq!(renamed.id, alpha.id, "one node for project 1");
    assert!(
        renamed.aliases.contains(&"group/alpha".to_string()),
        "{renamed:?}"
    );
    assert!(
        !renamed.aliases.contains(&"group/alpha-renamed".to_string()),
        "{renamed:?}"
    );
    assert_eq!(s.of(PROJECT).len(), 2);
    assert_eq!(s.children(TAG, 1).len(), 4);
    assert_linked(&s, TAG, 1);
    assert_linked(&s, EVENT, 1);
}

/// The skipped line of a run that names project `id` and `events.list`.
fn skipped_for(ran: &Value, id: i64) -> Vec<String> {
    ran["detail"]["skipped"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|s| s.contains(&format!("{PROJECT}{id}")) && s.contains("events.list"))
        .map(str::to_string)
        .collect()
}

/// Acceptance 4: the stand-in answers `forbidden` to project 2's events. Project 2 is imported
/// with its tags and without events, project 1 with all of its children, and the run's `skipped`
/// names project 2 and the operation with the reason.
#[test]
fn a_forbidden_child_call_imports_its_project_with_its_other_children_and_is_skipped() {
    let w = world();
    std::fs::write(w.root.join("forbidden-events-2"), "").unwrap();
    create(&w, "f", "");
    let ran = run(&w, "f");
    assert_eq!(ran["detail"]["documents_applied"], 2 + 6 + 4, "{ran}");
    assert_eq!(ran["detail"]["stopped"], Value::Null, "{ran}");
    let skipped = skipped_for(&ran, 2);
    assert_eq!(skipped.len(), 1, "{ran}");
    assert!(skipped[0].contains("forbidden"), "the reason: {ran}");
    assert!(skipped_for(&ran, 1).is_empty(), "{ran}");

    let s = store(&w, "f");
    assert_eq!(s.of(PROJECT).len(), 2);
    assert_eq!(s.children(TAG, 2).len(), 3);
    assert_eq!(s.children(EVENT, 2).len(), 0);
    assert_eq!(s.children(TAG, 1).len(), 3);
    assert_eq!(s.children(EVENT, 1).len(), 4);
    assert_linked(&s, TAG, 2);
    assert_linked(&s, EVENT, 1);
}

/// A child call that fails for one parent ends none of that parent's earlier children of the
/// operation (`dropped: Supersede`): project 2's events stay active with their edges, while an
/// event project 1 no longer lists is retracted and its edge removed in the same run.
#[test]
fn a_failed_child_call_ends_none_of_that_parents_earlier_children() {
    let w = world();
    create(&w, "d", "        dropped: Supersede");
    run(&w, "d");
    let first = store(&w, "d");
    assert_eq!(first.nodes.len(), 16);

    std::fs::write(w.root.join("forbidden-events-2"), "").unwrap();
    write_events(&w, 1, &events(1)[..3]);
    let ran = run(&w, "d");
    assert_eq!(skipped_for(&ran, 2).len(), 1, "{ran}");
    // Event 104's `action` and its `IN_PROJECT` relation; none of project 2's.
    assert_eq!(ran["detail"]["retracted"], 2, "{ran}");

    let s = store(&w, "d");
    let gone = s
        .nodes
        .iter()
        .find(|n| n.identity == format!("{EVENT}1:104"))
        .unwrap();
    assert!(s.active_about(&gone.id).is_empty(), "{gone:?}");
    assert!(s.edges_from(&gone.id).is_empty(), "{gone:?}");
    let kept = s.children(EVENT, 2);
    assert_eq!(kept.len(), 4);
    for event in kept {
        assert_eq!(
            s.active_about(&event.id).len(),
            2,
            "{event:?} kept its values"
        );
    }
    assert_linked(&s, EVENT, 2);
}

/// A child names its parent by all the parent's aliases, so a parent the run does not apply (its
/// mapped values are over EKR's evidence bound, so it is in `skipped`) is still named
/// `<name> (<identity>)` and known by its mapped alias, through the edges of its children.
#[test]
fn a_parent_the_run_does_not_apply_is_still_named_by_its_children() {
    let w = world();
    let projects = serde_json::json!({"projects": [
        {"id": 1, "name": "alpha", "path": "group/alpha"},
        {"id": 2, "name": "beta", "path": "group/beta", "notes": "n".repeat(17_000)},
    ]});
    std::fs::write(w.root.join("projects.json"), projects.to_string()).unwrap();
    create(&w, "b", "");
    let ran = run(&w, "b");
    assert_eq!(ran["detail"]["documents_applied"], 1 + 14, "{ran}");
    let skipped: Vec<&str> = ran["detail"]["skipped"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    assert!(
        matches!(&skipped[..], [one] if one.starts_with("forge:projects.list:2: its mapped values")),
        "{ran}"
    );

    let s = store(&w, "b");
    let beta = s.project(2);
    assert_eq!(beta.name, "beta (forge:projects.list:2)", "{beta:?}");
    assert!(beta.aliases.contains(&"group/beta".to_string()), "{beta:?}");
    assert_linked(&s, TAG, 2);
    assert_linked(&s, EVENT, 2);
}

/// A child record's identity names its child by the operation, so two children of one operation
/// would be one record set, mapped by the first: the spec is refused, naming the operation.
#[test]
fn a_spec_with_two_children_of_one_operation_is_refused() {
    assert!(cortex_cli::spec::parse(&spec_text("once", "")).is_ok());
    let twice =
        spec_text("twice", "").replace("- operation: events.list", "- operation: tags.list");
    let refused = cortex_cli::spec::parse(&twice).expect_err("a refusal");
    assert!(
        refused.contains("sources.projects: children: operation \"tags.list\" appears twice"),
        "{refused}"
    );
}
