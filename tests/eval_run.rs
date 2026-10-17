//! `cortex eval run <instance> --set <set> --runner mcp|ask|replay` (`story:eval-run`): each case
//! of a `cortex.eval-case/1` set is answered by the runner and scored with no model, by the fact
//! strings the answer must contain and the node ids it must cite, under `eval/<UTC stamp>/`.

mod common;

use std::path::{Path, PathBuf};

use common::{executable, World};
use serde_json::{json, Value};

/// An answering stand-in for the `mcp` runner: records its arguments and that it started, then
/// answers a question naming the Widget engine with its expected fact and node, and any other
/// question with neither, at a cost of 0.01 USD and 100 input and 10 output tokens.
fn answerer(w: &World) {
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
echo call >> "$R/eval-calls.log"
printf "%s\n" "$*" | tr "\n" " " >> "$R/eval-args.log"; echo >> "$R/eval-args.log"
prompt=$(cat)
case "$prompt" in
  *Widget*) A='"answer":"Example Labs develops the WIDGET   engine.","cited_nodes":["node-widget"]' ;;
  *) A='"answer":"Nothing in the store says.","cited_nodes":[]' ;;
esac
echo "{{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"total_cost_usd\":0.01,\"usage\":{{\"input_tokens\":100,\"output_tokens\":10}},\"structured_output\":{{$A}}}}"
"#,
            root = w.root.display()
        ),
    );
}

/// A stand-in that only notes it was started: no model may be asked.
fn no_model(w: &World) {
    executable(
        &w.bin.join("claude"),
        &format!(
            "echo started >> \"{}/model-started.log\"\nexit 1\n",
            w.root.display()
        ),
    );
}

/// An instance `name` with an empty store (no extraction), and the answering stand-in.
fn instance(name: &str) -> World {
    let w = World::new();
    let path = w.spec(name, "conn_test");
    let (code, created) = w.cortex(&[
        "create",
        "--spec",
        path.to_str().unwrap(),
        "--no-units",
        "--no-extract",
    ]);
    assert_eq!(code, 0, "{created}");
    answerer(&w);
    w
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

/// The two-case set of the acceptance: the first is answered with its fact and node, the second
/// with neither.
fn two_cases(w: &World) -> PathBuf {
    let path = w.root.join("set.json");
    let set = json!({"cases": [
        case("widget", "Who develops the Widget engine?", "widget engine", "node-widget"),
        case("gadget", "Who develops the Gadget runtime?", "Gadget runtime", "node-gadget"),
    ]});
    std::fs::write(&path, set.to_string()).unwrap();
    path
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

fn verdicts(dir: &Path) -> Vec<(String, bool)> {
    results(dir)
        .iter()
        .map(|r| (r["id"].as_str().unwrap().to_string(), r["passed"] == true))
        .collect()
}

fn eval(w: &World, args: &[&str]) -> (i32, Value) {
    let mut all = vec!["eval", "run"];
    all.extend_from_slice(args);
    w.cortex(&all)
}

/// The `mcp` run of the acceptance; answers its directory.
fn mcp_run(w: &World, name: &str, set: &Path) -> PathBuf {
    let (code, out) = eval(
        w,
        &[name, "--set", set.to_str().unwrap(), "--runner", "mcp"],
    );
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("evaluated")),
        "{out}"
    );
    PathBuf::from(
        out["detail"]["dir"]
            .as_str()
            .expect("the detail names the run's directory"),
    )
}

#[test]
fn two_cases_one_answered_with_its_fact_and_node_score_one_of_two_and_name_the_missing_fact() {
    let w = instance("asked");
    let set = two_cases(&w);
    let (code, out) = eval(
        &w,
        &["asked", "--set", set.to_str().unwrap(), "--runner", "mcp"],
    );
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("evaluated")),
        "{out}"
    );
    let d = &out["detail"];
    assert_eq!(
        (d["cases"].clone(), d["passed"].clone()),
        (json!(2), json!(1)),
        "{out}"
    );
    assert_eq!(d["runner"], "mcp", "{out}");
    assert_eq!(
        (d["facts_expected"].clone(), d["facts_found"].clone()),
        (json!(2), json!(1)),
        "{out}"
    );
    assert_eq!(
        (d["nodes_expected"].clone(), d["nodes_found"].clone()),
        (json!(2), json!(1)),
        "{out}"
    );
    assert_eq!(d["model_calls"], json!(2), "{out}");
    assert_eq!(
        (d["input_tokens"].clone(), d["output_tokens"].clone()),
        (json!(200), json!(20)),
        "{out}"
    );
    assert_eq!(d["cost_usd"], "0.0200", "{out}");

    let dirs = eval_dirs(&w, "asked");
    assert_eq!(dirs.len(), 1, "{dirs:?}");
    assert_eq!(d["dir"], dirs[0].to_str().unwrap(), "{out}");
    assert_eq!(
        d["stamp"],
        dirs[0].file_name().unwrap().to_str().unwrap(),
        "{out}"
    );

    let lines = results(&dirs[0]);
    assert_eq!(lines.len(), 2, "{lines:?}");
    for r in &lines {
        assert_eq!(r["format"], "cortex.eval-result/1", "{r}");
    }
    assert_eq!(lines[0]["id"], "widget");
    assert_eq!(lines[0]["passed"], true, "{}", lines[0]);
    assert_eq!(lines[0]["cited_nodes"], json!(["node-widget"]));
    assert_eq!(lines[1]["id"], "gadget");
    assert_eq!(lines[1]["passed"], false, "{}", lines[1]);
    assert_eq!(
        lines[1]["missing_facts"],
        json!(["Gadget runtime"]),
        "{}",
        lines[1]
    );
    assert_eq!(
        lines[1]["missing_nodes"],
        json!(["node-gadget"]),
        "{}",
        lines[1]
    );

    let summary: Value =
        serde_json::from_slice(&std::fs::read(dirs[0].join("summary.json")).unwrap()).unwrap();
    assert_eq!(summary["format"], "cortex.eval-run/1", "{summary}");
    assert_eq!(
        (summary["cases"].clone(), summary["passed"].clone()),
        (json!(2), json!(1)),
        "{summary}"
    );
    assert_eq!(summary["runner"], "mcp", "{summary}");
    assert_eq!(summary["facts_found"], json!(1), "{summary}");
    assert_eq!(summary["input_tokens"], json!(200), "{summary}");

    // The model is the instance's, with the store's MCP reader as its only tools.
    let args = w.lines("eval-args.log");
    assert_eq!(args.len(), 2, "{args:?}");
    for a in &args {
        assert!(a.contains("--model claude-haiku-4-5-20251001"), "{a}");
        assert!(a.contains("--strict-mcp-config"), "{a}");
        assert!(a.contains("--mcp-config"), "{a}");
        assert!(a.contains("\"mcp\""), "the ekr MCP reader: {a}");
        assert!(a.contains("--allowedTools mcp__ekr"), "{a}");
    }
}

#[test]
fn replay_of_a_run_gives_its_verdicts_again_and_starts_no_model() {
    let w = instance("replayed");
    let set = two_cases(&w);
    let first = mcp_run(&w, "replayed", &set);
    no_model(&w);
    let stamp = first.file_name().unwrap().to_str().unwrap();
    let (code, out) = eval(
        &w,
        &[
            "replayed",
            "--set",
            set.to_str().unwrap(),
            "--runner",
            "replay",
            "--from",
            stamp,
        ],
    );
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("evaluated")),
        "{out}"
    );
    assert!(
        w.lines("model-started.log").is_empty(),
        "replay started the model"
    );
    let d = &out["detail"];
    assert_eq!(
        (d["cases"].clone(), d["passed"].clone()),
        (json!(2), json!(1)),
        "{out}"
    );
    assert_eq!(d["runner"], "replay", "{out}");
    assert_eq!(d["model_calls"], json!(0), "{out}");
    assert_eq!(d["cost_usd"], "0.0000", "{out}");
    let again = PathBuf::from(d["dir"].as_str().unwrap());
    assert_ne!(again, first);
    assert_eq!(verdicts(&again), verdicts(&first));
    assert_eq!(
        results(&again)[1]["missing_facts"],
        json!(["Gadget runtime"])
    );
}

#[test]
fn two_replays_of_one_run_write_byte_identical_results() {
    let w = instance("twice");
    let set = two_cases(&w);
    let first = mcp_run(&w, "twice", &set);
    no_model(&w);
    let stamp = first.file_name().unwrap().to_str().unwrap().to_string();
    let replay = |w: &World| {
        let (code, out) = eval(
            w,
            &[
                "twice",
                "--set",
                set.to_str().unwrap(),
                "--runner",
                "replay",
                "--from",
                &stamp,
            ],
        );
        assert_eq!(code, 0, "{out}");
        PathBuf::from(out["detail"]["dir"].as_str().unwrap())
    };
    let (a, b) = (replay(&w), replay(&w));
    assert_ne!(a, b);
    let bytes = |d: &Path| std::fs::read(d.join("results.jsonl")).unwrap();
    assert_eq!(bytes(&a), bytes(&b));
    // A replay writes what the run it replays wrote, line for line.
    assert_eq!(bytes(&a), bytes(&first));
}

#[test]
fn a_set_that_is_not_cortex_eval_case_1_is_set_invalid_and_asks_no_model() {
    let w = instance("invalid");
    no_model(&w);
    let path = w.root.join("other.json");
    let mut bad = case("q", "Who develops the Widget engine?", "widget", "n");
    bad["format"] = json!("cortex.eval-case/2");
    std::fs::write(&path, json!({"cases": [bad]}).to_string()).unwrap();
    let (code, out) = eval(
        &w,
        &[
            "invalid",
            "--set",
            path.to_str().unwrap(),
            "--runner",
            "mcp",
        ],
    );
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("set-invalid")),
        "{out}"
    );
    assert!(
        out["detail"]["reason"]
            .as_str()
            .is_some_and(|r| r.contains("cortex.eval-case/1")),
        "{out}"
    );
    assert!(w.lines("model-started.log").is_empty(), "a model was asked");
    assert!(eval_dirs(&w, "invalid").is_empty(), "something was written");

    // Not JSON at all, no case, and two cases with one id are refused the same way.
    for text in [
        "not json".to_string(),
        json!({"cases": []}).to_string(),
        json!({"cases": [case("q", "a?", "x", "n"), case("q", "b?", "y", "m")]}).to_string(),
    ] {
        std::fs::write(&path, text).unwrap();
        let (code, out) = eval(
            &w,
            &[
                "invalid",
                "--set",
                path.to_str().unwrap(),
                "--runner",
                "mcp",
            ],
        );
        assert_eq!(
            (code, out["outcome"].as_str()),
            (1, Some("set-invalid")),
            "{out}"
        );
    }
    assert!(w.lines("model-started.log").is_empty(), "a model was asked");
}

#[test]
fn replay_without_from_and_from_beside_another_runner_are_set_invalid() {
    let w = instance("fromless");
    let set = two_cases(&w);
    let s = set.to_str().unwrap();
    let (code, out) = eval(&w, &["fromless", "--set", s, "--runner", "replay"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("set-invalid")),
        "{out}"
    );
    let (code, out) = eval(
        &w,
        &[
            "fromless",
            "--set",
            s,
            "--runner",
            "mcp",
            "--from",
            "20261017T000000Z",
        ],
    );
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("set-invalid")),
        "{out}"
    );
    assert!(eval_dirs(&w, "fromless").is_empty());
}

#[test]
fn replay_of_a_run_that_recorded_no_answer_for_a_case_is_runner_failed() {
    let w = instance("unrecorded");
    let set = two_cases(&w);
    let (code, out) = eval(
        &w,
        &[
            "unrecorded",
            "--set",
            set.to_str().unwrap(),
            "--runner",
            "replay",
            "--from",
            "20200101T000000Z",
        ],
    );
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("runner-failed")),
        "{out}"
    );
}

#[test]
fn the_ask_runner_without_a_running_service_is_runner_failed() {
    let w = instance("serviceless");
    no_model(&w);
    let set = two_cases(&w);
    let (code, out) = eval(
        &w,
        &[
            "serviceless",
            "--set",
            set.to_str().unwrap(),
            "--runner",
            "ask",
        ],
    );
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("runner-failed")),
        "{out}"
    );
    assert!(
        out["detail"]["reason"]
            .as_str()
            .is_some_and(|r| r.contains("cortex ask serve")),
        "{out}"
    );
    assert!(w.lines("model-started.log").is_empty());
    let dirs = eval_dirs(&w, "serviceless");
    assert!(
        dirs.iter().all(|d| !d.join("summary.json").exists()),
        "{dirs:?}"
    );
}

#[test]
fn an_unknown_instance_is_no_such_instance() {
    let w = instance("known");
    let set = two_cases(&w);
    let (code, out) = eval(
        &w,
        &["unknown", "--set", set.to_str().unwrap(), "--runner", "mcp"],
    );
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("no-such-instance")),
        "{out}"
    );
    assert_eq!(out["detail"]["name"], "unknown");
    assert!(w.lines("eval-calls.log").is_empty());
}

#[test]
fn a_failed_model_call_is_runner_failed_and_keeps_the_results_before_it() {
    let w = instance("broken");
    let set = two_cases(&w);
    // The first call answers; the second answers an error.
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
    let (code, out) = eval(
        &w,
        &["broken", "--set", set.to_str().unwrap(), "--runner", "mcp"],
    );
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("runner-failed")),
        "{out}"
    );
    let dirs = eval_dirs(&w, "broken");
    assert_eq!(dirs.len(), 1, "{dirs:?}");
    assert_eq!(verdicts(&dirs[0]), vec![("widget".to_string(), true)]);
    assert!(!dirs[0].join("summary.json").exists());
}
