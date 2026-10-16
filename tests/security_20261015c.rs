//! Conformance checks of wave 20261015c's own contract (`cortex.instance.ModelGateway`,
//! `cortex.instance.ModelSpec`'s token limits and `cortex.instance.DailySpend.spent_tokens`):
//!
//! - the gateway's token value is written into no unit, spec, registry, state or log file;
//! - `token_env` naming a variable a unit writes by value, or no variable name, is refused by
//!   `create`, `update` and `adopt` alike, before anything is written;
//! - `base_url` never reaches a unit file, so a newline in it cannot add a unit line;
//! - a call's tokens are counted from its answer's `usage`; a count that is not a count (below 0)
//!   must not lift the run budget or the daily ceiling in tokens.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{ekr, executable, World};
use serde_json::json;

const TOKEN: &str = "gw-token-value-c0ffee-5150";

/// The world's spec of `name` with `gateway` (YAML flow text) as its `model.gateway`.
fn gateway_spec(w: &World, name: &str, gateway: &str) -> PathBuf {
    let spec = w.spec(name, "conn_test");
    let text = std::fs::read_to_string(&spec).unwrap();
    let from = "timeout_s: 60}";
    assert!(text.contains(from), "{text}");
    std::fs::write(
        &spec,
        text.replacen(from, &format!("timeout_s: 60, gateway: {gateway}}}"), 1),
    )
    .unwrap();
    spec
}

/// `cortex` with the world's variables and `env`, whatever its exit code.
fn raw(w: &World, args: &[&str], env: &[(&str, &str)]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(args)
        .envs(env.iter().copied())
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .env("ANTHROPIC_API_KEY", "must-not-reach-claude")
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Every regular file under `dir` whose bytes contain `needle`.
fn files_holding(dir: &Path, needle: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return found;
    };
    for entry in entries {
        let path = entry.unwrap().path();
        let meta = std::fs::symlink_metadata(&path).unwrap();
        if meta.is_dir() {
            found.extend(files_holding(&path, needle));
        } else if meta.is_file() {
            let bytes = std::fs::read(&path).unwrap();
            if bytes
                .windows(needle.len())
                .any(|window| window == needle.as_bytes())
            {
                found.push(path);
            }
        }
    }
    found
}

/// Create (with units), run interactively, run once more with the token unset (a failure that
/// names the variable), and `status`: no file under the home or the unit directory, and no
/// output of cortex, holds the token's value.
#[test]
fn the_gateway_token_value_lands_in_no_file_cortex_writes_and_no_output() {
    let w = World::new();
    let spec = gateway_spec(
        &w,
        "leak",
        r#"{base_url: "http://gw.test", token_env: GW_TOKEN, max_context_tokens: 1000}"#,
    );
    let env = [("GW_TOKEN", TOKEN)];
    let mut outputs = Vec::new();
    let (code, out, err) = raw(&w, &["create", "--spec", spec.to_str().unwrap()], &env);
    assert_eq!(code, Some(0), "{out}{err}");
    outputs.push(out + &err);
    // A failed run records its failure (`--record-failure`) in the instance's state. It runs
    // first, while the documents are new, so it reaches the model call.
    let (_, out, err) = raw(&w, &["run", "leak/news", "--record-failure"], &[]);
    assert!((out.clone() + &err).contains("GW_TOKEN"), "{out}{err}");
    outputs.push(out + &err);
    let (code, out, err) = raw(&w, &["run", "leak/news"], &env);
    assert_eq!(code, Some(0), "{out}{err}");
    outputs.push(out + &err);
    let (_, out, err) = raw(&w, &["status", "leak"], &env);
    outputs.push(out + &err);

    for output in &outputs {
        assert!(
            !output.contains(TOKEN),
            "cortex printed the token:\n{output}"
        );
    }
    let mut holding = files_holding(&w.home, TOKEN);
    holding.extend(files_holding(&w.units, TOKEN));
    assert!(holding.is_empty(), "files holding the token: {holding:?}");
    assert_eq!(
        w.lines("claude-calls.log").len(),
        1,
        "the run with the token called claude once"
    );
}

/// The variable names a unit writes by value (`PATH`, `HOME`, the four `XDG_*` it copies, every
/// `CONNECTORS_*` and `CORTEX_*`), any other `XDG_*` or `ANTHROPIC_*` name, and texts that are no
/// variable name.
const REFUSED_TOKEN_ENVS: [&str; 17] = [
    "PATH",
    "HOME",
    "XDG_CONFIG_HOME",
    "XDG_STATE_HOME",
    "XDG_DATA_HOME",
    "XDG_RUNTIME_DIR",
    "XDG_CACHE_HOME",
    "XDG_GW",
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_GW",
    "CONNECTORS_GW",
    "CORTEX_HOME",
    "\"\"",
    "\"1GW\"",
    "\"GW\\nPassEnvironment=PATH\"",
    "\"GW=1\"",
];

#[test]
fn create_update_and_adopt_refuse_a_token_env_a_unit_writes_by_value_or_no_name() {
    for token_env in REFUSED_TOKEN_ENVS {
        let gateway = format!(r#"{{base_url: "http://gw.test", token_env: {token_env}}}"#);
        refused_everywhere(&gateway, "model.gateway.token_env");
    }
}

/// A `base_url` that is empty or not `http://` or `https://`, and a `max_context_tokens` of 0 or
/// less, are refused by `create`, `update` and `adopt` alike, naming the field.
#[test]
fn create_update_and_adopt_refuse_a_base_url_not_http_or_a_context_limit_below_1() {
    for (gateway, field) in [
        (r#"{base_url: "", token_env: GW}"#, "model.gateway.base_url"),
        (
            r#"{base_url: "gw.test", token_env: GW}"#,
            "model.gateway.base_url",
        ),
        (
            r#"{base_url: "ftp://gw.test", token_env: GW}"#,
            "model.gateway.base_url",
        ),
        (
            r#"{base_url: "file:///gw", token_env: GW}"#,
            "model.gateway.base_url",
        ),
        (
            r#"{base_url: "HTTP//gw.test", token_env: GW}"#,
            "model.gateway.base_url",
        ),
        (
            r#"{base_url: "http://gw.test", token_env: GW, max_context_tokens: 0}"#,
            "model.gateway.max_context_tokens",
        ),
        (
            r#"{base_url: "https://gw.test", token_env: GW, max_context_tokens: -1}"#,
            "model.gateway.max_context_tokens",
        ),
    ] {
        refused_everywhere(gateway, field);
    }
}

/// `create`, `adopt` and `update` with `gateway` (YAML flow text) as `model.gateway` each exit 2
/// naming `field`, and leave no instance, no changed frozen spec and no changed unit behind.
fn refused_everywhere(gateway: &str, field: &str) {
    let w = World::new();
    let bad = gateway_spec(&w, "r", gateway);
    let bad = bad.to_str().unwrap();
    // Not `CONNECTORS_GW`: a unit copies every `CONNECTORS_*` by value, by design.
    let env = [("GW", TOKEN)];

    let (code, _, stderr) = raw(&w, &["create", "--spec", bad], &env);
    assert_eq!(code, Some(2), "create, {gateway}: {stderr}");
    assert!(stderr.contains(field), "{gateway}: {stderr}");
    assert!(
        !w.home.join("instances/r").exists(),
        "create left an instance"
    );

    let (code, _, stderr) = raw(
        &w,
        &[
            "adopt",
            "--spec",
            bad,
            "--store",
            "absent.sqlite",
            "--no-units",
        ],
        &env,
    );
    assert_eq!(code, Some(2), "adopt, {gateway}: {stderr}");
    assert!(stderr.contains(field), "{gateway}: {stderr}");
    assert!(
        !w.home.join("instances/r").exists(),
        "adopt left an instance"
    );

    // An instance without a gateway, then an update that names the refused gateway.
    let good = w.root.join("good.yaml");
    std::fs::write(
        &good,
        std::fs::read_to_string(w.spec("r", "conn_test")).unwrap(),
    )
    .unwrap();
    let bad_path = w.root.join("bad.yaml");
    let text = std::fs::read_to_string(&good).unwrap().replacen(
        "timeout_s: 60}",
        &format!("timeout_s: 60, gateway: {gateway}}}"),
        1,
    );
    std::fs::write(&bad_path, text).unwrap();
    let (code, out, stderr) = raw(&w, &["create", "--spec", good.to_str().unwrap()], &env);
    assert_eq!(code, Some(0), "{out}{stderr}");
    let units_before = unit_texts(&w.units);
    let frozen = std::fs::read_to_string(w.home.join("instances/r/instance.yaml")).unwrap();
    let (code, _, stderr) = raw(
        &w,
        &["update", "r", "--spec", bad_path.to_str().unwrap()],
        &env,
    );
    assert_eq!(code, Some(2), "update, {gateway}: {stderr}");
    assert!(stderr.contains(field), "{gateway}: {stderr}");
    assert_eq!(
        std::fs::read_to_string(w.home.join("instances/r/instance.yaml")).unwrap(),
        frozen,
        "a refused update changed the frozen spec"
    );
    assert_eq!(
        unit_texts(&w.units),
        units_before,
        "a refused update changed a unit"
    );
    let holding = files_holding(&w.units, TOKEN);
    assert!(
        holding.is_empty(),
        "{gateway}: units holding the token: {holding:?}"
    );
}

fn unit_texts(dir: &Path) -> Vec<(PathBuf, String)> {
    let mut texts: Vec<_> = std::fs::read_dir(dir)
        .map(|d| {
            d.map(|e| {
                let p = e.unwrap().path();
                let t = std::fs::read_to_string(&p).unwrap_or_default();
                (p, t)
            })
            .collect()
        })
        .unwrap_or_default();
    texts.sort();
    texts
}

/// A `base_url` holding a newline and unit syntax reaches no unit file: units name only
/// `token_env`, so no line of a unit can come from `base_url`.
#[test]
fn a_base_url_with_a_newline_adds_no_line_to_any_unit() {
    let w = World::new();
    let spec = gateway_spec(
        &w,
        "u",
        r#"{base_url: "http://gw.test\nEnvironment=ANTHROPIC_API_KEY=injected\"", token_env: GW_TOKEN}"#,
    );
    let (code, out, err) = raw(
        &w,
        &["create", "--spec", spec.to_str().unwrap()],
        &[("GW_TOKEN", TOKEN)],
    );
    // A refusal of such a base_url (exit 2) is as good as an inert one: neither writes the line.
    assert!(matches!(code, Some(0) | Some(2)), "{out}{err}");
    for (path, text) in unit_texts(&w.units) {
        assert!(!text.contains("gw.test"), "{}:\n{text}", path.display());
        assert!(!text.contains("injected"), "{}:\n{text}", path.display());
        assert!(!text.contains(TOKEN), "{}:\n{text}", path.display());
    }
}

// --- token counts from an answer's `usage` ---------------------------------------------------

/// `n` pages of about 41 000 characters each; three fit one batch (as `tests/token_budget.rs`).
fn pages(n: usize) -> String {
    let body = "Example Labs develops the Widget engine. ".repeat(1000);
    let page = |n: usize| {
        json!({"url": format!("https://example.org/{n}"), "title": "T", "description": "D",
            "content": body, "content_truncated": false, "published": null, "score": "0.9"})
    };
    let pages = json!({"results": (0..n).map(page).collect::<Vec<_>>(),
        "complete": false, "truncation": ["provider_limit"],
        "provenance": {"instance": "t", "profile": "tavily/2026-10",
            "received_at": "2026-10-05T00:00:00.000Z"}});
    common::answer(&pages.to_string())
}

fn large_spec(w: &World, name: &str, model: &str) -> PathBuf {
    let path = w.root.join(format!("{name}.yaml"));
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: {name}
description: Test brain.
ekr: {{version: "0.0.30", bin: "{ekr}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "5", timeout_s: 60{model}}}
sources:
  - name: news
    schedule: daily
    settings:
      kind: web
      value:
        connection: conn_test
        input:
          input: search
          value:
            queries: [widget engine]
            policy: {{topic: news, max_results: 3, include_domains: [], exclude_domains: []}}
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 50000}}
serve: {{view_port: 18995}}
"#,
            ekr = ekr().display(),
        ),
    )
    .unwrap();
    path
}

/// A `usage` whose input, cache writes and cache reads are 1900 tokens and whose output is
/// reported as `-1900`: what a gateway that misreports a count would hand `claude`.
const NEGATIVE_USAGE: &str = r#""usage":{"input_tokens":1500,"cache_creation_input_tokens":100,"cache_read_input_tokens":300,"output_tokens":-1900},"#;

/// Instance `name` over `n` pages (one batch per three) whose `claude` answers no cost and
/// [`NEGATIVE_USAGE`].
fn negative_usage_instance(name: &str, n: usize, model: &str) -> World {
    let w = World::new();
    std::fs::write(w.root.join("answer.json"), pages(n)).unwrap();
    let path = large_spec(&w, name, model);
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    std::fs::write(w.root.join("no-cost"), "").unwrap();
    std::fs::rename(w.bin.join("claude"), w.bin.join("claude-real")).unwrap();
    executable(
        &w.bin.join("claude"),
        &format!(
            "\"{root}/bin/claude-real\" \"$@\" | sed 's/\"is_error\":false,/\"is_error\":false,{usage}/'\n",
            root = w.root.display(),
            usage = NEGATIVE_USAGE
        ),
    );
    w
}

/// Under `budget_tokens: 1000`, every answer read 1900 input tokens: the first answer alone spends
/// the budget, so a run of three batches must stop before its second. A negative output count is
/// not a count, and must neither lower the run's tokens nor be taken for none spent.
#[test]
fn a_negative_token_count_in_an_answer_does_not_lift_the_run_token_budget() {
    let w = negative_usage_instance("neg", 7, ", budget_tokens: 1000");
    let (code, ran) = w.cortex(&["run", "neg/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let calls = w.lines("claude-calls.log").len();
    assert_eq!(
        calls, 1,
        "the run kept calling past its token budget: {ran}"
    );
    assert!(
        ran["detail"]["stopped"]
            .as_str()
            .is_some_and(|s| s.contains("token budget")),
        "{ran}"
    );
}

/// Under `daily_ceiling_tokens: 1000`, as above for the day's tokens: `spent_tokens` must not stay
/// at 0 while answers that read 1900 input tokens each are made: the first answer is uncounted
/// and stops the run.
#[test]
fn a_negative_token_count_in_an_answer_does_not_lift_the_daily_token_ceiling() {
    let w = negative_usage_instance("negday", 7, ", daily_ceiling_tokens: 1000");
    let (code, ran) = w.cortex(&["run", "negday/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let calls = w.lines("claude-calls.log").len();
    // The negative count leaves the first answer uncounted, which stops the run and adds
    // nothing, so `spend.json` may not exist.
    let spend =
        std::fs::read_to_string(w.home.join("instances/negday/spend.json")).unwrap_or_default();
    assert_eq!(
        calls, 1,
        "the run kept calling past the daily token ceiling: {ran}\nspend.json: {spend}"
    );
    assert!(
        ran["detail"]["stopped"]
            .as_str()
            .is_some_and(|s| s.contains("could not be counted")),
        "{ran}"
    );
}
