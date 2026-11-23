//! The integration tests are one binary (`tests/main.rs`): a test file under `tests/` would be a
//! binary of its own, and a file under `tests/it/` without its `mod` line would never run.

use std::path::PathBuf;

fn tests_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}

fn rust_files(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "rs"))
        .map(|path| path.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn layout_is_one_binary() {
    let tests = tests_dir();
    assert_eq!(
        rust_files(&tests),
        ["main"],
        "move each test file under tests/ into tests/it/ and declare it in tests/main.rs"
    );
    let main = std::fs::read_to_string(tests.join("main.rs")).unwrap();
    let missing: Vec<String> = rust_files(&tests.join("it"))
        .into_iter()
        .filter(|name| {
            !main
                .lines()
                .any(|line| line.trim() == format!("mod {name};"))
        })
        .collect();
    assert!(
        missing.is_empty(),
        "tests/it/ files without a `mod` line in tests/main.rs, so they never run: {missing:?}"
    );
}
