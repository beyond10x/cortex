//! The home's own files hold through a failure (`story:durable-home-writes`): a `create` that
//! fails after it recorded the instance leaves no registry entry and no directory, so a retry is
//! not `name-taken`, and a status file that cannot be read is an error on `cortex sources`, never
//! rewritten with its counts started again.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{ekr, executable, World};
use serde_json::Value;

/// `cortex` in the world's home without `World::cortex`'s refusal of exit 2: the exit code,
/// stdout and stderr.
fn cortex_raw(w: &World, args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(args)
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// An `ekr` that, while the file `break-meta` exists, makes `meta.json` of instance `name` a
/// directory before it runs the real one: the instance's meta can then not be saved.
fn meta_breaking_ekr(w: &World, name: &str) -> PathBuf {
    let path = w.bin.join("ekr-breaking-meta");
    executable(
        &path,
        &format!(
            "if [ -e \"{root}/break-meta\" ]; then mkdir -p \"{home}/instances/{name}/meta.json/x\"; fi\nexec \"{ekr}\" \"$@\"\n",
            root = w.root.display(),
            home = w.home.display(),
            ekr = ekr().display()
        ),
    );
    path
}

fn names(w: &World) -> Vec<String> {
    let (code, out, err) = cortex_raw(w, &["list"]);
    assert_eq!(code, Some(0), "{out}{err}");
    out.lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap()["name"].to_string())
        .collect()
}

fn registry(w: &World) -> Value {
    serde_json::from_slice(&std::fs::read(w.home.join("registry.json")).unwrap()).unwrap()
}

#[test]
fn a_create_that_fails_after_recording_the_instance_leaves_no_registry_entry() {
    let w = World::new();
    let wrapper = meta_breaking_ekr(&w, "t");
    let spec = w.spec("t", "conn_test");
    let text = std::fs::read_to_string(&spec)
        .unwrap()
        .replace(&ekr().display().to_string(), &wrapper.display().to_string());
    std::fs::write(&spec, text).unwrap();
    std::fs::write(w.root.join("break-meta"), "").unwrap();

    let args = [
        "create",
        "--spec",
        spec.to_str().unwrap(),
        "--no-extract",
        "--no-units",
    ];
    let (code, out, err) = cortex_raw(&w, &args);
    assert_eq!(code, Some(2), "{out}{err}");
    assert!(err.contains("meta.json"), "{err}");
    assert_eq!(names(&w), Vec::<String>::new());
    let held = registry(&w);
    assert_eq!(held["instances"], Value::Array(vec![]), "{held}");
    assert_eq!(held["sources"], Value::Array(vec![]), "{held}");
    assert!(
        !w.home.join("instances/t").exists(),
        "the failed create left its directory"
    );

    std::fs::remove_file(w.root.join("break-meta")).unwrap();
    let (code, created) = w.cortex(&args);
    assert_eq!(
        (code, created["outcome"].as_str()),
        (0, Some("created")),
        "the retry: {created}"
    );
    assert_eq!(names(&w), ["\"t\""]);
}

fn status_file(w: &World) -> PathBuf {
    w.home.join("instances/t/status/news.json")
}

fn create(w: &World) {
    let spec = w.spec("t", "conn_test");
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(
        (code, created["outcome"].as_str()),
        (0, Some("created")),
        "{created}"
    );
}

fn corrupt(path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, b"").unwrap();
}

#[test]
fn a_corrupt_status_file_fails_cortex_sources_naming_it_and_is_not_rewritten() {
    let w = World::new();
    create(&w);
    let (code, ran) = w.cortex(&["run", "t/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let file = status_file(&w);
    assert!(file.is_file());
    corrupt(&file);

    let (code, out, err) = cortex_raw(&w, &["sources"]);
    assert!(code.is_some_and(|c| c != 0), "{code:?} {out}{err}");
    assert!(err.contains(&file.display().to_string()), "{err}");

    // A run still runs, and leaves the file as it found it; the log says why.
    let (code, out, err) = cortex_raw(&w, &["run", "t/news"]);
    assert_eq!(code, Some(0), "{out}{err}");
    assert_eq!(std::fs::read(&file).unwrap(), b"");
    let log = std::fs::read_to_string(w.home.join("instances/t/cortex.log")).unwrap();
    assert!(
        log.lines()
            .any(|l| l.contains("\"status\"") && l.contains("news.json")),
        "{log}"
    );
    // Every other command still reads the home.
    assert_eq!(names(&w), ["\"t\""]);
}
