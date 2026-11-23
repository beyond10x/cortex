//! `.github/scripts/scope.sh` decides whether the `Check` workflow builds and tests a pull request:
//! a list of only planning and prose paths prints `skip`, any other path prints `run`.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn scope(paths: &[&str]) -> String {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".github/scripts/scope.sh");
    let mut child = Command::new("/bin/sh")
        .arg(&script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = paths.join("\n");
    input.push('\n');
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "{} failed", script.display());
    String::from_utf8(out.stdout).unwrap().trim().to_owned()
}

#[test]
fn planning_and_prose_only_skips_the_gate() {
    assert_eq!(
        scope(&[
            ".engineering/planning/story/review-ci-budget.md",
            ".engineering/planning/epic/standalone-1-0.md",
            "CHANGELOG.md",
            "STATUS.md",
            "docs/handoff/2026-10-06-brain-and-cortex.md",
        ]),
        "skip"
    );
}

#[test]
fn a_source_file_runs_the_gate() {
    assert_eq!(
        scope(&[
            ".engineering/planning/story/review-ci-budget.md",
            "src/main.rs"
        ]),
        "run"
    );
    assert_eq!(scope(&["README.md", "Cargo.lock"]), "run");
    assert_eq!(scope(&[".github/workflows/check.yml"]), "run");
}

#[test]
fn markdown_a_check_reads_runs_the_gate() {
    for path in [
        "website/docs/intro.md",
        "spec/compat-ack.md",
        "generated/docs/docs/index.md",
        "tests/fixtures/example.md",
        "crates/cortex-docs/README.md",
        "examples/README.md",
    ] {
        assert_eq!(scope(&[".engineering/x.md", path]), "run", "{path}");
    }
}

#[test]
fn an_empty_list_runs_the_gate() {
    assert_eq!(scope(&[]), "run");
}
