//! `cortex quality <instance> --sample N` (`story:quality-judge`): `ekr sample` draws N facts with
//! their evidence, a stand-in judge answers each batch of 20, and `ekr fact-quality` turns the
//! verdicts into a pass rate with its interval, written under `quality/<UTC stamp>/`.

mod common;

use std::path::{Path, PathBuf};

use common::{executable, World};
use serde_json::{json, Value};

/// An extraction stand-in: `n` products, each with its own `version`, all citing the first
/// evidence id of the batch, so the store holds `n` distinct property facts besides the web pages'.
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
{{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{{"format":"ekr.extraction-document/1","ontology":{{"node_types":[{{"name":"Product","parents":[],"abstract_type":false,"properties":[{{"name":"version","value":{{"value_kind":"String"}},"cardinality":"One","required":false}}]}}],"edge_types":[]}},"entities":[$entities],"facts":[$facts]}}}}
EOF
"#
        ),
    );
}

/// A judge stand-in: records its arguments, its prompt and whether `ANTHROPIC_API_KEY` reached
/// it, then answers `no` for the first two facts of its first call and `yes` for every other one,
/// at a cost of 0.02 USD. With the file `judge-error` it answers an error instead. With the file
/// `judge-no-cost`, which holds a call number counted from 0, that call and every later one answer
/// with no cost.
fn judge(w: &World) {
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
n=$(cat "$R/judge-calls.log" 2>/dev/null | wc -l)
echo call >> "$R/judge-calls.log"
printf "%s\n" "$*" | tr "\n" " " >> "$R/judge-args.log"; echo >> "$R/judge-args.log"
env | grep -c '^ANTHROPIC_API_KEY=' >> "$R/judge-env.log"
prompt=$(cat)
printf '%s\n' "$prompt" > "$R/judge-prompt-$n.txt"
if [ -e "$R/judge-error" ]; then
  echo '{{"type":"result","subtype":"error_max_budget_usd","is_error":true,"total_cost_usd":0.02}}'
  exit 0
fi
verdicts=""
k=0
for id in $(printf '%s\n' "$prompt" | grep -oE '^=== Fact [0-9a-f-]{{36}}' | cut -d' ' -f3); do
  if [ "$n" -eq 0 ] && [ $k -lt 2 ]; then v=no; else v=yes; fi
  verdicts="$verdicts{{\"fact\":\"$id\",\"verdict\":\"$v\",\"reason\":\"the evidence says $v\"}},"
  k=$((k+1))
done
verdicts=${{verdicts%,}}
COST='"total_cost_usd":0.02,'
if [ -e "$R/judge-no-cost" ] && [ "$n" -ge "$(cat "$R/judge-no-cost")" ]; then COST=''; fi
echo "{{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,$COST\"structured_output\":{{\"verdicts\":[$verdicts]}}}}"
"#,
            root = w.root.display()
        ),
    );
}

/// An instance `name` whose store holds `products` product facts besides the web pages' facts,
/// with the judge stand-in in place of the extraction one.
fn instance(name: &str, products: usize, extra: &str) -> World {
    let w = World::new();
    let path = w.spec(name, "conn_test");
    if !extra.is_empty() {
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, format!("{text}{extra}")).unwrap();
    }
    extract_products(&w, products);
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let (code, ran) = w.cortex(&["run", &format!("{name}/news")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    judge(&w);
    w
}

fn quality_dirs(w: &World, name: &str) -> Vec<PathBuf> {
    let dir = w.home.join("instances").join(name).join("quality");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|d| d.map(|e| e.unwrap().path()).collect())
        .unwrap_or_default();
    dirs.sort();
    dirs
}

fn verdicts(dir: &Path) -> Vec<Value> {
    std::fs::read_to_string(dir.join("verdicts.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[test]
fn eighteen_yes_and_two_no_of_twenty_is_a_pass_rate_of_0_9() {
    let w = instance("judged", 30, "");
    let (code, out) = w.cortex(&["quality", "judged", "--sample", "20"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("measured")),
        "{out}"
    );

    let dirs = quality_dirs(&w, "judged");
    assert_eq!(dirs.len(), 1, "{dirs:?}");
    let stamp = dirs[0].file_name().unwrap().to_str().unwrap().to_string();
    assert_eq!(out["detail"]["stamp"], stamp.as_str(), "{out}");

    let report: Value =
        serde_json::from_slice(&std::fs::read(dirs[0].join("fact-quality.json")).unwrap()).unwrap();
    assert_eq!(report["meta"]["format"], "ekr.fact-quality/1", "{report}");
    assert_eq!(report["rate"], json!(0.9), "{report}");
    assert_eq!(
        (report["judged"].clone(), report["passed"].clone()),
        (json!(20), json!(18))
    );

    let lines = verdicts(&dirs[0]);
    assert_eq!(lines.len(), 20);
    let no = lines.iter().filter(|v| v["verdict"] == "no").count();
    assert_eq!(no, 2, "{lines:?}");
    for v in &lines {
        assert_eq!(v["format"], "cortex.quality-verdict/1", "{v}");
        assert!(v["fact"].as_str().is_some_and(|f| f.len() == 36), "{v}");
        assert!(v["reason"].as_str().is_some_and(|r| !r.is_empty()), "{v}");
    }

    assert_eq!(out["detail"]["judged"], 20, "{out}");
    assert_eq!(out["detail"]["passed"], 18, "{out}");
    assert_eq!(out["detail"]["rate"], json!(0.9), "{out}");
    assert_eq!(out["detail"]["cost_usd"], "0.0200", "{out}");
    assert_eq!(w.lines("judge-calls.log").len(), 1, "one batch of 20");
}

#[test]
fn the_judge_runs_isolated_with_its_own_system_prompt_and_sees_each_fact_with_its_evidence() {
    let w = instance("isolated", 3, "");
    let (code, out) = w.cortex(&["quality", "isolated", "--sample", "20"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("measured")),
        "{out}"
    );
    let args = w.lines("judge-args.log").join("\n");
    for flag in [
        "--tools  ",
        "--setting-sources ",
        "--strict-mcp-config",
        "--json-schema",
    ] {
        assert!(args.contains(flag), "{flag} missing: {args}");
    }
    assert!(args.contains("--system-prompt"), "{args}");
    assert!(!args.contains("ekr.extraction-document/1"), "{args}");
    assert_eq!(
        w.lines("judge-env.log"),
        ["0"],
        "ANTHROPIC_API_KEY reached the judge"
    );
    let prompt = std::fs::read_to_string(w.root.join("judge-prompt-0.txt")).unwrap();
    assert!(
        prompt.contains("Example Labs develops the Widget engine"),
        "{prompt}"
    );
}

#[test]
fn more_than_twenty_facts_are_judged_in_batches_of_twenty_and_their_costs_add_up() {
    let w = instance("batched", 30, "");
    let (code, out) = w.cortex(&["quality", "batched", "--sample", "25"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("measured")),
        "{out}"
    );
    assert_eq!(w.lines("judge-calls.log").len(), 2, "{out}");
    assert_eq!(out["detail"]["judged"], 25, "{out}");
    assert_eq!(out["detail"]["passed"], 23, "{out}");
    assert_eq!(out["detail"]["cost_usd"], "0.0400", "{out}");
    let lines = verdicts(&quality_dirs(&w, "batched")[0]);
    let mut facts: Vec<&str> = lines.iter().filter_map(|v| v["fact"].as_str()).collect();
    facts.sort();
    facts.dedup();
    assert_eq!(facts.len(), 25, "every sampled fact once");
}

#[test]
fn a_failed_judge_writes_no_pass_rate() {
    let w = instance("failing", 3, "");
    std::fs::write(w.root.join("judge-error"), "").unwrap();
    let (code, out) = w.cortex(&["quality", "failing", "--sample", "3"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("judge-failed")),
        "{out}"
    );
    let reason = out["detail"]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("cost 0.0200 USD"), "{out}");
    let dirs = quality_dirs(&w, "failing");
    assert_eq!(dirs.len(), 1, "{dirs:?}");
    assert!(!dirs[0].join("fact-quality.json").exists());
}

#[test]
fn a_sample_size_ekr_refuses_asks_no_model_and_writes_nothing() {
    let w = instance("refused", 3, "");
    let (code, out) = w.cortex(&["quality", "refused", "--sample", "0"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("sample-failed")),
        "{out}"
    );
    assert!(w.lines("judge-calls.log").is_empty());
    assert!(quality_dirs(&w, "refused").is_empty());
}

#[test]
fn an_unknown_instance_is_not_measured() {
    let w = World::new();
    let (code, out) = w.cortex(&["quality", "nobody", "--sample", "5"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("no-such-instance")),
        "{out}"
    );
}

/// `story:events-carry-measurements`: `QualityMeasured` carries `rate` and `cost_usd`. A store
/// holding no fact to draw is measured with no judge asked: `rate` is null, the interval is 0 to
/// 1, and the cost is 0, as no answer went without one.
#[test]
fn a_store_with_no_fact_to_draw_is_measured_with_a_null_rate() {
    let w = World::new();
    let path = w.spec("empty", "conn_test");
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    judge(&w);
    let (code, out) = w.cortex(&["quality", "empty", "--sample", "5"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("measured")),
        "{out}"
    );
    let d = &out["detail"];
    assert_eq!(d["judged"], 0, "{out}");
    assert_eq!(d.get("rate"), Some(&Value::Null), "{out}");
    assert_eq!(
        (d["lower"].as_f64(), d["upper"].as_f64()),
        (Some(0.0), Some(1.0)),
        "{out}"
    );
    assert_eq!(d["cost_usd"], "0.0000", "{out}");
    assert!(w.lines("judge-calls.log").is_empty(), "{out}");
}

/// `story:events-carry-measurements`: `cost_usd` is null as soon as one answer carried no cost,
/// never the sum of the costed ones and never 0. Of two batches the first costs 0.02 USD and the
/// second carries no cost; `rate` is still the pass rate `fact-quality.json` holds.
#[test]
fn a_judge_answer_without_a_cost_makes_the_measurements_cost_null() {
    let w = instance("uncosted", 30, "");
    std::fs::write(w.root.join("judge-no-cost"), "1").unwrap();
    let (code, out) = w.cortex(&["quality", "uncosted", "--sample", "25"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("measured")),
        "{out}"
    );
    assert_eq!(w.lines("judge-calls.log").len(), 2, "{out}");
    let d = &out["detail"];
    assert_eq!(d.get("cost_usd"), Some(&Value::Null), "{out}");
    let report: Value = serde_json::from_slice(
        &std::fs::read(quality_dirs(&w, "uncosted")[0].join("fact-quality.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(d["rate"].as_f64(), Some(0.92), "{out}");
    assert_eq!(d["rate"], report["rate"], "{out} {report}");
}

/// Adversary, `story:events-carry-measurements`: a pass rate of 0 is a rate, not a missing one.
/// The judge answers `no` for both facts of a sample of two, so `rate` is the number 0, as
/// `fact-quality.json` holds it, and never null, which is kept for a store with no fact to draw.
#[test]
fn adv_every_fact_judged_no_is_a_rate_of_zero_not_a_null_rate() {
    let w = instance("refuted", 30, "");
    let (code, out) = w.cortex(&["quality", "refuted", "--sample", "2"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("measured")),
        "{out}"
    );
    let d = &out["detail"];
    assert_eq!(
        (d["judged"].as_i64(), d["passed"].as_i64()),
        (Some(2), Some(0)),
        "{out}"
    );
    assert!(
        d["rate"].is_number(),
        "a rate of 0 printed as {}: {out}",
        d["rate"]
    );
    assert_eq!(d["rate"].as_f64(), Some(0.0), "{out}");
    let report: Value = serde_json::from_slice(
        &std::fs::read(quality_dirs(&w, "refuted")[0].join("fact-quality.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        d["rate"].to_string(),
        report["rate"].to_string(),
        "{report}"
    );
    assert_eq!(d["cost_usd"], "0.0200", "{out}");
}

/// Adversary, `story:events-carry-measurements`: the decimals a `measured` line prints are the
/// event's, digit for digit as `fact-quality.json` holds them and in the form the generated
/// `QualityMeasured` schema states for each. One `yes` of three facts is a rate EKR prints with
/// sixteen digits.
#[test]
fn adv_a_rate_of_one_third_is_printed_digit_for_digit_in_the_events_decimal_form() {
    let w = instance("third", 30, "");
    let (code, out) = w.cortex(&["quality", "third", "--sample", "3"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("measured")),
        "{out}"
    );
    let d = &out["detail"];
    assert_eq!(
        (d["judged"].as_i64(), d["passed"].as_i64()),
        (Some(3), Some(1)),
        "{out}"
    );
    let report: Value = serde_json::from_slice(
        &std::fs::read(quality_dirs(&w, "third")[0].join("fact-quality.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(d["rate"].to_string(), "0.3333333333333333", "{out}");
    for key in ["rate", "lower", "upper"] {
        assert!(d[key].is_number(), "{key}: {out}");
        assert_eq!(
            d[key].to_string(),
            report[key].to_string(),
            "{key}: {out} {report}"
        );
    }
    let schema: Value = serde_json::from_slice(
        &std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("generated/schema/schema/events/cortex.instance.QualityMeasured.schema.json"),
        )
        .unwrap(),
    )
    .unwrap();
    for key in ["rate", "lower", "upper", "cost_usd"] {
        let pattern = schema["properties"][key]["pattern"]
            .as_str()
            .unwrap_or_else(|| panic!("QualityMeasured.{key} has no pattern: {schema}"));
        let text = match &d[key] {
            Value::String(s) => s.clone(),
            v => v.to_string(),
        };
        assert!(
            regex::Regex::new(pattern).unwrap().is_match(&text),
            "{key} = {text} is not the event's decimal form {pattern}: {out}"
        );
    }
    assert_eq!(d["cost_usd"], "0.0200", "{out}");
}

#[test]
fn the_judge_is_shown_evidence_as_the_redaction_policy_shows_a_run() {
    let email = format!("{}@{}", "press", "example.org");
    let pages = common::PAGES.replace(
        "Its website is example.org.",
        &format!("Its press contact is {email}."),
    );
    let w = World::new();
    std::fs::write(w.root.join("answer.json"), common::answer(&pages)).unwrap();
    let path = w.spec("private", "conn_test");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        format!("{text}redaction: {{classes: [Email], rules: []}}\n"),
    )
    .unwrap();
    extract_products(&w, 3);
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let (code, ran) = w.cortex(&["run", "private/news"]);
    assert_eq!(code, 0, "{ran}");
    judge(&w);
    let (code, out) = w.cortex(&["quality", "private", "--sample", "20"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("measured")),
        "{out}"
    );
    let prompt = std::fs::read_to_string(w.root.join("judge-prompt-0.txt")).unwrap();
    assert!(!prompt.contains(&email), "{prompt}");
    assert!(prompt.contains("[Email-1]"), "{prompt}");
}

/// Adversary (wave 20261006g, unit a): no other case of the suite ever produces an `unclear`
/// verdict end to end, so mapping `unclear` to EKR's `Pass` (`src/quality.rs:502`) or dropping the
/// `unclear` count survives it. The judge here answers `unclear` for its first fact, nothing for
/// its second, `no` then `yes` for its third, and `yes` for a fact the batch does not hold.
#[test]
fn unclear_and_missing_verdicts_fail_and_an_injected_or_repeated_one_changes_nothing() {
    let w = instance("mixed", 3, "");
    let forged = "00000000-0000-4000-8000-00000000beef";
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"prompt=$(cat)
verdicts="{{\"fact\":\"{forged}\",\"verdict\":\"yes\",\"reason\":\"injected\"}},"
k=0
for id in $(printf '%s\n' "$prompt" | grep -oE '^=== Fact [0-9a-f-]{{36}}' | cut -d' ' -f3); do
  case $k in
    0) verdicts="$verdicts{{\"fact\":\"$id\",\"verdict\":\"unclear\",\"reason\":\"cannot tell\"}}," ;;
    1) ;;
    2) verdicts="$verdicts{{\"fact\":\"$id\",\"verdict\":\"no\",\"reason\":\"first\"}},{{\"fact\":\"$id\",\"verdict\":\"yes\",\"reason\":\"second\"}}," ;;
    *) verdicts="$verdicts{{\"fact\":\"$id\",\"verdict\":\"yes\",\"reason\":\"stated\"}}," ;;
  esac
  k=$((k+1))
done
verdicts=${{verdicts%,}}
echo "{{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"total_cost_usd\":0.02,\"structured_output\":{{\"verdicts\":[$verdicts]}}}}"
"#
        ),
    );
    let (code, out) = w.cortex(&["quality", "mixed", "--sample", "6"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("measured")),
        "{out}"
    );
    let d = &out["detail"];
    assert_eq!(
        (
            d["judged"].clone(),
            d["passed"].clone(),
            d["unclear"].clone()
        ),
        (json!(6), json!(3), json!(2)),
        "{out}"
    );
    let dir = &quality_dirs(&w, "mixed")[0];
    let report: Value =
        serde_json::from_slice(&std::fs::read(dir.join("fact-quality.json")).unwrap()).unwrap();
    assert_eq!(
        (
            report["judged"].clone(),
            report["passed"].clone(),
            report["rate"].clone()
        ),
        (json!(6), json!(3), json!(0.5)),
        "{report}"
    );
    let lines = verdicts(dir);
    let kinds: Vec<&str> = lines.iter().filter_map(|v| v["verdict"].as_str()).collect();
    assert_eq!(
        kinds,
        ["unclear", "unclear", "no", "yes", "yes", "yes"],
        "{lines:?}"
    );
    assert!(lines.iter().all(|v| v["fact"] != forged), "{lines:?}");
}

/// Adversary (wave 20261006g, unit a): `website/docs/commands.md`: "a failed model call's reason
/// states the cost so far". The second of two batches finds the budget spent and its call still
/// costs 0.02 USD: 0.04 USD was spent in all, and the reason says "0.0200 USD in all".
#[test]
fn a_judge_failing_mid_measurement_states_everything_spent_in_all() {
    let w = instance("spent", 30, "");
    // The judge stand-in answers its second call with the budget error at 0.02 USD.
    let calls = w.root.join("judge-calls.log");
    let script = std::fs::read_to_string(w.bin.join("claude")).unwrap();
    let script = script.replace(
        "if [ -e \"$R/judge-error\" ]; then",
        "if [ -e \"$R/judge-error\" ] || [ \"$n\" -ge 1 ]; then",
    );
    let body = script
        .split_once('\n')
        .map_or(script.as_str(), |(_, rest)| rest);
    executable(&w.bin.join("claude"), body);
    let (code, out) = w.cortex(&["quality", "spent", "--sample", "25"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("judge-failed")),
        "{out}"
    );
    assert_eq!(std::fs::read_to_string(&calls).unwrap().lines().count(), 2);
    assert_eq!(verdicts(&quality_dirs(&w, "spent")[0]).len(), 20, "{out}");
    let reason = out["detail"]["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("cost 0.0400 USD in all"),
        "two calls of 0.02 USD were made: {reason}"
    );
}

/// Adversary (wave 20261006g, unit a): `RareName` counts a capitalised word in a batch's document
/// texts only; the known entity names are not counted (`src/redact.rs`, module doc). A run shows
/// its model a name its evidence holds once as `[Name-1]`. The judge is shown the same evidence,
/// and the fact's statement names the same entity: counted twice, the name is no longer rare and
/// reaches the judge in the clear.
#[test]
fn a_rare_name_a_fact_and_its_evidence_both_hold_reaches_the_judge_masked_as_it_reached_the_run() {
    let name = format!("{}{}", "Kowal", "czyk");
    let pages = format!(
        r#"{{"results":[{{"url":"https://example.org/a","title":"a","description":"about the engine.","content":"The engine is kept by {name} today.","content_truncated":false,"published":null,"score":"0.9"}}],"complete":true,"truncation":[],"provenance":{{"instance":"t","profile":"tavily/2026-10","received_at":"2026-10-05T00:00:00.000Z"}}}}"#
    );
    let w = World::new();
    std::fs::write(w.root.join("answer.json"), common::answer(&pages)).unwrap();
    let path = w.spec("rare", "conn_test");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(
        &path,
        format!("{text}redaction:\n  classes: [RareName]\n  rules: []\n  rare_limit: 1\n"),
    )
    .unwrap();
    // The extraction stand-in keeps the prompt the run's model was shown, and records one person
    // named as the page names it, with one property citing the page.
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"prompt=$(cat)
printf '%s\n' "$prompt" > "{root}/run-prompt.txt"
id=$(printf '%s\n' "$prompt" | grep -oE 'evidence id [0-9a-f-]{{36}}' | head -1 | cut -d' ' -f3)
cat <<EOF
{{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{{"format":"ekr.extraction-document/1","ontology":{{"node_types":[{{"name":"Person","parents":[],"abstract_type":false,"properties":[{{"name":"role","value":{{"value_kind":"String"}},"cardinality":"One","required":false}}]}}],"edge_types":[]}},"entities":[{{"node_type":"Person","aliases":["{name}"]}}],"facts":[{{"!Property":{{"subject":{{"node_type":"Person","aliases":["{name}"]}},"property":"role","value":{{"value_kind":"String","value":"keeper"}},"evidence":["$id"]}}}}]}}}}
EOF
"#,
            root = w.root.display()
        ),
    );
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let (code, ran) = w.cortex(&["run", "rare/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let run_prompt = std::fs::read_to_string(w.root.join("run-prompt.txt")).unwrap();
    assert!(
        !run_prompt.contains(&name) && run_prompt.contains("[Name-"),
        "the run's model is shown the name masked: {run_prompt}"
    );

    judge(&w);
    let (code, out) = w.cortex(&["quality", "rare", "--sample", "20"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("measured")),
        "{out}"
    );
    let prompt = std::fs::read_to_string(w.root.join("judge-prompt-0.txt")).unwrap();
    assert!(
        prompt.contains("The engine is kept by"),
        "the judge is shown the evidence: {prompt}"
    );
    assert!(
        !prompt.contains(&name),
        "a name the run's model was shown as a placeholder reaches the judge in the clear:\n{prompt}"
    );
}

/// Correction F4: the measurement holds the home's lock only while it draws the sample, so a
/// scheduled run does not wait behind the judge's model calls. During the judge's call another
/// cortex command takes the lock: `restore`, which answers `busy` instead of waiting while the lock
/// is held, answers `no-such-snapshot` for a name the instance does not hold.
#[test]
fn the_judge_is_asked_without_the_home_lock_held() {
    let w = instance("unlocked", 3, "");
    std::fs::rename(w.bin.join("claude"), w.bin.join("claude-judge")).unwrap();
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
"{cortex}" restore unlocked no-such-snapshot > "$R/lock-probe.json" 2>&1
exec "$R/bin/claude-judge" "$@"
"#,
            root = w.root.display(),
            cortex = env!("CARGO_BIN_EXE_cortex"),
        ),
    );
    let (code, out) = w.cortex(&["quality", "unlocked", "--sample", "3"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("measured")),
        "{out}"
    );
    let probe: Value =
        serde_json::from_str(&std::fs::read_to_string(w.root.join("lock-probe.json")).unwrap())
            .unwrap_or(Value::Null);
    assert_eq!(
        probe["outcome"], "no-such-snapshot",
        "another command found the home's lock held while the judge was asked: {probe}"
    );
}
