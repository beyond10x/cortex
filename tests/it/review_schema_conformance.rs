//! Finding 25 of the 0.7.2 review: a number the published schema types `integer`
//! (`website/static/schemas/instance-spec.schema.json`, draft 2020-12) is any number with a zero
//! fractional part, so `timeout_s: 1.0` is valid there. `create` reads every such number as a
//! whole number however it is spelled, and still refuses a fraction.

use crate::common;

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::common::World;
use serde_json::Value;

/// A spec with no sources whose `model.timeout_s` is written `timeout`, as YAML or as JSON.
fn spec(w: &World, name: &str, timeout: &str, json: bool) -> PathBuf {
    let ekr = common::ekr();
    let (file, text) = if json {
        (
            format!("{name}.json"),
            format!(
                r#"{{"format": "cortex.instance/1", "name": "{name}", "description": "Test brain.",
 "ekr": {{"version": "0.0.34", "bin": "{ekr}"}}, "seed": {{"documents": []}},
 "model": {{"model": "claude-haiku-4-5-20251001", "budget_usd": "1", "timeout_s": {timeout}}},
 "sources": [], "serve": {{"view_port": 0}}}}"#,
                ekr = ekr.display()
            ),
        )
    } else {
        (
            format!("{name}.yaml"),
            format!(
                "format: cortex.instance/1\nname: {name}\ndescription: Test brain.\n\
                 ekr: {{version: \"0.0.34\", bin: \"{ekr}\"}}\nseed: {{documents: []}}\n\
                 model: {{model: claude-haiku-4-5-20251001, budget_usd: \"1\", timeout_s: {timeout}}}\n\
                 sources: []\nserve: {{view_port: 0}}\n",
                ekr = ekr.display()
            ),
        )
    };
    let path = w.root.join(file);
    std::fs::write(&path, text).unwrap();
    path
}

/// `cortex create --spec <spec> --no-extract --no-units` in the world: its exit status, the last
/// line of its standard output as JSON, and its standard error.
fn create(w: &World, spec: &Path) -> (i32, Value, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["create", "--spec", spec.to_str().unwrap()])
        .args(["--no-extract", "--no-units"])
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let last = stdout.lines().last().unwrap_or("null");
    (
        out.status.code().unwrap_or(-1),
        serde_json::from_str(last).unwrap_or(Value::Null),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn create_reads_a_whole_number_in_any_spelling_and_refuses_a_fraction() {
    let w = World::new();
    for json in [false, true] {
        let form = if json { "json" } else { "yaml" };
        for (timeout, label) in [("1", "one"), ("1.0", "one-point-zero")] {
            let name = format!("{form}-{label}");
            let (code, out, stderr) = create(&w, &spec(&w, &name, timeout, json));
            assert_eq!(
                (code, out["outcome"].as_str()),
                (0, Some("created")),
                "{form} timeout_s: {timeout}: {out} {stderr}"
            );
            let frozen =
                cortex_cli::spec::load(&w.home.join("instances").join(&name).join("instance.yaml"))
                    .unwrap_or_else(|e| {
                        panic!("{form} timeout_s: {timeout}: the frozen copy: {e}")
                    });
            assert_eq!(
                frozen.model.model.timeout_s, 1,
                "{form} timeout_s: {timeout}"
            );
        }
        let (code, out, stderr) = create(&w, &spec(&w, &format!("{form}-fraction"), "1.5", json));
        assert_eq!(code, 2, "{form} timeout_s: 1.5 was not refused: {out}");
        assert!(
            stderr.contains("model.timeout_s: 1.5 is not a whole number"),
            "{form}: {stderr}"
        );
    }
}

/// Every integer of the schema goes through one reading: a few of them, each spelled as a decimal
/// with a zero fraction or with an exponent, load; the same fields with a fraction are refused,
/// naming the field.
#[test]
fn every_integer_of_the_spec_reads_a_whole_number_in_any_spelling() {
    let base = |model: &str, extra: &str| {
        format!(
            "format: cortex.instance/1\nname: brain\ndescription: d\nekr: {{version: \"0.0.34\"}}\n\
             seed: {{documents: []}}\nmodel: {{model: m, {model}}}\nsources: []\nserve: {{}}\n{extra}"
        )
    };
    let loaded = cortex_cli::spec::parse(&base(
        "budget_tokens: 1e3, daily_ceiling_tokens: 2.50e2, timeout_s: 60.000, batch_chars: 10000.0, \
         gateway: {base_url: \"https://g.example\", token_env: G, max_context_tokens: 200000.0}",
        "snapshots: {keep: 3.0}\nruns: {keep_days: 0.7e1}\n",
    ))
    .expect("whole numbers in every spelling load");
    assert_eq!(loaded.model.budget_tokens, Some(1000));
    assert_eq!(loaded.model.daily_ceiling_tokens, Some(250));
    assert_eq!(loaded.model.timeout_s, 60);
    assert_eq!(loaded.model.batch_chars, Some(10_000));
    assert_eq!(
        loaded.model.gateway.and_then(|g| g.max_context_tokens),
        Some(200_000)
    );
    assert_eq!(loaded.snapshots.map(|s| s.keep), Some(3));
    assert_eq!(loaded.runs.map(|r| r.keep_days), Some(7));

    for (model, extra, refusal) in [
        (
            "budget_usd: \"1\", timeout_s: 1.5",
            "",
            "model.timeout_s: 1.5 is not a whole number",
        ),
        (
            "budget_usd: \"1\", timeout_s: 0.001",
            "",
            "model.timeout_s: 0.001 is not a whole number",
        ),
        (
            "budget_usd: \"1\", timeout_s: 1e19",
            "",
            "model.timeout_s: 1e+19 is not a whole number",
        ),
        (
            "budget_usd: \"1\", timeout_s: 60",
            "snapshots: {keep: 2.5}\n",
            "snapshots.keep: 2.5 is not a whole number",
        ),
    ] {
        let err = cortex_cli::spec::parse(&base(model, extra)).err();
        assert!(
            err.as_deref().is_some_and(|e| e.contains(refusal)),
            "{model} {extra}: {err:?}"
        );
    }
}
