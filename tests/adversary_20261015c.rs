//! Adversary, wave 20261015c (`story:token-budget`, `story:model-through-gateway`): token counts
//! that cannot be counted must fail closed, and `model.budget_tokens` bounds "the same runs and
//! calls" `budget_usd` does (`cortex.instance.ModelSpec` in `spec/domains/instance.yaml`).

mod common;

use std::path::PathBuf;

use common::{ekr, executable, World};
use cortex_cli::extract::Usage;
use serde_json::{json, Value};

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

/// A spec with large documents and one source `news`, the model fields `model` added after
/// `timeout_s`.
fn spec(w: &World, name: &str, model: &str) -> PathBuf {
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

/// Wraps the world's `claude` so no answer carries a cost and each carries `usage` (JSON text,
/// ending in a comma); with `usage` empty, none.
fn answering_usage(w: &World, usage: &str) {
    std::fs::write(w.root.join("no-cost"), "").unwrap();
    std::fs::rename(w.bin.join("claude"), w.bin.join("claude-real")).unwrap();
    executable(
        &w.bin.join("claude"),
        &format!(
            r#""{root}/bin/claude-real" "$@" | sed 's/"is_error":false,/"is_error":false,{usage}/'
"#,
            root = w.root.display(),
        ),
    );
}

fn instance(name: &str, n: usize, model: &str, usage: &str) -> World {
    let w = World::new();
    std::fs::write(w.root.join("answer.json"), pages(n)).unwrap();
    let path = spec(&w, name, model);
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    answering_usage(&w, usage);
    w
}

fn run(w: &World, source: &str) -> Value {
    let (code, ran) = w.cortex(&["run", source]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    ran
}

/// `spend::tokens` sums an answer's four counts with an unchecked `+`
/// (`src/spend.rs`, `tokens`): counts whose sum does not fit an i64 panic in a debug build and
/// wrap to a negative number in a release build, which lowers the run's `used_tokens`
/// (`src/run.rs`) instead of failing closed. A sum that cannot be represented cannot be counted.
#[test]
fn adv_a_token_sum_that_overflows_is_not_counted() {
    let usage = Usage {
        input_tokens: Some(i64::MAX - 10),
        cache_creation_input_tokens: Some(0),
        cache_read_input_tokens: Some(0),
        output_tokens: Some(100),
    };
    let counted = std::panic::catch_unwind(|| cortex_cli::spend::tokens(&usage));
    match counted {
        Ok(None) => {}
        Ok(Some(n)) => panic!("an overflowing sum is counted as {n} tokens"),
        Err(_) => panic!("counting an overflowing sum panics instead of failing closed"),
    }
}

/// An answer whose `usage` reports a negative count is counted as fewer tokens than nothing:
/// `used_tokens` goes down (`src/run.rs`, `used + spend::tokens(&usage)?`), so the run's token
/// budget never stops the run. Three batches, a budget of 3000 tokens, each answer reporting
/// -1 000 000 input tokens: every batch is asked for and no limit is named.
#[test]
fn adv_a_negative_token_count_does_not_lift_the_run_token_budget() {
    let usage = r#""usage":{"input_tokens":-1000000,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":2000},"#;
    let w = instance("negative", 7, ", budget_tokens: 3000", usage);
    let ran = run(&w, "negative/news");
    let calls = w.lines("claude-calls.log").len();
    let stopped = ran["detail"]["stopped"].as_str().unwrap_or_default();
    assert!(
        calls < 3 && stopped.contains("token budget"),
        "a negative token count lifts the budget: {calls} calls, stopped {stopped:?}: {ran}"
    );
}

/// `daily_ceiling_reached` is "true when the run stopped before a batch because the instance's
/// spend of the UTC day reached `model.daily_ceiling_usd` or its tokens reached
/// `model.daily_ceiling_tokens`: ... the rest are extracted by a run on a later day"
/// (`spec/domains/instance.yaml`). An answer with no `usage` stops the run under the daily token
/// ceiling without any ceiling being reached, and a run later the same day goes on; the report
/// still says the daily ceiling was reached.
#[test]
fn adv_an_uncounted_answer_is_not_reported_as_a_reached_daily_ceiling() {
    let w = instance("uncounted", 4, ", daily_ceiling_tokens: 1000000", "");
    let ran = run(&w, "uncounted/news");
    assert_eq!(w.lines("claude-calls.log").len(), 1, "{ran}");
    let stopped = ran["detail"]["stopped"].as_str().unwrap_or_default();
    assert!(stopped.contains("could not be counted"), "{ran}");
    assert_ne!(
        ran["detail"].get("daily_ceiling_reached"),
        Some(&Value::Bool(true)),
        "1 call of 1 000 000 tokens: the ceiling was not reached: {ran}"
    );
}

/// A quality stand-in: one product fact per number up to `n`, at no cost (`tests/quality.rs`).
fn extract_products(w: &World, n: usize) {
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"id=$(grep -oE 'evidence id [0-9a-f-]{{36}}' | head -1 | cut -d' ' -f3)
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
{{"type":"result","subtype":"success","is_error":false,"usage":{{"input_tokens":10,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":10}},"structured_output":{{"format":"ekr.extraction-document/1","ontology":{{"node_types":[{{"name":"Product","parents":[],"abstract_type":false,"properties":[{{"name":"version","value":{{"value_kind":"String"}},"cardinality":"One","required":false}}]}}],"edge_types":[]}},"entities":[$entities],"facts":[$facts]}}}}
EOF
"#
        ),
    );
}

/// A judge stand-in answering `yes` for every fact, with no cost and 2000 tokens in `usage`.
fn judge(w: &World) {
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
echo call >> "$R/judge-calls.log"
prompt=$(cat)
verdicts=""
for id in $(printf '%s\n' "$prompt" | grep -oE '^=== Fact [0-9a-f-]{{36}}' | cut -d' ' -f3); do
  verdicts="$verdicts{{\"fact\":\"$id\",\"verdict\":\"yes\",\"reason\":\"the evidence says yes\"}},"
done
verdicts=${{verdicts%,}}
echo "{{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"usage\":{{\"input_tokens\":1500,\"cache_creation_input_tokens\":100,\"cache_read_input_tokens\":300,\"output_tokens\":100}},\"structured_output\":{{\"verdicts\":[$verdicts]}}}}"
"#,
            root = w.root.display()
        ),
    );
}

/// `budget_tokens` and `daily_ceiling_tokens` "bound the same runs and calls in tokens" as
/// `budget_usd` and `daily_ceiling_usd`, which bound "every source run, seed extraction, quality
/// measurement and schema proposal" (`cortex.instance.ModelSpec`). The judge is held to
/// `budget_usd` per measurement (`src/quality.rs`, `budget - spent`), never to `budget_tokens`:
/// with a budget of 1000 tokens and 2000 tokens a call, a measurement of two batches asks both.
#[test]
fn adv_the_judge_is_held_to_the_run_token_budget() {
    let w = World::new();
    let path = w.spec("judged", "conn_test");
    let text = std::fs::read_to_string(&path).unwrap().replace(
        "budget_usd: \"1\", timeout_s: 60}",
        "budget_usd: \"1\", timeout_s: 60, budget_tokens: 1000}",
    );
    std::fs::write(&path, text).unwrap();
    extract_products(&w, 30);
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let ran = run(&w, "judged/news");
    assert_eq!(ran["detail"]["stopped"], Value::Null, "{ran}");
    judge(&w);
    let (_, out) = w.cortex(&["quality", "judged", "--sample", "25"]);
    assert_eq!(
        w.lines("judge-calls.log").len(),
        1,
        "the judge's first call spent 2000 of 1000 tokens and a second was asked: {out}"
    );
}
