//! `story:schema-apply-outcome-unknown`: a schema transaction EKR cannot say took effect stops
//! `cortex schema <instance>` as it stops a run: the outcome is `store-outcome-unknown`, exit 4,
//! naming the proposal whose write is unknown, and no later proposal is asked of EKR. A stand-in
//! `ekr` in front of the real one answers "publication outcome is unknown" for one `ekr commit`.

mod common;

use std::path::{Path, PathBuf};

use common::{executable, World};
use serde_json::{json, Value};

/// What EKR 0.0.34 prints for `StoreError::UnknownCommit` (`crates/ekr/src/exit.rs`: a fault,
/// `ekr: <message>`, exit 1).
const UNKNOWN: &str = "ekr: publication outcome is unknown; resolve the exact retained occurrence";

/// The stand-in: every verb goes to the real `ekr`, except that while `<root>/unknown-commit`
/// holds `n`, the `n`th `ekr commit` from then on commits nothing and answers the unknown outcome.
/// While `<root>/readonly-at-commit` exists, an `ekr commit` first makes every schema run's
/// `proposals.jsonl` read-only. Each `ekr commit` is logged to `commits.log`.
fn unknown_ekr(w: &World) -> PathBuf {
    let path = w.bin.join("ekr-unknown");
    let body = r#"R="@R@"; EKR="@EKR@"; UNKNOWN="@UNKNOWN@"; RUNS="@RUNS@"
if [ "$1" = commit ]; then
  echo "$*" >> "$R/commits.log"
  if [ -e "$R/readonly-at-commit" ]; then chmod 444 "$RUNS"/*/schema/*/proposals.jsonl; fi
  if [ -e "$R/unknown-commit" ]; then
    n=$(( $(cat "$R/commits-armed" 2>/dev/null || echo 0) + 1 )); echo "$n" > "$R/commits-armed"
    if [ "$n" = "$(cat "$R/unknown-commit")" ]; then echo "$UNKNOWN" >&2; exit 1; fi
  fi
fi
exec "$EKR" "$@"
"#
    .replace("@R@", &w.root.display().to_string())
    .replace("@EKR@", &common::ekr().display().to_string())
    .replace("@UNKNOWN@", UNKNOWN)
    .replace("@RUNS@", &w.home.join("instances").display().to_string());
    executable(&path, &body);
    path
}

/// An extraction stand-in: three products, each with its own `version`, all citing the first
/// evidence id of the batch.
fn extract_products(w: &World) {
    executable(
        &w.bin.join("claude"),
        r#"id=$(grep -oE 'evidence id [0-9a-f-]{36}' | head -1 | cut -d' ' -f3)
f() { printf '{"!Property":{"subject":{"node_type":"Product","aliases":["Product %s"]},"property":"version","value":{"value_kind":"String","value":"1.%s"},"evidence":["%s"]}}' "$1" "$1" "$id"; }
e() { printf '{"node_type":"Product","aliases":["Product %s"]}' "$1"; }
cat <<EOF
{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{"format":"ekr.extraction-document/1","ontology":{"node_types":[{"name":"Product","parents":[],"abstract_type":false,"properties":[{"name":"version","value":{"value_kind":"String"},"cardinality":"One","required":false}]}],"edge_types":[]},"entities":[$(e 1),$(e 2),$(e 3)],"facts":[$(f 1),$(f 2),$(f 3)]}}
EOF
"#,
    );
}

/// The proposing model's stand-in: answers `proposals.json`, with `FACT0` the first fact id of
/// its prompt.
fn proposer(w: &World) {
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
prompt=$(cat)
fact=$(printf '%s\n' "$prompt" | grep -oE '^=== Fact [0-9a-f-]{{36}}' | head -1 | cut -d' ' -f3)
proposals=$(sed -e "s/FACT0/$fact/g" "$R/proposals.json")
echo "{{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"total_cost_usd\":0.02,\"structured_output\":{{\"proposals\":$proposals}}}}"
"#,
            root = w.root.display()
        ),
    );
}

fn node_type(name: &str) -> Value {
    json!({"change": {"kind": "add_node_type", "value": {"name": name, "properties": [
            {"name": "country", "value_kind": "String", "cardinality": "One"}]}},
         "reason": format!("the evidence names a {name}"), "facts": ["FACT0"], "evidence": ["E1"]})
}

/// An instance `name` on the stand-in `ekr` whose store holds three product facts, with the
/// proposer in place of the extraction stand-in.
fn instance(name: &str) -> World {
    let w = World::new();
    let path = w.spec(name, "conn_test");
    let bin = unknown_ekr(&w);
    let text = std::fs::read_to_string(&path).unwrap().replace(
        &common::ekr().display().to_string(),
        &bin.display().to_string(),
    );
    std::fs::write(&path, text).unwrap();
    extract_products(&w);
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let (code, ran) = w.cortex(&["run", &format!("{name}/news")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    proposer(&w);
    w
}

fn ontology_types(w: &World, name: &str) -> Vec<String> {
    let dir = w.home.join("instances").join(name);
    let out = std::process::Command::new(common::ekr())
        .arg("ontology")
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
    let ontology: Value = serde_json::from_slice(&out.stdout).unwrap();
    ontology["node_types"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|t| t["name"].as_str().map(str::to_string))
        .collect()
}

fn lines(dir: &Path) -> Vec<Value> {
    std::fs::read_to_string(dir.join("proposals.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

/// The one schema run directory of the instance.
fn run_dir(w: &World, name: &str) -> PathBuf {
    let dir = w.home.join("instances").join(name).join("schema");
    let dirs: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(dirs.len(), 1, "{dirs:?}");
    dirs[0].clone()
}

/// Acceptance 1: EKR answers outcome-unknown for proposal 2 of 3. The run stops as
/// `store-outcome-unknown`, exit 4, naming proposal 2 and the unknown write; proposal 3 is never
/// asked of EKR; proposal 1 stays applied and recorded.
#[test]
fn an_unknown_commit_stops_the_proposals_and_names_the_write() {
    let w = instance("su");
    let head = w.head("su");
    std::fs::write(
        w.root.join("proposals.json"),
        json!([node_type("Maker"), node_type("Vendor"), node_type("Dealer")]).to_string(),
    )
    .unwrap();
    let _ = std::fs::remove_file(w.root.join("commits.log"));
    std::fs::write(w.root.join("unknown-commit"), "2").unwrap();

    let (code, out) = w.cortex(&["schema", "su", "--sample", "5"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (4, Some("store-outcome-unknown")),
        "{out}"
    );
    let reason = out["detail"]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("proposal 2 of 3"), "{out}");
    assert!(reason.contains("publication outcome is unknown"), "{out}");

    assert_eq!(
        w.lines("commits.log").len(),
        2,
        "proposal 3 was asked of EKR: {:?}",
        w.lines("commits.log")
    );
    let names = ontology_types(&w, "su");
    assert!(names.contains(&"Maker".to_string()), "{names:?}");
    assert!(!names.contains(&"Vendor".to_string()), "{names:?}");
    assert!(!names.contains(&"Dealer".to_string()), "{names:?}");
    assert_eq!(w.head("su"), head + 1, "only proposal 1 committed");

    let recorded = lines(&run_dir(&w, "su"));
    assert_eq!(recorded.len(), 1, "{recorded:?}");
    assert_eq!(recorded[0]["status"], "applied", "{recorded:?}");
}

/// R4-11: the line of an applied proposal cannot be written to `proposals.jsonl`. The round stops
/// as `propose-failed`, and its reason says the proposal is applied, how many are, and how many
/// lines are written, instead of implying nothing was applied; no later proposal is asked of EKR.
#[test]
fn a_line_that_cannot_be_written_after_an_apply_is_counted_in_the_reason() {
    let w = instance("ro");
    let head = w.head("ro");
    std::fs::write(
        w.root.join("proposals.json"),
        json!([node_type("Maker"), node_type("Vendor")]).to_string(),
    )
    .unwrap();
    let _ = std::fs::remove_file(w.root.join("commits.log"));
    std::fs::write(w.root.join("readonly-at-commit"), "").unwrap();

    let (code, out) = w.cortex(&["schema", "ro", "--sample", "5"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("propose-failed")),
        "{out}"
    );
    let reason = out["detail"]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("proposal 1 of 2, applied"), "{out}");
    assert!(reason.contains("1 proposal(s) are applied in all"), "{out}");
    assert!(
        reason.contains("the 0 line(s) before it are written"),
        "{out}"
    );
    assert_eq!(
        w.lines("commits.log").len(),
        1,
        "proposal 2 was asked of EKR"
    );
    assert_eq!(w.head("ro"), head + 1, "proposal 1 is committed");
}

/// The `postgres` stand-in: a `postgres` store is answered from `<root>/pg-as-sqlite.sqlite`
/// through the real `ekr` (stages work on SQLite too), as in `tests/store_outcome_unknown.rs`.
/// While `<root>/fail-head-after-publish` exists, an `ekr head` after a published stage fails as
/// an unavailable store does.
fn pg_ekr(w: &World) -> PathBuf {
    let path = w.bin.join("ekr-pg");
    let body = r#"R="@R@"; EKR="@EKR@"; DB="$R/pg-as-sqlite.sqlite"
if [ "$1" = postgres-schema ]; then echo '{"format":"ekr.postgres-schema/1","ready":true}'; exit 0; fi
if [ "$EKR_BACKEND" = postgres ]; then
  if [ "$1" = head ] && [ ! -e "$DB" ]; then echo "the lineage has no seed" >&2; exit 1; fi
  EKR_BACKEND=sqlite; EKR_STORE="$DB"; export EKR_BACKEND EKR_STORE
fi
if [ "$1 $2" = "stage publish" ] && [ -e "$R/fail-head-after-publish" ]; then
  "$EKR" "$@"; s=$?; [ "$s" = 0 ] && touch "$R/published"; exit "$s"
fi
if [ "$1" = head ] && [ -e "$R/published" ]; then
  echo "ekr: the store is unavailable: injected" >&2; exit 1
fi
exec "$EKR" "$@"
"#
    .replace("@R@", &w.root.display().to_string())
    .replace("@EKR@", &common::ekr().display().to_string());
    executable(&path, &body);
    path
}

/// An instance `pg` on a `postgres` store answered by [`pg_ekr`], one `files` source `docs`,
/// created and run once with the world's extraction stand-in, then the proposer in its place.
fn pg_instance() -> (World, PathBuf) {
    let w = World::new();
    let bin = pg_ekr(&w);
    let config = w.root.join("pg/app.json");
    let held = w.root.join("pg/password.txt");
    std::fs::create_dir_all(held.parent().unwrap()).unwrap();
    std::fs::write(&held, "unknown-not-a-secret").unwrap();
    let document = json!({
        "format": "ekr.postgres/1",
        "connection_file": "app.dsn",
        "password_file": held,
        "schema": "ekr",
        "database_connections": 100,
        "replicas": 1,
        "reserved_connections": 10,
    });
    std::fs::write(&config, document.to_string()).unwrap();
    std::fs::create_dir_all(w.root.join("docs")).unwrap();
    std::fs::write(
        w.root.join("docs/a.md"),
        "Example Labs develops the Widget engine.\n",
    )
    .unwrap();
    let spec = w.root.join("pg.yaml");
    std::fs::write(
        &spec,
        format!(
            r#"format: cortex.instance/1
name: pg
description: Test brain.
ekr: {{version: "0.0.30", bin: "{bin}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
  - name: docs
    schedule: daily
    settings:
      kind: files
      value: {{paths: [docs], glob: "**/*.md"}}
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
serve: {{view_port: 18995}}
store: {{backend: postgres, value: {{config: "{config}"}}}}
"#,
            bin = bin.display(),
            config = config.display(),
        ),
    )
    .unwrap();
    let (code, created) = w.cortex(&[
        "create",
        "--spec",
        spec.to_str().unwrap(),
        "--no-extract",
        "--no-units",
    ]);
    assert_eq!(code, 0, "{created}");
    let (code, ran) = w.cortex(&["run", "pg/docs"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    proposer(&w);
    let db = w.root.join("pg-as-sqlite.sqlite");
    (w, db)
}

/// `ekr <args>` on the SQLite file the stand-in answers the `postgres` store from.
fn pg_store(w: &World, db: &Path, args: &[&str]) -> Value {
    let out = std::process::Command::new(common::ekr())
        .args(args)
        .env("EKR_HOST", w.home.join("instances/pg/host.json"))
        .env("EKR_BACKEND", "sqlite")
        .env("EKR_STORE", db)
        .env_remove("EKR_STAGE")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

/// Acceptance 2 on `postgres`: the stage is published, then the store's head cannot be read.
/// `apply-failed` says the changes are in the store and names the head the stage was published
/// at, not that every approved proposal is still approved; the decisions are written.
#[test]
fn a_postgres_apply_whose_head_cannot_be_read_after_the_publish_names_the_stage_head() {
    let (w, db) = pg_instance();
    std::fs::write(
        w.root.join("proposals.json"),
        json!([node_type("Maker")]).to_string(),
    )
    .unwrap();
    let (code, out) = w.cortex(&["schema", "pg", "--sample", "5", "--review"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("reviewed")),
        "{out}"
    );
    let stamp = out["detail"]["stamp"].as_str().unwrap().to_string();
    let (code, out) = w.cortex(&["schema", "approve", "pg", &stamp]);
    assert_eq!(code, 0, "{out}");
    let head = pg_store(&w, &db, &["head"])["revision"].as_i64().unwrap();
    std::fs::write(w.root.join("fail-head-after-publish"), "").unwrap();

    let (code, out) = w.cortex(&["schema", "apply", "pg", &stamp]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("apply-failed")),
        "{out}"
    );
    let reason = out["detail"]["reason"].as_str().unwrap_or_default();
    assert!(!reason.contains("still approved"), "{out}");
    assert!(
        reason.contains(&format!("its stage was published at head {head}")),
        "{out}"
    );
    let ontology = pg_store(&w, &db, &["ontology"]);
    assert!(
        ontology["node_types"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["name"] == "Maker"),
        "the published stage does not hold Maker: {ontology}"
    );
    let dir = w.home.join("instances/pg/schema").join(&stamp);
    assert_eq!(lines(&dir)[0]["status"], "applied", "{:?}", lines(&dir));
}

/// Adversary: two review runs propose the same node type; the first is applied, so the second's
/// approved proposal is planned `invalid` and its stage commits nothing. When the head then cannot
/// be read after the (empty) publish, `apply-failed` must not say the changes are in the store.
#[test]
fn a_postgres_apply_that_committed_nothing_does_not_say_changes_are_in_the_store() {
    let (w, db) = pg_instance();
    std::fs::write(
        w.root.join("proposals.json"),
        json!([node_type("Maker")]).to_string(),
    )
    .unwrap();
    let mut stamps = Vec::new();
    for _ in 0..2 {
        let (code, out) = w.cortex(&["schema", "pg", "--sample", "5", "--review"]);
        assert_eq!(
            (code, out["outcome"].as_str()),
            (0, Some("reviewed")),
            "{out}"
        );
        let stamp = out["detail"]["stamp"].as_str().unwrap().to_string();
        let (code, out) = w.cortex(&["schema", "approve", "pg", &stamp]);
        assert_eq!(code, 0, "{out}");
        stamps.push(stamp);
    }
    let (code, out) = w.cortex(&["schema", "apply", "pg", &stamps[0]]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("applied")),
        "{out}"
    );
    let head = pg_store(&w, &db, &["head"])["revision"].as_i64().unwrap();
    std::fs::write(w.root.join("fail-head-after-publish"), "").unwrap();

    let (code, out) = w.cortex(&["schema", "apply", "pg", &stamps[1]]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("apply-failed")),
        "{out}"
    );
    let dir = w.home.join("instances/pg/schema").join(&stamps[1]);
    assert_eq!(lines(&dir)[0]["status"], "invalid", "{:?}", lines(&dir));
    assert_eq!(
        pg_store(&w, &db, &["head"])["revision"].as_i64().unwrap(),
        head,
        "the second apply committed something"
    );
    let reason = out["detail"]["reason"].as_str().unwrap_or_default();
    assert!(
        !reason.contains("the changes are in the store"),
        "nothing was committed, yet: {reason}"
    );
}
