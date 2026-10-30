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

/// Like [`costs_per_call`], except that a line `cut:<cost>` makes that call answer that its
/// answer was cut off at the output limit, at `<cost>`, and exit 1. The call still reaches the
/// world's `claude`, so its arguments are logged.
fn cut_off_per_call(w: &World, costs: &[&str]) {
    std::fs::rename(w.bin.join("claude"), w.bin.join("claude-real")).unwrap();
    std::fs::write(w.root.join("costs"), costs.join("\n") + "\n").unwrap();
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
n=$(cat "$R/claude-calls.log" 2>/dev/null | wc -l)
c=$(sed -n "$((n+1))p" "$R/costs")
case "$c" in
  cut:*)
    "$R/bin/claude-real" "$@" > /dev/null
    printf '{{"type":"result","subtype":"success","is_error":true,"result":"API Error: Claude'"'"'s response exceeded the 16000 output token maximum.","total_cost_usd":%s}}\n' "${{c#cut:}}"
    exit 1 ;;
esac
"$R/bin/claude-real" "$@" | sed "s/\"total_cost_usd\":0.01,/\"total_cost_usd\":$c,/"
"#,
            root = w.root.display()
        ),
    );
}

/// `story:run-cost-counted-on-refusal`: a first call cut off at the output limit is split, and
/// what it cost counts against the run's budget and in the run's cost, as it does in the day's.
#[test]
fn a_cut_off_call_counts_against_the_budget_and_in_the_run_cost() {
    let w = World::new();
    std::fs::write(w.root.join("answer.json"), two_batch_pages()).unwrap();
    let path = spec(&w, "cut", "5", "");
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    // The first batch of three is cut off and split into one page and two; then the fourth page.
    cut_off_per_call(&w, &["cut:0.3", "0.25", "0.25", "0.25"]);
    let (code, ran) = w.cortex(&["run", "cut/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["splits"], 1, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 4, "{ran}");
    assert_eq!(
        budgets_given(&w),
        ["5.0000", "4.7000", "4.4500", "4.2000"],
        "{ran}"
    );
    assert_eq!(ran["detail"]["cost_usd"], "1.0500", "{ran}");
    assert_eq!(last_log_line(&w, "cut")["cost_usd"], 1.05);
    // What remains after the run is the budget less the cost it reports.
    let reported: f64 = ran["detail"]["cost_usd"].as_str().unwrap().parse().unwrap();
    let given_last: f64 = budgets_given(&w)[3].parse().unwrap();
    assert!(
        ((5.0 - reported) - (given_last - 0.25)).abs() < 1e-9,
        "{ran}"
    );
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

/// Adversary, `story:daily-instance-budget`: answers of 0.7 and 0.1 USD spend a ceiling of 0.8
/// USD, so the third batch is not asked for and the run names the ceiling. No call is ever given
/// `--max-budget-usd 0.0000`.
#[test]
fn adv_a_ceiling_spent_to_the_cent_makes_no_further_call() {
    let w = World::new();
    std::fs::write(w.root.join("answer.json"), three_batch_pages()).unwrap();
    let path = ceiling_spec(&w, "cent", "5", "0.8");
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    costs_per_call(&w, &["0.7", "0.1", "0.1"]);

    let (code, ran) = w.cortex(&["run", "cent/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let given = budgets_given(&w);
    assert!(
        !given.iter().any(|b| b == "0.0000"),
        "a call was given --max-budget-usd 0.0000: {given:?}"
    );
    assert_eq!(given, ["0.8000", "0.1000"], "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 6, "{ran}");
    assert!(stopped_by_ceiling(&ran, "0.8"), "{ran}");
}

/// Adversary, `story:daily-instance-budget`: a failed extraction answer that states a cost adds
/// it to the spend of the day (`spec/domains/instance.yaml`, `cortex.instance.DailySpend`: "failed
/// answers included"). No other case reads `spend.json` after a failed answer.
#[test]
fn adv_a_failed_extraction_answer_adds_its_cost_to_the_spend_of_the_day() {
    let w = World::new();
    std::fs::write(w.root.join("answer.json"), two_batch_pages()).unwrap();
    let path = ceiling_spec(&w, "failed", "5", "1");
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    executable(
        &w.bin.join("claude"),
        &format!(
            "echo call >> \"{root}/claude-calls.log\"\n\
             echo '{{\"type\":\"result\",\"subtype\":\"error_max_budget_usd\",\"is_error\":true,\"total_cost_usd\":0.3}}'\n",
            root = w.root.display()
        ),
    );
    let (code, ran) = w.cortex(&["run", "failed/news"]);
    assert_ne!(code, 0, "{ran}");
    assert_eq!(w.lines("claude-calls.log").len(), 1, "{ran}");
    assert_eq!(
        spend(&w, "failed"),
        json!({"format": "cortex.daily-spend/1", "day": today(), "spent_usd": "0.3"}),
        "{ran}"
    );
}

/// The `usage` object every answer of [`usage_per_call`] carries, in the field names of the
/// pinned `claude` 2.1.294 (its result message's `usage`).
const USAGE: &str = r#""usage":{"input_tokens":1000,"cache_creation_input_tokens":0,"cache_read_input_tokens":200,"output_tokens":50},"#;

/// Wraps the world's `claude` so that each answer also carries [`USAGE`]. While the file
/// `errors` holds a number above zero, a call counts it down and answers instead an error that
/// cost zero with a zero `usage`, as claude answers a sign-in or a usage limit.
fn usage_per_call(w: &World) {
    std::fs::rename(w.bin.join("claude"), w.bin.join("claude-real")).unwrap();
    std::fs::write(
        w.root.join("error.json"),
        r#"{"type":"result","subtype":"success","is_error":true,"result":"You've hit your usage limit.","total_cost_usd":0,"usage":{"input_tokens":0,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":0}}"#,
    )
    .unwrap();
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
n=$(cat "$R/errors" 2>/dev/null || echo 0)
if [ "$n" -gt 0 ]; then
  echo $((n - 1)) > "$R/errors"
  echo error >> "$R/claude-errors.log"
  cat > /dev/null
  cat "$R/error.json"
  exit 1
fi
"$R/bin/claude-real" "$@" | sed 's/"total_cost_usd":0.01,/"total_cost_usd":0.01,{usage}/'
"#,
            root = w.root.display(),
            usage = USAGE
        ),
    );
}

/// Creates instance `name` over four pages (two batches) and runs its source once, its `claude`
/// answering [`USAGE`] after `errors` zero-cost error answers.
fn run_with_usage(name: &str, errors: u32) -> (World, Value) {
    let w = World::new();
    std::fs::write(w.root.join("answer.json"), two_batch_pages()).unwrap();
    let path = spec(&w, name, "5", "");
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    usage_per_call(&w);
    std::fs::write(w.root.join("errors"), errors.to_string()).unwrap();
    let (code, ran) = w.cortex_env(
        &["run", &format!("{name}/news")],
        &[("CORTEX_MODEL_RETRY_MS", std::path::Path::new("10"))],
    );
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    (w, ran)
}

/// The token fields of a run, as its result or its `cortex.log` line states them.
const TOKENS: [&str; 6] = [
    "input_tokens",
    "cache_read_input_tokens",
    "cache_creation_input_tokens",
    "output_tokens",
    "fixed_input_tokens",
    "documents_input_tokens",
];

/// `model_calls` and the token counts of `fields`, numbers in the result as in the log line.
fn calls_and_tokens(fields: &Value) -> Vec<Value> {
    std::iter::once("model_calls")
        .chain(TOKENS)
        .map(|f| fields[f].clone())
        .collect()
}

#[test]
fn a_two_batch_run_reports_its_model_calls_and_the_tokens_of_both_answers() {
    let (w, ran) = run_with_usage("usage", 0);
    assert_eq!(w.lines("claude-calls.log").len(), 2, "two model calls");
    let detail = &ran["detail"];
    let line = last_log_line(&w, "usage");
    for fields in [detail, &line] {
        assert_eq!(fields["model_calls"], 2, "{fields}");
        assert_eq!(fields["input_tokens"], 2000, "{fields}");
        assert_eq!(fields["cache_read_input_tokens"], 400, "{fields}");
        assert_eq!(fields["cache_creation_input_tokens"], 0, "{fields}");
        assert_eq!(fields["output_tokens"], 100, "{fields}");
        // The split covers every input token the calls reported, cache reads and writes too.
        let fixed = fields["fixed_input_tokens"].as_i64().expect("fixed tokens");
        let documents = fields["documents_input_tokens"]
            .as_i64()
            .expect("documents");
        assert_eq!(fixed + documents, 2000 + 400, "{fields}");
        assert!(fixed > 0 && documents > 0, "{fields}");
        // Four pages of about 16 300 characters each outweigh the fixed part.
        assert!(documents > fixed, "{fields}");
    }
    assert_eq!(calls_and_tokens(detail), calls_and_tokens(&line));
    assert_eq!(detail["cost_usd"], "0.0200", "{ran}");
}

#[test]
fn a_call_asked_again_after_a_zero_cost_error_counts_as_a_model_call() {
    let (w, ran) = run_with_usage("retried", 1);
    assert_eq!(w.lines("claude-errors.log").len(), 1, "one error answer");
    assert_eq!(w.lines("claude-calls.log").len(), 2, "two answered calls");
    for fields in [&ran["detail"], &last_log_line(&w, "retried")] {
        assert_eq!(fields["model_calls"], 3, "{fields}");
        assert_eq!(fields["input_tokens"], 2000, "{fields}");
        assert_eq!(fields["cache_read_input_tokens"], 400, "{fields}");
        assert_eq!(fields["output_tokens"], 100, "{fields}");
    }
}

#[test]
fn answers_without_usage_report_their_model_calls_and_null_tokens_never_zero() {
    // The world's own stand-in answers no `usage`.
    let (w, ran) = run_with_costs("nousage", "5", &["0.25", "0.5"]);
    assert_eq!(w.lines("claude-calls.log").len(), 2, "two model calls");
    let line = last_log_line(&w, "nousage");
    for fields in [&ran["detail"], &line] {
        assert_eq!(fields["model_calls"], 2, "{fields}");
        for token in TOKENS {
            assert!(
                fields.get(token).is_some_and(Value::is_null),
                "{token} is null: {fields}"
            );
        }
    }
    assert_eq!(ran["detail"]["cost_usd"], "0.7500", "{ran}");
    assert_eq!(line["cost_usd"], 0.75);
}

#[test]
fn a_run_with_nothing_new_reports_no_model_call_and_no_tokens() {
    let (w, first) = run_with_usage("nothing", 0);
    assert_eq!(first["detail"]["model_calls"], 2, "{first}");
    let (code, ran) = w.cortex(&["run", "nothing/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_new"], 0, "{ran}");
    assert_eq!(
        w.lines("claude-calls.log").len(),
        2,
        "no further model call"
    );
    let line = last_log_line(&w, "nothing");
    for fields in [&ran["detail"], &line] {
        assert_eq!(fields["model_calls"], 0, "{fields}");
        for token in TOKENS {
            assert_eq!(fields[token], 0, "{token}: {fields}");
        }
    }
}
