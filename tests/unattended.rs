//! A source's systemd unit runs with nobody at the keyboard: the user manager starts it with its
//! own environment (a `PATH` without the user's tool directories, its own `HOME`, the home as
//! working directory) plus what the unit file sets. These tests read the unit `cortex create`
//! writes and run its `ExecStart` exactly that way, against the stand-ins of `tests/common`.

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use common::{executable, World};
use serde_json::Value;

/// The `PATH` a systemd user manager gives a service: no `~/.local/bin`, no `~/.cargo/bin`.
const MANAGER_PATH: &str = "/usr/bin:/bin";

/// Splits a unit's `ExecStart=` value into words: double quotes group, `\"` and `\\` escape.
fn words(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    let mut started = false;
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            '\\' if quoted => word.extend(chars.next()),
            ' ' if !quoted => {
                if started || !word.is_empty() {
                    out.push(std::mem::take(&mut word));
                }
                started = false;
            }
            c => word.push(c),
        }
    }
    if started || !word.is_empty() {
        out.push(word);
    }
    out
}

/// The `Environment=` assignments and the `ExecStart=` words of a service file.
fn read_unit(path: &Path) -> (BTreeMap<String, String>, Vec<String>) {
    let text = std::fs::read_to_string(path).unwrap();
    let mut env = BTreeMap::new();
    let mut exec = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("Environment=") {
            let assignment = words(rest).remove(0);
            let (key, value) = assignment.split_once('=').unwrap();
            env.insert(key.to_string(), value.to_string());
        } else if let Some(rest) = line.strip_prefix("ExecStart=") {
            exec = words(rest);
        }
    }
    assert!(!exec.is_empty(), "no ExecStart in {}", path.display());
    (env, exec)
}

/// Runs a service as the user manager would: a cleared environment holding the manager's `HOME`
/// and `PATH`, overridden by the unit's `Environment=`, in the manager's `HOME`, with no stdin.
fn run_as_manager(service: &Path, manager_home: &Path) -> (i32, Value) {
    let (env, exec) = read_unit(service);
    std::fs::create_dir_all(manager_home).unwrap();
    let out = Command::new(&exec[0])
        .args(&exec[1..])
        .env_clear()
        .env("HOME", manager_home)
        .env("PATH", MANAGER_PATH)
        .envs(&env)
        .current_dir(manager_home)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let first: Value = stdout
        .lines()
        .next()
        .and_then(|l| serde_json::from_str(l).ok())
        .unwrap_or(Value::Null);
    assert!(
        out.status.code() != Some(2),
        "the unit's cortex failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code().unwrap_or(-1), first)
}

/// `cortex create` as a person runs it from a shell: tools named by `CORTEX_*` and found on no
/// directory of the manager's `PATH`, `home` given as `--home`, run from `cwd`.
fn create(w: &World, home: &str, cwd: &Path, user_home: &Path, claude: &Path) -> (i32, Value) {
    let spec = w.spec("t", "conn_test");
    create_from(w, spec.to_str().unwrap(), home, cwd, user_home, claude)
}

/// [`create`] with the spec file `spec` as written on the command line.
fn create_from(
    w: &World,
    spec: &str,
    home: &str,
    cwd: &Path,
    user_home: &Path,
    claude: &Path,
) -> (i32, Value) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cortex"));
    cmd.args(["--home", home, "create", "--spec", spec])
        .env_clear()
        .env("PATH", MANAGER_PATH)
        .env("HOME", user_home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", claude)
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .current_dir(cwd);
    if let Some(tmp) = std::env::var_os("TMPDIR") {
        cmd.env("TMPDIR", tmp);
    }
    let out = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let last = stdout.lines().last().unwrap_or("null");
    (
        out.status.code().unwrap_or(-1),
        serde_json::from_str(last).unwrap_or(Value::Null),
    )
}

fn service(w: &World) -> PathBuf {
    w.units.join("cortex-t-news.service")
}

#[test]
fn a_source_unit_runs_the_connectors_and_claude_cortex_was_created_with() {
    let w = World::new();
    let user_home = w.root.join("user-home");
    std::fs::create_dir_all(&user_home).unwrap();
    let home = w.home.to_str().unwrap().to_string();
    let (code, created) = create(&w, &home, &w.root, &user_home, &w.bin.join("claude"));
    assert_eq!(
        (code, created["outcome"].as_str()),
        (0, Some("created")),
        "{created}"
    );

    let (code, ran) = run_as_manager(&service(&w), &w.root.join("manager-home"));
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
    assert_eq!(
        w.lines("claude-calls.log").len(),
        1,
        "the stand-in claude ran"
    );
}

#[test]
fn a_source_unit_made_from_a_relative_home_runs_from_the_manager_s_directory() {
    let w = World::new();
    let user_home = w.root.join("user-home");
    std::fs::create_dir_all(&user_home).unwrap();
    // `home` relative to `w.root`, which is `w.home`.
    let (code, created) = create(&w, "home", &w.root, &user_home, &w.bin.join("claude"));
    assert_eq!(
        (code, created["outcome"].as_str()),
        (0, Some("created")),
        "{created}"
    );

    let (_, exec) = read_unit(&service(&w));
    assert!(Path::new(&exec[0]).is_absolute(), "{exec:?}");
    let home_arg = exec.iter().position(|a| a == "--home").unwrap() + 1;
    assert_eq!(Path::new(&exec[home_arg]), w.home, "{exec:?}");
    let (view_env, _) = read_unit(&w.units.join("cortex-t-view.service"));
    for key in ["EKR_HOST", "EKR_STORE"] {
        assert!(
            Path::new(&view_env[key]).starts_with(&w.home),
            "the viewer's {key} is {:?}",
            view_env[key]
        );
    }

    let (code, ran) = run_as_manager(&service(&w), &w.root.join("manager-home"));
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
}

#[test]
fn a_source_unit_runs_with_the_home_directory_cortex_was_created_with() {
    let w = World::new();
    let user_home = w.root.join("user-home");
    std::fs::create_dir_all(&user_home).unwrap();
    // A claude that records the `HOME` it finds its sign-in under, then answers as the stand-in.
    let claude = w.bin.join("claude-home");
    executable(
        &claude,
        &format!(
            "echo \"$HOME\" >> \"{root}/claude-home.log\"\nexec \"{root}/bin/claude\" \"$@\"\n",
            root = w.root.display()
        ),
    );
    let home = w.home.to_str().unwrap().to_string();
    let (code, created) = create(&w, &home, &w.root, &user_home, &claude);
    assert_eq!(
        (code, created["outcome"].as_str()),
        (0, Some("created")),
        "{created}"
    );

    let (code, ran) = run_as_manager(&service(&w), &w.root.join("manager-home"));
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(
        w.lines("claude-home.log"),
        [user_home.display().to_string()],
        "claude looks for its sign-in under the HOME cortex was created with"
    );
}

#[test]
fn a_files_source_created_from_a_bare_spec_file_name_reads_its_files_from_the_unit() {
    let w = World::new();
    let user_home = w.root.join("user-home");
    let spec_dir = w.root.join("spec");
    std::fs::create_dir_all(spec_dir.join("docs")).unwrap();
    std::fs::create_dir_all(&user_home).unwrap();
    std::fs::write(
        spec_dir.join("docs/a.md"),
        "Example Labs develops the Widget engine.",
    )
    .unwrap();
    std::fs::write(
        spec_dir.join("files.yaml"),
        format!(
            r#"format: cortex.instance/1
name: t
description: Test brain.
ekr: {{version: "0.0.30", bin: "{ekr}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
  - name: news
    schedule: daily
    settings:
      kind: files
      value: {{paths: [docs], glob: "**/*.md"}}
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
serve: {{view_port: 18995}}
"#,
            ekr = common::ekr().display()
        ),
    )
    .unwrap();
    // Run from the spec's own directory with the file's bare name, as a person would.
    let home = w.home.to_str().unwrap().to_string();
    let (code, created) = create_from(
        &w,
        "files.yaml",
        &home,
        &spec_dir,
        &user_home,
        &w.bin.join("claude"),
    );
    assert_eq!(
        (code, created["outcome"].as_str()),
        (0, Some("created")),
        "{created}"
    );

    let (code, ran) = run_as_manager(&service(&w), &w.root.join("manager-home"));
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
}
