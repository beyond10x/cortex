//! `story:spec-compat-gate`: `task compat` lets a change of `spec/` against the newest release tag
//! pass only when `spec/compat-ack.json` acknowledges it. That file is ESS's
//! `ess-diff-acknowledgements/1`, which holds change ids and no reasons, so each reason is a row of
//! `spec/compat-ack.md`. This test holds the two files to each other: every acknowledged change has
//! one row with a reason, and every row names an acknowledged change.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde_json::Value;

fn read(file: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(file);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The acknowledged change ids of `spec/compat-ack.json`.
fn acknowledged() -> Vec<String> {
    let ack: Value = serde_json::from_str(&read("spec/compat-ack.json")).expect("JSON");
    assert_eq!(ack["format"], "ess-diff-acknowledgements/1");
    ack["acknowledged"]
        .as_array()
        .expect("acknowledged is a list")
        .iter()
        .map(|id| id.as_str().expect("an id is a string").to_string())
        .collect()
}

/// The rows of `spec/compat-ack.md`'s table: `` | `<change id>` | <reason> | ``.
fn reasons() -> Vec<(String, String)> {
    read("spec/compat-ack.md")
        .lines()
        .filter_map(|line| line.strip_prefix("| `"))
        .map(|row| {
            let (id, rest) = row
                .split_once("` |")
                .expect("a row is | `<id>` | <reason> |");
            let reason = rest.trim().trim_end_matches('|').trim();
            (id.to_string(), reason.to_string())
        })
        .collect()
}

#[test]
fn every_acknowledged_change_has_a_reason_and_every_reason_an_acknowledgement() {
    let acknowledged = acknowledged();
    let rows = reasons();
    let mut seen = BTreeSet::new();
    for (id, reason) in &rows {
        assert!(seen.insert(id.clone()), "`{id}` has two rows");
        assert!(!reason.is_empty(), "`{id}` has no reason");
    }
    let ids: BTreeSet<_> = acknowledged.iter().cloned().collect();
    assert_eq!(ids.len(), acknowledged.len(), "an id is acknowledged twice");
    let without_reason: Vec<_> = ids.difference(&seen).collect();
    let without_acknowledgement: Vec<_> = seen.difference(&ids).collect();
    assert!(
        without_reason.is_empty(),
        "acknowledged without a reason in spec/compat-ack.md: {without_reason:?}"
    );
    assert!(
        without_acknowledgement.is_empty(),
        "a reason in spec/compat-ack.md for a change spec/compat-ack.json does not acknowledge: \
         {without_acknowledgement:?}"
    );
}
