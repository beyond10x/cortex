//! A `structured` source imports records without a model call (`story:structured-source`): its
//! stand-in `connectors` operation answers records, each record becomes a node by the source's
//! `mapping`, with its mapped properties and relations, every fact citing the evidence cortex
//! issued for that record. The stand-in `claude` fails when called, and is never called.

mod common;

use std::path::PathBuf;
use std::process::Command;

use common::{ekr, executable, World};
use serde_json::Value;

/// A stand-in `connectors` with one ready `directory` connection whose `people.list` answers the
/// file `people.json` as `{"people": [...]}`, logging each invocation to `people.log`.
const CONNECTORS: &str = r#"R="@ROOT@"
case "$*" in
  *"connections list"*) echo '{"ok":true,"result":{"connections":[{"adapter":"directory","connection":"conn_test","state":"ready","revision":"rev1"}]}}' ;;
  *"connections revalidate"*) echo '{"ok":true,"result":{}}' ;;
  *"operations describe"*) echo '{"ok":true,"result":{"schema":"s","revision":"r"}}' ;;
  *"--operation people.list"*)
    echo "$*" >> "$R/people.log"
    printf '{"ok":true,"result":{"adapter":"directory","operation":"people.list","revision":"r","result":{"people":%s}}}\n' "$(cat "$R/people.json")" ;;
  *) echo '{"ok":false}'; exit 2 ;;
esac
"#;

/// A stand-in `claude` that logs the call and fails.
const CLAUDE: &str = r#"echo call >> "@ROOT@/claude-calls.log"
echo '{"type":"result","subtype":"error","is_error":true}'
exit 1
"#;

/// Three people; Grace reports to Ada, the one relation the mapping yields.
const PEOPLE: &str = r#"[
 {"id": "P-1", "name": "Ada Lovelace", "handle": "ada", "role": "Engineer", "contact": {"email": "ada@example.org"}},
 {"id": "P-2", "name": "Grace Hopper", "handle": "grace", "role": "Admiral", "manager": "Ada Lovelace", "contact": {"email": "grace@example.org"}},
 {"id": "P-3", "name": "Linus Example", "role": "Maintainer"}
]"#;

const MAPPING: &str = r#"        records: "$.people"
        mapping:
          node_type: Person
          id: "$.id"
          name: "$.name"
          aliases: ["$.handle"]
          properties:
            - {property: role, path: "$.role"}
            - {property: email, path: "$.contact.email"}
          relations:
            - {relation: REPORTS_TO, target_type: Person, target_name: "$.manager"}"#;

fn world(people: &str) -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(
        &w.bin.join("connectors"),
        &CONNECTORS.replace("@ROOT@", &root),
    );
    executable(&w.bin.join("claude"), &CLAUDE.replace("@ROOT@", &root));
    std::fs::write(w.root.join("people.json"), people).unwrap();
    w
}

/// The source's input: the stand-in `people.list`.
const FROM_CONNECTORS: &str = "from: connectors\n          value: {adapter: directory, connection: conn_test, operation: people.list, inputs: [{}]}";

/// Creates instance `name` with one structured source, `people`, and `extra` (a top-level block
/// such as a `redaction` policy) after `serve`.
fn create(w: &World, name: &str, extra: &str) {
    create_from(w, name, FROM_CONNECTORS, extra);
}

/// As `create`, with `input` as the source's `input` block.
fn create_from(w: &World, name: &str, input: &str, extra: &str) {
    let path = w.root.join(format!("{name}.yaml"));
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: {name}
description: Test brain.
ekr: {{version: "0.0.30", bin: "{ekr}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
  - name: people
    schedule: daily
    settings:
      kind: structured
      value:
        input:
          {input}
{MAPPING}
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
serve: {{view_port: 18995}}
{extra}"#,
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

fn last_log_line(w: &World, name: &str) -> Value {
    let text = std::fs::read_to_string(instance(w, name).join("cortex.log")).unwrap();
    serde_json::from_str(text.lines().last().expect("a log line")).unwrap()
}

/// The store's active facts, read back through `ekr sample` (`ekr.fact-sample/1`).
fn stored_facts(w: &World, name: &str) -> Vec<Value> {
    let dir = instance(w, name);
    let out = Command::new(ekr())
        .args(["sample", "--seed", "1", "--size", "1000"])
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
    let sample: Value = serde_json::from_slice(&out.stdout).unwrap();
    sample["items"].as_array().cloned().unwrap_or_default()
}

/// The facts of `ekr sample` as `(subject, predicate, value or object)` triples of text: a
/// property's value, or a relation's object node by name.
fn triples(facts: &[Value]) -> Vec<(String, String, String)> {
    facts
        .iter()
        .map(|f| {
            let text = |v: &Value| match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            let object = match f["assertion"]["object_kind"].as_str() {
                Some("Node") => &f["object_name"],
                _ => &f["assertion"]["object_value"]["value"],
            };
            (
                text(&f["subject_name"]),
                text(&f["predicate_name"]),
                text(object),
            )
        })
        .collect()
}

/// The node names of `PEOPLE`: each record's name and its identity, `<adapter>:<operation>:<id>`.
const ADA: &str = "Ada Lovelace (directory:people.list:P-1)";
const GRACE: &str = "Grace Hopper (directory:people.list:P-2)";
const LINUS: &str = "Linus Example (directory:people.list:P-3)";

#[test]
fn a_structured_source_applies_mapped_records_with_no_model_call_and_no_cost() {
    let w = world(PEOPLE);
    create(&w, "s", "");
    let before = w.head("s");

    let (code, ran) = w.cortex(&["run", "s/people"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_new"], 3, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 3, "{ran}");
    assert_eq!(ran["detail"]["facts_refused"], 0, "{ran}");
    assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");
    assert_eq!(ran["detail"]["cost_usd"], "0.0000", "{ran}");
    assert_eq!(last_log_line(&w, "s")["cost_usd"], 0.0);
    assert!(
        w.lines("claude-calls.log").is_empty(),
        "the stand-in model was called"
    );
    assert_eq!(w.lines("people.log").len(), 1);
    assert!(w.head("s") > before, "the run committed to the store");

    let facts = stored_facts(&w, "s");
    let found = triples(&facts);
    for (who, role, email) in [
        (ADA, "Engineer", Some("ada@example.org")),
        (GRACE, "Admiral", Some("grace@example.org")),
        (LINUS, "Maintainer", None),
    ] {
        assert!(
            found.contains(&(who.into(), "role".into(), role.into())),
            "{who} has role {role}: {found:?}"
        );
        if let Some(email) = email {
            assert!(
                found.contains(&(who.into(), "email".into(), email.into())),
                "{who} has email {email}: {found:?}"
            );
        }
    }
    // Three nodes, all of one type; the relation's object is one of them, not a fourth.
    let people: std::collections::BTreeSet<_> = facts
        .iter()
        .map(|f| f["subject"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(people.len(), 3, "three nodes: {facts:?}");
    let types: std::collections::BTreeSet<_> = facts
        .iter()
        .map(|f| f["subject_type"].to_string())
        .collect();
    assert_eq!(types.len(), 1, "one node type: {facts:?}");
    for f in facts
        .iter()
        .filter(|f| f["assertion"]["object_kind"] == "Node")
    {
        let object = f["assertion"]["object_ref"].as_str().unwrap();
        assert!(people.contains(object), "{f}");
    }
    // Every fact cites the evidence cortex issued for its own record.
    let ids = [(ADA, "P-1"), (GRACE, "P-2"), (LINUS, "P-3")];
    for f in &facts {
        let (_, id) = ids
            .iter()
            .find(|(name, _)| f["subject_name"] == *name)
            .expect("a known subject");
        let evidence = f["evidence"].as_array().unwrap();
        assert_eq!(evidence.len(), 1, "{f}");
        assert_eq!(
            evidence[0]["locator"],
            format!("record:directory:people.list:{id}"),
            "{f}"
        );
    }
    let relations: Vec<_> = found.iter().filter(|(_, p, _)| p == "REPORTS_TO").collect();
    assert_eq!(
        relations,
        [&(GRACE.to_string(), "REPORTS_TO".to_string(), ADA.to_string())],
        "{facts:?}"
    );

    // Unchanged records: nothing new, nothing applied, still no model call.
    let head = w.head("s");
    let (code, again) = w.cortex(&["run", "s/people"]);
    assert_eq!(
        (code, again["detail"]["documents_new"].as_i64()),
        (0, Some(0)),
        "{again}"
    );
    assert_eq!(w.head("s"), head);
    assert!(w.lines("claude-calls.log").is_empty());

    // A changed record is applied again, onto the same node, and the mapping's ontology, declared
    // again, applies cleanly. EKR 0.0.30 does not supersede yet: the new value is added and the
    // old one stays active beside it (spec-file.md, `kind: structured`).
    std::fs::write(
        w.root.join("people.json"),
        PEOPLE.replace("Maintainer", "Lead"),
    )
    .unwrap();
    let (code, changed) = w.cortex(&["run", "s/people"]);
    assert_eq!(
        (code, changed["detail"]["documents_applied"].as_i64()),
        (0, Some(1)),
        "{changed}"
    );
    assert_eq!(changed["detail"]["parts_rejected"], 0, "{changed}");
    let facts = stored_facts(&w, "s");
    let mut roles: Vec<String> = triples(&facts)
        .into_iter()
        .filter(|(who, p, _)| who == LINUS && p == "role")
        .map(|(.., role)| role)
        .collect();
    roles.sort();
    assert_eq!(roles, ["Lead", "Maintainer"], "{facts:?}");
    let nodes: std::collections::BTreeSet<_> = facts
        .iter()
        .map(|f| f["subject"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(nodes, people, "no new node for a changed record");
    assert!(w.lines("claude-calls.log").is_empty());
}

#[test]
fn credentials_and_irreversible_rules_in_a_record_never_reach_the_store() {
    let people = r#"[
 {"id": "P-1", "name": "Ada Lovelace", "role": "password=hunter2-hunter2", "contact": {"email": "ada@example.org"}},
 {"id": "P-2", "name": "Grace Hopper", "role": "holds key AK-1234ABCD", "contact": {"email": "grace@example.org"}}
]"#;
    let w = world(people);
    create(
        &w,
        "c",
        "redaction:\n  classes: [Email]\n  rules:\n    - {name: api-key, pattern: \"AK-[0-9A-F]{8}\", replacement: \"[api key]\"}\n",
    );
    let (code, ran) = w.cortex(&["run", "c/people"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
    assert!(w.lines("claude-calls.log").is_empty());

    let facts = stored_facts(&w, "c");
    let all = serde_json::to_string(&facts).unwrap();
    assert!(
        !all.contains("hunter2-hunter2"),
        "a credential was stored: {all}"
    );
    assert!(
        !all.contains("AK-1234ABCD"),
        "an irreversible match was stored: {all}"
    );
    let triples = triples(&facts);
    assert!(
        triples.contains(&(
            "Grace Hopper (directory:people.list:P-2)".into(),
            "role".into(),
            "holds key [api key]".into()
        )),
        "{triples:?}"
    );
    // Pseudonymisation is for what a model is shown; with no model the store keeps the original.
    assert!(
        triples.contains(&(
            "Ada Lovelace (directory:people.list:P-1)".into(),
            "email".into(),
            "ada@example.org".into()
        )),
        "{triples:?}"
    );
    // The evidence each record is stored as is cleaned the same way.
    let evidence = std::fs::read_dir(instance(&w, "c").join("runs"))
        .unwrap()
        .flat_map(|r| std::fs::read_dir(r.unwrap().path()).unwrap())
        .map(|b| std::fs::read_to_string(b.unwrap().path().join("extraction.yaml")).unwrap())
        .collect::<String>();
    assert!(!evidence.is_empty());
    assert!(!evidence.contains("AK-1234ABCD"));
}

#[test]
fn a_structured_source_reading_files_fails_naming_the_story_that_builds_it() {
    let w = world(PEOPLE);
    std::fs::create_dir_all(w.root.join("records")).unwrap();
    create_from(
        &w,
        "f",
        "from: files\n          value: {paths: [records], glob: \"**/*.json\"}",
        "",
    );
    let (code, ran) = w.cortex(&["run", "f/people"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("fetch-failed")),
        "{ran}"
    );
    assert!(
        ran["detail"]["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("story:structured-from-files-and-drops"),
        "{ran}"
    );
    assert!(w.lines("claude-calls.log").is_empty());
}

/// The adversary's cases of pass 1: identity, paging, values and repeats, against the same
/// stand-ins with a stand-in `connectors` that answers any operation from a file.
mod cases {
    use std::collections::BTreeSet;
    use std::path::PathBuf;
    use std::process::Command;

    use crate::common::{ekr, executable, World};
    use serde_json::Value;

    /// A stand-in `connectors`: operation `<op>` answers the file `<op>.json` as `{"people": [...]}`,
    /// logging each invocation to `<op>.log`. When the directory `pages-<op>` exists, it answers
    /// `pages-<op>/<n>.json` for the input's `"page":<n>` (1 when absent), and `[]` past the last.
    const CONNECTORS: &str = r#"R="@ROOT@"
case "$*" in
  *"connections list"*) echo '{"ok":true,"result":{"connections":[{"adapter":"directory","connection":"conn_test","state":"ready","revision":"rev1"}]}}' ;;
  *"connections revalidate"*) echo '{"ok":true,"result":{}}' ;;
  *"operations describe"*) echo '{"ok":true,"result":{"schema":"s","revision":"r"}}' ;;
  *"--operation "*)
    OP=$(printf '%s' "$*" | sed -n 's/.*--operation \([^ ]*\).*/\1/p')
    echo "$*" >> "$R/$OP.log"
    if [ -d "$R/pages-$OP" ]; then
      PAGE=$(printf '%s' "$*" | sed -n 's/.*"page":\([0-9]*\).*/\1/p'); [ -n "$PAGE" ] || PAGE=1
      if [ -f "$R/pages-$OP/$PAGE.json" ]; then BODY=$(cat "$R/pages-$OP/$PAGE.json"); else BODY='[]'; fi
    else
      BODY=$(cat "$R/$OP.json")
    fi
    printf '{"ok":true,"result":{"adapter":"directory","operation":"%s","revision":"r","result":{"people":%s}}}\n' "$OP" "$BODY" ;;
  *) echo '{"ok":false}'; exit 2 ;;
esac
"#;

    const CLAUDE: &str = r#"echo call >> "@ROOT@/claude-calls.log"
echo '{"type":"result","subtype":"error","is_error":true}'
exit 1
"#;

    const MAPPING: &str = r#"        mapping:
          node_type: Person
          id: "$.id"
          name: "$.name"
          aliases: ["$.handle"]
          properties:
            - {property: role, path: "$.role"}
            - {property: email, path: "$.contact.email"}
          relations:
            - {relation: REPORTS_TO, target_type: Person, target_name: "$.manager"}"#;

    fn world() -> World {
        let w = World::new();
        let root = w.root.display().to_string();
        executable(
            &w.bin.join("connectors"),
            &CONNECTORS.replace("@ROOT@", &root),
        );
        executable(&w.bin.join("claude"), &CLAUDE.replace("@ROOT@", &root));
        w
    }

    fn records(w: &World, op: &str, json: &str) {
        std::fs::write(w.root.join(format!("{op}.json")), json).unwrap();
    }

    /// One structured source `name` reading operation `op`, with `mapping` and `paging` (a line
    /// such as `paging: {...}` indented as a field of the input value, or empty).
    fn source(name: &str, op: &str, mapping: &str, paging: &str) -> String {
        format!(
            r#"  - name: {name}
    schedule: daily
    settings:
      kind: structured
      value:
        input:
          from: connectors
          value:
            adapter: directory
            connection: conn_test
            operation: {op}
            inputs: [{{}}]
            {paging}
        records: "$.people"
{mapping}
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 1000, max_chars_per_document: 5000}}
"#
        )
    }

    fn create(w: &World, name: &str, sources: &str, extra: &str) {
        let path = w.root.join(format!("{name}.yaml"));
        std::fs::write(
            &path,
            format!(
                r#"format: cortex.instance/1
name: {name}
description: Test brain.
ekr: {{version: "0.0.30", bin: "{ekr}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
{sources}serve: {{view_port: 18994}}
{extra}"#,
                ekr = ekr().display()
            ),
        )
        .unwrap();
        let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
        assert_eq!(code, 0, "{created}");
    }

    fn people(w: &World, name: &str, extra: &str) {
        create(
            w,
            name,
            &source("people", "people.list", MAPPING, ""),
            extra,
        );
    }

    fn run(w: &World, target: &str) -> Value {
        let (code, ran) = w.cortex(&["run", target]);
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

    fn stored_facts(w: &World, name: &str) -> Vec<Value> {
        let dir = instance(w, name);
        let out = Command::new(ekr())
            .args(["sample", "--seed", "1", "--size", "1000"])
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
        let sample: Value = serde_json::from_slice(&out.stdout).unwrap();
        sample["items"].as_array().cloned().unwrap_or_default()
    }

    fn text(v: &Value) -> String {
        match v {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        }
    }

    /// `(subject node id, subject name, predicate, value or object node id)` of each fact.
    fn quads(facts: &[Value]) -> Vec<(String, String, String, String)> {
        facts
            .iter()
            .map(|f| {
                let object = match f["assertion"]["object_kind"].as_str() {
                    Some("Node") => text(&f["assertion"]["object_ref"]),
                    _ => text(&f["assertion"]["object_value"]["value"]),
                };
                (
                    text(&f["subject"]),
                    text(&f["subject_name"]),
                    text(&f["predicate_name"]),
                    object,
                )
            })
            .collect()
    }

    /// The node that holds `predicate` = `value`, which must be exactly one.
    fn node_with(facts: &[Value], predicate: &str, value: &str) -> String {
        let found: Vec<_> = quads(facts)
            .into_iter()
            .filter(|(_, _, p, v)| p == predicate && v == value)
            .map(|(n, ..)| n)
            .collect();
        assert_eq!(
            found.len(),
            1,
            "one node with {predicate}={value}: {facts:#?}"
        );
        found[0].clone()
    }

    fn values_of(facts: &[Value], node: &str, predicate: &str) -> Vec<String> {
        quads(facts)
            .into_iter()
            .filter(|(n, _, p, _)| n == node && p == predicate)
            .map(|(.., v)| v)
            .collect()
    }

    fn nodes(facts: &[Value]) -> BTreeSet<String> {
        facts.iter().map(|f| text(&f["subject"])).collect()
    }

    /// Two different people with the same name are two records with two ids: two nodes, each with
    /// its own role. A directory with two "John Smith"s is ordinary.
    #[test]
    fn two_records_with_one_name_and_two_ids_are_two_nodes() {
        let w = world();
        records(
            &w,
            "people.list",
            r#"[{"id": "P-1", "name": "John Smith", "role": "Engineer"},
            {"id": "P-2", "name": "John Smith", "role": "Accountant"}]"#,
        );
        people(&w, "dup", "");
        let ran = run(&w, "dup/people");
        assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
        assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");
        let facts = stored_facts(&w, "dup");
        let engineer = node_with(&facts, "role", "Engineer");
        let accountant = node_with(&facts, "role", "Accountant");
        assert_ne!(
            engineer, accountant,
            "two people merged into one node: {facts:#?}"
        );
    }

    /// A record whose name changes stays the same node (spec-file.md, `kind: structured`).
    #[test]
    fn a_renamed_record_stays_the_same_node() {
        let w = world();
        records(
            &w,
            "people.list",
            r#"[{"id": "P-1", "name": "Ada Byron", "role": "Engineer"}]"#,
        );
        people(&w, "ren", "");
        run(&w, "ren/people");
        let before = nodes(&stored_facts(&w, "ren"));
        assert_eq!(before.len(), 1);

        records(
            &w,
            "people.list",
            r#"[{"id": "P-1", "name": "Ada Lovelace", "role": "Countess"}]"#,
        );
        let ran = run(&w, "ren/people");
        assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
        assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");
        let facts = stored_facts(&w, "ren");
        assert_eq!(
            nodes(&facts),
            before,
            "the renamed record is a new node: {facts:#?}"
        );
    }

    /// One record's id equal to another record's alias: still two people, two nodes.
    #[test]
    fn an_id_equal_to_another_records_alias_does_not_merge_them() {
        let w = world();
        records(
            &w,
            "people.list",
            r#"[{"id": "ada", "name": "Ada Lovelace", "role": "Engineer"},
            {"id": "P-2", "name": "Bob Builder", "handle": "ada", "role": "Builder"}]"#,
        );
        people(&w, "col", "");
        let ran = run(&w, "col/people");
        assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
        assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");
        let facts = stored_facts(&w, "col");
        let ada = node_with(&facts, "role", "Engineer");
        let bob = node_with(&facts, "role", "Builder");
        assert_ne!(
            ada, bob,
            "two people merged through an id/alias collision: {facts:#?}"
        );
    }

    /// Two structured sources over two directories that both number their records from 1: the
    /// record `1` of each is a different person, and each source's facts stay on its own node.
    #[test]
    fn two_sources_with_overlapping_numeric_ids_keep_their_people_apart() {
        let w = world();
        records(
            &w,
            "people.list",
            r#"[{"id": 1, "name": "Ada Lovelace", "role": "Engineer"}]"#,
        );
        records(
            &w,
            "staff.list",
            r#"[{"id": 1, "name": "Grace Hopper", "role": "Admiral"}]"#,
        );
        let sources = format!(
            "{}{}",
            source("people", "people.list", MAPPING, ""),
            source("staff", "staff.list", MAPPING, "")
        );
        create(&w, "two", &sources, "");
        run(&w, "two/people");
        let ran = run(&w, "two/staff");
        assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");
        let facts = stored_facts(&w, "two");
        let ada = node_with(&facts, "role", "Engineer");
        let grace = node_with(&facts, "role", "Admiral");
        assert_ne!(
            ada, grace,
            "two people merged through a shared numeric id: {facts:#?}"
        );
    }

    /// A record that names itself as its own manager: the run still applies every record and its
    /// properties; at most the self-relation is dropped.
    #[test]
    fn a_relation_to_self_does_not_cost_the_batch() {
        let w = world();
        records(
            &w,
            "people.list",
            r#"[{"id": "P-1", "name": "Ada Lovelace", "role": "Engineer"},
            {"id": "P-2", "name": "Grace Hopper", "role": "Admiral", "manager": "Grace Hopper"}]"#,
        );
        people(&w, "slf", "");
        let ran = run(&w, "slf/people");
        assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
        let facts = stored_facts(&w, "slf");
        node_with(&facts, "role", "Engineer");
        node_with(&facts, "role", "Admiral");
    }

    /// A relation by id to a record applied by an earlier run (and unchanged since, so not in this
    /// run's batch) lands on that record's node, not on a new node named by the id.
    #[test]
    fn a_relation_by_id_to_a_record_of_an_earlier_run_reaches_its_node() {
        let w = world();
        let ada = r#"{"id": "P-1", "name": "Ada Lovelace", "role": "Engineer"}"#;
        records(&w, "people.list", &format!("[{ada}]"));
        people(&w, "lat", "");
        run(&w, "lat/people");
        records(
            &w,
            "people.list",
            &format!(
                r#"[{ada}, {{"id": "P-2", "name": "Grace Hopper", "role": "Admiral", "manager": "P-1"}}]"#
            ),
        );
        let ran = run(&w, "lat/people");
        assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
        let facts = stored_facts(&w, "lat");
        let ada = node_with(&facts, "role", "Engineer");
        let grace = node_with(&facts, "role", "Admiral");
        assert_eq!(
            values_of(&facts, &grace, "REPORTS_TO"),
            [ada],
            "the relation by id missed Ada's node: {facts:#?}"
        );
        assert_eq!(nodes(&facts).len(), 2, "a third node: {facts:#?}");
    }

    /// A relation by id to a record that arrives in a later batch of the same run lands on that
    /// record's node. The filler pushes the target past one batch of 60 000 characters.
    #[test]
    fn a_relation_by_id_to_a_record_of_a_later_batch_reaches_its_node() {
        let w = world();
        let filler: Vec<String> = (0..8)
            .map(|i| {
                format!(
                    r#"{{"id": "F-{i}", "name": "Filler {i}", "role": "{}"}}"#,
                    "x".repeat(9_000)
                )
            })
            .collect();
        let json = format!(
            r#"[{{"id": "P-2", "name": "Grace Hopper", "role": "Admiral", "manager": "P-1"}}, {},
            {{"id": "P-1", "name": "Ada Lovelace", "role": "Engineer"}}]"#,
            filler.join(", ")
        );
        records(&w, "people.list", &json);
        people(&w, "bat", "");
        let ran = run(&w, "bat/people");
        assert_eq!(ran["detail"]["documents_applied"], 10, "{ran}");
        let batches = std::fs::read_dir(instance(&w, "bat").join("runs"))
            .unwrap()
            .flat_map(|r| std::fs::read_dir(r.unwrap().path()).unwrap())
            .count();
        assert!(batches > 1, "the case needs more than one batch");
        let facts = stored_facts(&w, "bat");
        let ada = node_with(&facts, "role", "Engineer");
        let grace = node_with(&facts, "role", "Admiral");
        assert_eq!(
            values_of(&facts, &grace, "REPORTS_TO"),
            [ada],
            "the relation by id missed Ada's node: {facts:#?}"
        );
        assert_eq!(nodes(&facts).len(), 10, "an extra node: {facts:#?}");
    }

    /// A record keyed by its email, under a redaction rule with a `replacement` for emails: the email
    /// never reaches the store, not in a property, not in an alias, and not in the evidence.
    #[test]
    fn an_id_matching_an_irreversible_rule_never_reaches_the_store() {
        let w = world();
        records(
            &w,
            "people.list",
            r#"[{"id": "ada.lovelace@example.org", "name": "Ada Lovelace", "role": "Engineer"}]"#,
        );
        people(
        &w,
        "red",
        "redaction:\n  classes: []\n  rules:\n    - {name: email, pattern: \"[a-z.]+@example\\\\.org\", replacement: \"[email]\"}\n",
    );
        let ran = run(&w, "red/people");
        assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
        let all = serde_json::to_string(&stored_facts(&w, "red")).unwrap();
        assert!(all.contains("Engineer"), "{all}");
        assert!(
            !all.contains("ada.lovelace@example.org"),
            "an irreversible match reached the store: {all}"
        );
        let evidence = std::fs::read_dir(instance(&w, "red").join("runs"))
            .unwrap()
            .flat_map(|r| std::fs::read_dir(r.unwrap().path()).unwrap())
            .map(|b| std::fs::read_to_string(b.unwrap().path().join("extraction.yaml")).unwrap())
            .collect::<String>();
        assert!(
            !evidence.contains("ada.lovelace@example.org"),
            "an irreversible match is in the applied document: {evidence}"
        );
    }

    /// Two records keyed by their emails under the same irreversible email rule are still two people:
    /// the replacement text both ids become is not an identity they share.
    #[test]
    fn two_ids_scrubbed_to_one_replacement_stay_two_nodes() {
        let w = world();
        records(
            &w,
            "people.list",
            r#"[{"id": "ada@example.org", "name": "Ada Lovelace", "role": "Engineer"},
            {"id": "grace@example.org", "name": "Grace Hopper", "role": "Admiral"}]"#,
        );
        people(
        &w,
        "rep",
        "redaction:\n  classes: []\n  rules:\n    - {name: email, pattern: \"[a-z.]+@example\\\\.org\", replacement: \"[email]\"}\n",
    );
        let ran = run(&w, "rep/people");
        assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
        let facts = stored_facts(&w, "rep");
        let ada = node_with(&facts, "role", "Engineer");
        let grace = node_with(&facts, "role", "Admiral");
        assert_ne!(
            ada, grace,
            "two people merged through a shared redaction replacement: {facts:#?}"
        );
    }

    /// A changed property adds the new value on the same node, and, until EKR supersession lands
    /// (EKR wave 20261005b), the old value stays active beside it. Re-scoped by the coordinator's
    /// decision 4 from "leaves one value", which EKR 0.0.30 cannot do.
    #[test]
    fn a_changed_property_adds_the_new_value_beside_the_old_until_supersession() {
        let w = world();
        records(
            &w,
            "people.list",
            r#"[{"id": "P-1", "name": "Ada Lovelace", "role": "Engineer"}]"#,
        );
        people(&w, "chg", "");
        run(&w, "chg/people");
        records(
            &w,
            "people.list",
            r#"[{"id": "P-1", "name": "Ada Lovelace", "role": "Lead"}]"#,
        );
        run(&w, "chg/people");
        let facts = stored_facts(&w, "chg");
        let ada = node_with(&facts, "role", "Lead");
        assert_eq!(node_with(&facts, "role", "Engineer"), ada, "{facts:#?}");
        let mut roles = values_of(&facts, &ada, "role");
        roles.sort();
        assert_eq!(roles, ["Engineer", "Lead"], "{facts:#?}");
        assert_eq!(nodes(&facts).len(), 1, "{facts:#?}");
    }

    /// Two structured sources mapping one node type with different properties: the second source's
    /// ontology adds its property to the type, and its records apply.
    #[test]
    fn a_second_mapping_of_one_node_type_with_other_properties_applies() {
        let w = world();
        records(
            &w,
            "people.list",
            r#"[{"id": "P-1", "name": "Ada Lovelace", "role": "Engineer"}]"#,
        );
        records(
            &w,
            "staff.list",
            r#"[{"id": "S-1", "name": "Grace Hopper", "office": "Arlington"}]"#,
        );
        let other = r#"        mapping:
          node_type: Person
          id: "$.id"
          name: "$.name"
          aliases: []
          properties:
            - {property: office, path: "$.office"}
          relations: []"#;
        let sources = format!(
            "{}{}",
            source("people", "people.list", MAPPING, ""),
            source("staff", "staff.list", other, "")
        );
        create(&w, "ont", &sources, "");
        run(&w, "ont/people");
        let ran = run(&w, "ont/staff");
        assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
        assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");
        let facts = stored_facts(&w, "ont");
        node_with(&facts, "office", "Arlington");
    }

    /// A paged structured input whose `param` is written from the root, as the structured section
    /// says a path may be (`$.page`), reads every page.
    #[test]
    fn a_root_paging_param_reads_every_page() {
        let w = world();
        let pages = w.root.join("pages-people.list");
        std::fs::create_dir_all(&pages).unwrap();
        std::fs::write(
            pages.join("1.json"),
            r#"[{"id": "P-1", "name": "Ada Lovelace", "role": "Engineer"}]"#,
        )
        .unwrap();
        std::fs::write(
            pages.join("2.json"),
            r#"[{"id": "P-2", "name": "Grace Hopper", "role": "Admiral"}]"#,
        )
        .unwrap();
        std::fs::write(
            pages.join("3.json"),
            r#"[{"id": "P-3", "name": "Linus Example", "role": "Maintainer"}]"#,
        )
        .unwrap();
        let paging = r#"paging: {style: PageNumber, param: "$.page", max_pages: 10}"#;
        create(
            &w,
            "pag",
            &source("people", "people.list", MAPPING, paging),
            "",
        );
        let ran = run(&w, "pag/people");
        let facts = stored_facts(&w, "pag");
        for role in ["Engineer", "Admiral", "Maintainer"] {
            assert!(
                quads(&facts)
                    .iter()
                    .any(|(_, _, p, v)| p == "role" && v == role),
                "the page with {role} was never read and nothing says so ({ran}); asked {:#?}",
                w.lines("people.list.log")
            );
        }
    }

    /// The values of `hostile_and_unusual_values_round_trip_as_text`, one record per case, so a
    /// rejection names the value it is for: each record applies with no part rejected and its role
    /// reads back exactly.
    fn one_value(name: &str, json: Value, who: &str, role: &str) {
        // The node is named by the record's name and its identity.
        let who = &format!("{who} (directory:people.list:{})", text(&json["id"]));
        let w = world();
        records(&w, "people.list", &serde_json::json!([json]).to_string());
        people(&w, name, "");
        let ran = run(&w, &format!("{name}/people"));
        assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
        let facts = stored_facts(&w, name);
        let found: Vec<_> = quads(&facts)
            .into_iter()
            .map(|(_, s, p, v)| (s, p, v))
            .collect();
        assert!(
            found.contains(&(who.to_string(), "role".to_string(), role.to_string())),
            "{who:?} lost role {:?} ({ran}): {:?}",
            &role[..role.len().min(40)],
            found
                .iter()
                .map(|(s, p, v)| (s, p, &v[..v.len().min(40)]))
                .collect::<Vec<_>>()
        );
        assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");
    }

    #[test]
    fn value_yaml_and_tag_syntax() {
        let (who, role) = (
            "!Relation {relation: OWNS}",
            "*anchor &x [1, 2]: {a: b}\n- c",
        );
        one_value(
            "vy",
            serde_json::json!({"id": "P-1", "name": who, "role": role}),
            who,
            role,
        );
    }

    #[test]
    fn value_unicode_and_bidi() {
        let (who, role) = ("Zoë Ünïcødé 🚀 ニコ", "\u{202e}rtl");
        one_value(
            "vu",
            serde_json::json!({"id": "P-1", "name": who, "role": role}),
            who,
            role,
        );
    }

    #[test]
    fn value_nul_character() {
        let role = "before\u{0000}after";
        one_value(
            "vn",
            serde_json::json!({"id": "P-1", "name": "Ada", "role": role}),
            "Ada",
            role,
        );
    }

    #[test]
    fn value_number_name_and_boolean_property() {
        one_value(
            "vb",
            serde_json::json!({"id": 3, "name": 42, "role": true}),
            "42",
            "true",
        );
    }

    /// A 200 000-character value is over EKR's 65 536-byte string limit, so EKR rejects the fact. The
    /// run reports the rejected part, does not count the record as applied and does not mark it seen:
    /// the next run reads and tries it again. Its other records apply. Re-scoped by the coordinator's
    /// decision 5 from "the value applies", which EKR 0.0.30 refuses.
    #[test]
    fn a_value_over_ekrs_string_limit_is_rejected_reported_and_retried() {
        let w = world();
        let long = "y".repeat(200_000);
        let json = serde_json::json!([
            {"id": "P-1", "name": "Ada", "role": long},
            {"id": "P-2", "name": "Grace", "role": "Admiral"},
        ]);
        records(&w, "people.list", &json.to_string());
        people(&w, "vl", "");
        let ran = run(&w, "vl/people");
        assert_eq!(ran["detail"]["documents_new"], 2, "{ran}");
        assert_eq!(ran["detail"]["parts_rejected"], 1, "{ran}");
        assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
        let facts = stored_facts(&w, "vl");
        let roles: Vec<_> = quads(&facts)
            .into_iter()
            .filter(|(_, _, p, _)| p == "role")
            .map(|(_, s, _, v)| (s, v))
            .collect();
        assert_eq!(
            roles,
            [(
                "Grace (directory:people.list:P-2)".to_string(),
                "Admiral".to_string()
            )],
            "{facts:#?}"
        );

        // Nothing changed upstream: the rejected record is read and tried again, the applied one is not.
        let again = run(&w, "vl/people");
        assert_eq!(again["detail"]["documents_new"], 1, "{again}");
        assert_eq!(again["detail"]["parts_rejected"], 1, "{again}");
        assert_eq!(again["detail"]["documents_applied"], 0, "{again}");
    }

    /// Hostile and unusual values: YAML syntax, tag-looking text, unicode, a number and a boolean all
    /// round-trip as text onto the right node. The 200 000-character value this case also carried is
    /// over EKR's string limit; `a_value_over_ekrs_string_limit_is_rejected_reported_and_retried`
    /// holds it (decision 5).
    #[test]
    fn hostile_and_unusual_values_round_trip_as_text() {
        let w = world();
        let json = serde_json::json!([
            {"id": "P-1", "name": "!Relation {relation: OWNS}", "role": "*anchor &x [1, 2]: {a: b}\n- c"},
            {"id": "P-2", "name": "Zoë Ünïcødé 🚀 ニコ", "role": "\u{202e}rtl\u{0000}nul"},
            {"id": 3, "name": 42, "role": true},
        ]);
        records(&w, "people.list", &json.to_string());
        people(&w, "hos", "");
        let ran = run(&w, "hos/people");
        assert_eq!(ran["detail"]["documents_applied"], 3, "{ran}");
        assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");
        let facts = stored_facts(&w, "hos");
        let found: Vec<_> = quads(&facts)
            .into_iter()
            .map(|(_, s, p, v)| (s, p, v))
            .collect();
        for (who, role) in [
            (
                "!Relation {relation: OWNS} (directory:people.list:P-1)",
                "*anchor &x [1, 2]: {a: b}\n- c",
            ),
            (
                "Zoë Ünïcødé 🚀 ニコ (directory:people.list:P-2)",
                "\u{202e}rtl\u{0000}nul",
            ),
            ("42 (directory:people.list:3)", "true"),
        ] {
            assert!(
                found.contains(&(who.to_string(), "role".to_string(), role.to_string())),
                "{who:?} lost role {:?}: {:?}",
                &role[..role.len().min(40)],
                found
                    .iter()
                    .map(|(s, p, v)| (s, p, &v[..v.len().min(40)]))
                    .collect::<Vec<_>>()
            );
        }
        assert_eq!(
            found.iter().filter(|(_, p, _)| p == "OWNS").count(),
            0,
            "{found:?}"
        );
    }

    /// The same records answered for two inputs are applied once.
    #[test]
    fn records_answered_twice_apply_once() {
        let w = world();
        records(
            &w,
            "people.list",
            r#"[{"id": "P-1", "name": "Ada Lovelace", "role": "Engineer"},
            {"id": "P-1", "name": "Ada Lovelace", "role": "Engineer"}]"#,
        );
        let twice = source("people", "people.list", MAPPING, "")
            .replace("inputs: [{}]", "inputs: [{}, {a: 1}]");
        create(&w, "twi", &twice, "");
        let ran = run(&w, "twi/people");
        assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
        let facts = stored_facts(&w, "twi");
        assert_eq!(nodes(&facts).len(), 1, "{facts:#?}");
    }
}
