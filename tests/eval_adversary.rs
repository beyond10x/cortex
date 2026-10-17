//! Adversary cases for `cortex eval run` (`story:eval-run`): the set's published schema, blank
//! expectations, replays of another set's or an unfinished run, the home's lock during model
//! calls, a timeout and a spent budget mid-set, and totals that go absent with one answer's count.

mod common;

use std::path::{Path, PathBuf};

use common::{executable, World};
use serde_json::{json, Value};

/// A stand-in `claude`: answers a question naming the Widget engine with its fact and node and any
/// other with neither, at `cost` USD and 100 input and 10 output tokens, and counts its calls.
fn answerer(w: &World, cost: &str) {
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
echo call >> "$R/eval-calls.log"
prompt=$(cat)
case "$prompt" in
  *Widget*) A='"answer":"Example Labs develops the Widget engine.","cited_nodes":["node-widget"]' ;;
  *) A='"answer":"Nothing in the store says.","cited_nodes":[]' ;;
esac
echo "{{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"total_cost_usd\":{cost},\"usage\":{{\"input_tokens\":100,\"output_tokens\":10}},\"structured_output\":{{$A}}}}"
"#,
            root = w.root.display()
        ),
    );
}

/// An instance `name` whose spec's model line is `model` (the default spec's line when `None`).
fn instance_with(name: &str, model: Option<&str>) -> World {
    let w = World::new();
    let path = w.spec(name, "conn_test");
    if let Some(model) = model {
        let text = std::fs::read_to_string(&path).unwrap();
        let line = "model: {model: claude-haiku-4-5-20251001, budget_usd: \"1\", timeout_s: 60}";
        assert!(text.contains(line), "the common spec changed: {text}");
        std::fs::write(&path, text.replace(line, model)).unwrap();
    }
    let (code, created) = w.cortex(&[
        "create",
        "--spec",
        path.to_str().unwrap(),
        "--no-units",
        "--no-extract",
    ]);
    assert_eq!(code, 0, "{created}");
    answerer(&w, "0.01");
    w
}

fn instance(name: &str) -> World {
    instance_with(name, None)
}

fn case(id: &str, question: &str, fact: &str, node: &str) -> Value {
    json!({
        "format": "cortex.eval-case/1",
        "id": id,
        "question": question,
        "expect_facts": [fact],
        "expect_nodes": [node],
    })
}

fn write_set(w: &World, file: &str, cases: Vec<Value>) -> PathBuf {
    let path = w.root.join(file);
    std::fs::write(&path, json!({ "cases": cases }).to_string()).unwrap();
    path
}

fn widget() -> Value {
    case(
        "widget",
        "Who develops the Widget engine?",
        "widget engine",
        "node-widget",
    )
}

fn gadget() -> Value {
    case(
        "gadget",
        "Who develops the Gadget runtime?",
        "Gadget runtime",
        "node-gadget",
    )
}

fn eval(w: &World, args: &[&str]) -> (i32, Value) {
    let mut all = vec!["eval", "run"];
    all.extend_from_slice(args);
    w.cortex(&all)
}

fn mcp(w: &World, name: &str, set: &Path) -> (i32, Value) {
    eval(w, &[name, "--set", set.to_str().unwrap(), "--runner", "mcp"])
}

fn replay(w: &World, name: &str, set: &Path, from: &str) -> (i32, Value) {
    eval(
        w,
        &[
            name,
            "--set",
            set.to_str().unwrap(),
            "--runner",
            "replay",
            "--from",
            from,
        ],
    )
}

fn eval_dirs(w: &World, name: &str) -> Vec<PathBuf> {
    let dir = w.home.join("instances").join(name).join("eval");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|d| d.map(|e| e.unwrap().path()).collect())
        .unwrap_or_default();
    dirs.sort();
    dirs
}

fn results(dir: &Path) -> Vec<Value> {
    std::fs::read_to_string(dir.join("results.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn outcome(code: i32, out: &Value) -> (i32, Option<&str>) {
    (code, out["outcome"].as_str())
}

/// `generated/schema/.../cortex.instance.EvalCase.schema.json` requires `expect_facts` and
/// `expect_nodes`, types `source` as a string and admits no other property. A case it refuses is
/// not a `cortex.eval-case/1` case, and the misspelt key of the first one would otherwise score a
/// case with no expectation as passed.
#[test]
#[ignore = "adversary: a case the published EvalCase schema refuses (misspelt expect_facts, no expect lists, numeric source) is evaluated, and passes with no expectation"]
fn a_case_the_published_eval_case_schema_refuses_is_set_invalid() {
    let w = instance("schema");
    let misspelt = json!({
        "format": "cortex.eval-case/1",
        "id": "gadget",
        "question": "Who develops the Gadget runtime?",
        "expect_fact": ["Gadget runtime"],
        "expect_node": ["node-gadget"],
    });
    let listless = json!({
        "format": "cortex.eval-case/1",
        "id": "gadget",
        "question": "Who develops the Gadget runtime?",
    });
    let mut numeric_source = gadget();
    numeric_source["source"] = json!(7);
    for bad in [misspelt, listless, numeric_source] {
        let set = write_set(&w, "set.json", vec![bad.clone()]);
        let (code, out) = mcp(&w, "schema", &set);
        assert_eq!(
            outcome(code, &out),
            (1, Some("set-invalid")),
            "{bad} -> {out}"
        );
    }
    assert!(w.lines("eval-calls.log").is_empty(), "a model was asked");
}

/// A blank expected fact is contained in every answer: the case passes on "Nothing in the store
/// says." and `facts_found` counts it.
#[test]
#[ignore = "adversary: an expected fact of only white space (or empty) is found in every answer, so the case passes on a non-answer"]
fn a_blank_expected_fact_is_set_invalid() {
    let w = instance("blank");
    for blank in ["", "   "] {
        let mut c = gadget();
        c["expect_facts"] = json!([blank]);
        c["expect_nodes"] = json!([]);
        let set = write_set(&w, "set.json", vec![c]);
        let (code, out) = mcp(&w, "blank", &set);
        assert_eq!(
            outcome(code, &out),
            (1, Some("set-invalid")),
            "{blank:?} -> {out}"
        );
    }
}

/// `replay` keys the recorded answers by case id only. A run of another set whose case of the same
/// id asked another question is replayed as if it had answered this one: the new results line
/// pairs this set's question with an answer to a different question.
#[test]
#[ignore = "adversary: replay pairs a recorded answer with a different question of the same id and scores it as evaluated"]
fn replay_of_a_run_that_asked_another_question_under_the_id_is_refused() {
    let w = instance("otherset");
    let first = write_set(&w, "first.json", vec![widget()]);
    let (code, out) = mcp(&w, "otherset", &first);
    assert_eq!(outcome(code, &out), (0, Some("evaluated")), "{out}");
    let stamp = out["detail"]["stamp"].as_str().unwrap().to_string();

    let other = case(
        "widget",
        "Which company sells the Sprocket line?",
        "widget engine",
        "node-widget",
    );
    let second = write_set(&w, "second.json", vec![other]);
    let (code, out) = replay(&w, "otherset", &second, &stamp);
    let dir = out["detail"]["dir"].as_str().map(PathBuf::from);
    assert_ne!(
        outcome(code, &out),
        (0, Some("evaluated")),
        "replayed: {:?}",
        dir.map(|d| results(&d))
    );
}

/// The run `replay` names must be an eval run: a directory whose run failed has a `results.jsonl`
/// with the cases answered before the failure and no `summary.json`, and is no `EvalRun`.
#[test]
#[ignore = "adversary: replay accepts a run that failed mid-set (no summary.json) as its source and answers evaluated"]
fn replay_of_a_run_that_failed_and_wrote_no_summary_is_runner_failed() {
    let w = instance("halfdone");
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
n=$(cat "$R/eval-calls.log" 2>/dev/null | wc -l)
echo call >> "$R/eval-calls.log"
cat > /dev/null
if [ "$n" -ge 1 ]; then echo '{{"type":"result","subtype":"error_during_execution","is_error":true,"total_cost_usd":0.01}}'; exit 0; fi
echo '{{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{{"answer":"Example Labs develops the Widget engine.","cited_nodes":["node-widget"]}}}}'
"#,
            root = w.root.display()
        ),
    );
    let both = write_set(&w, "both.json", vec![widget(), gadget()]);
    let (code, out) = mcp(&w, "halfdone", &both);
    assert_eq!(outcome(code, &out), (1, Some("runner-failed")), "{out}");
    let dirs = eval_dirs(&w, "halfdone");
    assert_eq!(dirs.len(), 1, "{dirs:?}");
    assert!(!dirs[0].join("summary.json").exists());
    let stamp = dirs[0].file_name().unwrap().to_str().unwrap().to_string();

    let one = write_set(&w, "one.json", vec![widget()]);
    let (code, out) = replay(&w, "halfdone", &one, &stamp);
    assert_eq!(outcome(code, &out), (1, Some("runner-failed")), "{out}");
}

/// While the model answers, another command of the home takes the lock: the stand-in runs a
/// second `cortex eval run` (a replay, which asks no model) under `timeout`, and records its exit
/// status; 124 means it waited on the lock.
#[test]
fn the_home_lock_is_not_held_while_the_model_answers() {
    let w = instance("unlocked");
    let set = write_set(&w, "set.json", vec![widget()]);
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
cat > /dev/null
timeout 20 "{cortex}" eval run unlocked --set "{set}" --runner replay --from 20200101T000000Z > /dev/null 2>&1
echo $? >> "$R/inner.log"
echo '{{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{{"answer":"Example Labs develops the Widget engine.","cited_nodes":["node-widget"]}}}}'
"#,
            root = w.root.display(),
            cortex = env!("CARGO_BIN_EXE_cortex"),
            set = set.display(),
        ),
    );
    let (code, out) = mcp(&w, "unlocked", &set);
    assert_eq!(outcome(code, &out), (0, Some("evaluated")), "{out}");
    // The inner replay names a run that does not exist: runner-failed, exit 1, not a timeout.
    assert_eq!(w.lines("inner.log"), vec!["1".to_string()]);
}

/// A model call that outlives `model.timeout_s` ends the run as runner-failed, keeping the result
/// of the case answered before it and writing no summary.
#[test]
fn a_model_call_that_times_out_mid_set_is_runner_failed_and_keeps_the_answers_before_it() {
    let w = instance_with(
        "slow",
        Some("model: {model: claude-haiku-4-5-20251001, budget_usd: \"1\", timeout_s: 2}"),
    );
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
n=$(cat "$R/eval-calls.log" 2>/dev/null | wc -l)
echo call >> "$R/eval-calls.log"
cat > /dev/null
if [ "$n" -ge 1 ]; then exec sleep 40; fi
echo '{{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{{"answer":"Example Labs develops the Widget engine.","cited_nodes":["node-widget"]}}}}'
"#,
            root = w.root.display()
        ),
    );
    let set = write_set(&w, "set.json", vec![widget(), gadget(), widget_too()]);
    let started = std::time::Instant::now();
    let (code, out) = mcp(&w, "slow", &set);
    assert!(
        started.elapsed() < std::time::Duration::from_secs(30),
        "the timeout did not end the call: {:?}",
        started.elapsed()
    );
    assert_eq!(outcome(code, &out), (1, Some("runner-failed")), "{out}");
    assert_eq!(w.lines("eval-calls.log").len(), 2, "a case after it was asked");
    let dirs = eval_dirs(&w, "slow");
    assert_eq!(dirs.len(), 1, "{dirs:?}");
    let kept = results(&dirs[0]);
    assert_eq!(kept.len(), 1, "{kept:?}");
    assert_eq!(kept[0]["id"], "widget");
    assert!(!dirs[0].join("summary.json").exists());
}

fn widget_too() -> Value {
    case(
        "widget-2",
        "Who else develops the Widget engine?",
        "widget engine",
        "node-widget",
    )
}

/// `model.budget_usd` bounds the run: with 0.6 USD an answer and a budget of 1 USD, the third case
/// finds the budget spent and is not asked.
#[test]
fn a_spent_budget_mid_set_is_runner_failed_and_asks_no_further_case() {
    let w = instance("spent");
    answerer(&w, "0.6");
    let set = write_set(&w, "set.json", vec![widget(), gadget(), widget_too()]);
    let (code, out) = mcp(&w, "spent", &set);
    assert_eq!(outcome(code, &out), (1, Some("runner-failed")), "{out}");
    assert!(
        out["detail"]["reason"]
            .as_str()
            .is_some_and(|r| r.contains("budget")),
        "{out}"
    );
    assert_eq!(w.lines("eval-calls.log").len(), 2);
    let dirs = eval_dirs(&w, "spent");
    assert_eq!(results(&dirs[0]).len(), 2);
    assert!(!dirs[0].join("summary.json").exists());
}

/// `input_tokens`, `output_tokens` and `cost_usd` are absent as soon as one answer carried no
/// count; the per-check counts still add up.
#[test]
fn one_answer_without_tokens_or_cost_makes_those_totals_absent_and_the_counts_add_up() {
    let w = instance("uncounted");
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
n=$(cat "$R/eval-calls.log" 2>/dev/null | wc -l)
echo call >> "$R/eval-calls.log"
cat > /dev/null
if [ "$n" -ge 1 ]; then echo '{{"type":"result","subtype":"success","is_error":false,"structured_output":{{"answer":"Nothing in the store says.","cited_nodes":[]}}}}'; exit 0; fi
echo '{{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"usage":{{"input_tokens":100,"output_tokens":10}},"structured_output":{{"answer":"Example Labs develops the Widget engine.","cited_nodes":["node-widget"]}}}}'
"#,
            root = w.root.display()
        ),
    );
    let set = write_set(&w, "set.json", vec![widget(), gadget()]);
    let (code, out) = mcp(&w, "uncounted", &set);
    assert_eq!(outcome(code, &out), (0, Some("evaluated")), "{out}");
    let d = &out["detail"];
    assert_eq!(d["input_tokens"], Value::Null, "{out}");
    assert_eq!(d["output_tokens"], Value::Null, "{out}");
    assert_eq!(d["cost_usd"], Value::Null, "{out}");
    assert_eq!(
        [&d["cases"], &d["passed"], &d["facts_found"], &d["nodes_found"]],
        [&json!(2), &json!(1), &json!(1), &json!(1)],
        "{out}"
    );
    let dir = PathBuf::from(d["dir"].as_str().unwrap());
    let summary: Value =
        serde_json::from_slice(&std::fs::read(dir.join("summary.json")).unwrap()).unwrap();
    assert_eq!(summary["cost_usd"], Value::Null, "{summary}");
    assert_eq!(summary["input_tokens"], Value::Null, "{summary}");
}
