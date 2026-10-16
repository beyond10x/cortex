//! A run budget and a daily ceiling in tokens (`story:token-budget`): `model.budget_tokens` bounds
//! the tokens of one run and `model.daily_ceiling_tokens` the tokens of an instance per UTC day,
//! counted from what each answer reports in its `usage`. A run that reaches either stops before
//! its next batch and names the limit; under a token limit, an answer with no `usage` stops the
//! run before its next batch.

mod common;

use std::path::PathBuf;

use common::{ekr, executable, World};
use serde_json::{json, Value};

/// The `usage` every answer of [`tokens_per_call`] carries: 2000 tokens in all (input, cache
/// writes, cache reads and output).
const USAGE: &str = r#""usage":{"input_tokens":1500,"cache_creation_input_tokens":100,"cache_read_input_tokens":300,"output_tokens":100},"#;

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

/// A spec with large documents, the model fields `model` added after `timeout_s`, and two sources,
/// `news` and `more`, reading the same pages.
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
model: {{model: claude-haiku-4-5-20251001, budget_usd: "5", timeout_s: 60{model}}}
sources:
{news}{more}serve: {{view_port: 18997}}
"#,
            ekr = ekr().display(),
            news = source("news"),
            more = source("more"),
        ),
    )
    .unwrap();
    path
}

/// Wraps the world's `claude` so that no answer carries `total_cost_usd`, as for a model `claude`
/// cannot price, and each carries [`USAGE`]; while the file `no-usage` exists, no `usage` either.
fn tokens_per_call(w: &World) {
    std::fs::write(w.root.join("no-cost"), "").unwrap();
    std::fs::rename(w.bin.join("claude"), w.bin.join("claude-real")).unwrap();
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
if [ -e "$R/no-usage" ]; then exec "$R/bin/claude-real" "$@"; fi
"$R/bin/claude-real" "$@" | sed 's/"is_error":false,/"is_error":false,{usage}/'
"#,
            root = w.root.display(),
            usage = USAGE
        ),
    );
}

/// Creates instance `name` over `n` pages with the model fields `model`, its `claude` answering
/// 2000 tokens a call and no cost.
fn instance(name: &str, n: usize, model: &str) -> World {
    let w = World::new();
    std::fs::write(w.root.join("answer.json"), pages(n)).unwrap();
    let path = spec(&w, name, model);
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    tokens_per_call(&w);
    w
}

fn run(w: &World, source: &str) -> Value {
    let (code, ran) = w.cortex(&["run", source]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    ran
}

fn stopped(ran: &Value) -> &str {
    ran["detail"]["stopped"].as_str().unwrap_or_default()
}

fn calls(w: &World) -> usize {
    w.lines("claude-calls.log").len()
}

fn spend(w: &World, name: &str) -> Value {
    let path = w.home.join("instances").join(name).join("spend.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn today() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    cortex_cli::spend::day(now)
}

#[test]
fn a_run_stops_before_the_batch_after_its_token_budget_is_spent() {
    // Three batches, 2000 tokens a call, a budget of 3000: the first call leaves 1000, the second
    // spends the budget, so the third batch is not asked for.
    let w = instance("budget", 7, ", budget_tokens: 3000");
    let ran = run(&w, "budget/news");
    assert_eq!(calls(&w), 2, "{ran}");
    let detail = &ran["detail"];
    assert_eq!(detail["documents_new"], 7, "{ran}");
    assert_eq!(detail["documents_applied"], 6, "{ran}");
    assert!(
        stopped(&ran).starts_with("token budget of 3000 tokens spent"),
        "{ran}"
    );
    assert_eq!(detail.get("daily_ceiling_reached"), None, "{ran}");
    // The report carries both calls' tokens.
    assert_eq!(detail["model_calls"], 2, "{ran}");
    assert_eq!(detail["input_tokens"], 3000, "{ran}");
    assert_eq!(detail["cache_creation_input_tokens"], 200, "{ran}");
    assert_eq!(detail["cache_read_input_tokens"], 600, "{ran}");
    assert_eq!(detail["output_tokens"], 200, "{ran}");
    assert_eq!(detail["cost_usd"], Value::Null, "{ran}");
}

#[test]
fn the_daily_token_ceiling_spans_runs_and_the_next_utc_day_starts_from_zero() {
    // Two batches per source, 2000 tokens a call, a ceiling of 5000 tokens a day: the first run
    // makes two calls (4000), the second run one more (6000) and then stops.
    let w = instance("daily", 4, ", daily_ceiling_tokens: 5000");
    let first = run(&w, "daily/news");
    assert_eq!(calls(&w), 2, "{first}");
    assert_eq!(first["detail"]["stopped"], Value::Null, "{first}");
    assert_eq!(spend(&w, "daily")["spent_tokens"], 4000);

    let second = run(&w, "daily/more");
    assert_eq!(calls(&w), 3, "stops after the third call in all: {second}");
    assert_eq!(second["detail"]["documents_applied"], 3, "{second}");
    assert_eq!(second["detail"]["daily_ceiling_reached"], true, "{second}");
    assert!(
        stopped(&second).starts_with("daily ceiling of 5000 tokens reached"),
        "{second}"
    );
    let spent = spend(&w, "daily");
    assert_eq!(spent["day"], today());
    assert_eq!(spent["spent_tokens"], 6000);

    // Another day's tokens count as nothing: the next answer starts today's count again.
    let path = w.home.join("instances/daily/spend.json");
    std::fs::write(
        &path,
        json!({"format": "cortex.daily-spend/1", "day": "2000-01-01", "spent_usd": "0",
            "spent_tokens": 999_999})
        .to_string(),
    )
    .unwrap();
    let third = run(&w, "daily/more");
    assert_eq!(calls(&w), 4, "{third}");
    assert_eq!(third["detail"]["documents_applied"], 1, "{third}");
    assert_eq!(third["detail"]["stopped"], Value::Null, "{third}");
    let spent = spend(&w, "daily");
    assert_eq!(spent["day"], today());
    assert_eq!(spent["spent_tokens"], 2000);
}

#[test]
fn a_reached_daily_token_ceiling_stops_a_run_before_any_model_call() {
    let w = instance("reached", 4, ", daily_ceiling_tokens: 5000");
    std::fs::write(
        w.home.join("instances/reached/spend.json"),
        json!({"format": "cortex.daily-spend/1", "day": today(), "spent_usd": "0",
            "spent_tokens": 5000})
        .to_string(),
    )
    .unwrap();
    let ran = run(&w, "reached/news");
    assert_eq!(calls(&w), 0, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 0, "{ran}");
    assert_eq!(ran["detail"]["daily_ceiling_reached"], true, "{ran}");
    assert!(
        stopped(&ran).starts_with("daily ceiling of 5000 tokens reached"),
        "{ran}"
    );
}

#[test]
fn under_a_run_token_budget_an_answer_without_usage_stops_the_run_before_its_next_batch() {
    let w = instance("uncounted", 4, ", budget_tokens: 1000000");
    std::fs::write(w.root.join("no-usage"), "").unwrap();
    let ran = run(&w, "uncounted/news");
    assert_eq!(calls(&w), 1, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 3, "{ran}");
    assert!(stopped(&ran).contains("could not be counted"), "{ran}");
    assert!(stopped(&ran).contains("token budget"), "{ran}");
}

#[test]
fn under_a_daily_token_ceiling_an_answer_without_usage_stops_the_run_before_its_next_batch() {
    let w = instance("uncounted-day", 4, ", daily_ceiling_tokens: 1000000");
    std::fs::write(w.root.join("no-usage"), "").unwrap();
    let ran = run(&w, "uncounted-day/news");
    assert_eq!(calls(&w), 1, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 3, "{ran}");
    assert!(stopped(&ran).contains("could not be counted"), "{ran}");
    assert!(stopped(&ran).contains("daily ceiling"), "{ran}");
}

#[test]
fn without_a_token_limit_an_answer_without_usage_stops_nothing() {
    let w = instance("unlimited", 4, "");
    std::fs::write(w.root.join("no-usage"), "").unwrap();
    let ran = run(&w, "unlimited/news");
    assert_eq!(calls(&w), 2, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 4, "{ran}");
    assert_eq!(ran["detail"]["stopped"], Value::Null, "{ran}");
}

/// `cortex quality` and `cortex schema` add their answers' tokens to the day's and are refused
/// before a model call once the daily token ceiling is reached.
#[test]
fn quality_and_schema_count_against_the_daily_token_ceiling() {
    let w = instance("judged", 2, ", daily_ceiling_tokens: 5000");
    let ran = run(&w, "judged/news");
    assert_eq!(calls(&w), 1, "{ran}");
    assert_eq!(spend(&w, "judged")["spent_tokens"], 2000);

    // The stand-in answers an extraction, not verdicts or proposals: whatever each command makes
    // of that answer, its tokens are added to the day's.
    let (_, out) = w.cortex(&["quality", "judged", "--sample", "3"]);
    assert_eq!(calls(&w), 2, "{out}");
    assert_eq!(spend(&w, "judged")["spent_tokens"], 4000, "{out}");
    let (_, out) = w.cortex(&["schema", "judged", "--sample", "3"]);
    assert_eq!(calls(&w), 3, "{out}");
    assert_eq!(spend(&w, "judged")["spent_tokens"], 6000, "{out}");

    for (command, outcome) in [("quality", "judge-failed"), ("schema", "propose-failed")] {
        let (code, out) = w.cortex(&[command, "judged", "--sample", "3"]);
        assert_eq!((code, out["outcome"].as_str()), (1, Some(outcome)), "{out}");
        let reason = out["detail"]["reason"].as_str().unwrap_or_default();
        assert!(
            reason.starts_with("daily ceiling of 5000 tokens reached"),
            "{out}"
        );
    }
    assert_eq!(calls(&w), 3, "no model call once the ceiling is reached");
}

/// A spec text with `model` as its model line.
fn spec_text(model: &str) -> String {
    format!(
        "format: cortex.instance/1\nname: t\ndescription: T.\nekr: {{version: \"0.0.34\"}}\n\
         seed: {{documents: []}}\nmodel: {model}\nsources: []\nserve: {{view_port: 18996}}\n"
    )
}

#[test]
fn a_token_limit_of_zero_or_less_is_refused() {
    for (field, value) in [
        ("budget_tokens", "0"),
        ("budget_tokens", "-1"),
        ("daily_ceiling_tokens", "0"),
        ("daily_ceiling_tokens", "-5"),
    ] {
        let text = spec_text(&format!(
            "{{model: m, budget_usd: \"1\", timeout_s: 60, {field}: {value}}}"
        ));
        let refused = cortex_cli::spec::parse(&text).unwrap_err();
        assert_eq!(refused, format!("model.{field} must be above 0"), "{value}");
    }
    let text = spec_text(
        "{model: m, budget_usd: \"1\", timeout_s: 60, budget_tokens: 1, daily_ceiling_tokens: 1}",
    );
    let parsed = cortex_cli::spec::parse(&text).expect("limits of 1 are valid");
    assert_eq!(parsed.model.budget_tokens, Some(1));
    assert_eq!(parsed.model.daily_ceiling_tokens, Some(1));
}

/// A `spend.json` written before tokens were counted reads as no tokens spent, and one of today
/// whose `spent_tokens` is not a whole number of 0 or more fails closed under a ceiling.
#[test]
fn a_spend_file_without_tokens_reads_as_zero_and_a_bad_count_fails_closed() {
    use cortex_cli::instance::Layout;
    use cortex_cli::spend::{self, Daily};
    let tmp = tempfile::tempdir().unwrap();
    let layout = Layout::new(tmp.path().to_path_buf());
    let model = cortex_model::instance::ModelSpec {
        model: "m".into(),
        backend: None,
        budget_usd: Some(cortex_model::primitives::Decimal("5".into())),
        daily_ceiling_usd: None,
        budget_tokens: None,
        daily_ceiling_tokens: Some(100),
        gateway: None,
        timeout_s: 60,
        instructions: None,
    };
    std::fs::write(
        spend::path(&layout),
        json!({"format": spend::FORMAT, "day": today(), "spent_usd": "0.5"}).to_string(),
    )
    .unwrap();
    assert_eq!(spend::tokens_on(&layout, &today()), Ok(0));
    assert_eq!(Daily::new(&layout, &model, None).allow(5.0), Ok(5.0));
    for bad in [json!(-1), json!("100"), json!(1.5)] {
        std::fs::write(
            spend::path(&layout),
            json!({"format": spend::FORMAT, "day": today(), "spent_usd": "0",
                "spent_tokens": bad})
            .to_string(),
        )
        .unwrap();
        let allowed = Daily::new(&layout, &model, None).allow(5.0);
        assert!(allowed.is_err(), "spent_tokens {bad} allows {allowed:?}");
    }
}
