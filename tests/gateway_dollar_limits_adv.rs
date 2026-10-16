//! Adversary cases for `story:gateway-dollar-limits`: behind a model gateway the reported dollar
//! cost bounds nothing and the instance is bounded in tokens; without one, nothing changes.
//! The paths `tests/gateway_dollar_limits.rs` leaves out: a schema round end to end, the run
//! gate's `cost_usd` measure, a spend file shared by gateway and plain runs of one day, an
//! instance created before `budget_tokens` was required, the boundaries of `budget_tokens`, the
//! warning's line count, a cost past what a day can sum, and a plain spec whose dollar budget
//! reads as infinite.

mod common;

use std::path::PathBuf;
use std::process::Command;

use common::{ekr, executable, World};
use serde_json::{json, Value};

/// 2000 tokens an answer.
const USAGE: &str = r#""usage":{"input_tokens":1500,"cache_creation_input_tokens":100,"cache_read_input_tokens":300,"output_tokens":100},"#;

const GATEWAY: &str = r#"{base_url: "http://gw.test", token_env: GW_TOKEN}"#;

/// `n` pages of about 41 000 characters each, from `https://example.org/<start>` on; three fit
/// one batch.
fn pages(start: usize, n: usize) -> String {
    let body = "Example Labs develops the Widget engine. ".repeat(1000);
    let page = |n: usize| {
        json!({"url": format!("https://example.org/{n}"), "title": "T", "description": "D",
            "content": body, "content_truncated": false, "published": null, "score": "0.9"})
    };
    let pages = json!({"results": (start..start + n).map(page).collect::<Vec<_>>(),
        "complete": false, "truncation": ["provider_limit"],
        "provenance": {"instance": "t", "profile": "tavily/2026-10",
            "received_at": "2026-10-05T00:00:00.000Z"}});
    common::answer(&pages.to_string())
}

/// A spec of `name` with the model fields `model` after `model` and `timeout_s`, two sources
/// `news` and `more` reading the same pages, and `extra` appended at the top level.
fn spec(w: &World, file: &str, name: &str, model: &str, extra: &str) -> PathBuf {
    let path = w.root.join(format!("{file}.yaml"));
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
{news}{more}serve: {{view_port: 18995}}
{extra}"#,
            ekr = ekr().display(),
            news = source("news"),
            more = source("more"),
        ),
    )
    .unwrap();
    path
}

/// Wraps the world's `claude` so that every answer reports `cost` (a JSON number) and [`USAGE`]
/// in place of its own cost.
fn priced_claude(w: &World, cost: &str) {
    if !w.bin.join("claude-real").exists() {
        std::fs::rename(w.bin.join("claude"), w.bin.join("claude-real")).unwrap();
    }
    executable(
        &w.bin.join("claude"),
        &format!(
            r#""{root}/bin/claude-real" "$@" | sed 's/"total_cost_usd":[0-9.]*,//; s/"is_error":false,/"is_error":false,"total_cost_usd":{cost},{USAGE}/'
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

/// Creates instance `name` over `n` pages with the model fields `model` and the top-level
/// `extra`, its `claude` priced at `cost`; answers `create`'s stderr.
fn instance(name: &str, n: usize, model: &str, extra: &str, cost: &str) -> (World, String) {
    let w = World::new();
    std::fs::write(w.root.join("answer.json"), pages(0, n)).unwrap();
    let path = spec(&w, name, name, model, extra);
    let (code, created, stderr) = raw(
        &w,
        &["create", "--spec", path.to_str().unwrap(), "--no-units"],
    );
    assert_eq!(
        (code, created["outcome"].as_str()),
        (Some(0), Some("created")),
        "{created}{stderr}"
    );
    priced_claude(&w, cost);
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

fn update(w: &World, name: &str, path: &std::path::Path) -> (Option<i32>, Value, String) {
    raw(
        w,
        &[
            "update",
            name,
            "--spec",
            path.to_str().unwrap(),
            "--no-units",
        ],
    )
}

fn calls(w: &World) -> usize {
    w.lines("claude-calls.log").len()
}

fn spend(w: &World, name: &str) -> Value {
    let path = w.home.join("instances").join(name).join("spend.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// The lines of `stderr` that are the dollar-limit warning.
fn warnings(stderr: &str) -> usize {
    stderr
        .lines()
        .filter(|l| l.contains("bound nothing"))
        .count()
}

// --- schema rounds ----------------------------------------------------------------------------

/// An extraction stand-in answering `n` products with a `version` each, all citing the first
/// evidence id of the batch, at 0.01 USD and [`USAGE`].
fn extract_products(w: &World, n: usize) {
    executable(
        &w.bin.join("claude"),
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
{{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,{USAGE}"structured_output":{{"format":"ekr.extraction-document/1","ontology":{{"node_types":[{{"name":"Product","parents":[],"abstract_type":false,"properties":[{{"name":"version","value":{{"value_kind":"String"}},"cardinality":"One","required":false}}]}}],"edge_types":[]}},"entities":[$entities],"facts":[$facts]}}}}
EOF
"#,
            root = w.root.display()
        ),
    );
}

/// A proposing stand-in that proposes nothing, each answer reported at 50.5 USD and [`USAGE`],
/// recording its calls and arguments.
fn priced_proposer(w: &World) {
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
cat > /dev/null
echo call >> "$R/schema-calls.log"
printf "%s\n" "$*" | tr "\n" " " >> "$R/schema-args.log"; echo >> "$R/schema-args.log"
echo '{{"type":"result","subtype":"success","is_error":false,"total_cost_usd":50.5,{USAGE}"structured_output":{{"proposals":[]}}}}'
"#,
            root = w.root.display()
        ),
    );
}

#[test]
fn adv_a_schema_round_behind_a_gateway_is_not_stopped_by_its_dollar_budget_or_the_ceiling() {
    // Two proposal batches of 20 facts, each reported at 50.5 USD, against a budget and a daily
    // ceiling of 0.01 USD: neither stops the round, claude is given no dollar budget, and the
    // reported cost is kept as uncounted.
    let w = World::new();
    std::fs::write(w.root.join("answer.json"), pages(0, 1)).unwrap();
    let path = spec(
        &w,
        "sch",
        "sch",
        &format!(
            ", budget_usd: \"0.01\", daily_ceiling_usd: \"0.01\", budget_tokens: 100000, \
             gateway: {GATEWAY}"
        ),
        "",
    );
    let (code, created, stderr) = raw(
        &w,
        &["create", "--spec", path.to_str().unwrap(), "--no-units"],
    );
    assert_eq!(code, Some(0), "{created}{stderr}");
    extract_products(&w, 45);
    run(&w, "sch/news");
    let before = spend(&w, "sch");
    assert_eq!(before["spent_usd"], "0", "{before}");
    priced_proposer(&w);
    for args in [
        &["schema", "sch", "--sample", "40", "--dry-run"][..],
        &["schema", "sch", "--sample", "40"][..],
    ] {
        let (code, out, stderr) = raw(&w, args);
        assert_eq!(code, Some(0), "{args:?}: {out}{stderr}");
        assert!(
            matches!(out["outcome"].as_str(), Some("proposed" | "applied")),
            "{args:?}: {out}{stderr}"
        );
    }
    let schema_calls = w.lines("schema-calls.log").len();
    assert_eq!(schema_calls, 4, "two batches a round");
    for line in w.lines("schema-args.log") {
        assert!(!line.contains("--max-budget-usd"), "{line}");
    }
    let after = spend(&w, "sch");
    assert_eq!(after["spent_usd"], "0", "{after}");
    assert_eq!(after["uncounted_usd"], "202.01", "{after}");
    assert_eq!(after["spent_tokens"], 2000 * 5, "{after}");
}

#[test]
fn adv_a_schema_round_behind_a_gateway_stops_at_its_token_budget() {
    // 2000 tokens an answer and, for the round, a budget of 1000: the first batch spends it, so
    // the second is not asked for, whatever the dollars.
    let w = World::new();
    std::fs::write(w.root.join("answer.json"), pages(0, 1)).unwrap();
    let path = spec(
        &w,
        "schtok",
        "schtok",
        &format!(", budget_tokens: 3000, gateway: {GATEWAY}"),
        "",
    );
    let (code, created, stderr) = raw(
        &w,
        &["create", "--spec", path.to_str().unwrap(), "--no-units"],
    );
    assert_eq!(code, Some(0), "{created}{stderr}");
    extract_products(&w, 45);
    run(&w, "schtok/news");
    // Lower the token budget in the frozen spec to below one answer: a fresh `update` would do
    // the same.
    let lower = spec(
        &w,
        "schtok-low",
        "schtok",
        &format!(", budget_tokens: 1000, gateway: {GATEWAY}"),
        "",
    );
    let (code, out, stderr) = update(&w, "schtok", &lower);
    assert_eq!(code, Some(0), "{out}{stderr}");
    priced_proposer(&w);
    let (code, out, stderr) = raw(&w, &["schema", "schtok", "--sample", "40", "--dry-run"]);
    assert_ne!(code, Some(0), "{out}{stderr}");
    assert_eq!(out["outcome"], "propose-failed", "{out}");
    let reason = out["detail"]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("token budget of 1000 tokens"), "{out}");
    assert_eq!(w.lines("schema-calls.log").len(), 1, "{out}");
}

// --- the run gate --------------------------------------------------------------------------

#[test]
fn adv_behind_a_gateway_a_run_gate_on_cost_usd_does_not_undo_a_run_over_reported_dollars() {
    // The story's title: behind a gateway the reported dollar cost bounds nothing. A gate check
    // on `cost_usd` reads that same reported cost, priced at claude's default model's rate.
    let (w, stderr) = instance(
        "gated",
        1,
        &format!(", budget_tokens: 100000, gateway: {GATEWAY}"),
        "gate:\n  checks:\n    - {measure: cost_usd, max: \"1\"}\n",
        "50.5",
    );
    let (code, ran, run_stderr) = raw(&w, &["run", "gated/news"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (Some(0), Some("ran")),
        "a run behind a gateway was undone over the {}-USD cost claude reported; create's \
         stderr said nothing about the gate: {stderr:?}; {ran}{run_stderr}",
        50.5
    );
}

// --- spend.json shared by gateway and plain runs of one day ---------------------------------

#[test]
fn adv_a_day_shared_by_plain_and_gateway_runs_counts_only_the_plain_dollars() {
    // Plain first: a budget of 100 USD a run, a daily ceiling of 120 USD, 50.5 USD an answer. The
    // run spends 101 and stops at its budget; the file is what 0.5.2 writes (no uncounted_usd).
    let (w, _) = instance(
        "mix",
        7,
        ", budget_usd: \"100\", daily_ceiling_usd: \"120\", budget_tokens: 100000",
        "",
        "50.5",
    );
    let ran = run(&w, "mix/news");
    assert_eq!(calls(&w), 2, "{ran}");
    let day = spend(&w, "mix");
    assert_eq!(day["spent_usd"], "101", "{day}");
    assert_eq!(day.get("uncounted_usd"), None, "{day}");

    // Behind a gateway the same day: the ceiling's 19 USD left stop nothing, and nothing is added
    // to spent_usd.
    let gw = spec(
        &w,
        "mix-gw",
        "mix",
        &format!(
            ", budget_usd: \"100\", daily_ceiling_usd: \"120\", budget_tokens: 100000, \
             gateway: {GATEWAY}"
        ),
        "",
    );
    let (code, out, stderr) = update(&w, "mix", &gw);
    assert_eq!(code, Some(0), "{out}{stderr}");
    assert_eq!(warnings(&stderr), 1, "{stderr}");
    let ran = run(&w, "mix/more");
    assert_eq!(calls(&w), 5, "{ran}");
    assert_eq!(ran["detail"]["stopped"], Value::Null, "{ran}");
    assert_eq!(ran["detail"]["cost_counted"], false, "{ran}");
    let day = spend(&w, "mix");
    assert_eq!(day["spent_usd"], "101", "{day}");
    assert_eq!(day["uncounted_usd"], "151.5", "{day}");

    // Plain again, new pages: the ceiling leaves 19 USD of the plain 101, not of 252.5, and claude
    // is given those 19; the uncounted sum is kept with the day.
    let plain = spec(
        &w,
        "mix-plain",
        "mix",
        ", budget_usd: \"100\", daily_ceiling_usd: \"120\", budget_tokens: 100000",
        "",
    );
    let (code, out, stderr) = update(&w, "mix", &plain);
    assert_eq!(code, Some(0), "{out}{stderr}");
    assert_eq!(warnings(&stderr), 0, "no gateway, no warning: {stderr}");
    std::fs::write(w.root.join("answer.json"), pages(100, 3)).unwrap();
    let ran = run(&w, "mix/news");
    assert_eq!(calls(&w), 6, "{ran}");
    let args = w.lines("claude-args.log");
    assert!(
        args.last().unwrap().contains("--max-budget-usd 19.0000"),
        "{args:?}"
    );
    let day = spend(&w, "mix");
    assert_eq!(day["spent_usd"], "151.5", "{day}");
    assert_eq!(day["uncounted_usd"], "151.5", "{day}");

    // And the ceiling now holds for the plain instance.
    std::fs::write(w.root.join("answer.json"), pages(200, 3)).unwrap();
    let ran = run(&w, "mix/more");
    assert_eq!(calls(&w), 6, "{ran}");
    assert_eq!(ran["detail"]["daily_ceiling_reached"], true, "{ran}");
}

#[test]
fn adv_a_cost_past_what_a_day_can_sum_leaves_the_gateway_day_readable() {
    // 1e300 USD an answer behind a gateway: uncounted_usd saturates rather than breaking the file,
    // and the next run of the day still runs.
    let (w, _) = instance(
        "huge",
        4,
        &format!(", budget_tokens: 100000, gateway: {GATEWAY}"),
        "",
        "1e300",
    );
    let ran = run(&w, "huge/news");
    assert_eq!(calls(&w), 2, "{ran}");
    let ran = run(&w, "huge/more");
    assert_eq!(calls(&w), 4, "{ran}");
    assert_eq!(ran["detail"]["stopped"], Value::Null, "{ran}");
    let day = spend(&w, "huge");
    assert_eq!(day["spent_usd"], "0", "{day}");
    let uncounted: f64 = day["uncounted_usd"].as_str().unwrap().parse().unwrap();
    assert!(uncounted > 9.0e12, "{day}");
}

// --- an instance created before budget_tokens was required ----------------------------------

/// An instance created with a gateway and `budget_tokens: 100000`, then given the frozen spec an
/// earlier release would have written: no `budget_tokens`.
fn legacy(name: &str, n: usize, model: &str) -> World {
    let (w, _) = instance(
        name,
        n,
        &format!("{model}, budget_tokens: 100000, gateway: {GATEWAY}"),
        "",
        "50.5",
    );
    let frozen = w.home.join("instances").join(name).join("instance.yaml");
    let text = std::fs::read_to_string(&frozen).unwrap();
    assert!(text.contains(", budget_tokens: 100000"), "{text}");
    std::fs::write(&frozen, text.replacen(", budget_tokens: 100000", "", 1)).unwrap();
    w
}

#[test]
fn adv_a_legacy_gateway_instance_still_counts_its_daily_ceiling_in_dollars() {
    let w = legacy(
        "old",
        7,
        ", budget_usd: \"100\", daily_ceiling_usd: \"60\"",
    );
    let ran = run(&w, "old/news");
    // 50.5, then min(49.5, 9.5) for the second: 101 spent, then the ceiling.
    assert_eq!(calls(&w), 2, "{ran}");
    let args = w.lines("claude-args.log");
    assert!(args[1].contains("--max-budget-usd 9.5000"), "{args:?}");
    let ran = run(&w, "old/more");
    assert_eq!(calls(&w), 2, "{ran}");
    assert_eq!(ran["detail"]["daily_ceiling_reached"], true, "{ran}");
    let day = spend(&w, "old");
    assert_eq!(day["spent_usd"], "101", "{day}");
    assert_eq!(day.get("uncounted_usd"), None, "{day}");
}

#[test]
fn adv_a_legacy_gateway_instance_cannot_be_updated_without_budget_tokens() {
    let w = legacy("upold", 1, ", budget_usd: \"1\"");
    let frozen = w.home.join("instances/upold/instance.yaml");
    let before = std::fs::read_to_string(&frozen).unwrap();
    // The operator's spec file, as it was when the instance was created.
    let same = spec(
        &w,
        "upold-same",
        "upold",
        &format!(", budget_usd: \"1\", gateway: {GATEWAY}"),
        "",
    );
    let (code, out, stderr) = update(&w, "upold", &same);
    assert_eq!(code, Some(2), "{out}{stderr}");
    assert!(stderr.contains("model.budget_tokens"), "{stderr}");
    assert_eq!(std::fs::read_to_string(&frozen).unwrap(), before);
    // With a token budget the update is taken and the dollars stop counting.
    let fixed = spec(
        &w,
        "upold-fixed",
        "upold",
        &format!(", budget_usd: \"1\", budget_tokens: 100000, gateway: {GATEWAY}"),
        "",
    );
    let (code, out, stderr) = update(&w, "upold", &fixed);
    assert_eq!(code, Some(0), "{out}{stderr}");
    assert_eq!(warnings(&stderr), 1, "{stderr}");
    let ran = run(&w, "upold/news");
    assert_eq!(ran["detail"]["cost_counted"], false, "{ran}");
    assert_eq!(ran["detail"]["stopped"], Value::Null, "{ran}");
}

// --- the boundaries of budget_tokens -----------------------------------------------------------

#[test]
fn adv_a_gateway_budget_tokens_of_zero_below_zero_or_null_is_refused_naming_it() {
    // The type errors (null, a string, a fraction) are refused by the spec's types, which need not
    // name the field; a whole number of 0 or less must.
    for (value, named) in [
        ("0", true),
        ("-5", true),
        ("null", false),
        ("\"100\"", false),
        ("1.5", false),
    ] {
        let w = World::new();
        let path = spec(
            &w,
            "bt",
            "bt",
            &format!(", budget_tokens: {value}, gateway: {GATEWAY}"),
            "",
        );
        let (code, out, stderr) = raw(
            &w,
            &["create", "--spec", path.to_str().unwrap(), "--no-units"],
        );
        assert_eq!(code, Some(2), "budget_tokens: {value}: {out}{stderr}");
        assert!(
            !named || stderr.contains("model.budget_tokens"),
            "budget_tokens: {value}: {stderr}"
        );
        assert!(
            !w.home.join("instances/bt").exists(),
            "budget_tokens: {value}: an instance was left"
        );
    }
}

// --- the warning line --------------------------------------------------------------------------

#[test]
fn adv_create_update_and_adopt_each_print_exactly_one_warning_line_and_none_without_a_gateway() {
    let both = format!(
        ", budget_usd: \"1\", daily_ceiling_usd: \"1\", budget_tokens: 100000, gateway: {GATEWAY}"
    );
    let (w, stderr) = instance("warn", 1, &both, "", "0.01");
    assert_eq!(warnings(&stderr), 1, "create: {stderr}");
    let line = stderr.lines().find(|l| l.contains("bound nothing")).unwrap();
    assert!(
        line.contains("model.budget_usd") && line.contains("model.daily_ceiling_usd"),
        "{line}"
    );
    let again = spec(&w, "warn-again", "warn", &both, "");
    let (code, out, stderr) = update(&w, "warn", &again);
    assert_eq!(code, Some(0), "{out}{stderr}");
    assert_eq!(warnings(&stderr), 1, "update: {stderr}");

    let other = World::new();
    let adopted = spec(&other, "ad", "ad", &both, "");
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
    assert_eq!(warnings(&stderr), 1, "adopt: {out}{stderr}");

    // No gateway, the same dollar limits: no warning anywhere.
    let plain = World::new();
    let path = spec(
        &plain,
        "plain",
        "plain",
        ", budget_usd: \"1\", daily_ceiling_usd: \"1\", budget_tokens: 100000",
        "",
    );
    let (code, out, stderr) = raw(
        &plain,
        &["create", "--spec", path.to_str().unwrap(), "--no-units"],
    );
    assert_eq!(code, Some(0), "{out}{stderr}");
    assert_eq!(warnings(&stderr), 0, "{stderr}");
    let (code, out, stderr) = update(&plain, "plain", &path);
    assert_eq!(code, Some(0), "{out}{stderr}");
    assert_eq!(warnings(&stderr), 0, "{stderr}");
}

// --- without a gateway, as 0.5.2 ---------------------------------------------------------------

#[test]
fn adv_without_a_gateway_a_dollar_budget_that_reads_as_infinite_still_reaches_claude() {
    // `budget_usd` is a decimal string; one with more digits than an f64 holds reads as infinity.
    // 0.5.2 passed it to claude (`--max-budget-usd inf`); an infinite budget is the sign the
    // change uses for "behind a gateway", so a plain instance must not lose the flag. Refusing
    // the spec would be as good.
    let huge = format!("1{}", "0".repeat(400));
    let w = World::new();
    std::fs::write(w.root.join("answer.json"), pages(0, 1)).unwrap();
    let path = spec(&w, "inf", "inf", &format!(", budget_usd: \"{huge}\""), "");
    let (code, out, stderr) = raw(
        &w,
        &["create", "--spec", path.to_str().unwrap(), "--no-units"],
    );
    if code == Some(2) {
        return;
    }
    assert_eq!(code, Some(0), "{out}{stderr}");
    run(&w, "inf/news");
    let args = w.lines("claude-args.log");
    assert!(!args.is_empty(), "claude was not asked");
    for line in &args {
        assert!(
            line.contains("--max-budget-usd"),
            "a plain instance's call went to claude with no dollar budget: {line}"
        );
    }
}
