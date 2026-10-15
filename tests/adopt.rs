//! An existing EKR store becomes an instance without being reseeded (`story:adopt-existing-store`):
//! `cortex adopt` copies a SQLite store with its whole history into the instance, writes nothing
//! to the store it read, and the instance's sources grow the copy from there.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{ekr, World};
use serde_json::Value;

/// The store's ontology after the legacy extraction: the types the stand-in model answers with.
const LEGACY_EXTRACTION: &str = r#"format: ekr.extraction-document/1
ontology:
  node_types:
  - name: Organization
    parents: []
    abstract_type: false
    properties:
    - {name: website, value: {value_kind: String}, cardinality: One, required: false}
  - {name: Product, parents: [], abstract_type: false, properties: []}
  edge_types:
  - {name: DEVELOPS, source_types: [Organization], target_types: [Product], cardinality: Many, properties: []}
entities:
- {node_type: Organization, aliases: [Example Labs]}
- {node_type: Product, aliases: [Widget engine]}
facts: []
evidence: []
"#;

/// A seed schema naming two of the store's types.
const SCHEMA: &str = r#"format: ekr.extraction-document/1
ontology:
  node_types:
  - {name: Organization, parents: [], abstract_type: false, properties: []}
  - {name: Product, parents: [], abstract_type: false, properties: []}
  edge_types: []
entities: []
facts: []
evidence: []
"#;

/// A store seeded and grown by `ekr` directly, under a host document of its own (tenant `legacy`),
/// as a brain that cortex never created holds it.
struct Legacy {
    dir: PathBuf,
}

impl Legacy {
    fn new(w: &World) -> Self {
        let dir = w.root.join("legacy");
        std::fs::create_dir_all(&dir).unwrap();
        let out = Command::new(ekr())
            .args(["example", "ekr.cli-host/1"])
            .output()
            .unwrap();
        let mut host: Value = serde_json::from_slice(&out.stdout).unwrap();
        host["tenant"] = "legacy".into();
        let profile = &mut host["authority"]["validation_profile"];
        profile["application"] = "ekr.p2-apply/1".into();
        profile["ruleset"] = "ekr.p2-deterministic/1".into();
        std::fs::write(dir.join("host.json"), host.to_string()).unwrap();
        std::fs::write(dir.join("seed.yaml"), cortex_cli::ekr::MINIMAL_SEED).unwrap();
        std::fs::write(dir.join("one.yaml"), LEGACY_EXTRACTION).unwrap();
        let legacy = Self { dir };
        legacy.ekr(&["seed", "seed.yaml"]);
        let report = legacy.ekr(&["apply-extraction", "one.yaml"]);
        assert_eq!(
            report["committed"].as_array().map(Vec::len),
            Some(2),
            "{report}"
        );
        legacy
    }

    fn store(&self) -> PathBuf {
        self.dir.join("store.sqlite")
    }

    fn host(&self) -> PathBuf {
        self.dir.join("host.json")
    }

    fn ekr(&self, args: &[&str]) -> Value {
        ekr_json(&self.host(), &self.store(), &self.dir, args)
    }

    fn head(&self) -> Value {
        self.ekr(&["head"])
    }
}

fn ekr_json(host: &Path, store: &Path, dir: &Path, args: &[&str]) -> Value {
    let out = Command::new(ekr())
        .args(args)
        .current_dir(dir)
        .env("EKR_HOST", host)
        .env("EKR_BACKEND", "sqlite")
        .env("EKR_STORE", store)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "ekr {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

/// `ekr head` of the instance's own store, through its own host document.
fn instance_head(w: &World, name: &str) -> Value {
    let dir = w.home.join("instances").join(name);
    ekr_json(
        &dir.join("host.json"),
        &dir.join("store.sqlite"),
        &dir,
        &["head"],
    )
}

/// The spec of instance `name`: a `files` source over `docs/*.md` and the seed schema `schema`.
fn spec(w: &World, name: &str, schema: &str) -> PathBuf {
    std::fs::write(w.root.join(format!("{name}-schema.yaml")), schema).unwrap();
    let path = w.root.join(format!("{name}.yaml"));
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: {name}
description: Adopted brain.
ekr: {{version: "0.0.30", bin: "{ekr}"}}
seed: {{schema: {name}-schema.yaml, documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
  - name: notes
    schedule: daily
    settings: {{kind: files, value: {{paths: [docs], glob: "*.md"}}}}
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
serve: {{view_port: 18994}}
"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    std::fs::create_dir_all(w.root.join("docs")).unwrap();
    std::fs::write(
        w.root.join("docs/a.md"),
        "Example Labs develops the Widget engine.\n",
    )
    .unwrap();
    std::fs::write(
        w.root.join("docs/b.md"),
        "Example Labs ships a new release of the Widget engine.\n",
    )
    .unwrap();
    path
}

fn adopt(w: &World, spec: &Path, store: &Path, extra: &[&str]) -> (i32, Value) {
    let mut args = vec![
        "adopt",
        "--spec",
        spec.to_str().unwrap(),
        "--store",
        store.to_str().unwrap(),
        "--no-units",
    ];
    args.extend_from_slice(extra);
    w.cortex(&args)
}

/// Every file directly in `dir` with its bytes: what a write to the store would change.
fn files(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap())
        .filter(|e| e.file_type().unwrap().is_file())
        .map(|e| {
            (
                e.file_name().to_string_lossy().into_owned(),
                std::fs::read(e.path()).unwrap(),
            )
        })
        .collect();
    out.sort();
    out
}

/// Asserts that `after` holds every file of `before` with the same bytes, and nothing new but the
/// side files SQLite itself writes beside a database a read-only connection opens: a `-shm`
/// (created or updated), and an empty `-wal` where none was. The database and the revisions in an
/// existing `-wal` are untouched.
fn only_sqlite_side_files_added(before: &[(String, Vec<u8>)], after: &[(String, Vec<u8>)]) {
    for (name, bytes) in before.iter().filter(|(n, _)| !n.ends_with("-shm")) {
        assert!(
            after.iter().any(|(n, b)| n == name && b == bytes),
            "{name} changed or went"
        );
    }
    for (name, bytes) in after {
        if before.iter().any(|(n, _)| n == name) {
            continue;
        }
        assert!(
            name.ends_with("-shm") || (name.ends_with("-wal") && bytes.is_empty()),
            "{name} appeared beside the store"
        );
    }
}

/// The names `cortex list` prints.
fn registered(w: &World) -> Vec<String> {
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .arg("list")
        .env("CORTEX_HOME", &w.home)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap()["name"].to_string())
        .collect()
}

#[test]
fn an_adopted_store_keeps_its_head_and_one_run_of_two_documents_adds_two_revisions() {
    let w = World::new();
    let legacy = Legacy::new(&w);
    let head = legacy.head();
    let n = head["revision"].as_i64().unwrap();
    assert_eq!(n, 2, "{head}");
    let before = files(&legacy.dir);

    let spec = spec(&w, "brain", SCHEMA);
    let host = legacy.host();
    let (code, adopted) = adopt(
        &w,
        &spec,
        &legacy.store(),
        &["--host", host.to_str().unwrap()],
    );
    assert_eq!(
        (code, adopted["outcome"].as_str()),
        (0, Some("adopted")),
        "{adopted}"
    );
    assert_eq!(adopted["detail"]["revision"], n, "{adopted}");
    assert_eq!(adopted["detail"]["sources"][0], "brain/notes", "{adopted}");
    assert_eq!(adopted["detail"]["seen_documents"], 0, "{adopted}");
    assert_eq!(adopted["detail"]["seen_without_evidence"], 0, "{adopted}");

    // Nothing was seeded or written: the store's directory holds the same files with the same
    // bytes, beside only the side files SQLite's read-only connection writes (checked before
    // anything opens the store again); the store answers the same head with the same root, and
    // the instance's copy answers it too.
    only_sqlite_side_files_added(&before, &files(&legacy.dir));
    assert_eq!(legacy.head(), head);
    assert_eq!(instance_head(&w, "brain"), head);

    let (code, ran) = w.cortex(&["run", "brain/notes"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
    // EKR 0.0.31 commits a relation fact whose edge an earlier fact proposes in a later round:
    // the two documents' `DEVELOPS` facts name one edge, so the one apply commits twice.
    assert_eq!(instance_head(&w, "brain")["revision"], n + 2);
    assert_eq!(legacy.head(), head, "the run grew the instance's copy only");

    let (code, list) = w.cortex(&["list"]);
    assert_eq!(code, 0);
    assert_eq!(
        (list["name"].as_str(), list["state"].as_str()),
        (Some("brain"), Some("Active")),
        "{list}"
    );
}

#[test]
fn a_seen_file_starts_the_source_from_the_documents_it_names() {
    let w = World::new();
    let legacy = Legacy::new(&w);
    let spec = spec(&w, "brain", SCHEMA);
    let root = std::fs::canonicalize(&w.root).unwrap();
    let a = root.join("docs/a.md").display().to_string();
    let text = std::fs::read_to_string(w.root.join("docs/a.md")).unwrap();
    let seen = w.root.join("notes.seen.json");
    std::fs::write(
        &seen,
        serde_json::json!({
            "format": "cortex.seen/1",
            "documents": {a.clone(): {"hash": cortex_cli::state::text_hash(&text), "applied_at": 1}},
        })
        .to_string(),
    )
    .unwrap();

    let host = legacy.host();
    let seen_arg = format!("notes={}", seen.display());
    let (code, adopted) = adopt(
        &w,
        &spec,
        &legacy.store(),
        &["--host", host.to_str().unwrap(), "--seen", &seen_arg],
    );
    assert_eq!(
        (code, adopted["outcome"].as_str()),
        (0, Some("adopted")),
        "{adopted}"
    );
    // a.md is taken as seen and is not extracted again, although the store holds no evidence of
    // it; the adoption says so.
    assert_eq!(adopted["detail"]["seen_documents"], 1, "{adopted}");
    assert_eq!(adopted["detail"]["seen_without_evidence"], 1, "{adopted}");

    let (code, ran) = w.cortex(&["run", "brain/notes"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_new"], 1, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
}

/// A seen document whose evidence the store holds (the identity a `files` source gives it,
/// `file:<key>`) is counted as seen; one with no evidence there is counted apart.
#[test]
fn seen_documents_without_evidence_in_the_store_are_counted_apart() {
    let w = World::new();
    let legacy = Legacy::new(&w);
    let spec = spec(&w, "brain", SCHEMA);
    let root = std::fs::canonicalize(&w.root).unwrap();
    let key = |doc: &str| root.join(format!("docs/{doc}.md")).display().to_string();
    let text = |doc: &str| std::fs::read_to_string(w.root.join(format!("docs/{doc}.md"))).unwrap();

    let payload = format!("Source: file:{}\n\n{}", key("a"), text("a")).into_bytes();
    let extraction = format!(
        r#"format: ekr.extraction-document/1
ontology: {{node_types: [], edge_types: []}}
entities: []
facts:
- !Property
  subject: {{node_type: Organization, aliases: [Example Labs]}}
  property: website
  value: {{value_kind: String, value: example.org}}
  evidence: [00000000-0000-4000-8000-0000000000e1]
evidence:
- evidence:
    id: 00000000-0000-4000-8000-0000000000e1
    source: !HumanStatement
      identity: "file:{key}"
    content_hash: {hash}
    extracted_by: 00000000-0000-4000-8000-000000000101
    observed_at: 1773273600000
    confidence: 8000
  payload: {payload:?}
"#,
        key = key("a"),
        hash = cortex_cli::evidence::content_hash(&payload),
    );
    std::fs::write(legacy.dir.join("evidence.yaml"), extraction).unwrap();
    let report = legacy.ekr(&["apply-extraction", "evidence.yaml"]);
    assert!(
        report["rejected"].as_array().is_some_and(Vec::is_empty)
            && report["committed"]
                .as_array()
                .is_some_and(|c| !c.is_empty()),
        "{report}"
    );

    let seen = w.root.join("notes.seen.json");
    let entry = |doc: &str| serde_json::json!({"hash": cortex_cli::state::text_hash(&text(doc)), "applied_at": 1});
    std::fs::write(
        &seen,
        serde_json::json!({
            "format": "cortex.seen/1",
            "documents": {key("a"): entry("a"), key("b"): entry("b")},
        })
        .to_string(),
    )
    .unwrap();
    let seen_arg = format!("notes={}", seen.display());
    let host = legacy.host();
    let (code, adopted) = adopt(
        &w,
        &spec,
        &legacy.store(),
        &["--host", host.to_str().unwrap(), "--seen", &seen_arg],
    );
    assert_eq!(
        (code, adopted["outcome"].as_str()),
        (0, Some("adopted")),
        "{adopted}"
    );
    assert_eq!(adopted["detail"]["seen_documents"], 2, "{adopted}");
    assert_eq!(adopted["detail"]["seen_without_evidence"], 1, "{adopted}");
}

#[test]
fn a_seen_file_of_another_format_or_for_no_source_of_the_spec_adopts_nothing() {
    let w = World::new();
    let legacy = Legacy::new(&w);
    let spec = spec(&w, "brain", SCHEMA);
    let host = legacy.host();
    let wrong = w.root.join("wrong.json");
    std::fs::write(&wrong, r#"{"format": "other/1", "documents": {}}"#).unwrap();
    let good = w.root.join("good.json");
    std::fs::write(&good, r#"{"format": "cortex.seen/1", "documents": {}}"#).unwrap();
    for seen in [
        format!("notes={}", wrong.display()),
        format!("news={}", good.display()),
        good.display().to_string(),
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
            .args(["adopt", "--spec", spec.to_str().unwrap()])
            .args(["--store", legacy.store().to_str().unwrap()])
            .args([
                "--host",
                host.to_str().unwrap(),
                "--seen",
                &seen,
                "--no-units",
            ])
            .env("CORTEX_HOME", &w.home)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2), "{seen}");
        assert!(!w.home.join("instances/brain").exists(), "{seen}");
    }
    assert!(registered(&w).is_empty());

    // The refusals were the seen files': with a good one the same adoption succeeds.
    let seen = format!("notes={}", good.display());
    let (code, adopted) = adopt(
        &w,
        &spec,
        &legacy.store(),
        &["--host", host.to_str().unwrap(), "--seen", &seen],
    );
    assert_eq!(
        (code, adopted["outcome"].as_str()),
        (0, Some("adopted")),
        "{adopted}"
    );
}

#[test]
fn a_store_another_host_cannot_open_is_unreadable_and_nothing_is_registered() {
    let w = World::new();
    let legacy = Legacy::new(&w);
    let before = files(&legacy.dir);
    let spec = spec(&w, "brain", SCHEMA);

    // Without `--host`, cortex opens the store as tenant `brain`, which holds no lineage there.
    let (code, refused) = adopt(&w, &spec, &legacy.store(), &[]);
    assert_eq!(
        (code, refused["outcome"].as_str()),
        (1, Some("store-unreadable")),
        "{refused}"
    );
    assert!(
        refused["detail"]["reason"]
            .as_str()
            .is_some_and(|r| r.contains("no seed")),
        "{refused}"
    );
    let missing = w.root.join("nowhere.sqlite");
    let (code, refused) = adopt(&w, &spec, &missing, &[]);
    assert_eq!(
        (code, refused["outcome"].as_str()),
        (1, Some("store-unreadable")),
        "{refused}"
    );
    assert!(!missing.exists());
    assert!(!w.home.join("instances/brain").exists());
    assert!(registered(&w).is_empty());
    only_sqlite_side_files_added(&before, &files(&legacy.dir));
}

#[test]
fn a_store_lacking_a_seed_type_is_refused_naming_the_type() {
    let w = World::new();
    let legacy = Legacy::new(&w);
    let schema = SCHEMA.replace(
        "  edge_types: []",
        "  - {name: Gadget, parents: [], abstract_type: false, properties: []}\n  edge_types:\n  - {name: SHIPS, source_types: [Organization], target_types: [Gadget], cardinality: Many, properties: []}",
    );
    let spec = spec(&w, "brain", &schema);
    let host = legacy.host();
    let (code, refused) = adopt(
        &w,
        &spec,
        &legacy.store(),
        &["--host", host.to_str().unwrap()],
    );
    assert_eq!(
        (code, refused["outcome"].as_str()),
        (1, Some("seed-types-missing")),
        "{refused}"
    );
    assert_eq!(refused["detail"]["types"], "Gadget, SHIPS", "{refused}");
    assert!(!w.home.join("instances/brain").exists());
    assert!(registered(&w).is_empty());
}

#[test]
fn a_store_on_another_backend_than_the_spec_names_is_a_backend_mismatch() {
    let w = World::new();
    let legacy = Legacy::new(&w);
    let host = legacy.host();
    // An `ekr.postgres/1` file given for a `sqlite` spec.
    let config = w.root.join("pg.json");
    std::fs::write(&config, r#"{"format": "ekr.postgres/1"}"#).unwrap();
    let sqlite = spec(&w, "brain", SCHEMA);
    let (code, refused) = adopt(&w, &sqlite, &config, &["--host", host.to_str().unwrap()]);
    assert_eq!(
        (code, refused["outcome"].as_str()),
        (1, Some("backend-mismatch")),
        "{refused}"
    );

    // The SQLite store given for a `postgres` spec.
    let text = std::fs::read_to_string(&sqlite).unwrap().replace(
        "serve:",
        &format!(
            "store: {{backend: postgres, value: {{config: \"{}\"}}}}\nserve:",
            config.display()
        ),
    );
    let postgres = w.root.join("pg-brain.yaml");
    std::fs::write(&postgres, text).unwrap();
    let (code, refused) = adopt(
        &w,
        &postgres,
        &legacy.store(),
        &["--host", host.to_str().unwrap()],
    );
    assert_eq!(
        (code, refused["outcome"].as_str()),
        (1, Some("backend-mismatch")),
        "{refused}"
    );
    assert!(!w.home.join("instances/brain").exists());
    assert!(registered(&w).is_empty());
}

#[test]
fn a_taken_name_adopts_nothing() {
    let w = World::new();
    let legacy = Legacy::new(&w);
    let spec = spec(&w, "brain", SCHEMA);
    let host = legacy.host();
    let host = host.to_str().unwrap();
    let (code, adopted) = adopt(&w, &spec, &legacy.store(), &["--host", host]);
    assert_eq!(code, 0, "{adopted}");
    let frozen = files(&w.home.join("instances/brain"));
    let (code, refused) = adopt(&w, &spec, &legacy.store(), &["--host", host]);
    assert_eq!(
        (code, refused["outcome"].as_str()),
        (1, Some("name-taken")),
        "{refused}"
    );
    assert_eq!(files(&w.home.join("instances/brain")), frozen);
}

/// A third revision of the legacy store that changes no type: one more `Product`.
const LEGACY_SECOND: &str = r#"format: ekr.extraction-document/1
ontology:
  node_types:
  - {name: Product, parents: [], abstract_type: false, properties: []}
  edge_types: []
entities:
- {node_type: Product, aliases: [Gadget runtime]}
facts: []
evidence: []
"#;

/// The legacy store at head 3 while another process (a viewer, say) holds it open: the third
/// revision is still in `store.sqlite-wal`, because a connection that closes while another is open
/// does not checkpoint. Answers the holder, which must outlive the adoption.
fn held_at_head_three(legacy: &Legacy) -> rusqlite::Connection {
    let holder = rusqlite::Connection::open(legacy.store()).unwrap();
    holder
        .query_row("select count(*) from sqlite_master", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap();
    std::fs::write(legacy.dir.join("two.yaml"), LEGACY_SECOND).unwrap();
    let report = legacy.ekr(&["apply-extraction", "two.yaml"]);
    assert_eq!(
        report["committed"].as_array().map(Vec::len),
        Some(1),
        "{report}"
    );
    assert!(legacy.dir.join("store.sqlite-wal").exists());
    holder
}

/// A store another process holds open keeps its newest revision in its `-wal`. Given through a
/// symlink, the store is still copied with that revision, and `adopted` reports it as the head.
#[test]
fn a_symlinked_store_another_process_holds_is_copied_with_its_newest_revision() {
    let w = World::new();
    let legacy = Legacy::new(&w);
    let _holder = held_at_head_three(&legacy);
    let head = legacy.head();
    assert_eq!(head["revision"], 3, "{head}");

    let links = w.root.join("links");
    std::fs::create_dir_all(&links).unwrap();
    let link = links.join("store.sqlite");
    std::os::unix::fs::symlink(legacy.store(), &link).unwrap();

    let spec = spec(&w, "brain", SCHEMA);
    let host = legacy.host();
    let (code, adopted) = adopt(&w, &spec, &link, &["--host", host.to_str().unwrap()]);
    assert_eq!(
        (code, adopted["outcome"].as_str()),
        (0, Some("adopted")),
        "{adopted}"
    );
    assert_eq!(
        adopted["detail"]["revision"], 3,
        "adopt lost the revisions in the -wal: {adopted}"
    );
    assert_eq!(instance_head(&w, "brain"), head);
}

/// The store's database and its `-wal`, without the `-shm` (a copy of a held store, or a store
/// whose writer died): the copy is whole, with the revision in the `-wal`. The database and the
/// `-wal` stay as they were; the read-only connection may create a `-shm` beside them.
#[test]
fn a_store_with_a_wal_and_no_shm_is_copied_whole_and_only_a_shm_appears_beside_it() {
    let w = World::new();
    let legacy = Legacy::new(&w);
    let holder = held_at_head_three(&legacy);
    let kept = w.root.join("kept");
    std::fs::create_dir_all(&kept).unwrap();
    for file in ["store.sqlite", "store.sqlite-wal"] {
        std::fs::copy(legacy.dir.join(file), kept.join(file)).unwrap();
    }
    drop(holder);
    let before = files(&kept);

    let spec = spec(&w, "brain", SCHEMA);
    let host = legacy.host();
    let (code, adopted) = adopt(
        &w,
        &spec,
        &kept.join("store.sqlite"),
        &["--host", host.to_str().unwrap()],
    );
    assert_eq!(
        (code, adopted["outcome"].as_str()),
        (0, Some("adopted")),
        "{adopted}"
    );
    assert_eq!(adopted["detail"]["revision"], 3, "{adopted}");
    only_sqlite_side_files_added(&before, &files(&kept));
}

/// A destination that cannot hold the copy (a full disk; here a file-size limit, which fails the
/// write the same way) is a copy failure, exit 2, not `store-unreadable`: the store can be read,
/// the instance directory could not hold it.
#[test]
fn a_destination_that_cannot_hold_the_copy_is_not_reported_as_an_unreadable_store() {
    let w = World::new();
    let legacy = Legacy::new(&w);
    assert!(std::fs::metadata(legacy.store()).unwrap().len() > 128 * 1024);
    let spec = spec(&w, "brain", SCHEMA);
    let host = legacy.host();
    let out = Command::new("sh")
        .args(["-c", "trap '' XFSZ; ulimit -f 64; exec \"$0\" \"$@\""])
        .arg(env!("CARGO_BIN_EXE_cortex"))
        .args(["adopt", "--spec", spec.to_str().unwrap()])
        .args(["--store", legacy.store().to_str().unwrap()])
        .args(["--host", host.to_str().unwrap(), "--no-units"])
        .env("CORTEX_HOME", &w.home)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let refused: Value =
        serde_json::from_str(stdout.lines().last().unwrap_or("null")).unwrap_or(Value::Null);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_ne!(out.status.code(), Some(0), "{stdout}");
    assert!(!w.home.join("instances/brain").exists());
    assert_ne!(
        refused["outcome"].as_str(),
        Some("store-unreadable"),
        "a full destination is blamed on the store: {stdout}"
    );
    assert_eq!(out.status.code(), Some(2), "{stdout}{stderr}");
    assert!(stderr.contains("cannot copy --store into"), "{stderr}");
}

/// `--seen` is "the seen documents the source starts from … Repeat it for each source". Given twice
/// for one source, the adoption is refused rather than one file replacing the other.
#[test]
fn two_seen_files_for_one_source_are_refused() {
    let w = World::new();
    let legacy = Legacy::new(&w);
    let spec = spec(&w, "brain", SCHEMA);
    let root = std::fs::canonicalize(&w.root).unwrap();
    let mut args = Vec::new();
    for doc in ["a", "b"] {
        let id = root.join(format!("docs/{doc}.md")).display().to_string();
        let text = std::fs::read_to_string(w.root.join(format!("docs/{doc}.md"))).unwrap();
        let file = w.root.join(format!("{doc}.seen.json"));
        std::fs::write(
            &file,
            serde_json::json!({
                "format": "cortex.seen/1",
                "documents": {id: {"hash": cortex_cli::state::text_hash(&text), "applied_at": 1}},
            })
            .to_string(),
        )
        .unwrap();
        args.push(format!("notes={}", file.display()));
    }
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["adopt", "--spec", spec.to_str().unwrap()])
        .args(["--store", legacy.store().to_str().unwrap()])
        .args(["--host", legacy.host().to_str().unwrap(), "--no-units"])
        .args(["--seen", &args[0], "--seen", &args[1]])
        .env("CORTEX_HOME", &w.home)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(2),
        "two --seen files for one source were taken: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(!w.home.join("instances/brain").exists());
}

/// A PostgreSQL store "is used where it is". Adopted a second time under another name, with the
/// same host, it would become a second instance on the very lineage the first one grows; it is
/// refused as `store-held`, naming the first. `ekr` is a stand-in that answers the store's
/// ontology: what is under test is cortex's registry, not the database.
#[test]
fn a_postgres_lineage_another_instance_already_uses_is_not_adopted_again() {
    let w = World::new();
    let fake = w.bin.join("ekr");
    common::executable(
        &fake,
        r#"case "$1" in
  ontology) echo '{"revision": 7, "node_types": [{"name": "Organization"}, {"name": "Product"}], "edge_types": []}' ;;
  head) echo '{"revision": 7, "root": {}}' ;;
  *) exit 1 ;;
esac
"#,
    );
    let config = w.root.join("pg.json");
    std::fs::write(&config, r#"{"format": "ekr.postgres/1"}"#).unwrap();
    let out = Command::new(ekr())
        .args(["example", "ekr.cli-host/1"])
        .output()
        .unwrap();
    let mut host: Value = serde_json::from_slice(&out.stdout).unwrap();
    host["tenant"] = "legacy".into();
    let host_file = w.root.join("host.json");
    std::fs::write(&host_file, host.to_string()).unwrap();

    let mut outcomes = Vec::new();
    let mut details = Vec::new();
    for name in ["brain", "second"] {
        let path = spec(&w, name, SCHEMA);
        let text = std::fs::read_to_string(&path)
            .unwrap()
            .replace(&ekr().display().to_string(), &fake.display().to_string())
            .replace(
                "serve:",
                &format!(
                    "store: {{backend: postgres, value: {{config: \"{}\"}}}}\nserve:",
                    config.display()
                ),
            );
        std::fs::write(&path, text).unwrap();
        let (code, done) = adopt(&w, &path, &config, &["--host", host_file.to_str().unwrap()]);
        outcomes.push((code, done["outcome"].as_str().map(str::to_string)));
        details.push(done["detail"].clone());
    }
    assert_eq!(outcomes[0], (0, Some("adopted".into())));
    assert_ne!(
        outcomes[1].1.as_deref(),
        Some("adopted"),
        "one PostgreSQL lineage became two instances"
    );
    assert_eq!(
        outcomes[1],
        (1, Some("store-held".into())),
        "{}",
        details[1]
    );
    assert_eq!(details[1]["name"], "brain", "{}", details[1]);
    assert!(!w.home.join("instances/second").exists());
}

/// Three chat records of one archive file; record `1.0` is the one the adopter's store holds.
const ARCHIVE: &str = r#"{"ts":"1.0","user":"ana","text":"Example Labs adopted the Widget engine early."}
{"ts":"2.0","user":"ben","text":"Example Labs ships the Widget engine on Friday."}
{"ts":"3.0","user":"cem","text":"Example Labs runs the Widget engine in two regions."}
"#;

/// The spec of instance `name`: one `files` source `chat` over `archive/*.jsonl`, whose records
/// are named by `ts`, with `extra` appended.
fn chat_spec(w: &World, name: &str, extra: &str) -> PathBuf {
    std::fs::write(w.root.join(format!("{name}-schema.yaml")), SCHEMA).unwrap();
    let path = w.root.join(format!("{name}.yaml"));
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: {name}
description: Adopted brain.
ekr: {{version: "0.0.30", bin: "{ekr}"}}
seed: {{schema: {name}-schema.yaml, documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
  - name: chat
    schedule: daily
    settings:
      kind: files
      value:
        paths: [archive]
        glob: "*.jsonl"
        records: {{format: JsonLines, id: ts, author: user, text: ["{{text}}"], filters: []}}
    policy: {{refresh_after_days: 7, change: ContentHash, max_documents_per_run: 50, max_chars_per_document: 5000}}
serve: {{view_port: 18994}}
{extra}"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    std::fs::create_dir_all(w.root.join("archive")).unwrap();
    std::fs::write(w.root.join("archive/2026-09.jsonl"), ARCHIVE).unwrap();
    path
}

/// The key cortex gives record `id` of the archive file: the file's path, `#`, the record's id.
fn record_key(w: &World, id: &str) -> String {
    let root = std::fs::canonicalize(&w.root).unwrap();
    format!("{}#{id}", root.join("archive/2026-09.jsonl").display())
}

/// A `cortex.seen/1` file naming record `1.0` by its key alone, applied at `1`.
fn seen_by_key(w: &World) -> String {
    let seen = w.root.join("chat.seen.json");
    std::fs::write(
        &seen,
        serde_json::json!({
            "format": "cortex.seen/1",
            "documents": {record_key(w, "1.0"): {"applied_at": 1}},
        })
        .to_string(),
    )
    .unwrap();
    format!("chat={}", seen.display())
}

/// The entry `state/chat.json` of instance `brain` holds for `key`.
fn chat_state(w: &World, key: &str) -> Value {
    let path = w.home.join("instances/brain/state/chat.json");
    let state: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    state["documents"][key].clone()
}

/// Every file named `name` in the batch directories of instance `brain`'s runs, newest run last.
fn batch_files(w: &World, name: &str) -> Vec<String> {
    let mut runs: Vec<_> = std::fs::read_dir(w.home.join("instances/brain/runs"))
        .map(|e| e.map(|r| r.unwrap().path()).collect())
        .unwrap_or_default();
    runs.sort();
    let mut out = Vec::new();
    for run in runs {
        let mut batches: Vec<_> = std::fs::read_dir(run)
            .unwrap()
            .map(|b| b.unwrap().path())
            .collect();
        batches.sort();
        out.extend(
            batches
                .into_iter()
                .filter_map(|b| std::fs::read_to_string(b.join(name)).ok()),
        );
    }
    out
}

/// `story:adopt-seen-by-key`: an adopter that knows only the keys of the records its store holds
/// adopts it without re-extracting them. The first run records the hash of each such record as it
/// renders and masks it, extracts the others, and a later run extracts the record again only when
/// its text changes.
#[test]
fn a_record_seen_by_key_alone_is_hashed_by_the_first_run_and_not_extracted() {
    let w = World::new();
    let legacy = Legacy::new(&w);
    let spec = chat_spec(&w, "brain", "");
    let host = legacy.host();
    let seen_arg = seen_by_key(&w);
    let (code, adopted) = adopt(
        &w,
        &spec,
        &legacy.store(),
        &["--host", host.to_str().unwrap(), "--seen", &seen_arg],
    );
    assert_eq!(
        (code, adopted["outcome"].as_str()),
        (0, Some("adopted")),
        "{adopted}"
    );
    assert_eq!(adopted["detail"]["seen_documents"], 1, "{adopted}");
    let one = record_key(&w, "1.0");
    assert_eq!(chat_state(&w, &one), serde_json::json!({"applied_at": 1}));

    let (code, ran) = w.cortex(&["run", "brain/chat"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_new"], 2, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
    assert_eq!(ran["detail"]["hashed"], 1, "{ran}");
    let prompts = batch_files(&w, "prompt.txt").join("\n");
    assert!(
        !prompts.contains("adopted the Widget engine early") && !prompts.contains(&one),
        "record 1.0 reached the model: {prompts}"
    );
    for id in ["2.0", "3.0"] {
        assert!(
            prompts.contains(&format!("Source: file:{}\n", record_key(&w, id))),
            "record {id} was not extracted: {prompts}"
        );
    }
    let applied = batch_files(&w, "extraction.yaml").join("\n");
    assert!(!applied.contains(&one), "{applied}");
    let entry = chat_state(&w, &one);
    let hash = entry["hash"].as_str().unwrap_or_default();
    assert!(
        hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()),
        "{entry}"
    );
    assert_eq!(entry["applied_at"], 1, "{entry}");

    // Nothing changed: nothing is extracted, nothing is hashed again.
    let calls = w.lines("claude-calls.log").len();
    let (code, again) = w.cortex(&["run", "brain/chat"]);
    assert_eq!(
        (code, again["outcome"].as_str()),
        (0, Some("ran")),
        "{again}"
    );
    assert_eq!(again["detail"]["documents_new"], 0, "{again}");
    assert!(again["detail"].get("hashed").is_none(), "{again}");
    assert_eq!(w.lines("claude-calls.log").len(), calls);
    assert_eq!(chat_state(&w, &one), entry);

    // Record 1.0's text changes: the next run extracts it.
    std::fs::write(
        w.root.join("archive/2026-09.jsonl"),
        ARCHIVE.replace(
            "adopted the Widget engine early",
            "retired the Widget engine",
        ),
    )
    .unwrap();
    let (code, changed) = w.cortex(&["run", "brain/chat"]);
    assert_eq!(
        (code, changed["outcome"].as_str()),
        (0, Some("ran")),
        "{changed}"
    );
    assert_eq!(changed["detail"]["documents_new"], 1, "{changed}");
    assert_eq!(changed["detail"]["documents_applied"], 1, "{changed}");
    let last = batch_files(&w, "prompt.txt").pop().unwrap();
    assert!(last.contains(&format!("Source: file:{one}\n")), "{last}");
    assert!(last.contains("retired the Widget engine"), "{last}");
    assert_ne!(chat_state(&w, &one)["hash"], entry["hash"]);
}

/// The hash a run records for a document seen by key alone is written with the rest of the run:
/// a run its gate undoes puts `state/` back, and the key is again seen with no hash.
#[test]
fn a_hash_recorded_by_a_run_its_gate_undoes_is_put_back_with_it() {
    let w = World::new();
    let legacy = Legacy::new(&w);
    // The stand-in model refuses one fact per batch, so `facts_refused max 0` fails every run.
    let spec = chat_spec(
        &w,
        "brain",
        "gate:\n  checks:\n    - {measure: facts_refused, max: \"0\"}\n",
    );
    let host = legacy.host();
    let seen_arg = seen_by_key(&w);
    let (code, adopted) = adopt(
        &w,
        &spec,
        &legacy.store(),
        &["--host", host.to_str().unwrap(), "--seen", &seen_arg],
    );
    assert_eq!(code, 0, "{adopted}");

    let (code, ran) = w.cortex(&["run", "brain/chat"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("apply-refused")),
        "{ran}"
    );
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("facts_refused = 1"), "{reason}");
    let one = record_key(&w, "1.0");
    assert_eq!(chat_state(&w, &one), serde_json::json!({"applied_at": 1}));
    for id in ["2.0", "3.0"] {
        assert_eq!(chat_state(&w, &record_key(&w, id)), Value::Null, "{id}");
    }
}

/// A `--seen` entry that names a key with neither a hash nor `applied_at` is refused, naming the
/// key, and nothing is adopted.
#[test]
fn a_seen_entry_with_neither_hash_nor_applied_at_is_refused_naming_its_key() {
    let w = World::new();
    let legacy = Legacy::new(&w);
    let spec = chat_spec(&w, "brain", "");
    let one = record_key(&w, "1.0");
    let seen = w.root.join("chat.seen.json");
    std::fs::write(
        &seen,
        serde_json::json!({"format": "cortex.seen/1", "documents": {one.clone(): {}}}).to_string(),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["adopt", "--spec", spec.to_str().unwrap()])
        .args(["--store", legacy.store().to_str().unwrap()])
        .args(["--host", legacy.host().to_str().unwrap()])
        .args(["--seen", &format!("chat={}", seen.display()), "--no-units"])
        .env("CORTEX_HOME", &w.home)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains(&one), "{stderr}");
    assert!(!w.home.join("instances/brain").exists());
    assert!(registered(&w).is_empty());
}

/// Adversary: the story's acceptance, word for word. `seen.json` names record `1.0` as
/// `archive/2026-09.jsonl#1.0`, the path the spec gives (`paths: [archive]`), and the first run
/// must send no prompt holding record `1.0` and leave a 64-hex hash under that key.
#[test]
fn adv_the_acceptance_seen_file_word_for_word_keeps_record_one_from_the_model() {
    let w = World::new();
    let legacy = Legacy::new(&w);
    let spec = chat_spec(&w, "brain", "");
    let host = legacy.host();
    let seen = w.root.join("seen.json");
    std::fs::write(
        &seen,
        r#"{"format":"cortex.seen/1","documents":{"archive/2026-09.jsonl#1.0":{"applied_at":1}}}"#,
    )
    .unwrap();
    let seen_arg = format!("chat={}", seen.display());
    let (code, adopted) = adopt(
        &w,
        &spec,
        &legacy.store(),
        &["--host", host.to_str().unwrap(), "--seen", &seen_arg],
    );
    assert_eq!(code, 0, "{adopted}");
    assert_eq!(adopted["detail"]["seen_documents"], 1, "{adopted}");

    let (code, ran) = w.cortex(&["run", "brain/chat"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let prompts = batch_files(&w, "prompt.txt").join("\n");
    assert!(
        !prompts.contains("adopted the Widget engine early"),
        "record 1.0 reached the model although seen.json names it as the story's acceptance \
         does: {ran}"
    );
    let entry = chat_state(&w, "archive/2026-09.jsonl#1.0");
    let hash = entry["hash"].as_str().unwrap_or_default();
    assert!(
        hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()),
        "no hash recorded for the acceptance key: {entry}"
    );
}

/// Adversary: an entry whose `applied_at` (and `hash`) is JSON `null` names neither; it is
/// refused, and the refusal names its key as the one with no `applied_at` does.
#[test]
fn adv_a_seen_entry_with_null_applied_at_is_refused_naming_its_key() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("chat.json");
    for entry in [
        r#"{"applied_at": null}"#,
        r#"{"hash": null, "applied_at": null}"#,
    ] {
        std::fs::write(
            &path,
            format!(r#"{{"format": "cortex.seen/1", "documents": {{"a#7": {entry}}}}}"#),
        )
        .unwrap();
        let refused = cortex_cli::state::SeenState::read_document(&path).unwrap_err();
        assert!(refused.contains("a#7"), "{entry}: {refused}");
    }
}

/// Adversary: a structured change record adopted by key, applied recently, whose text holds
/// links. The first run records exactly what applying it would have recorded (`record`), its hash
/// without links included, so a later change of its links alone is extracted at once.
#[test]
fn adv_a_hashed_record_carries_its_unlinked_hash_as_an_applied_one_does() {
    use cortex_cli::sources::{Document, Origin};
    use cortex_cli::state::{text_hash, Seen, SeenState};
    const DAY: i64 = 24 * 60 * 60 * 1000;
    let now = 100 * DAY;
    let doc = |text: &str| Document {
        key: "changes:1".into(),
        origin: Origin::Record,
        title: None,
        description: None,
        published: None,
        text: text.into(),
        hash: Some(text_hash(text)),
    };
    let unlinked = text_hash("body");
    let mut adopted = SeenState::default();
    adopted.documents.insert(
        "changes:1".into(),
        Seen {
            hash: None,
            applied_at: now,
            unlinked_hash: None,
        },
    );
    adopted.unlinked = [("changes:1".to_string(), unlinked.clone())].into();
    let first = doc("body [link: a]");
    assert_eq!(adopted.hash_unhashed(std::slice::from_ref(&first)), 1);

    let mut applied = SeenState::default();
    applied.unlinked = adopted.unlinked.clone();
    applied.record(&first, now);
    assert_eq!(
        adopted.documents["changes:1"],
        applied.documents["changes:1"]
    );

    // The next day only its links change: extracted at once, as for an applied record.
    let relinked = doc("body [link: b]");
    assert!(adopted.wants(&relinked, now + DAY, 7));
    assert!(!adopted.holds(&relinked, now + DAY, 7));
    // Its text changes beyond its links: held back by the refresh window like any applied one.
    adopted.unlinked = [("changes:1".to_string(), text_hash("other body"))].into();
    let rewritten = doc("other body [link: a]");
    assert!(!adopted.wants(&rewritten, now + DAY, 7));
    assert!(adopted.holds(&rewritten, now + DAY, 7));
}

/// Adversary: on SQLite (the PostgreSQL case is skipped without a container runtime), a seen file
/// naming every record by key alone: the first run asks no model, applies nothing, records the
/// three hashes and keeps each `applied_at`; the second run hashes nothing.
#[test]
fn adv_every_record_seen_by_key_alone_asks_no_model_and_hashes_all_three() {
    let w = World::new();
    let legacy = Legacy::new(&w);
    let spec = chat_spec(&w, "brain", "");
    let host = legacy.host();
    let seen = w.root.join("all.seen.json");
    let ids = ["1.0", "2.0", "3.0"];
    let documents: serde_json::Map<String, Value> = ids
        .iter()
        .enumerate()
        .map(|(n, id)| (record_key(&w, id), serde_json::json!({"applied_at": n + 1})))
        .collect();
    std::fs::write(
        &seen,
        serde_json::json!({"format": "cortex.seen/1", "documents": documents}).to_string(),
    )
    .unwrap();
    let seen_arg = format!("chat={}", seen.display());
    let (code, adopted) = adopt(
        &w,
        &spec,
        &legacy.store(),
        &["--host", host.to_str().unwrap(), "--seen", &seen_arg],
    );
    assert_eq!(code, 0, "{adopted}");
    let head = instance_head(&w, "brain");

    let (code, ran) = w.cortex(&["run", "brain/chat"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_new"], 0, "{ran}");
    assert_eq!(ran["detail"]["hashed"], 3, "{ran}");
    assert!(w.lines("claude-calls.log").is_empty());
    assert_eq!(instance_head(&w, "brain"), head);
    for (n, id) in ids.iter().enumerate() {
        let entry = chat_state(&w, &record_key(&w, id));
        assert_eq!(
            entry["hash"].as_str().map(str::len),
            Some(64),
            "{id}: {entry}"
        );
        assert_eq!(entry["applied_at"], n + 1, "{id}: {entry}");
    }

    let (code, again) = w.cortex(&["run", "brain/chat"]);
    assert_eq!(
        (code, again["outcome"].as_str()),
        (0, Some("ran")),
        "{again}"
    );
    assert_eq!(again["detail"]["documents_new"], 0, "{again}");
    assert!(again["detail"].get("hashed").is_none(), "{again}");
    assert!(w.lines("claude-calls.log").is_empty());
}
