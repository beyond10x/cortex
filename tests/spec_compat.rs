//! The 1.0 types change nothing for an existing spec file (`story:spec-standalone-types`).
//!
//! `examples/example.yaml`, the same file with every 1.0 field at its default (the SQLite store
//! both as the Acceptance writes it, `store: {backend: sqlite}`, and with `value: {}`), and the
//! same file with the organisation-scale fields at their defaults too, each create an instance
//! whose frozen spec, registry entry and first-run report equal what `main` at `f2ce283` produced
//! for `examples/example.yaml`, with the stand-in `connectors` and `claude` of `tests/common`.
//!
//! `main`'s behaviour is the committed fixture `tests/fixtures/spec_compat/example.json`. It was
//! recorded by `the_example_spec_behaves_as_it_did_before_the_1_0_types` itself, run with
//! `CORTEX_SPEC_COMPAT_RECORD=<file>` in a tree whose `src/`, `generated/`, `spec/`, `examples/`,
//! `Cargo.toml` and `Cargo.lock` were those of `f2ce283`. Values that differ on every run (the
//! temporary directory, the viewer's free port, the run's start time and duration) are replaced
//! by placeholders on both sides.

mod common;

use std::path::{Path, PathBuf};

use common::{ekr, World};
use serde_json::{json, Value};

const FIXTURE: &str = "tests/fixtures/spec_compat/example.json";
/// The connection id `examples/example.yaml` names; the stand-in `connectors` lists it.
const CONNECTION: &str = "conn_replace_with_your_tavily_connection_id";

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn example() -> String {
    std::fs::read_to_string(manifest().join("examples/example.yaml")).unwrap()
}

/// `text` with the one occurrence of `from` replaced by `to`.
fn replace_once(text: &str, from: &str, to: &str) -> String {
    assert_eq!(
        text.matches(from).count(),
        1,
        "expected exactly one {from:?} in the example"
    );
    text.replacen(from, to, 1)
}

/// The default store as the Acceptance writes it.
const SQLITE_SHORT: &str = "store: {backend: sqlite}\n";
/// The same store with its (optional) settings written out, empty.
const SQLITE_EMPTY_VALUE: &str = "store:\n  backend: sqlite\n  value: {}\n";

/// The example with every 1.0 field at its default: a SQLite store (written as `store`), the
/// Claude backend, an empty redaction policy and no snapshots.
fn with_1_0_defaults(text: &str, store: &str) -> String {
    let text = replace_once(
        text,
        "  model: claude-sonnet-5-5\n",
        "  model: claude-sonnet-5-5\n  backend: Claude\n",
    );
    replace_once(
        &text,
        "\nserve: {}\n",
        &format!("\nserve: {{}}\n{store}redaction:\n  classes: []\n  rules: []\n"),
    )
}

/// The 1.0 defaults plus the organisation-scale fields an existing file can carry, at their
/// defaults: no known names, no class that refuses a run, and a run gate with no checks.
fn with_organisation_defaults(text: &str) -> String {
    let text = with_1_0_defaults(text, SQLITE_SHORT);
    replace_once(
        &text,
        "  rules: []\n",
        "  rules: []\n  known_names: []\n  refuse_if_left: []\ngate:\n  checks: []\n",
    )
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let dest = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &dest);
        } else {
            std::fs::copy(entry.path(), &dest).unwrap();
        }
    }
}

/// Every file under `dir`, by its path relative to `root`, with its text.
fn files(root: &Path, dir: &Path, out: &mut serde_json::Map<String, Value>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            files(root, &path, out);
        } else {
            let rel = path.strip_prefix(root).unwrap().display().to_string();
            out.insert(rel, json!(std::fs::read_to_string(&path).unwrap()));
        }
    }
}

/// Creates an instance from `spec_text` (in a directory beside a copy of `examples/seed`) and runs
/// its first source once: the frozen spec, the registry, and the create and run reports.
fn observe(spec_text: &str) -> Value {
    let w = World::new();
    std::fs::write(w.root.join("connection"), CONNECTION).unwrap();
    // The example names no `ekr.bin`, so cortex takes the pinned binary under `$HOME`.
    let user = w.root.join("user");
    let pinned = user.join(".cache/cortex/bin/0.0.30/bin");
    std::fs::create_dir_all(&pinned).unwrap();
    std::os::unix::fs::symlink(ekr(), pinned.join("ekr")).unwrap();
    let dir = w.root.join("spec");
    copy_dir(&manifest().join("examples/seed"), &dir.join("seed"));
    let path = dir.join("example.yaml");
    std::fs::write(&path, spec_text).unwrap();
    let env = [("HOME", user.as_path())];

    let (code, mut created) = w.cortex_env(
        &["create", "--spec", path.to_str().unwrap(), "--no-units"],
        &env,
    );
    assert_eq!(code, 0, "{created}");
    let (code, ran) = w.cortex_env(&["run", "example/news"], &env);
    assert_eq!(code, 0, "{ran}");

    let instance = w.home.join("instances/example");
    let mut frozen = serde_json::Map::new();
    frozen.insert(
        "instance.yaml".into(),
        json!(std::fs::read_to_string(instance.join("instance.yaml")).unwrap()),
    );
    files(&instance, &instance.join("seed"), &mut frozen);

    let registry: Value =
        serde_json::from_slice(&std::fs::read(w.home.join("registry.json")).unwrap()).unwrap();

    let log: Vec<Value> = std::fs::read_to_string(instance.join("cortex.log"))
        .unwrap()
        .lines()
        .map(|line| {
            let mut v: Value = serde_json::from_str(line).unwrap();
            let o = v.as_object_mut().unwrap();
            for key in ["at", "seconds"] {
                assert!(o.remove(key).is_some(), "the log line has {key}: {line}");
            }
            v
        })
        .collect();

    assert_eq!(
        created["detail"]["dir"].as_str(),
        Some(instance.to_str().unwrap()),
        "{created}"
    );
    let detail = &mut created["detail"];
    detail["dir"] = json!("<home>/instances/example");
    let view = detail["view"].as_str().unwrap().to_string();
    assert!(view.starts_with("http://127.0.0.1:"), "{view}");
    detail["view"] = json!("http://127.0.0.1:<port>/");

    json!({
        "create": created,
        "frozen": frozen,
        "registry": registry,
        "run": ran,
        "log": log,
    })
}

fn expected(spec_text: &str) -> Value {
    let text = std::fs::read_to_string(manifest().join(FIXTURE)).unwrap();
    let mut main: Value = serde_json::from_str(&text).unwrap();
    // `main` freezes the spec file's text as it was given; the file under test is what was given.
    main["frozen"]["instance.yaml"] = json!(spec_text);
    main
}

#[test]
fn the_example_spec_behaves_as_it_did_before_the_1_0_types() {
    let text = example();
    let observed = observe(&text);
    if let Some(out) = std::env::var_os("CORTEX_SPEC_COMPAT_RECORD") {
        std::fs::write(out, serde_json::to_string_pretty(&observed).unwrap() + "\n").unwrap();
        return;
    }
    assert_eq!(observed, expected(&text));
}

#[test]
fn the_example_with_every_1_0_field_at_its_default_behaves_as_before() {
    let text = with_1_0_defaults(&example(), SQLITE_SHORT);
    assert_eq!(observe(&text), expected(&text));
}

#[test]
fn the_example_with_the_sqlite_store_value_written_out_empty_behaves_as_before() {
    let text = with_1_0_defaults(&example(), SQLITE_EMPTY_VALUE);
    assert_eq!(observe(&text), expected(&text));
}

#[test]
fn the_example_with_the_organisation_scale_fields_at_their_defaults_behaves_as_before() {
    let text = with_organisation_defaults(&example());
    assert_eq!(observe(&text), expected(&text));
}
