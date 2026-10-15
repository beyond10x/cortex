//! `story:model-through-gateway`: an instance's `model.gateway` names a base URL and the variable
//! holding the gateway's token, and cortex passes exactly that set to `claude`
//! (`ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`, `CLAUDE_CODE_MAX_CONTEXT_TOKENS`) from an
//! interactive run and a timer unit alike, and removes `ANTHROPIC_API_KEY` always. Without a
//! gateway the gateway variables are removed too. A unit file names the token variable, never its
//! value.

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use common::{executable, World};
use serde_json::Value;

/// The variables a gateway call depends on, as the stand-in records them.
const WATCHED: &str = "^(ANTHROPIC_|CLAUDE_CODE_MAX_CONTEXT_TOKENS=)";

/// A `claude` that records, per call, the watched variables it was started with, then answers as
/// the world's stand-in does.
fn recording_claude(w: &World) -> PathBuf {
    let path = w.bin.join("claude-env");
    executable(
        &path,
        &format!(
            "{{ echo call; env | grep -E '{WATCHED}' | sort; }} >> \"{root}/gateway-env.log\"\n\
             exec \"{root}/bin/claude\" \"$@\"\n",
            root = w.root.display()
        ),
    );
    path
}

/// The watched variables of each call the recording `claude` saw.
fn seen(w: &World) -> Vec<BTreeMap<String, String>> {
    let mut calls = Vec::new();
    for line in w.lines("gateway-env.log") {
        if line == "call" {
            calls.push(BTreeMap::new());
        } else if let Some((key, value)) = line.split_once('=') {
            calls
                .last_mut()
                .expect("a call line first")
                .insert(key.to_string(), value.to_string());
        }
    }
    calls
}

/// The world's spec of `name`, with `gateway` (YAML flow text) as its `model.gateway`.
fn spec_with(w: &World, name: &str, gateway: Option<&str>) -> PathBuf {
    let spec = w.spec(name, "conn_test");
    if let Some(gateway) = gateway {
        let text = std::fs::read_to_string(&spec).unwrap();
        let from = "timeout_s: 60}";
        assert!(text.contains(from), "{text}");
        std::fs::write(
            &spec,
            text.replacen(from, &format!("timeout_s: 60, gateway: {gateway}}}"), 1),
        )
        .unwrap();
    }
    spec
}

const GATEWAY: &str = r#"{base_url: "http://gw.test", token_env: GW_TOKEN}"#;

fn created(w: &World, spec: &Path, env: &[(&str, &Path)]) {
    let (code, out) = w.cortex_env(
        &["create", "--spec", spec.to_str().unwrap(), "--no-units"],
        env,
    );
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("created")),
        "{out}"
    );
}

#[test]
fn a_gateway_run_passes_its_url_and_token_to_claude_and_never_the_api_key() {
    let w = World::new();
    let claude = recording_claude(&w);
    let spec = spec_with(&w, "g", Some(GATEWAY));
    created(&w, &spec, &[]);
    // `World` sets ANTHROPIC_API_KEY on every cortex it runs.
    let (code, ran) = w.cortex_env(
        &["--claude", claude.to_str().unwrap(), "run", "g/news"],
        &[("GW_TOKEN", Path::new("t1"))],
    );
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let calls = seen(&w);
    assert_eq!(calls.len(), 1, "{calls:?}");
    let call = &calls[0];
    assert_eq!(
        call.get("ANTHROPIC_BASE_URL").map(String::as_str),
        Some("http://gw.test"),
        "{call:?}"
    );
    assert_eq!(
        call.get("ANTHROPIC_AUTH_TOKEN").map(String::as_str),
        Some("t1"),
        "{call:?}"
    );
    assert!(!call.contains_key("ANTHROPIC_API_KEY"), "{call:?}");
    assert!(
        !call.contains_key("CLAUDE_CODE_MAX_CONTEXT_TOKENS"),
        "not given: {call:?}"
    );
}

#[test]
fn a_gateway_with_max_context_tokens_passes_them_and_replaces_an_inherited_url_and_token() {
    let w = World::new();
    let claude = recording_claude(&w);
    let spec = spec_with(
        &w,
        "m",
        Some(r#"{base_url: "http://gw.test", token_env: GW_TOKEN, max_context_tokens: 200000}"#),
    );
    created(&w, &spec, &[]);
    let (code, ran) = w.cortex_env(
        &["--claude", claude.to_str().unwrap(), "run", "m/news"],
        &[
            ("GW_TOKEN", Path::new("t2")),
            ("ANTHROPIC_BASE_URL", Path::new("http://elsewhere.test")),
            ("ANTHROPIC_AUTH_TOKEN", Path::new("inherited")),
        ],
    );
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let call = &seen(&w)[0];
    let expected: BTreeMap<String, String> = [
        ("ANTHROPIC_AUTH_TOKEN", "t2"),
        ("ANTHROPIC_BASE_URL", "http://gw.test"),
        ("CLAUDE_CODE_MAX_CONTEXT_TOKENS", "200000"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();
    assert_eq!(call, &expected, "exactly the gateway's set");
}

#[test]
fn a_gateway_whose_token_variable_is_unset_or_empty_fails_naming_it_before_claude_starts() {
    let w = World::new();
    let claude = recording_claude(&w);
    let spec = spec_with(&w, "u", Some(GATEWAY));
    created(&w, &spec, &[]);
    for env in [vec![], vec![("GW_TOKEN", Path::new(""))]] {
        let (code, ran) = w.cortex_env(
            &["--claude", claude.to_str().unwrap(), "run", "u/news"],
            &env,
        );
        assert_eq!(
            (code, ran["outcome"].as_str()),
            (1, Some("extraction-failed")),
            "{ran}"
        );
        let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
        assert!(reason.contains("GW_TOKEN"), "{ran}");
        assert!(seen(&w).is_empty(), "claude was started: {:?}", seen(&w));
        assert!(w.lines("claude-calls.log").is_empty(), "claude was asked");
    }
}

#[test]
fn without_a_gateway_claude_sees_no_gateway_variable_and_no_api_key() {
    let w = World::new();
    let claude = recording_claude(&w);
    let spec = spec_with(&w, "n", None);
    created(&w, &spec, &[]);
    let (code, ran) = w.cortex_env(
        &["--claude", claude.to_str().unwrap(), "run", "n/news"],
        &[
            ("ANTHROPIC_BASE_URL", Path::new("http://gw.test")),
            ("ANTHROPIC_AUTH_TOKEN", Path::new("t1")),
            ("CLAUDE_CODE_MAX_CONTEXT_TOKENS", Path::new("1000")),
        ],
    );
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let calls = seen(&w);
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert!(calls[0].is_empty(), "{:?}", calls[0]);
}

/// Splits a unit's value into words: double quotes group, `\"` and `\\` escape.
fn words(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    let (mut quoted, mut started) = (false, false);
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => (quoted, started) = (!quoted, true),
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

/// Runs a service as the user manager would: a cleared environment holding the manager's `HOME`,
/// `PATH` and `manager` variables, of which only those `PassEnvironment=` names reach the
/// service, overridden by the unit's `Environment=`.
fn run_as_manager(service: &Path, manager: &[(&str, &str)]) -> (i32, Value) {
    let text = std::fs::read_to_string(service).unwrap();
    let mut env = BTreeMap::new();
    let mut passed = Vec::new();
    let mut exec = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("Environment=") {
            let assignment = words(rest).remove(0);
            let (key, value) = assignment.split_once('=').unwrap();
            env.insert(key.to_string(), value.to_string());
        } else if let Some(rest) = line.strip_prefix("PassEnvironment=") {
            passed.extend(rest.split_whitespace().map(str::to_string));
        } else if let Some(rest) = line.strip_prefix("ExecStart=") {
            exec = words(rest);
        }
    }
    let home = service.parent().unwrap().join("manager-home");
    std::fs::create_dir_all(&home).unwrap();
    let out = Command::new(&exec[0])
        .args(&exec[1..])
        .env_clear()
        .env("HOME", &home)
        .env("PATH", "/usr/bin:/bin")
        .envs(
            manager
                .iter()
                .filter(|(key, _)| passed.iter().any(|p| p == key))
                .copied(),
        )
        .envs(&env)
        .current_dir(&home)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let first = stdout
        .lines()
        .next()
        .and_then(|l| serde_json::from_str(l).ok())
        .unwrap_or(Value::Null);
    (out.status.code().unwrap_or(-1), first)
}

#[test]
fn a_unit_names_the_token_variable_never_its_value_and_its_run_reads_the_managers() {
    let w = World::new();
    let claude = recording_claude(&w);
    let spec = spec_with(&w, "t", Some(GATEWAY));
    let token = "gw-token-value-7f3a91";
    let (code, out) = w.cortex_env(
        &[
            "--claude",
            claude.to_str().unwrap(),
            "create",
            "--spec",
            spec.to_str().unwrap(),
        ],
        &[("GW_TOKEN", Path::new(token))],
    );
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("created")),
        "{out}"
    );
    let mut files = 0;
    for entry in std::fs::read_dir(&w.units).unwrap() {
        let path = entry.unwrap().path();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains(token), "{}:\n{text}", path.display());
        files += 1;
    }
    assert!(files >= 3, "the source's service and timer, and the viewer");
    let service = w.units.join("cortex-t-news.service");
    let text = std::fs::read_to_string(&service).unwrap();
    assert!(text.contains("PassEnvironment=GW_TOKEN\n"), "{text}");

    // Without the token in the user manager's environment, the timer's run fails naming the
    // variable and claude is not started. It runs first, while the documents are new.
    let (code, ran) = run_as_manager(&service, &[("OTHER", "t-manager")]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("extraction-failed")),
        "{ran}"
    );
    assert!(ran.to_string().contains("GW_TOKEN"), "{ran}");
    assert!(seen(&w).is_empty(), "claude was started: {:?}", seen(&w));

    // With it there, the timer's run reads it from the user manager's environment.
    let (code, ran) = run_as_manager(&service, &[("GW_TOKEN", "t-manager")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let calls = seen(&w);
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert_eq!(
        calls[0].get("ANTHROPIC_AUTH_TOKEN").map(String::as_str),
        Some("t-manager"),
        "{calls:?}"
    );
    assert_eq!(
        calls[0].get("ANTHROPIC_BASE_URL").map(String::as_str),
        Some("http://gw.test")
    );
}

#[test]
fn a_token_variable_a_unit_writes_by_value_or_no_variable_name_is_refused() {
    for token_env in [
        "CONNECTORS_GW_TOKEN",
        "PATH",
        "CORTEX_CLAUDE",
        "\"GW TOKEN\"",
    ] {
        let w = World::new();
        let spec = spec_with(
            &w,
            "r",
            Some(&format!(
                r#"{{base_url: "http://gw.test", token_env: {token_env}}}"#
            )),
        );
        // A refused spec exits 2, which `World::cortex_env` takes for a crash.
        let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
            .args(["create", "--spec", spec.to_str().unwrap()])
            .env("CORTEX_HOME", &w.home)
            .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
            .env("CORTEX_CLAUDE", w.bin.join("claude"))
            .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
            .env("CORTEX_UNIT_DIR", &w.units)
            .env("CONNECTORS_GW_TOKEN", "gw-token-value-7f3a91")
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{token_env}: {stderr}");
        assert!(
            stderr.contains("model.gateway.token_env"),
            "{token_env}: {stderr}"
        );
        assert!(
            std::fs::read_dir(&w.units).map_or(true, |mut d| d.next().is_none()),
            "{token_env}: a unit was written"
        );
    }
}
