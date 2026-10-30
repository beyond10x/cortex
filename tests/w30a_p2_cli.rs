//! Adversary pass 2 on unit cli of wave 20261030a (`story:spec-integer-bounds`, correction
//! f37f52e8). The correction moved the upper bounds into `spec::load_given`, so that an instance
//! 0.6.1 created with a `refresh_after_days` above 36500 keeps running from its frozen copy. But
//! `load_given` is also what `cortex ask serve --spec <file>` reads (`src/main.rs`, `ask_serve`),
//! and it read it in 0.6.1 too, without the bounds. An answer service that started under 0.6.1
//! over such a spec no longer starts, for a field the answer service never reads.

mod common;

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use common::ekr;

/// Starts `cortex ask serve` over an instance spec whose one `files` source has the
/// `refresh_after_days` `days`, and stops it: the first line it printed, its exit code and what it
/// wrote to standard error.
fn ask_serve(days: u64) -> (String, Option<i32>, String) {
    let tmp = tempfile::Builder::new()
        .prefix("cortex-p2-cli")
        .tempdir_in(
            std::env::var_os("CARGO_TARGET_TMPDIR")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir),
        )
        .unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("chat")).unwrap();
    let spec = root.join("asker.yaml");
    std::fs::write(
        &spec,
        format!(
            r#"format: cortex.instance/1
name: asker
description: Test brain.
ekr: {{version: "0.0.34", bin: "{ekr}"}}
seed: {{documents: []}}
model:
  model: test-model
  budget_tokens: 100000
  timeout_s: 60
  gateway: {{base_url: "http://127.0.0.1:9", token_env: P2_GATEWAY_TOKEN}}
sources:
  - name: chat
    schedule: daily
    settings:
      kind: files
      value:
        paths: [chat]
        glob: "*.jsonl"
        records: {{format: JsonLines, id: id, time: time, text: ["{{text}}"], filters: []}}
    policy: {{refresh_after_days: {days}, change: ContentHash, max_documents_per_run: 50, max_chars_per_document: 5000}}
serve: {{view_port: 18998}}
ask:
  listen: "127.0.0.1:0"
  identity_header: X-Identity
  limits: {{max_steps: 4, max_seconds: 30, max_result_bytes: 10000, per_identity_per_minute: 10}}
"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["ask", "serve", "--spec"])
        .arg(&spec)
        .env("CORTEX_HOME", root.join("home"))
        .env("P2_GATEWAY_TOKEN", "t0ken")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let _ = child.kill();
    let out = child.wait_with_output().unwrap();
    (
        line,
        out.status.code(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// The control: the same spec with a refresh window of 7 days starts and prints its ready line.
#[test]
fn ask_serve_starts_over_a_spec_with_a_short_refresh_window() {
    let (line, code, stderr) = ask_serve(7);
    assert!(
        line.contains("listening"),
        "no ready line; exit {code:?}; stderr: {stderr}"
    );
}

/// A `refresh_after_days` of 40000, which 0.6.1 created, ran and served (40000 days is far inside
/// `i64` milliseconds). The service must start as it did: print its ready line.
#[test]
fn ask_serve_starts_over_a_spec_0_6_1_served_with_a_long_refresh_window() {
    let (line, code, stderr) = ask_serve(40000);
    assert!(
        line.contains("listening"),
        "no ready line; exit {code:?}; stderr: {stderr}"
    );
}
