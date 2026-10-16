//! Behind a model gateway the dollar limits bound nothing (`story:gateway-dollar-limits`): with
//! `model.gateway` given, `model.budget_usd` may be omitted and `model.budget_tokens` is required,
//! the cost `claude` reports counts against no `budget_usd`, no `daily_ceiling_usd` and no
//! measurement's budget, `--max-budget-usd` is not passed to `claude`, and the reported cost is
//! recorded as not counted (`cost_counted: false` in the run's report, `uncounted_usd` in
//! `spend.json`). A gateway spec that still gives a dollar limit is accepted with one warning line.
//! Without a gateway nothing changes.

mod common;

use std::path::PathBuf;
use std::process::Command;

use common::{ekr, executable, World};
use serde_json::{json, Value};

/// What every answer of the stand-in reports: 2000 tokens, and a cost far above any dollar limit
/// these tests set.
const USAGE: &str = r#""usage":{"input_tokens":1500,"cache_creation_input_tokens":100,"cache_read_input_tokens":300,"output_tokens":100},"#;
const COST: &str = r#""total_cost_usd":50.5,"#;

const GATEWAY: &str = r#"{base_url: "http://gw.test", token_env: GW_TOKEN}"#;

/// `n` pages of about 41 000 characters each; three fit one batch (see `tests/cost.rs`).
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

/// A spec of `name` with the model fields `model` (YAML flow entries after `model`) and two
/// sources, `news` and `more`, reading the same pages.
fn spec(w: &World, name: &str, model: &str) -> PathBuf {
    let path = w.root.join(format!("{name}.yaml"));
    let source = |source: &str| {
        format!(
            r#"  - name: {source}
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
"#
        )
    };
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: {name}
description: Test brain.
ekr: {{version: "0.0.30", bin: "{ekr}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, timeout_s: 60{model}}}
sources:
{news}{more}serve: {{view_port: 18996}}
"#,
            ekr = ekr().display(),
            news = source("news"),
            more = source("more"),
        ),
    )
    .unwrap();
    path
}

/// Wraps the world's `claude` so that every answer, a judge's included, reports [`COST`] and
/// [`USAGE`] in place of its own cost.
fn priced_claude(w: &World) {
    std::fs::rename(w.bin.join("claude"), w.bin.join("claude-real")).unwrap();
    executable(
        &w.bin.join("claude"),
        &format!(
            r#""{root}/bin/claude-real" "$@" | sed 's/"total_cost_usd":[0-9.]*,//; s/"is_error":false,/"is_error":false,{COST}{USAGE}/'
"#,
            root = w.root.display()
        ),
    );
}

/// `cortex` with the world's variables, the gateway's token and `args`, whatever its exit code:
/// the exit code, the last stdout line as JSON, and stderr.
fn raw(w: &World, args: &[&str]) -> (Option<i32>, Value, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(args)
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .env("ANTHROPIC_API_KEY", "must-not-reach-claude")
        .env("GW_TOKEN", "gw-token")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let last = stdout.lines().last().unwrap_or("null");
    (
        out.status.code(),
        serde_json::from_str(last).unwrap_or(Value::Null),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Creates instance `name` over `n` pages with the model fields `model`, its `claude` priced at
/// [`COST`]; answers `create`'s stderr.
fn instance(name: &str, n: usize, model: &str) -> (World, String) {
    let w = World::new();
    std::fs::write(w.root.join("answer.json"), pages(n)).unwrap();
    let path = spec(&w, name, model);
    let (code, created, stderr) = raw(
        &w,
        &["create", "--spec", path.to_str().unwrap(), "--no-units"],
    );
    assert_eq!(
        (code, created["outcome"].as_str()),
        (Some(0), Some("created")),
        "{created}{stderr}"
    );
    priced_claude(&w);
    (w, stderr)
}

fn run(w: &World, source: &str) -> Value {
    let (code, ran, stderr) = raw(w, &["run", source]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (Some(0), Some("ran")),
        "{ran}{stderr}"
    );
    ran
}

fn calls(w: &World) -> usize {
    w.lines("claude-calls.log").len()
}

fn spend(w: &World, name: &str) -> Value {
    let path = w.home.join("instances").join(name).join("spend.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// No call the stand-in `claude` recorded was passed `--max-budget-usd`.
fn never_given_a_dollar_budget(w: &World) {
    let args = w.lines("claude-args.log");
    assert!(!args.is_empty(), "claude was not asked");
    for line in &args {
        assert!(!line.contains("--max-budget-usd"), "{line}");
    }
}

/// The warning `create`, `update` and `adopt` print for a gateway spec with a dollar limit.
fn warns(stderr: &str) -> bool {
    stderr
        .lines()
        .any(|l| l.contains("model.gateway") && l.contains("_usd") && l.contains("bound nothing"))
}

#[test]
fn a_gateway_instance_without_budget_usd_runs_every_batch_and_reports_the_cost_as_not_counted() {
    // Three batches at 50.5 reported USD each; the token budget leaves room for all three.
    let (w, stderr) = instance(
        "gw",
        7,
        &format!(", budget_tokens: 100000, gateway: {GATEWAY}"),
    );
    assert!(!warns(&stderr), "no dollar limit given: {stderr}");
    let ran = run(&w, "gw/news");
    let detail = &ran["detail"];
    assert_eq!(calls(&w), 3, "{ran}");
    assert_eq!(detail["documents_applied"], 7, "{ran}");
    assert_eq!(detail["stopped"], Value::Null, "{ran}");
    assert_eq!(detail["cost_usd"], "151.5000", "the reported cost: {ran}");
    assert_eq!(detail["cost_counted"], false, "{ran}");
    let spend = spend(&w, "gw");
    assert_eq!(spend["spent_usd"], "0", "{spend}");
    assert_eq!(spend["uncounted_usd"], "151.5", "{spend}");
    assert_eq!(spend["spent_tokens"], 6000, "{spend}");
    never_given_a_dollar_budget(&w);
}

#[test]
fn a_gateway_instance_is_bounded_by_its_token_budget_only() {
    // 2000 tokens a call and a budget of 3000: the second call spends it, and the third batch is
    // not asked for; the reported dollars stop nothing.
    let (w, _) = instance(
        "tok",
        7,
        &format!(", budget_tokens: 3000, gateway: {GATEWAY}"),
    );
    let ran = run(&w, "tok/news");
    assert_eq!(calls(&w), 2, "{ran}");
    let stopped = ran["detail"]["stopped"].as_str().unwrap_or_default();
    assert!(stopped.contains("token budget of 3000 tokens"), "{ran}");
    assert_eq!(ran["detail"]["cost_counted"], false, "{ran}");
    never_given_a_dollar_budget(&w);
}

#[test]
fn dollar_limits_given_behind_a_gateway_stop_nothing_and_create_warns_once() {
    let (w, stderr) = instance(
        "both",
        7,
        &format!(
            ", budget_usd: \"0.01\", daily_ceiling_usd: \"0.01\", budget_tokens: 100000, \
             gateway: {GATEWAY}"
        ),
    );
    assert_eq!(
        stderr.lines().filter(|l| warns(l)).count(),
        1,
        "one warning line: {stderr}"
    );
    let ran = run(&w, "both/news");
    assert_eq!(calls(&w), 3, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 7, "{ran}");
    assert_eq!(ran["detail"]["stopped"], Value::Null, "{ran}");
    assert_eq!(ran["detail"].get("daily_ceiling_reached"), None, "{ran}");
    assert_eq!(ran["detail"]["cost_counted"], false, "{ran}");
    // The day's reported spend is far past the ceiling, and a second source still runs.
    let ran = run(&w, "both/more");
    assert_eq!(calls(&w), 6, "{ran}");
    assert_eq!(ran["detail"]["stopped"], Value::Null, "{ran}");
    let spend = spend(&w, "both");
    assert_eq!(spend["spent_usd"], "0", "{spend}");
    assert_eq!(spend["uncounted_usd"], "303", "{spend}");
    never_given_a_dollar_budget(&w);
}

#[test]
fn update_and_adopt_warn_about_dollar_limits_behind_a_gateway() {
    let (w, _) = instance(
        "up",
        1,
        &format!(", budget_tokens: 100000, gateway: {GATEWAY}"),
    );
    let with_dollars = spec(
        &w,
        "up",
        &format!(", daily_ceiling_usd: \"1\", budget_tokens: 100000, gateway: {GATEWAY}"),
    );
    let (code, out, stderr) = raw(
        &w,
        &[
            "update",
            "up",
            "--spec",
            with_dollars.to_str().unwrap(),
            "--no-units",
        ],
    );
    assert_eq!(code, Some(0), "{out}{stderr}");
    assert!(warns(&stderr), "update: {stderr}");

    let other = World::new();
    let adopted = spec(
        &other,
        "ad",
        &format!(", budget_usd: \"1\", budget_tokens: 100000, gateway: {GATEWAY}"),
    );
    let (_, out, stderr) = raw(
        &other,
        &[
            "adopt",
            "--spec",
            adopted.to_str().unwrap(),
            "--store",
            "absent.sqlite",
            "--no-units",
        ],
    );
    // The store is absent, so adopt fails after the spec is read: the warning is printed first.
    assert!(warns(&stderr), "adopt: {out}{stderr}");
}

/// `create`, `adopt` and `update` of a spec with the model fields `model` each exit 2 naming
/// `field`, and leave no instance and no changed frozen spec behind.
fn refused_everywhere(model: &str, field: &str) {
    let w = World::new();
    let bad = spec(&w, "r", model);
    let bad = bad.to_str().unwrap();

    let (code, out, stderr) = raw(&w, &["create", "--spec", bad, "--no-units"]);
    assert_eq!(code, Some(2), "create {model}: {out}{stderr}");
    assert!(stderr.contains(field), "create {model}: {stderr}");
    assert!(
        !w.home.join("instances/r").exists(),
        "create left an instance"
    );

    let (code, out, stderr) = raw(
        &w,
        &[
            "adopt",
            "--spec",
            bad,
            "--store",
            "absent.sqlite",
            "--no-units",
        ],
    );
    assert_eq!(code, Some(2), "adopt {model}: {out}{stderr}");
    assert!(stderr.contains(field), "adopt {model}: {stderr}");
    assert!(
        !w.home.join("instances/r").exists(),
        "adopt left an instance"
    );

    let good = w.root.join("good.yaml");
    std::fs::write(
        &good,
        std::fs::read_to_string(spec(&w, "r", ", budget_usd: \"1\"")).unwrap(),
    )
    .unwrap();
    let (code, out, stderr) = raw(
        &w,
        &["create", "--spec", good.to_str().unwrap(), "--no-units"],
    );
    assert_eq!(code, Some(0), "{out}{stderr}");
    let frozen = std::fs::read_to_string(w.home.join("instances/r/instance.yaml")).unwrap();
    let bad = spec(&w, "r", model);
    let (code, out, stderr) = raw(
        &w,
        &["update", "r", "--spec", bad.to_str().unwrap(), "--no-units"],
    );
    assert_eq!(code, Some(2), "update {model}: {out}{stderr}");
    assert!(stderr.contains(field), "update {model}: {stderr}");
    assert_eq!(
        std::fs::read_to_string(w.home.join("instances/r/instance.yaml")).unwrap(),
        frozen,
        "a refused update changed the frozen spec"
    );
}

#[test]
fn a_gateway_instance_without_budget_tokens_is_refused_naming_it() {
    refused_everywhere(&format!(", gateway: {GATEWAY}"), "model.budget_tokens");
    refused_everywhere(
        &format!(", budget_usd: \"1\", daily_ceiling_tokens: 1000, gateway: {GATEWAY}"),
        "model.budget_tokens",
    );
}

#[test]
fn an_instance_without_a_gateway_and_without_budget_usd_is_refused_naming_it() {
    refused_everywhere(", budget_tokens: 100000", "model.budget_usd");
    refused_everywhere("", "model.budget_usd");
}

#[test]
fn without_a_gateway_the_dollar_budget_still_stops_the_run_and_reaches_claude() {
    // The same priced answers without a gateway: the first answer spends the budget of 1 USD, so
    // the second batch is not asked for, and `claude` is given the budget.
    let (w, _) = instance("plain", 7, ", budget_usd: \"1\", budget_tokens: 100000");
    let ran = run(&w, "plain/news");
    assert_eq!(calls(&w), 1, "{ran}");
    let stopped = ran["detail"]["stopped"].as_str().unwrap_or_default();
    assert!(stopped.contains("budget of 1 USD spent"), "{ran}");
    assert_eq!(ran["detail"].get("cost_counted"), None, "{ran}");
    let spend = spend(&w, "plain");
    assert_eq!(spend["spent_usd"], "50.5", "{spend}");
    assert_eq!(spend.get("uncounted_usd"), None, "{spend}");
    let args = w.lines("claude-args.log");
    assert!(args[0].contains("--max-budget-usd 1.0000"), "{args:?}");
}

#[test]
fn a_frozen_gateway_spec_without_budget_tokens_keeps_running_and_counts_dollars_as_before() {
    // An instance created with a gateway before `budget_tokens` was required for one: its frozen
    // spec is read as it is, and with no token budget its dollar limits hold as in 0.5.2.
    let (w, _) = instance(
        "legacy",
        7,
        &format!(", budget_usd: \"1\", budget_tokens: 100000, gateway: {GATEWAY}"),
    );
    let frozen = w.home.join("instances/legacy/instance.yaml");
    let text = std::fs::read_to_string(&frozen).unwrap();
    assert!(text.contains(", budget_tokens: 100000"), "{text}");
    std::fs::write(&frozen, text.replacen(", budget_tokens: 100000", "", 1)).unwrap();
    let ran = run(&w, "legacy/news");
    assert_eq!(calls(&w), 1, "{ran}");
    let stopped = ran["detail"]["stopped"].as_str().unwrap_or_default();
    assert!(stopped.contains("budget of 1 USD spent"), "{ran}");
    assert_eq!(ran["detail"].get("cost_counted"), None, "{ran}");
    assert_eq!(spend(&w, "legacy")["spent_usd"], "50.5");
    let args = w.lines("claude-args.log");
    assert!(args[0].contains("--max-budget-usd 1.0000"), "{args:?}");
}

/// Replaces the extraction behind [`priced_claude`] with one that answers `n` products, each
/// with its own `version`, all citing the first evidence id of the batch (as `tests/quality.rs`
/// does), so the store holds `n` distinct property facts.
fn extract_products(w: &World, n: usize) {
    executable(
        &w.bin.join("claude-real"),
        &format!(
            r#"R="{root}"
echo call >> "$R/claude-calls.log"
printf "%s\n" "$*" | tr "\n" " " >> "$R/claude-args.log"; echo >> "$R/claude-args.log"
id=$(grep -oE 'evidence id [0-9a-f-]{{36}}' | head -1 | cut -d' ' -f3)
entities=""
facts=""
i=1
while [ $i -le {n} ]; do
  entities="$entities{{\"node_type\":\"Product\",\"aliases\":[\"Product $i\"]}},"
  facts="$facts{{\"!Property\":{{\"subject\":{{\"node_type\":\"Product\",\"aliases\":[\"Product $i\"]}},\"property\":\"version\",\"value\":{{\"value_kind\":\"String\",\"value\":\"1.$i\"}},\"evidence\":[\"$id\"]}}}},"
  i=$((i+1))
done
entities=${{entities%,}}
facts=${{facts%,}}
cat <<EOF
{{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{{"format":"ekr.extraction-document/1","ontology":{{"node_types":[{{"name":"Product","parents":[],"abstract_type":false,"properties":[{{"name":"version","value":{{"value_kind":"String"}},"cardinality":"One","required":false}}]}}],"edge_types":[]}},"entities":[$entities],"facts":[$facts]}}}}
EOF
"#,
            root = w.root.display()
        ),
    );
}

/// A judge stand-in priced at [`COST`] and [`USAGE`] for each batch of facts it is shown,
/// answering `yes` for every one.
fn judge(w: &World) {
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
echo call >> "$R/judge-calls.log"
printf "%s\n" "$*" | tr "\n" " " >> "$R/claude-args.log"; echo >> "$R/claude-args.log"
verdicts=""
for id in $(grep -oE '^=== Fact [0-9a-f-]{{36}}' | cut -d' ' -f3); do
  verdicts="$verdicts{{\"fact\":\"$id\",\"verdict\":\"yes\",\"reason\":\"the evidence says so\"}},"
done
verdicts=${{verdicts%,}}
cat <<EOF
{{"type":"result","subtype":"success","is_error":false,{COST}{USAGE}"structured_output":{{"verdicts":[$verdicts]}}}}
EOF
"#,
            root = w.root.display()
        ),
    );
}

#[test]
fn a_quality_measurement_behind_a_gateway_is_not_stopped_by_its_dollar_budget() {
    // One page whose extraction gives 45 product facts; a sample of 40 is two judge batches of
    // 20, each reported at 50.5 USD against a budget of 0.01.
    let (w, _) = instance(
        "judged",
        1,
        &format!(", budget_usd: \"0.01\", budget_tokens: 100000, gateway: {GATEWAY}"),
    );
    extract_products(&w, 45);
    run(&w, "judged/news");
    judge(&w);
    let (code, out, stderr) = raw(&w, &["quality", "judged", "--sample", "40"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (Some(0), Some("measured")),
        "{out}{stderr}"
    );
    assert!(w.lines("judge-calls.log").len() >= 2, "{out}");
    never_given_a_dollar_budget(&w);
}
