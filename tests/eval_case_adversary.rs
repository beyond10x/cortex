//! Adversary cases for `cortex eval case add <instance> --set <set> --from-review <review id>`
//! (`cortex.instance.AddEvalCase`, `story:eval-cases-from-review`): what a refusal must leave
//! untouched, what a closed item with nothing expected yields, what an older answer-record file
//! holds, and what two adds at once leave in the set.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use common::World;
use cortex_cli::ask::records::{Asked, Keeper, Records};
use cortex_model::ask as ma;
use serde_json::{json, Value};

fn instance(name: &str) -> World {
    let w = World::new();
    let path = w.spec(name, "conn_test");
    let (code, created) = w.cortex(&[
        "create",
        "--spec",
        path.to_str().unwrap(),
        "--no-units",
        "--no-extract",
    ]);
    assert_eq!(code, 0, "{created}");
    w
}

fn records_path(w: &World, name: &str) -> PathBuf {
    w.home.join("instances").join(name).join("ask.sqlite")
}

fn keeper(w: &World, name: &str) -> Keeper {
    Keeper {
        store: Arc::new(Records::open(&records_path(w, name)).expect("ask.sqlite opens")),
        instance: name.into(),
        retention_days: 90,
        slow_after_ms: None,
    }
}

fn failed_answer(keeper: &Keeper, answer_id: &str, question: &str) -> String {
    let asked = Asked {
        answer_id: answer_id.into(),
        identity: "alice".into(),
        question: question.into(),
        tool_calls: vec![],
        latency_ms: 5,
        at_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64,
    };
    let failed = ma::AskOutcome::ModelFailed {
        error: ma::ModelFailed {
            reason: "down".into(),
        },
    };
    keeper.record(asked, &failed).expect("recorded");
    format!("{answer_id}:failed")
}

fn closed(keeper: &Keeper, review: &str, facts: &[&str], nodes: &[&str]) {
    let (outcome, _) = keeper
        .close(
            review,
            "checked",
            facts.iter().map(|s| s.to_string()).collect(),
            nodes.iter().map(|s| s.to_string()).collect(),
        )
        .expect("closed");
    assert!(
        matches!(outcome, ma::CloseReviewOutcome::Closed { .. }),
        "{outcome:?}"
    );
}

fn two_cases(w: &World) -> PathBuf {
    let path = w.root.join("set.json");
    let case = |id: &str, q: &str, f: &str, n: &str| {
        json!({"format": "cortex.eval-case/1", "id": id, "question": q,
               "expect_facts": [f], "expect_nodes": [n]})
    };
    let set = json!({"cases": [
        case("widget", "Who develops the Widget engine?", "widget engine", "node-widget"),
        case("gadget", "Who develops the Gadget runtime?", "Gadget runtime", "node-gadget"),
    ]});
    std::fs::write(&path, set.to_string()).unwrap();
    path
}

fn case_add(w: &World, name: &str, set: &Path, review: &str) -> (i32, Value) {
    w.cortex(&[
        "eval",
        "case",
        "add",
        name,
        "--set",
        set.to_str().unwrap(),
        "--from-review",
        review,
    ])
}

/// `eval case add` as its own process, its exit code and last line, without the world's
/// assertion on exit code 2 (a concurrent run may fail, and the test counts that).
fn case_add_raw(w: &World, name: &str, set: &Path, review: &str) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["eval", "case", "add", name, "--set"])
        .arg(set)
        .args(["--from-review", review])
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .output()
        .unwrap();
    (
        out.status.code(),
        format!(
            "{} {}",
            String::from_utf8_lossy(&out.stdout).trim(),
            String::from_utf8_lossy(&out.stderr).trim()
        ),
    )
}

// A closed item whose reviewer wrote no fact and no node yields a case every answer passes
// (`score` finds nothing missing), so an eval set would gain a case that can never fail.
#[test]
fn a_closed_review_with_no_expected_fact_or_node_is_refused_and_the_set_is_unchanged() {
    let w = instance("cases");
    let records = keeper(&w, "cases");
    let review = failed_answer(&records, "a1", "Who develops the Widget engine?");
    closed(&records, &review, &[], &[]);
    let set = two_cases(&w);
    let before = std::fs::read(&set).unwrap();

    let (code, out) = case_add(&w, "cases", &set, &review);
    assert_ne!(code, 0, "a case nothing can fail was added: {out}");
    assert_eq!(code, 1, "{out}");
    assert_eq!(out["outcome"], "no-expectations", "{out}");
    assert_eq!(std::fs::read(&set).unwrap(), before, "the set changed");
}

// The same, when the set file does not exist yet: a refusal creates no file.
#[test]
fn a_closed_review_with_no_expectations_creates_no_set_file() {
    let w = instance("cases");
    let records = keeper(&w, "cases");
    let review = failed_answer(&records, "a1", "Who develops the Widget engine?");
    closed(&records, &review, &[], &[]);
    let set = w.root.join("new-set.json");

    let (code, out) = case_add(&w, "cases", &set, &review);
    assert_ne!(code, 0, "a case nothing can fail was added: {out}");
    assert!(!set.exists(), "a refusal created the set file");
}

/// Takes the instance's `ask.sqlite` back to schema version 1: no expectation columns.
fn to_version_one(path: &Path) {
    let db = rusqlite::Connection::open(path).unwrap();
    db.execute_batch(
        "ALTER TABLE reviews DROP COLUMN expect_facts;
         ALTER TABLE reviews DROP COLUMN expect_nodes;
         PRAGMA user_version = 1;",
    )
    .unwrap();
}

// A file of schema version 1 holds items closed before expectations existed; migrated, they have
// none, and must not become always-passing cases.
#[test]
fn an_item_closed_under_schema_version_one_yields_no_case() {
    let w = instance("cases");
    let path = records_path(&w, "cases");
    {
        let records = keeper(&w, "cases");
        let review = failed_answer(&records, "a1", "Who develops the Widget engine?");
        assert_eq!(review, "a1:failed");
    }
    {
        let db = rusqlite::Connection::open(&path).unwrap();
        db.execute(
            "UPDATE reviews SET state = 'Closed', close_reason = 'done', closed_ms = 2 \
             WHERE review_id = 'a1:failed'",
            [],
        )
        .unwrap();
    }
    to_version_one(&path);
    let set = two_cases(&w);
    let before = std::fs::read(&set).unwrap();

    let (code, out) = case_add(&w, "cases", &set, "a1:failed");
    assert_ne!(
        code, 0,
        "an item closed with no expectations became a case: {out}"
    );
    assert_eq!(std::fs::read(&set).unwrap(), before, "the set changed");
}

// "The records are read, never written": reading a version-1 file for a case must not migrate it.
#[test]
fn case_add_does_not_migrate_the_answer_records_it_reads() {
    let w = instance("cases");
    let path = records_path(&w, "cases");
    {
        let records = keeper(&w, "cases");
        failed_answer(&records, "a1", "Who develops the Widget engine?");
    }
    to_version_one(&path);
    let set = two_cases(&w);

    let _ = case_add(&w, "cases", &set, "nothing:failed");
    let version: i64 = rusqlite::Connection::open(&path)
        .unwrap()
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, 1, "a read migrated ask.sqlite");
}

// An instance that never served a question has no `ask.sqlite`; a read must not create it.
#[test]
fn case_add_on_an_instance_with_no_answer_records_creates_none_and_changes_no_set() {
    let w = instance("cases");
    let path = records_path(&w, "cases");
    assert!(!path.exists(), "the instance starts with no ask.sqlite");
    let set = two_cases(&w);
    let before = std::fs::read(&set).unwrap();

    let (code, out) = case_add(&w, "cases", &set, "a1:failed");
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("no-such-review")),
        "{out}"
    );
    assert_eq!(std::fs::read(&set).unwrap(), before);
    assert!(!path.exists(), "a read created ask.sqlite");
}

#[test]
fn every_refusal_leaves_a_missing_malformed_or_duplicate_set_byte_for_byte() {
    let w = instance("cases");
    let records = keeper(&w, "cases");
    let review = failed_answer(&records, "a1", "Who develops the Widget engine?");
    closed(&records, &review, &["Example Labs"], &[]);
    let open = failed_answer(&records, "a2", "Who develops the Gadget runtime?");

    // Missing set, refused review: no file appears.
    let missing = w.root.join("missing.json");
    for r in ["nothing:failed", open.as_str()] {
        let (code, out) = case_add(&w, "cases", &missing, r);
        assert_eq!(code, 1, "{out}");
        assert!(!missing.exists(), "{r}: a refusal created the set");
    }

    // Malformed sets of several shapes: set-invalid, bytes unchanged.
    for (n, text) in [
        "{not json",
        "",
        "[]",
        r#"{"cases": []}"#,
        r#"{"cases": {}}"#,
        "\u{feff}{\"cases\": []}",
    ]
    .iter()
    .enumerate()
    {
        let path = w.root.join(format!("bad-{n}.json"));
        std::fs::write(&path, text).unwrap();
        let (code, out) = case_add(&w, "cases", &path, &review);
        assert_eq!(
            (code, out["outcome"].as_str()),
            (1, Some("set-invalid")),
            "{text:?}: {out}"
        );
        assert_eq!(std::fs::read(&path).unwrap(), text.as_bytes(), "{text:?}");
    }

    // A case whose id differs but whose review_id is the item: case-exists.
    let dup = w.root.join("dup.json");
    let text = json!({"cases": [{"format": "cortex.eval-case/1", "id": "hand-made",
        "question": "q", "expect_facts": ["f"], "expect_nodes": [], "review_id": review}]})
    .to_string();
    std::fs::write(&dup, &text).unwrap();
    let (code, out) = case_add(&w, "cases", &dup, &review);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("case-exists")),
        "{out}"
    );
    assert_eq!(std::fs::read_to_string(&dup).unwrap(), text);
    assert!(
        !w.root.join(".dup.json.cortex-tmp").exists(),
        "a temporary file was left behind"
    );
}

// Two adds of different items to one set at once: both answer `added`, so both cases must be in
// the set afterwards. The read-modify-rename has no lock, and both share one temporary name.
#[test]
fn concurrent_adds_of_different_reviews_lose_no_case() {
    const N: usize = 8;
    let w = instance("cases");
    let records = keeper(&w, "cases");
    let reviews: Vec<String> = (0..N)
        .map(|i| {
            let r = failed_answer(&records, &format!("a{i}"), &format!("Question number {i}?"));
            closed(&records, &r, &[&format!("fact {i}")], &[]);
            r
        })
        .collect();
    let set = two_cases(&w);

    let results: Vec<(Option<i32>, String)> = std::thread::scope(|s| {
        let handles: Vec<_> = reviews
            .iter()
            .map(|r| {
                let (w, set) = (&w, &set);
                s.spawn(move || case_add_raw(w, "cases", set, r))
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let added = results.iter().filter(|(c, _)| *c == Some(0)).count();
    let parsed = cortex_cli::eval::parse_set(&std::fs::read_to_string(&set).unwrap())
        .expect("the set still parses");
    assert_eq!(
        parsed.cases.len(),
        2 + added,
        "{added} adds answered `added` but the set holds {} cases: {results:?}",
        parsed.cases.len()
    );
    assert_eq!(added, N, "an add failed: {results:?}");
}

#[test]
fn a_question_of_odd_unicode_and_great_length_is_kept_exactly() {
    let w = instance("cases");
    let records = keeper(&w, "cases");
    let mut question = String::from(
        "Wer entwickelt die Widget\u{2011}Engine? \u{202e}evil\u{202c} \u{200b}zw \
         e\u{301} \u{1f9ea} \u{2028}line \u{0}nul \"quoted\" \\back ",
    );
    while question.len() < 200_000 {
        question.push_str("lorem ipsum dolor sit amet ");
    }
    let review = failed_answer(&records, "a1", &question);
    closed(
        &records,
        &review,
        &["Example Labs \u{1f9ea}"],
        &["node-\u{e9}"],
    );
    let set = two_cases(&w);

    let (code, out) = case_add(&w, "cases", &set, &review);
    assert_eq!((code, out["outcome"].as_str()), (0, Some("added")), "{out}");
    let parsed = cortex_cli::eval::parse_set(&std::fs::read_to_string(&set).unwrap())
        .expect("the set parses");
    let case = &parsed.cases[2];
    assert!(
        case.question == question,
        "the question was not kept exactly"
    );
    assert_eq!(case.expect_facts, ["Example Labs \u{1f9ea}"]);
    assert_eq!(case.expect_nodes, ["node-\u{e9}"]);
}

// A set kept elsewhere and linked into place: the add must write the file the link names, not
// replace the link with a copy.
#[cfg(unix)]
#[test]
fn a_set_reached_through_a_symlink_is_appended_in_place() {
    let w = instance("cases");
    let records = keeper(&w, "cases");
    let review = failed_answer(&records, "a1", "Who develops the Widget engine?");
    closed(&records, &review, &["Example Labs"], &[]);
    let target = two_cases(&w);
    let link = w.root.join("linked.json");
    std::os::unix::fs::symlink(&target, &link).unwrap();

    let (code, out) = case_add(&w, "cases", &link, &review);
    assert_eq!((code, out["outcome"].as_str()), (0, Some("added")), "{out}");
    assert!(
        std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink(),
        "the link was replaced by a regular file"
    );
    let parsed = cortex_cli::eval::parse_set(&std::fs::read_to_string(&target).unwrap()).unwrap();
    assert_eq!(
        parsed.cases.len(),
        3,
        "the linked-to set did not gain the case"
    );
}
