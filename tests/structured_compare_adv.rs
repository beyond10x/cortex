//! Adversary pass 1 on `story:structured-compare-links`: a structured source whose `links` link
//! each change to the first tag whose comparison holds it. Each case drives the `cortex` binary
//! against a real `ekr` with a stand-in `connectors` and asserts what the wave's decisions or the
//! documentation (`website/docs/spec-file.md`, `kind: structured`, **Links**) say. No model is
//! called.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{ekr, executable, World};
use serde_json::{json, Value};

/// A stand-in `connectors` with one ready `forge` connection. `projects.list` answers
/// `projects.json`. `tags.list` answers `tags-<project>.json`, `changes.list`
/// `changes-<project>.json` and `compare` `compare-<project>-<to>.json`, each an empty list when
/// the file is missing; page `n` above 1 of a paged call (the input's `"page"`) answers the same
/// name with `-p<n>` before `.json`. `compare` answers `forbidden` when the file
/// `fail-compare-<project>-<to>` exists. Every invocation is logged to `invocations.log`, a
/// compare as `compare <project> <from>-><to>`, and a page above 1 with ` p<n>` after it.
const CONNECTORS: &str = r#"R="@ROOT@"
ARGS="$*"
field() { printf '%s\n' "$ARGS" | sed -n "s/.*\"$1\":\"\([^\"]*\)\".*/\1/p"; }
P=$(field project); F=$(field from); T=$(field to)
N=$(printf '%s\n' "$ARGS" | sed -n 's/.*"page":\([0-9][0-9]*\).*/\1/p')
if [ -n "$N" ] && [ "$N" != 1 ]; then S="-p$N"; L=" p$N"; else S=""; L=""; fi
answer() { if [ -e "$2" ]; then B=$(cat "$2"); else B="$3"; fi; printf '{"ok":true,"result":{"adapter":"forge","operation":"%s","revision":"r","result":%s}}\n' "$1" "$B"; }
case "$ARGS" in
  *"connections list"*) echo '{"ok":true,"result":{"connections":[{"adapter":"forge","connection":"conn_test","state":"ready","revision":"rev1"}]}}' ;;
  *"connections revalidate"*) echo '{"ok":true,"result":{}}' ;;
  *"operations describe"*) echo '{"ok":true,"result":{"schema":"s","revision":"r"}}' ;;
  *"--operation projects.list"*) echo "projects.list" >> "$R/invocations.log"; answer projects.list "$R/projects.json" '{"projects":[]}' ;;
  *"--operation tags.list"*) echo "tags.list $P$L" >> "$R/invocations.log"; answer tags.list "$R/tags-$P$S.json" '{"tags":[]}' ;;
  *"--operation changes.list"*) echo "changes.list $P" >> "$R/invocations.log"; answer changes.list "$R/changes-$P.json" '{"changes":[]}' ;;
  *"--operation compare"*)
    echo "compare $P $F->$T$L" >> "$R/invocations.log"
    if [ -e "$R/fail-compare-$P-$T" ]; then echo '{"ok":false,"error":{"code":"failure","data":{"code":"forbidden","stage":"execution"}}}'; exit 1; fi
    answer compare "$R/compare-$P-$T$S.json" '{"commits":[]}' ;;
  *) echo '{"ok":false}'; exit 2 ;;
esac
"#;

/// A stand-in `claude` that logs the call and fails: a structured source never calls it.
const CLAUDE: &str = r#"echo call >> "@ROOT@/claude-calls.log"
echo '{"type":"result","subtype":"error","is_error":true}'
exit 1
"#;

const JAN_2: &str = "2026-01-02T00:00:00Z";
const JAN_5: &str = "2026-01-05T00:00:00Z";
const JAN_9: &str = "2026-01-09T00:00:00Z";

const PROJECT: &str = "forge:projects.list:";
const TAG: &str = "forge:projects.list/tags.list:";
const CHANGE: &str = "forge:projects.list/changes.list:";

fn project(id: &str) -> String {
    format!("{PROJECT}{id}")
}

fn tag(project: &str, name: &str) -> String {
    format!("{TAG}{project}:{name}")
}

fn change(project: &str, id: &str) -> String {
    format!("{CHANGE}{project}:{id}")
}

/// A world whose stand-in answers the projects of `ids`, with no tags, changes or compare answers
/// yet.
fn world(ids: &[&str]) -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(
        &w.bin.join("connectors"),
        &CONNECTORS.replace("@ROOT@", &root),
    );
    executable(&w.bin.join("claude"), &CLAUDE.replace("@ROOT@", &root));
    let projects: Vec<Value> = ids
        .iter()
        .map(|id| json!({"id": id, "name": format!("project-{id}")}))
        .collect();
    write(&w, "projects.json", json!({ "projects": projects }));
    w
}

fn write(w: &World, file: &str, value: Value) {
    std::fs::write(w.root.join(file), value.to_string()).unwrap();
}

/// `name` for page `page` of a paged answer: `<name>.json`, or `<name>-p<page>.json` above 1.
fn paged(name: &str, page: u32) -> String {
    if page <= 1 {
        format!("{name}.json")
    } else {
        format!("{name}-p{page}.json")
    }
}

/// Page `page` of project `project`'s tags, each `(name, created_at)`; a tag with `None` has no
/// `created_at`.
fn tags(w: &World, project: &str, page: u32, tags: &[(&str, Option<&str>)]) {
    let tags: Vec<Value> = tags
        .iter()
        .map(|(name, at)| match at {
            Some(at) => json!({"name": name, "created_at": at}),
            None => json!({"name": name}),
        })
        .collect();
    write(
        w,
        &paged(&format!("tags-{project}"), page),
        json!({ "tags": tags }),
    );
}

/// Project `project`'s changes, each `(sha, title)`.
fn changes(w: &World, project: &str, changes: &[(&str, &str)]) {
    let changes: Vec<Value> = changes
        .iter()
        .map(|(sha, title)| json!({"sha": sha, "title": title}))
        .collect();
    write(
        w,
        &format!("changes-{project}.json"),
        json!({ "changes": changes }),
    );
}

/// Page `page` of what the compare of project `project` up to tag `to` lists: the commits of
/// `ids`.
fn compare(w: &World, project: &str, to: &str, page: u32, ids: &[&str]) {
    let commits: Vec<Value> = ids.iter().map(|id| json!({"id": id})).collect();
    write(
        w,
        &paged(&format!("compare-{project}-{to}"), page),
        json!({ "commits": commits }),
    );
}

/// What a case varies in the spec of its one source.
#[derive(Clone, Copy)]
struct Opts {
    refresh_after_days: i64,
    supersede: bool,
    max_documents_per_run: i64,
    /// `max_pages` of a `PageNumber` paging (param `page`) on the tags child, when paged.
    tags_max_pages: Option<i64>,
    /// The same on the compare link.
    compare_max_pages: Option<i64>,
    /// Whether the changes' child is listed before the tags'.
    changes_first: bool,
}

const OPTS: Opts = Opts {
    refresh_after_days: 0,
    supersede: false,
    max_documents_per_run: 100,
    tags_max_pages: None,
    compare_max_pages: None,
    changes_first: false,
};

/// The `, page: 1` an input starts its walk with, and the `paging` line, for `max_pages`.
fn paging(max_pages: Option<i64>) -> (String, String) {
    match max_pages {
        Some(n) => (
            ", page: 1".into(),
            format!("\n                paging: {{style: PageNumber, param: page, max_pages: {n}}}"),
        ),
        None => (String::new(), String::new()),
    }
}

/// The spec of one structured source, `projects`, with tags and changes as children and one
/// compare link, `shipped_in`, between them.
fn spec_text(name: &str, o: Opts) -> String {
    let (tags_page, tags_paging) = paging(o.tags_max_pages);
    let (compare_page, compare_paging) = paging(o.compare_max_pages);
    let tags = format!(
        r#"              - operation: tags.list
                input: {{project: "{{id}}"{tags_page}}}
                records: "$.tags"
                parent: IN_PROJECT{tags_paging}
                mapping:
                  node_type: Tag
                  id: "$.name"
                  name: "$.name"
                  aliases: []
                  properties: [{{property: created_at, path: "$.created_at"}}]
                  relations: []
"#
    );
    let changes = r#"              - operation: changes.list
                input: {project: "{id}"}
                records: "$.changes"
                parent: IN_PROJECT
                mapping:
                  node_type: Change
                  id: "$.sha"
                  name: "$.title"
                  aliases: []
                  properties: [{property: title, path: "$.title"}]
                  relations: []
"#;
    let children = if o.changes_first {
        format!("{changes}{tags}")
    } else {
        format!("{tags}{changes}")
    };
    let dropped = if o.supersede {
        "        dropped: Supersede"
    } else {
        ""
    };
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
{children}            links:
              - operation: compare
                input: {{project: "{{id}}", from: "{{from.name}}", to: "{{to.name}}"{compare_page}}}
                records: "$.commits"{compare_paging}
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
    policy: {{refresh_after_days: {refresh}, change: ContentHash, max_documents_per_run: {max}, max_chars_per_document: 5000}}
serve: {{view_port: 18994}}
"#,
        ekr = ekr().display(),
        refresh = o.refresh_after_days,
        max = o.max_documents_per_run,
    )
}

fn create(w: &World, name: &str, o: Opts) {
    let path = w.root.join(format!("{name}.yaml"));
    std::fs::write(&path, spec_text(name, o)).unwrap();
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

fn skipped(ran: &Value) -> Vec<String> {
    ran["detail"]["skipped"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

/// The compare invocations the stand-in logged, in order.
fn compares(w: &World) -> Vec<String> {
    w.lines("invocations.log")
        .into_iter()
        .filter(|l| l.starts_with("compare"))
        .collect()
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

#[derive(Debug, Clone)]
struct Node {
    id: String,
    aliases: Vec<String>,
    identity: String,
}

/// A store's nodes, and its edges as `(source node id, edge type name, target node id)`.
struct Store {
    nodes: Vec<Node>,
    edges: Vec<(String, String, String)>,
}

fn store(w: &World, name: &str) -> Store {
    let snapshot = ekr_json(w, name, &["snapshot"]);
    let ontology = ekr_json(w, name, &["ontology"]);
    let edge_types: std::collections::BTreeMap<String, String> = ontology["edge_types"]
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
    Store { nodes, edges }
}

impl Store {
    fn by_identity(&self, identity: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.identity == identity)
    }

    fn by_id(&self, id: &str) -> &Node {
        self.nodes.iter().find(|n| n.id == id).unwrap()
    }

    /// The node one of whose aliases is `<name> (<identity>)`.
    fn named(&self, name: &str) -> &Node {
        let head = format!("{name} (");
        self.nodes
            .iter()
            .find(|n| n.aliases.iter().any(|a| a.starts_with(&head)))
            .unwrap_or_else(|| panic!("no node named {name}: {:?}", self.nodes))
    }

    /// The identities of the targets of `relation` edges from the node whose identity is
    /// `identity`.
    fn targets(&self, identity: &str, relation: &str) -> Vec<String> {
        let Some(node) = self.by_identity(identity) else {
            return Vec::new();
        };
        self.targets_of(&node.id, relation)
    }

    fn targets_of(&self, id: &str, relation: &str) -> Vec<String> {
        self.edges
            .iter()
            .filter(|(s, t, _)| s == id && t == relation)
            .map(|(_, _, target)| self.by_id(target).identity.clone())
            .collect()
    }
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

/// The wave's decision ("a change merged before its tag gets the edge in the run that first sees
/// the tag") and spec-file.md, **Links**: "a change applied before its tag existed is applied
/// again in the run that first sees the tag, and gains its edge". The link is part of the
/// change's text, and a changed text waits for `refresh_after_days` (spec-file.md, `policy`).
/// The documentation advises `refresh_after_days` of 1 or more to read history on a first run,
/// and the examples use 7 and 30; `tests/structured_compare.rs` uses 0 only. With 1, the change
/// applied in run 1 has no edge after run 2, the run that first sees its tag.
#[test]
fn adv_a_change_merged_before_its_tag_gains_its_edge_when_the_tag_appears_under_a_refresh_window() {
    let w = world(&["1"]);
    changes(&w, "1", &[("c1", "change c1")]);
    compare(&w, "1", "v1", 1, &["c1"]);
    create(
        &w,
        "r",
        Opts {
            refresh_after_days: 1,
            ..OPTS
        },
    );
    let first = run(&w, "r");
    assert_eq!(first["detail"]["documents_applied"], 2, "{first}");
    assert!(store(&w, "r")
        .targets(&change("1", "c1"), "shipped_in")
        .is_empty());

    tags(&w, "1", 1, &[("v1", Some(JAN_2))]);
    let second = run(&w, "r");
    assert!(skipped(&second).is_empty(), "{second}");
    assert_eq!(
        store(&w, "r").targets(&change("1", "c1"), "shipped_in"),
        [tag("1", "v1")],
        "the run that first sees v1 gives c1 its edge: {second}"
    );
}

/// The wave's decision on failures: "changes of that range get no new edge in that run, and
/// edges already stored are not ended for it". A tag with no usable `order` holds every value of
/// every change of its parent instead (`run.rs`, `failed_links` → `held`), so with `dropped:
/// Supersede` a change the parent no longer lists keeps its edge to the project, on every run
/// for as long as the tag stays unusable. That edge is no link.
#[test]
fn adv_a_failing_link_does_not_keep_a_change_its_parent_no_longer_lists() {
    let w = world(&["1"]);
    tags(&w, "1", 1, &[("v1", Some(JAN_2))]);
    changes(&w, "1", &[("c1", "change c1"), ("c2", "change c2")]);
    compare(&w, "1", "v1", 1, &["c1", "c2"]);
    create(
        &w,
        "h",
        Opts {
            supersede: true,
            ..OPTS
        },
    );
    run(&w, "h");
    let before = store(&w, "h");
    assert_eq!(
        before.targets(&change("1", "c2"), "IN_PROJECT"),
        [project("1")]
    );

    // v2 has no time at `order`, and c2 is no longer listed: two runs.
    tags(&w, "1", 1, &[("v1", Some(JAN_2)), ("v2", None)]);
    changes(&w, "1", &[("c1", "change c1")]);
    for _ in 0..2 {
        let ran = run(&w, "h");
        let lines = skipped(&ran);
        assert_eq!(lines.len(), 1, "{ran}");
        assert!(lines[0].contains(&tag("1", "v2")), "{ran}");
    }
    let after = store(&w, "h");
    assert_eq!(
        after.targets(&change("1", "c1"), "shipped_in"),
        [tag("1", "v1")],
        "the stored link is held"
    );
    assert!(
        after.targets(&change("1", "c2"), "IN_PROJECT").is_empty(),
        "c2 is no longer listed and its edge to the project is no link, yet two runs keep it: {:?}",
        after.edges
    );
}

/// Tags are ordered by the instant at `order`, earliest first (spec-file.md: "read as a `time`
/// is ... ordered by it, earliest first"), not by name and not by local time: `v10` sorts first
/// by name, and `v8` (03:00 at +05:00, 22:00 the day before in UTC) sorts after `v9` by local
/// time. Every tag pair of `tests/structured_compare.rs` sorts the same by name as by time, so a
/// sort by name alone passes that file. `c8` is in two ranges and goes to the earliest. An
/// unchanged second run with `dropped: Supersede` applies nothing and ends nothing.
#[test]
fn adv_tags_are_ordered_by_their_instant_not_their_name_or_local_time() {
    let w = world(&["1"]);
    tags(
        &w,
        "1",
        1,
        &[
            ("v10", Some("2026-01-10T00:00:00Z")),
            ("v9", Some(JAN_9)),
            ("v8", Some("2026-01-09T03:00:00+05:00")),
        ],
    );
    changes(
        &w,
        "1",
        &[
            ("c8", "change c8"),
            ("c9", "change c9"),
            ("c10", "change c10"),
        ],
    );
    compare(&w, "1", "v8", 1, &["c8"]);
    compare(&w, "1", "v9", 1, &["c8", "c9"]);
    compare(&w, "1", "v10", 1, &["c10"]);
    create(
        &w,
        "o",
        Opts {
            supersede: true,
            ..OPTS
        },
    );
    let first = run(&w, "o");
    assert!(skipped(&first).is_empty(), "{first}");
    assert_eq!(
        compares(&w),
        ["compare 1 ->v8", "compare 1 v8->v9", "compare 1 v9->v10"]
    );
    let s = store(&w, "o");
    assert_eq!(
        s.targets(&change("1", "c8"), "shipped_in"),
        [tag("1", "v8")]
    );
    assert_eq!(
        s.targets(&change("1", "c9"), "shipped_in"),
        [tag("1", "v9")]
    );
    assert_eq!(
        s.targets(&change("1", "c10"), "shipped_in"),
        [tag("1", "v10")]
    );

    let again = run(&w, "o");
    assert_eq!(again["detail"]["documents_new"], 0, "{again}");
    assert!(again["detail"]["retracted"].is_null(), "{again}");
    assert!(again["detail"]["superseded"].is_null(), "{again}");
    assert_eq!(store(&w, "o").edges.len(), s.edges.len());
}

/// The compare call is paged as its `paging` says (spec-file.md: "`paging` optional, as for the
/// input"): a change on its second page is linked like one on its first. No case of
/// `tests/structured_compare.rs` pages a compare.
#[test]
fn adv_a_paged_compare_links_the_changes_of_every_page() {
    let w = world(&["1"]);
    tags(&w, "1", 1, &[("v1", Some(JAN_2)), ("v2", Some(JAN_5))]);
    changes(
        &w,
        "1",
        &[
            ("c1", "change c1"),
            ("c2", "change c2"),
            ("c3", "change c3"),
        ],
    );
    compare(&w, "1", "v1", 1, &["c1"]);
    compare(&w, "1", "v1", 2, &["c2"]);
    compare(&w, "1", "v2", 1, &["c3"]);
    create(
        &w,
        "g",
        Opts {
            compare_max_pages: Some(3),
            ..OPTS
        },
    );
    let ran = run(&w, "g");
    assert!(skipped(&ran).is_empty(), "{ran}");
    assert!(ran["detail"]["stopped"].is_null(), "{ran}");
    assert_eq!(
        compares(&w),
        [
            "compare 1 ->v1",
            "compare 1 ->v1 p2",
            "compare 1 ->v1 p3",
            "compare 1 v1->v2",
            "compare 1 v1->v2 p2"
        ]
    );
    let s = store(&w, "g");
    assert_eq!(
        s.targets(&change("1", "c1"), "shipped_in"),
        [tag("1", "v1")]
    );
    assert_eq!(
        s.targets(&change("1", "c2"), "shipped_in"),
        [tag("1", "v1")]
    );
    assert_eq!(
        s.targets(&change("1", "c3"), "shipped_in"),
        [tag("1", "v2")]
    );
}

/// A compare call that leaves pages unread is named in `skipped` and links nothing, and no later
/// range is asked for (spec-file.md: "a compare call that fails ... or leaves pages unread"; "the
/// changes the calls before it linked keep their links; the parent's other changes get none").
#[test]
fn adv_a_compare_with_pages_left_is_skipped_and_nothing_after_it_is_linked() {
    let w = world(&["1"]);
    tags(
        &w,
        "1",
        1,
        &[
            ("v1", Some(JAN_2)),
            ("v2", Some(JAN_5)),
            ("v3", Some(JAN_9)),
        ],
    );
    changes(
        &w,
        "1",
        &[
            ("c1", "change c1"),
            ("c2", "change c2"),
            ("c3", "change c3"),
        ],
    );
    compare(&w, "1", "v1", 1, &["c1"]);
    compare(&w, "1", "v2", 1, &["c2"]);
    compare(&w, "1", "v2", 2, &["c3"]);
    compare(&w, "1", "v3", 1, &["c3"]);
    create(
        &w,
        "l",
        Opts {
            compare_max_pages: Some(2),
            ..OPTS
        },
    );
    let ran = run(&w, "l");
    let lines = skipped(&ran);
    assert_eq!(lines.len(), 1, "{ran}");
    assert!(
        lines[0].contains(&project("1"))
            && lines[0].contains(&tag("1", "v2"))
            && lines[0].contains("max_pages 2"),
        "{ran}"
    );
    assert!(
        !compares(&w).iter().any(|l| l.contains("->v3")),
        "{:?}",
        compares(&w)
    );
    let s = store(&w, "l");
    assert_eq!(
        s.targets(&change("1", "c1"), "shipped_in"),
        [tag("1", "v1")]
    );
    assert!(s.targets(&change("1", "c2"), "shipped_in").is_empty());
    assert!(s.targets(&change("1", "c3"), "shipped_in").is_empty());
}

/// Tags not all read make no compare call for their parent, which is named in `skipped`
/// (spec-file.md: "tags not all read (their child call failed or left pages unread)"). Ranges
/// built from the first page alone would make `v2` the first tag and link `c1` to it.
#[test]
fn adv_tags_left_unread_make_no_compare_call_and_are_skipped() {
    let w = world(&["1"]);
    tags(&w, "1", 1, &[("v2", Some(JAN_5))]);
    tags(&w, "1", 2, &[("v1", Some(JAN_2))]);
    changes(&w, "1", &[("c1", "change c1"), ("c2", "change c2")]);
    compare(&w, "1", "v1", 1, &["c1"]);
    compare(&w, "1", "v2", 1, &["c1", "c2"]);
    create(
        &w,
        "u",
        Opts {
            tags_max_pages: Some(1),
            ..OPTS
        },
    );
    let ran = run(&w, "u");
    let lines = skipped(&ran);
    assert_eq!(lines.len(), 1, "{ran}");
    assert!(
        lines[0].contains("tags.list") && lines[0].contains("not all read"),
        "{ran}"
    );
    assert!(compares(&w).is_empty(), "{:?}", compares(&w));
    let s = store(&w, "u");
    assert!(s.targets(&change("1", "c1"), "shipped_in").is_empty());
    assert!(s.targets(&change("1", "c2"), "shipped_in").is_empty());
}

/// spec-file.md: "For a parent with no change read, no compare call is made".
#[test]
fn adv_a_parent_with_tags_and_no_change_makes_no_compare_call() {
    let w = world(&["1"]);
    tags(&w, "1", 1, &[("v1", Some(JAN_2)), ("v2", Some(JAN_5))]);
    create(&w, "n", OPTS);
    let ran = run(&w, "n");
    assert!(skipped(&ran).is_empty(), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 3, "{ran}");
    assert!(compares(&w).is_empty(), "{:?}", compares(&w));
}

/// Ids masking would change still link: a parent `top-secret` (its part of a child's identity is
/// written in hex), a change id and a tag name that are each an assigned credential (their
/// identities fall back to the id's digest). The change is matched to the compare record through
/// those identities, every identity in the store is one masking leaves alone, and no raw
/// credential value is written under the cortex home.
#[test]
fn adv_ids_masking_would_change_link_through_clean_identities_and_are_never_stored() {
    // Assembled at runtime so no credential-shaped literal sits in the source.
    let tag_value = "r".repeat(12);
    let change_value = "q".repeat(14);
    let tag_name = format!("{}_{}={tag_value}", "api", "key");
    let change_id = format!("{}{}={change_value}", "pass", "word");
    let w = world(&["top-secret"]);
    tags(
        &w,
        "top-secret",
        1,
        &[("v1", Some(JAN_2)), (&tag_name, Some(JAN_5))],
    );
    changes(
        &w,
        "top-secret",
        &[("c1", "change one"), (&change_id, "change two")],
    );
    compare(&w, "top-secret", "v1", 1, &["c1"]);
    compare(&w, "top-secret", &tag_name, 1, &[&change_id]);
    create(&w, "k", OPTS);
    let ran = run(&w, "k");
    assert!(skipped(&ran).is_empty(), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 1 + 2 + 2, "{ran}");

    let s = store(&w, "k");
    let (one, two) = (s.named("change one"), s.named("change two"));
    let v1 = s.named("v1");
    let masked = s.named(&format!("{}_{}=[masked:secret-assignment]", "api", "key"));
    assert_eq!(
        s.targets_of(&one.id, "shipped_in"),
        [v1.identity.as_str()],
        "{:?}",
        s.edges
    );
    assert_eq!(
        s.targets_of(&two.id, "shipped_in"),
        [masked.identity.as_str()],
        "{:?}",
        s.edges
    );
    for n in &s.nodes {
        assert_eq!(
            cortex_cli::mask::mask_key(&n.identity).1,
            0,
            "an identity masking would change: {}",
            n.identity
        );
    }
    for file in files(&w.home) {
        let bytes = std::fs::read(&file).unwrap();
        for value in [&tag_value, &change_value] {
            assert!(
                !bytes.windows(value.len()).any(|b| b == value.as_bytes()),
                "{} holds a raw credential value",
                file.display()
            );
        }
    }
}

/// spec-file.md, **Links**: "With `dropped: Supersede`, a link a change no longer has is ended as
/// any value is." `c1` is in `v1`'s range on run 1 and in `v2`'s on run 2: its edge to `v1` goes
/// and its edge to `v2` comes, one edge in all. No case of `tests/structured_compare.rs` ends a
/// link.
#[test]
fn adv_a_link_a_change_no_longer_has_is_ended_under_supersede() {
    let w = world(&["1"]);
    tags(&w, "1", 1, &[("v1", Some(JAN_2)), ("v2", Some(JAN_5))]);
    changes(&w, "1", &[("c1", "change c1")]);
    compare(&w, "1", "v1", 1, &["c1"]);
    create(
        &w,
        "e",
        Opts {
            supersede: true,
            ..OPTS
        },
    );
    run(&w, "e");
    assert_eq!(
        store(&w, "e").targets(&change("1", "c1"), "shipped_in"),
        [tag("1", "v1")]
    );

    compare(&w, "1", "v1", 1, &[]);
    compare(&w, "1", "v2", 1, &["c1"]);
    let ran = run(&w, "e");
    assert!(skipped(&ran).is_empty(), "{ran}");
    assert_eq!(ran["detail"]["retracted"], 1, "{ran}");
    let s = store(&w, "e");
    assert_eq!(
        s.targets(&change("1", "c1"), "shipped_in"),
        [tag("1", "v2")],
        "{:?}",
        s.edges
    );
}

/// A spec may list the changes' child before the tags'. With `max_documents_per_run` 2, the
/// first run applies the project and the change, which names its tag before the tag is applied;
/// the next run applies the tag. The change ends with one `shipped_in` edge, to the one node of
/// `v1`, and `v1` has its own value.
#[test]
fn adv_a_change_applied_before_its_tag_names_the_node_the_tag_later_becomes() {
    let w = world(&["1"]);
    tags(&w, "1", 1, &[("v1", Some(JAN_2))]);
    changes(&w, "1", &[("c1", "change c1")]);
    compare(&w, "1", "v1", 1, &["c1"]);
    create(
        &w,
        "b",
        Opts {
            changes_first: true,
            max_documents_per_run: 2,
            ..OPTS
        },
    );
    let first = run(&w, "b");
    assert_eq!(first["detail"]["documents_applied"], 2, "{first}");
    let second = run(&w, "b");
    assert_eq!(second["detail"]["documents_applied"], 1, "{second}");
    let s = store(&w, "b");
    let v1: Vec<&Node> = s
        .nodes
        .iter()
        .filter(|n| n.identity == tag("1", "v1"))
        .collect();
    assert_eq!(v1.len(), 1, "{:?}", s.nodes);
    assert_eq!(
        s.targets(&change("1", "c1"), "shipped_in"),
        [tag("1", "v1")]
    );
    assert_eq!(s.targets(&tag("1", "v1"), "IN_PROJECT"), [project("1")]);
}
