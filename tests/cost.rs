//! The cost rule and the budget across more than one model call (`story:spec-standalone-types`):
//! the sum while every answer is costed, `null` as soon as one is not, `0` for answers that cost
//! nothing, and an answer with no cost spends none of the dollar budget.
//!
//! The cases were written by the wave's adversary (pass 1, findings F2 and F3; pass 2, the
//! three-call budget cases for M1-M4) and moved here.

mod common;

use std::path::PathBuf;

use common::{ekr, executable, World};
use serde_json::{json, Value};
/// `n` pages of about 41 000 characters each. Each is cut to what a 16 384-byte evidence payload
/// holds (about 16 300 characters), so three fit one 60 000-character batch and a fourth starts
/// the next.
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

/// Four pages: three in the first batch, one in the second, so the run asks the model twice.
fn two_batch_pages() -> String {
    pages(4)
}

/// A spec like `World::spec`, with large documents and the given budget.
fn spec(w: &World, name: &str, budget: &str, extra: &str) -> PathBuf {
    let path = w.root.join(format!("{name}.yaml"));
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: {name}
description: Test brain.
ekr: {{version: "0.0.30", bin: "{ekr}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "{budget}", timeout_s: 60}}
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
serve: {{view_port: 18998}}
{extra}"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    path
}

/// Wraps the world's `claude` so that its n-th call reports the n-th line of `costs`: a number,
/// or `none` for an answer with no `total_cost_usd`.
fn costs_per_call(w: &World, costs: &[&str]) {
    std::fs::rename(w.bin.join("claude"), w.bin.join("claude-real")).unwrap();
    std::fs::write(w.root.join("costs"), costs.join("\n") + "\n").unwrap();
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
n=$(cat "$R/claude-calls.log" 2>/dev/null | wc -l)
c=$(sed -n "$((n+1))p" "$R/costs")
if [ "$c" = none ]; then touch "$R/no-cost"; else rm -f "$R/no-cost"; fi
"$R/bin/claude-real" "$@" | sed "s/\"total_cost_usd\":0.01,/\"total_cost_usd\":$c,/"
"#,
            root = w.root.display()
        ),
    );
}

fn last_log_line(w: &World, name: &str) -> Value {
    let log = w.home.join("instances").join(name).join("cortex.log");
    let text = std::fs::read_to_string(&log).unwrap();
    serde_json::from_str(text.lines().last().expect("a log line")).unwrap()
}

/// Creates instance `name`, then runs its source once with the per-call costs given.
fn run_with_costs(name: &str, budget: &str, costs: &[&str]) -> (World, Value) {
    let w = World::new();
    std::fs::write(w.root.join("answer.json"), two_batch_pages()).unwrap();
    let path = spec(&w, name, budget, "");
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    costs_per_call(&w, costs);
    let (code, ran) = w.cortex(&["run", &format!("{name}/news")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    (w, ran)
}

#[test]
fn two_costed_batches_report_the_sum_of_both_answers() {
    let (w, ran) = run_with_costs("sum", "5", &["0.25", "0.5"]);
    assert_eq!(w.lines("claude-calls.log").len(), 2, "two model calls");
    assert_eq!(ran["detail"]["documents_applied"], 4, "{ran}");
    assert_eq!(ran["detail"]["cost_usd"], "0.7500", "{ran}");
    assert_eq!(last_log_line(&w, "sum")["cost_usd"], 0.75);
}

#[test]
fn an_uncosted_first_answer_makes_a_two_batch_run_uncosted() {
    let (w, ran) = run_with_costs("first", "5", &["none", "0.25"]);
    assert_eq!(w.lines("claude-calls.log").len(), 2, "two model calls");
    assert_eq!(ran["detail"]["cost_usd"], Value::Null, "{ran}");
    assert_eq!(last_log_line(&w, "first")["cost_usd"], Value::Null);
}

#[test]
fn an_uncosted_last_answer_makes_a_two_batch_run_uncosted() {
    let (w, ran) = run_with_costs("last", "5", &["0.25", "none"]);
    assert_eq!(w.lines("claude-calls.log").len(), 2, "two model calls");
    assert_eq!(ran["detail"]["cost_usd"], Value::Null, "{ran}");
    assert_eq!(last_log_line(&w, "last")["cost_usd"], Value::Null);
}

#[test]
fn a_zero_cost_answer_is_zero_not_null() {
    let (w, ran) = run_with_costs("zero", "5", &["0", "0"]);
    assert_eq!(w.lines("claude-calls.log").len(), 2, "two model calls");
    assert_eq!(ran["detail"]["cost_usd"], "0.0000", "{ran}");
    assert_eq!(last_log_line(&w, "zero")["cost_usd"], 0.0);
}

#[test]
fn uncosted_answers_spend_no_budget_and_costed_ones_do() {
    // Budget 0.1: two uncosted answers both run; two answers of 0.25 stop after the first.
    let (w, ran) = run_with_costs("free", "0.1", &["none", "none"]);
    assert_eq!(w.lines("claude-calls.log").len(), 2, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 4, "{ran}");
    assert_eq!(ran["detail"]["stopped"], Value::Null, "{ran}");

    let (w, ran) = run_with_costs("paid", "0.1", &["0.25", "0.25"]);
    assert_eq!(w.lines("claude-calls.log").len(), 1, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 3, "{ran}");
    assert_eq!(ran["detail"]["cost_usd"], "0.2500", "{ran}");
    assert!(
        ran["detail"]["stopped"]
            .as_str()
            .unwrap_or_default()
            .contains("budget"),
        "{ran}"
    );
}

/// Seven pages: three in each of the first two batches, one in the third, so the run asks the
/// model three times.
fn three_batch_pages() -> String {
    pages(7)
}

fn run_three(name: &str, budget: &str, costs: &[&str]) -> (World, Value) {
    let w = World::new();
    std::fs::write(w.root.join("answer.json"), three_batch_pages()).unwrap();
    let path = spec(&w, name, budget, "");
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    costs_per_call(&w, costs);
    let (code, ran) = w.cortex(&["run", &format!("{name}/news")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    (w, ran)
}

/// The `--max-budget-usd` each model call was given, in call order.
fn budgets_given(w: &World) -> Vec<String> {
    w.lines("claude-args.log")
        .iter()
        .map(|line| {
            let mut words = line.split(' ');
            words.find(|w| *w == "--max-budget-usd");
            words
                .next()
                .expect("a budget after --max-budget-usd")
                .to_string()
        })
        .collect()
}

fn stopped_by_budget(ran: &Value) -> bool {
    ran["detail"]["stopped"]
        .as_str()
        .unwrap_or_default()
        .contains("budget")
}

#[test]
fn a_budget_spent_exactly_stops_the_next_call() {
    // 0.25 + 0.25 is exactly the budget of 0.5: nothing remains, so the third call is not made.
    let (w, ran) = run_three("exact", "0.5", &["0.25", "0.25", "0.25"]);
    assert_eq!(w.lines("claude-calls.log").len(), 2, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 6, "{ran}");
    assert_eq!(ran["detail"]["cost_usd"], "0.5000", "{ran}");
    assert!(stopped_by_budget(&ran), "{ran}");
}

#[test]
fn an_uncosted_answer_does_not_lift_the_budget_for_later_costed_ones() {
    // Budget 0.3: the uncosted first answer spends nothing, the second spends 0.5 and so the
    // third call is not made — even though the run's reported cost is already null.
    let (w, ran) = run_three("lift", "0.3", &["none", "0.5", "0.5"]);
    assert_eq!(w.lines("claude-calls.log").len(), 2, "{ran}");
    assert_eq!(ran["detail"]["cost_usd"], Value::Null, "{ran}");
    assert!(stopped_by_budget(&ran), "{ran}");
}

#[test]
fn each_call_is_given_the_budget_that_remains() {
    // Budget 1: the calls are given 1, then 1 - 0.25, then 1 - 0.25 - 0.25.
    let (w, ran) = run_three("remain", "1", &["0.25", "0.25", "none"]);
    assert_eq!(budgets_given(&w), ["1.0000", "0.7500", "0.5000"], "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 7, "{ran}");
    assert_eq!(ran["detail"]["cost_usd"], Value::Null, "{ran}");
    assert_eq!(ran["detail"]["stopped"], Value::Null, "{ran}");
}

/// A spec like [`spec`] with a second source, `more`, reading the same pages, and the instance's
/// daily ceiling `ceiling` (`story:daily-instance-budget`).
fn ceiling_spec(w: &World, name: &str, budget: &str, ceiling: &str) -> PathBuf {
    let path = spec(w, name, budget, "");
    let text = std::fs::read_to_string(&path).unwrap();
    let text = text.replace(
        "timeout_s: 60}",
        &format!("timeout_s: 60, daily_ceiling_usd: \"{ceiling}\"}}"),
    );
    let (head, rest) = text.split_once("sources:\n").unwrap();
    let (news, tail) = rest.split_once("serve:").unwrap();
    let more = news.replace("name: news", "name: more");
    std::fs::write(&path, format!("{head}sources:\n{news}{more}serve:{tail}")).unwrap();
    path
}

/// The instance's `spend.json`, the spend of the day.
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

fn stopped_by_ceiling(ran: &Value, ceiling: &str) -> bool {
    ran["detail"]["daily_ceiling_reached"] == true
        && ran["detail"]["stopped"]
            .as_str()
            .unwrap_or_default()
            .starts_with(&format!("daily ceiling of {ceiling} USD reached"))
}

#[test]
fn a_run_that_reaches_the_daily_ceiling_between_batches_stops_before_the_next_batch() {
    // Ceiling 0.25 of a budget of 5: the first call is given the 0.25 the ceiling leaves and
    // spends it, so the second batch is not asked for; the first stays applied.
    let w = World::new();
    std::fs::write(w.root.join("answer.json"), two_batch_pages()).unwrap();
    let path = ceiling_spec(&w, "between", "5", "0.25");
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    costs_per_call(&w, &["0.25", "0.25"]);

    let (code, ran) = w.cortex(&["run", "between/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(w.lines("claude-calls.log").len(), 1, "{ran}");
    assert_eq!(
        budgets_given(&w),
        ["0.2500"],
        "the lesser of budget and ceiling"
    );
    assert_eq!(ran["detail"]["documents_new"], 4, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 3, "{ran}");
    assert_eq!(ran["detail"]["cost_usd"], "0.2500", "{ran}");
    assert!(stopped_by_ceiling(&ran, "0.25"), "{ran}");
    assert_eq!(
        spend(&w, "between"),
        json!({"format": "cortex.daily-spend/1", "day": today(), "spent_usd": "0.25"})
    );
}

#[test]
fn a_reached_daily_ceiling_stops_a_run_of_any_source_before_a_model_call() {
    let w = World::new();
    std::fs::write(w.root.join("answer.json"), two_batch_pages()).unwrap();
    let path = ceiling_spec(&w, "reached", "5", "1");
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let spent = w.home.join("instances/reached/spend.json");
    let day = |day: &str, usd: &str| {
        json!({"format": "cortex.daily-spend/1", "day": day, "spent_usd": usd}).to_string()
    };
    std::fs::write(&spent, day(&today(), "1")).unwrap();

    for source in ["news", "more"] {
        let (code, ran) = w.cortex(&["run", &format!("reached/{source}")]);
        assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
        assert_eq!(ran["detail"]["documents_new"], 4, "{ran}");
        assert_eq!(ran["detail"]["documents_applied"], 0, "{ran}");
        assert!(stopped_by_ceiling(&ran, "1"), "{ran}");
    }
    assert_eq!(w.lines("claude-calls.log").len(), 0, "no model call");

    // The day is the UTC calendar day: what another day spent counts as nothing, and the next
    // costed answers start today's spend again.
    std::fs::write(&spent, day("2000-01-01", "100")).unwrap();
    let (code, ran) = w.cortex(&["run", "reached/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(w.lines("claude-calls.log").len(), 2, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 4, "{ran}");
    assert_eq!(ran["detail"]["stopped"], Value::Null, "{ran}");
    assert_eq!(ran["detail"].get("daily_ceiling_reached"), None, "{ran}");
    assert_eq!(spend(&w, "reached")["day"], today());
    assert_eq!(spend(&w, "reached")["spent_usd"], "0.02");
}
