//! A `structured` source with a compare link (`story:structured-compare-links`): the stand-in
//! `connectors` answers projects, and per project its tags and its changes as children, and a
//! compare operation that lists the changes between two tags. Each change is linked to the first
//! tag whose comparison holds it by one `shipped_in` edge. No model is called.

mod common;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

use common::{ekr, executable, World};
use serde_json::{json, Value};

/// A stand-in `connectors` with one ready `forge` connection. `projects.list` answers the file
/// `projects.json`; `tags.list` and `changes.list` answer `tags-<project>.json` and
/// `changes-<project>.json`, an empty list when the file is missing. `compare` answers
/// `compare-<project>-<to>.json`, an empty list when it is missing, and `forbidden` when the file
/// `fail-compare-<project>-<to>` exists. The project, `from` and `to` are read from the input; every
/// invocation is logged to `invocations.log`, a compare as `compare <project> <from>-><to>`.
const CONNECTORS: &str = r#"R="@ROOT@"
field() { printf '%s\n' "$ARGS" | sed -n "s/.*\"$1\":\"\([^\"]*\)\".*/\1/p"; }
ARGS="$*"
P=$(field project); F=$(field from); T=$(field to)
answer() { if [ -e "$2" ]; then B=$(cat "$2"); else B="$3"; fi; printf '{"ok":true,"result":{"adapter":"forge","operation":"%s","revision":"r","result":%s}}\n' "$1" "$B"; }
case "$*" in
  *"connections list"*) echo '{"ok":true,"result":{"connections":[{"adapter":"forge","connection":"conn_test","state":"ready","revision":"rev1"}]}}' ;;
  *"connections revalidate"*) echo '{"ok":true,"result":{}}' ;;
  *"operations describe"*) echo '{"ok":true,"result":{"schema":"s","revision":"r"}}' ;;
  *"--operation projects.list"*) echo "projects.list" >> "$R/invocations.log"; answer projects.list "$R/projects.json" '{"projects":[]}' ;;
  *"--operation tags.list"*) echo "tags.list $P" >> "$R/invocations.log"; answer tags.list "$R/tags-$P.json" '{"tags":[]}' ;;
  *"--operation changes.list"*) echo "changes.list $P" >> "$R/invocations.log"; answer changes.list "$R/changes-$P.json" '{"changes":[]}' ;;
  *"--operation compare"*)
    echo "compare $P $F->$T" >> "$R/invocations.log"
    if [ -e "$R/fail-compare-$P-$T" ]; then echo '{"ok":false,"error":{"code":"failure","data":{"code":"forbidden","stage":"execution"}}}'; exit 1; fi
    answer compare "$R/compare-$P-$T.json" '{"commits":[]}' ;;
  *) echo '{"ok":false}'; exit 2 ;;
esac
"#;

/// A stand-in `claude` that logs the call and fails: a structured source never calls it.
const CLAUDE: &str = r#"echo call >> "@ROOT@/claude-calls.log"
echo '{"type":"result","subtype":"error","is_error":true}'
exit 1
"#;

/// A world whose stand-in answers the projects of `projects` (ids), with no tags, changes or
/// compare answers yet.
fn world(projects: &[i64]) -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(
        &w.bin.join("connectors"),
        &CONNECTORS.replace("@ROOT@", &root),
    );
    executable(&w.bin.join("claude"), &CLAUDE.replace("@ROOT@", &root));
    let projects: Vec<Value> = projects
        .iter()
        .map(|id| json!({"id": id, "name": format!("project-{id}")}))
        .collect();
    write(&w, "projects.json", json!({ "projects": projects }));
    w
}

fn write(w: &World, file: &str, value: Value) {
    std::fs::write(w.root.join(file), value.to_string()).unwrap();
}

/// Project `project`'s tags, each `(name, created_at)`, in the order given.
fn tags(w: &World, project: i64, tags: &[(&str, &str)]) {
    let tags: Vec<Value> = tags
        .iter()
        .map(|(name, at)| json!({"name": name, "created_at": at}))
        .collect();
    write(w, &format!("tags-{project}.json"), json!({ "tags": tags }));
}

/// Project `project`'s changes, by id.
fn changes(w: &World, project: i64, ids: &[&str]) {
    let changes: Vec<Value> = ids
        .iter()
        .map(|id| json!({"sha": id, "title": format!("change {id}")}))
        .collect();
    write(
        w,
        &format!("changes-{project}.json"),
        json!({ "changes": changes }),
    );
}

/// What the compare of project `project` up to tag `to` lists: the commits of `ids`.
fn compare(w: &World, project: i64, to: &str, ids: &[&str]) {
    let commits: Vec<Value> = ids.iter().map(|id| json!({"id": id})).collect();
    write(
        w,
        &format!("compare-{project}-{to}.json"),
        json!({ "commits": commits }),
    );
}

/// The spec of one structured source, `projects`, with tags and changes as children and one
/// compare link between them; `dropped` as given.
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
                  properties: [{{property: created_at, path: "$.created_at"}}]
                  relations: []
              - operation: changes.list
                input: {{project: "{{id}}"}}
                records: "$.changes"
                parent: IN_PROJECT
                mapping:
                  node_type: Change
                  id: "$.sha"
                  name: "$.title"
                  aliases: []
                  properties: [{{property: title, path: "$.title"}}]
                  relations: []
            links:
              - operation: compare
                input: {{project: "{{id}}", from: "{{from.name}}", to: "{{to.name}}"}}
                records: "$.commits"
                tags: tags.list
                changes: changes.list
                order: "$.created_at"
                change_id: "$.id"
                relation: shipped_in
        records: "$.projects"
        mapping:
          node_type: Project
          id: "$.id"
          name: "$.name"
          aliases: []
          properties: []
          relations: []
{dropped}
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 100, max_chars_per_document: 5000}}
serve: {{view_port: 18993}}
"#,
        ekr = ekr().display()
    )
}

fn create(w: &World, name: &str, dropped: &str) {
    let path = w.root.join(format!("{name}.yaml"));
    std::fs::write(&path, spec_text(name, dropped)).unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
}

fn run(w: &World, name: &str) -> Value {
    let (code, ran) = w.cortex(&["run", &format!("{name}/projects")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");
    assert!(
        w.lines("claude-calls.log").is_empty(),
        "the stand-in model was called"
    );
    ran
}

fn ekr_json(w: &World, name: &str, args: &[&str]) -> Value {
    let dir: PathBuf = w.home.join("instances").join(name);
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

/// The identities a tag's and a change's node are known by start with these, then
/// `<project id>:<id>`.
const TAG: &str = "forge:projects.list/tags.list:";
const CHANGE: &str = "forge:projects.list/changes.list:";

/// The edges of a store, as `(source identity, edge type name, target identity)`: each node named
/// by the identity among its aliases.
fn edges(w: &World, name: &str) -> Vec<(String, String, String)> {
    let snapshot = ekr_json(w, name, &["snapshot"]);
    let ontology = ekr_json(w, name, &["ontology"]);
    let edge_types: BTreeMap<&str, &str> = ontology["edge_types"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| (t["id"].as_str().unwrap(), t["name"].as_str().unwrap()))
        .collect();
    let graph = &snapshot["graph"]["graph"];
    let identity = |node: &str| -> String {
        graph["nodes"][node]["aliases"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .find(|a| a.starts_with("forge:"))
            .unwrap_or_else(|| panic!("node {node} has no identity"))
            .to_string()
    };
    graph["edges"]
        .as_object()
        .unwrap()
        .values()
        .map(|e| {
            (
                identity(e["source"].as_str().unwrap()),
                edge_types[e["type_id"].as_str().unwrap()].to_string(),
                identity(e["target"].as_str().unwrap()),
            )
        })
        .collect()
}

/// The targets of the `shipped_in` edges from the change `<project>:<id>`, by identity.
fn shipped_in(edges: &[(String, String, String)], project: i64, id: &str) -> Vec<String> {
    let change = format!("{CHANGE}{project}:{id}");
    edges
        .iter()
        .filter(|(s, t, _)| *s == change && t == "shipped_in")
        .map(|(_, _, target)| target.clone())
        .collect()
}

fn tag(project: i64, name: &str) -> String {
    format!("{TAG}{project}:{name}")
}

/// The run's `skipped` lines.
fn skipped(ran: &Value) -> Vec<String> {
    ran["detail"]["skipped"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

/// Acceptance: tags `v1` and `v2`, a compare that puts `c1` in `v1` and `c2` in `v2`: `c1` has one
/// `shipped_in` edge to `v1`, `c2` one to `v2`, and `c3`, in no tag, none. The tags are listed
/// newest first; the ranges follow their time, and the first tag's range is asked with an empty
/// `from`.
#[test]
fn each_change_is_linked_to_the_tag_whose_comparison_holds_it_and_a_change_in_none_is_not() {
    let w = world(&[1]);
    tags(
        &w,
        1,
        &[
            ("v2", "2026-01-05T00:00:00Z"),
            ("v1", "2026-01-02T00:00:00Z"),
        ],
    );
    changes(&w, 1, &["c1", "c2", "c3"]);
    compare(&w, 1, "v1", &["c1"]);
    compare(&w, 1, "v2", &["c2"]);
    create(&w, "a", "");
    let ran = run(&w, "a");
    assert_eq!(ran["detail"]["documents_applied"], 1 + 2 + 3, "{ran}");
    assert!(skipped(&ran).is_empty(), "{ran}");
    assert_eq!(
        w.lines("invocations.log"),
        [
            "projects.list",
            "tags.list 1",
            "changes.list 1",
            "compare 1 ->v1",
            "compare 1 v1->v2"
        ]
    );

    let e = edges(&w, "a");
    assert_eq!(shipped_in(&e, 1, "c1"), [tag(1, "v1")], "{e:?}");
    assert_eq!(shipped_in(&e, 1, "c2"), [tag(1, "v2")], "{e:?}");
    assert!(shipped_in(&e, 1, "c3").is_empty(), "{e:?}");
    assert_eq!(
        e.iter().filter(|(_, t, _)| t == "shipped_in").count(),
        2,
        "{e:?}"
    );
    // Each change keeps its one edge to its project.
    for c in ["c1", "c2", "c3"] {
        let change = format!("{CHANGE}1:{c}");
        let in_project: Vec<_> = e
            .iter()
            .filter(|(s, t, _)| *s == change && t == "IN_PROJECT")
            .collect();
        assert_eq!(in_project.len(), 1, "{change}: {e:?}");
    }

    // Unchanged: nothing new is applied, and no edge is added.
    let again = run(&w, "a");
    assert_eq!(again["detail"]["documents_new"], 0, "{again}");
    assert_eq!(edges(&w, "a").len(), e.len());
}

/// A change merged before its tag: run 1 lists the change and no tag, so it has no edge; run 2
/// lists the tag, and the change, unchanged itself, gains its edge.
#[test]
fn a_change_merged_before_its_tag_gains_its_edge_in_the_run_that_first_sees_the_tag() {
    let w = world(&[1]);
    changes(&w, 1, &["c1"]);
    compare(&w, 1, "v1", &["c1"]);
    create(&w, "m", "");
    let first = run(&w, "m");
    assert_eq!(first["detail"]["documents_applied"], 2, "{first}");
    assert!(shipped_in(&edges(&w, "m"), 1, "c1").is_empty());
    assert!(
        !w.lines("invocations.log")
            .iter()
            .any(|l| l.starts_with("compare")),
        "no tag, no compare"
    );

    tags(&w, 1, &[("v1", "2026-01-02T00:00:00Z")]);
    let second = run(&w, "m");
    assert!(skipped(&second).is_empty(), "{second}");
    assert_eq!(
        second["detail"]["documents_applied"], 2,
        "the tag, and the change with its link: {second}"
    );
    let e = edges(&w, "m");
    assert_eq!(shipped_in(&e, 1, "c1"), [tag(1, "v1")], "{e:?}");
    let change = format!("{CHANGE}1:c1");
    assert_eq!(
        e.iter()
            .filter(|(s, t, _)| *s == change && t == "IN_PROJECT")
            .count(),
        1,
        "{e:?}"
    );
}

/// Tags of one time are ordered by identity: `a` before `b`, so `a`'s range comes first.
#[test]
fn tags_of_one_time_are_ordered_by_identity() {
    let w = world(&[1]);
    tags(
        &w,
        1,
        &[("b", "2026-01-02T00:00:00Z"), ("a", "2026-01-02T00:00:00Z")],
    );
    changes(&w, 1, &["c1"]);
    compare(&w, 1, "a", &["c1"]);
    compare(&w, 1, "b", &["c1"]);
    create(&w, "i", "");
    run(&w, "i");
    let compares: Vec<String> = w
        .lines("invocations.log")
        .into_iter()
        .filter(|l| l.starts_with("compare"))
        .collect();
    assert_eq!(compares, ["compare 1 ->a", "compare 1 a->b"]);
    assert_eq!(shipped_in(&edges(&w, "i"), 1, "c1"), [tag(1, "a")]);
}

/// A change two ranges list is linked to the earliest tag only.
#[test]
fn a_change_in_two_ranges_is_linked_to_the_earliest_tag_only() {
    let w = world(&[1]);
    tags(
        &w,
        1,
        &[
            ("v1", "2026-01-02T00:00:00Z"),
            ("v2", "2026-01-05T00:00:00Z"),
        ],
    );
    changes(&w, 1, &["c1", "c2"]);
    compare(&w, 1, "v1", &["c1"]);
    compare(&w, 1, "v2", &["c1", "c2"]);
    create(&w, "t", "");
    run(&w, "t");
    let e = edges(&w, "t");
    assert_eq!(shipped_in(&e, 1, "c1"), [tag(1, "v1")], "{e:?}");
    assert_eq!(shipped_in(&e, 1, "c2"), [tag(1, "v2")], "{e:?}");
}

/// A compare call that fails is named in `skipped` with the reason, and the edges stored before
/// stay (`dropped: Supersede`): neither the failed range's change nor a later range's is ended.
#[test]
fn a_failed_compare_call_is_skipped_and_the_edges_stored_before_stay() {
    let w = world(&[1]);
    tags(
        &w,
        1,
        &[
            ("v1", "2026-01-02T00:00:00Z"),
            ("v2", "2026-01-05T00:00:00Z"),
        ],
    );
    changes(&w, 1, &["c1", "c2"]);
    compare(&w, 1, "v1", &["c1"]);
    compare(&w, 1, "v2", &["c2"]);
    create(&w, "f", "        dropped: Supersede");
    run(&w, "f");
    let before = edges(&w, "f");
    assert_eq!(shipped_in(&before, 1, "c1"), [tag(1, "v1")], "{before:?}");
    assert_eq!(shipped_in(&before, 1, "c2"), [tag(1, "v2")], "{before:?}");

    std::fs::write(w.root.join("fail-compare-1-v1"), "").unwrap();
    let ran = run(&w, "f");
    let lines = skipped(&ran);
    assert_eq!(lines.len(), 1, "{ran}");
    assert!(
        lines[0].contains("compare")
            && lines[0].contains("forge:projects.list:1")
            && lines[0].contains("forbidden"),
        "{ran}"
    );
    assert_eq!(ran["detail"]["retracted"].as_i64().unwrap_or(0), 0, "{ran}");
    assert_eq!(
        ran["detail"]["superseded"].as_i64().unwrap_or(0),
        0,
        "{ran}"
    );
    let after = edges(&w, "f");
    assert_eq!(shipped_in(&after, 1, "c1"), [tag(1, "v1")], "{after:?}");
    assert_eq!(shipped_in(&after, 1, "c2"), [tag(1, "v2")], "{after:?}");

    // The call answers again: each change is linked as before, by one edge, and nothing is ended.
    std::fs::remove_file(w.root.join("fail-compare-1-v1")).unwrap();
    let healed = run(&w, "f");
    assert!(skipped(&healed).is_empty(), "{healed}");
    assert_eq!(
        healed["detail"]["retracted"].as_i64().unwrap_or(0),
        0,
        "{healed}"
    );
    let e = edges(&w, "f");
    assert_eq!(shipped_in(&e, 1, "c1"), [tag(1, "v1")], "{e:?}");
    assert_eq!(shipped_in(&e, 1, "c2"), [tag(1, "v2")], "{e:?}");
    assert_eq!(e.len(), before.len(), "{e:?}");
}

/// Two projects with tags and changes of the same names: each change is linked to its own
/// project's tag, as its own project's compare says, and to no tag of the other.
#[test]
fn same_named_tags_and_changes_of_two_projects_stay_apart() {
    let w = world(&[1, 2]);
    for project in [1, 2] {
        tags(
            &w,
            project,
            &[
                ("v1", "2026-01-02T00:00:00Z"),
                ("v2", "2026-01-05T00:00:00Z"),
            ],
        );
        changes(&w, project, &["c1", "c2"]);
    }
    compare(&w, 1, "v1", &["c1"]);
    compare(&w, 1, "v2", &["c2"]);
    compare(&w, 2, "v1", &["c2"]);
    compare(&w, 2, "v2", &["c1"]);
    create(&w, "p", "");
    run(&w, "p");
    let e = edges(&w, "p");
    assert_eq!(shipped_in(&e, 1, "c1"), [tag(1, "v1")], "{e:?}");
    assert_eq!(shipped_in(&e, 1, "c2"), [tag(1, "v2")], "{e:?}");
    assert_eq!(shipped_in(&e, 2, "c1"), [tag(2, "v2")], "{e:?}");
    assert_eq!(shipped_in(&e, 2, "c2"), [tag(2, "v1")], "{e:?}");
    assert_eq!(
        e.iter().filter(|(_, t, _)| t == "shipped_in").count(),
        4,
        "{e:?}"
    );
}

/// A tag whose `order` value is no time is named in `skipped`; its project's ranges are not asked
/// for, so none of its changes gains an edge, while another project's changes are linked.
#[test]
fn a_tag_with_no_usable_order_is_skipped_and_its_projects_changes_gain_no_edge() {
    let w = world(&[1, 2]);
    tags(
        &w,
        1,
        &[("v1", "2026-01-02T00:00:00Z"), ("v2", "not a time")],
    );
    tags(&w, 2, &[("v1", "2026-01-02T00:00:00Z")]);
    for project in [1, 2] {
        changes(&w, project, &["c1"]);
        compare(&w, project, "v1", &["c1"]);
    }
    create(&w, "o", "");
    let ran = run(&w, "o");
    let lines = skipped(&ran);
    assert_eq!(lines.len(), 1, "{ran}");
    assert!(
        lines[0].contains(&tag(1, "v2")) && lines[0].contains("$.created_at"),
        "{ran}"
    );
    assert!(
        !w.lines("invocations.log")
            .iter()
            .any(|l| l.starts_with("compare 1 ")),
        "{:?}",
        w.lines("invocations.log")
    );
    let e = edges(&w, "o");
    assert!(shipped_in(&e, 1, "c1").is_empty(), "{e:?}");
    assert_eq!(shipped_in(&e, 2, "c1"), [tag(2, "v1")], "{e:?}");
}

/// A link's `tags` and `changes` name child operations of the same source: a spec naming another
/// is refused, naming the field.
#[test]
fn a_spec_whose_link_names_no_child_operation_is_refused() {
    assert!(cortex_cli::spec::parse(&spec_text("ok", "")).is_ok());
    for (field, from, to) in [
        ("tags", "tags: tags.list", "tags: releases.list"),
        ("changes", "changes: changes.list", "changes: merges.list"),
    ] {
        let text = spec_text("bad", "").replace(from, to);
        let refused = cortex_cli::spec::parse(&text).expect_err("a refusal");
        assert!(
            refused.contains(&format!("sources.projects: links[0].{field}: ")),
            "{refused}"
        );
        assert!(refused.contains("names no child operation"), "{refused}");
    }
}
