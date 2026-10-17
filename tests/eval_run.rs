//! `cortex eval run <instance> --set <set> --runner mcp|ask|replay` (`story:eval-run`): each case
//! of a `cortex.eval-case/1` set is answered by the runner and scored with no model, by the fact
//! strings the answer must contain and the node ids it must cite, under `eval/<UTC stamp>/`.

mod common;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use common::{executable, World};
use cortex_cli::ask::records::{Asked, Keeper, Records};
use cortex_model::ask as ma;
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
line="$*"
prev=
for a in "$@"; do
  [ "$prev" = --mcp-config ] && line="$line $(cat "$a")"
  prev="$a"
done
printf "%s\n" "$line" | tr "\n" " " >> "$R/eval-args.log"; echo >> "$R/eval-args.log"
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

// `story:ask-redaction`: under the instance's `redaction` policy the `mcp` runner's model reads
// the store only redacted, and the case is scored on the restored answer.

/// A stand-in `ekr` whose `mcp` speaks MCP over stdio: one tool, `search`, answering `text`.
fn mcp_reader(w: &World, text: &str) -> PathBuf {
    let path = w.root.join("reader/ekr");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    executable(
        &path,
        &format!(
            r#"[ "$1" = mcp ] || {{ echo "ekr 0.0.34"; exit 0; }}
while IFS= read -r line; do
  id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9][0-9]*\).*/\1/p')
  case "$line" in
    *'"method":"initialize"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"protocolVersion":"2025-06-18","capabilities":{{"tools":{{}}}},"serverInfo":{{"name":"ekr","version":"0"}}}}}}\n' "$id" ;;
    *'"method":"tools/list"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"tools":[{{"name":"search","description":"Search the store","inputSchema":{{"type":"object"}}}}]}}}}\n' "$id" ;;
    *'"method":"tools/call"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"content":[{{"type":"text","text":"{text}"}}]}}}}\n' "$id" ;;
  esac
done
"#
        ),
    );
    path
}

/// A stand-in `claude` that reads the store through the MCP server its `--mcp-config` names
/// over HTTP (`initialize`, then `tools/call search`), records its prompt and each response in
/// `<root>/model-saw.log`, and answers with both placeholders.
fn mcp_client(w: &World) {
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
prev=
for a in "$@"; do
  [ "$prev" = --mcp-config ] && conf=$(cat "$a")
  prev="$a"
done
url=$(printf '%s' "$conf" | sed -n 's/.*"url":"\([^"]*\)".*/\1/p')
auth=$(printf '%s' "$conf" | sed -n 's/.*"Authorization":"\([^"]*\)".*/\1/p')
cat >> "$R/model-saw.log"
post() {{ curl -s -X POST "$url" -H "authorization: $auth" -H 'content-type: application/json' -H 'accept: application/json, text/event-stream' -d "$1"; echo; }}
post '{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"2025-06-18","capabilities":{{}},"clientInfo":{{"name":"stand-in","version":"0"}}}}}}' >> "$R/model-saw.log"
post '{{"jsonrpc":"2.0","method":"notifications/initialized"}}' >> "$R/model-saw.log"
post '{{"jsonrpc":"2.0","id":2,"method":"tools/list"}}' >> "$R/model-saw.log"
post '{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"search","arguments":{{"query":"widget"}}}}}}' >> "$R/model-saw.log"
echo '{{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"usage":{{"input_tokens":100,"output_tokens":10}},"structured_output":{{"answer":"[Name-1] leads the widget team; write to [Email-1].","cited_nodes":["node-team"]}}}}'
"#,
            root = w.root.display()
        ),
    );
}

#[test]
fn the_mcp_runner_under_a_policy_shows_the_model_no_value_and_scores_the_restored_answer() {
    let w = instance("hidden");
    let reader = mcp_reader(
        &w,
        "Zorblat leads the widget team, write to help@acme.example.",
    );
    mcp_client(&w);
    let spec_path = w.home.join("instances/hidden/instance.yaml");
    let spec = std::fs::read_to_string(&spec_path).unwrap();
    let real = common::ekr().display().to_string();
    assert!(spec.contains(&real), "{spec}");
    let spec = spec.replace(&real, &reader.display().to_string())
        + "redaction:\n  classes: [Email]\n  rules: []\n  known_names: [Zorblat]\n";
    std::fs::write(&spec_path, spec).unwrap();
    let set = w.root.join("set.json");
    let mut team = case(
        "team",
        "What does Zorblat lead?",
        "Zorblat leads the widget team",
        "node-team",
    );
    team["expect_facts"] = json!(["Zorblat leads the widget team", "help@acme.example"]);
    std::fs::write(&set, json!({"cases": [team]}).to_string()).unwrap();

    let dir = mcp_run(&w, "hidden", &set);
    let saw = std::fs::read_to_string(w.root.join("model-saw.log")).unwrap();
    assert!(saw.contains("What does [Name-1] lead?"), "{saw}");
    assert!(
        saw.contains("[Name-1] leads the widget team, write to [Email-1]."),
        "{saw}"
    );
    assert!(!saw.contains("Zorblat"), "{saw}");
    assert!(!saw.contains("help@acme.example"), "{saw}");
    let result = &results(&dir)[0];
    assert_eq!(
        result["answer"], "Zorblat leads the widget team; write to help@acme.example.",
        "{result}"
    );
    assert_eq!(result["passed"], true, "{result}");
    assert_eq!(result["question"], "What does Zorblat lead?", "{result}");
}

// `cortex eval case add <instance> --set <set> --from-review <review id>`
// (`story:eval-cases-from-review`): a closed review item of the instance's answer records becomes
// one case of the set; an open one, an unknown one or one the set already holds is refused and the
// set file is left as it was.

/// The answer records of the instance `name`, as `cortex ask serve` keeps them.
fn keeper(w: &World, name: &str) -> Keeper {
    let path = w.home.join("instances").join(name).join("ask.sqlite");
    Keeper {
        store: Arc::new(Records::open(&path).expect("ask.sqlite opens")),
        instance: name.into(),
        retention_days: 90,
        slow_after_ms: None,
    }
}

/// Records a failed answer to `question` under `answer_id`, which opens its `failed` review item;
/// answers the item's id.
fn failed_answer(keeper: &Keeper, answer_id: &str, question: &str) -> String {
    let asked = Asked {
        answer_id: answer_id.into(),
        identity: "alice".into(),
        question: question.into(),
        tool_calls: vec![],
        latency_ms: 5,
        at_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64,
    };
    let failed = ma::AskOutcome::ModelFailed {
        error: ma::ModelFailed {
            reason: "down".into(),
        },
    };
    keeper.record(asked, &failed).expect("recorded");
    format!("{answer_id}:failed")
}

fn case_add(w: &World, name: &str, set: &Path, review: &str) -> (i32, Value) {
    w.cortex(&[
        "eval",
        "case",
        "add",
        name,
        "--set",
        set.to_str().unwrap(),
        "--from-review",
        review,
    ])
}

#[test]
fn a_closed_review_becomes_one_case_with_its_question_expectations_and_review_id() {
    let w = instance("cases");
    let records = keeper(&w, "cases");
    let review = failed_answer(&records, "a1", "Who develops the Widget engine?");
    let (closed, _) = records
        .close(
            &review,
            "the answer should name the maker",
            vec!["Example Labs".into()],
            vec!["node-widget".into()],
        )
        .expect("closed");
    assert!(
        matches!(closed, ma::CloseReviewOutcome::Closed { .. }),
        "{closed:?}"
    );
    let set = two_cases(&w);

    let (code, out) = case_add(&w, "cases", &set, &review);
    assert_eq!((code, out["outcome"].as_str()), (0, Some("added")), "{out}");
    assert_eq!(out["detail"]["review_id"], review.as_str());
    assert_eq!(out["detail"]["id"], review.as_str());
    assert_eq!(out["detail"]["cases"], 3);

    let doc: Value = serde_json::from_str(&std::fs::read_to_string(&set).unwrap()).unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 3, "{doc}");
    assert_eq!(cases[0]["id"], "widget");
    assert_eq!(cases[1]["id"], "gadget");
    assert_eq!(
        cases[2],
        json!({
            "format": "cortex.eval-case/1",
            "id": review,
            "question": "Who develops the Widget engine?",
            "expect_facts": ["Example Labs"],
            "expect_nodes": ["node-widget"],
            "review_id": review,
        })
    );
    // The set is still one `eval run` reads.
    let parsed = cortex_cli::eval::parse_set(&std::fs::read_to_string(&set).unwrap())
        .expect("the set parses");
    assert_eq!(
        parsed.cases[2].review_id.as_ref().map(|r| r.0.as_str()),
        Some(review.as_str())
    );
}

#[test]
fn an_open_review_is_refused_and_the_set_file_is_unchanged() {
    let w = instance("cases");
    let records = keeper(&w, "cases");
    let review = failed_answer(&records, "a1", "Who develops the Widget engine?");
    let set = two_cases(&w);
    let before = std::fs::read(&set).unwrap();

    let (code, out) = case_add(&w, "cases", &set, &review);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("review-open")),
        "{out}"
    );
    assert_eq!(out["detail"]["review_id"], review.as_str());
    assert_eq!(std::fs::read(&set).unwrap(), before);
}

#[test]
fn an_unknown_review_a_second_add_an_invalid_set_and_an_unknown_instance_change_no_set() {
    let w = instance("cases");
    let records = keeper(&w, "cases");
    let review = failed_answer(&records, "a1", "Who develops the Widget engine?");
    records
        .close(&review, "checked", vec!["Example Labs".into()], vec![])
        .expect("closed");
    let set = two_cases(&w);
    let before = std::fs::read(&set).unwrap();

    let (code, out) = case_add(&w, "cases", &set, "nothing:failed");
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("no-such-review")),
        "{out}"
    );
    assert_eq!(std::fs::read(&set).unwrap(), before);

    let (code, out) = case_add(&w, "unknown", &set, &review);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("no-such-instance")),
        "{out}"
    );
    assert_eq!(std::fs::read(&set).unwrap(), before);

    let invalid = w.root.join("invalid.json");
    std::fs::write(&invalid, r#"{"cases": [{"format": "other/1"}]}"#).unwrap();
    let (code, out) = case_add(&w, "cases", &invalid, &review);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("set-invalid")),
        "{out}"
    );
    assert_eq!(
        std::fs::read_to_string(&invalid).unwrap(),
        r#"{"cases": [{"format": "other/1"}]}"#
    );

    let (code, out) = case_add(&w, "cases", &set, &review);
    assert_eq!((code, out["outcome"].as_str()), (0, Some("added")), "{out}");
    let after = std::fs::read(&set).unwrap();
    let (code, out) = case_add(&w, "cases", &set, &review);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("case-exists")),
        "{out}"
    );
    assert_eq!(std::fs::read(&set).unwrap(), after);
}

#[test]
fn a_set_file_that_does_not_exist_is_created_with_the_one_case() {
    let w = instance("cases");
    let records = keeper(&w, "cases");
    let review = failed_answer(&records, "a1", "Who develops the Widget engine?");
    records
        .close(&review, "checked", vec![], vec!["node-widget".into()])
        .expect("closed");
    let set = w.root.join("new-set.json");

    let (code, out) = case_add(&w, "cases", &set, &review);
    assert_eq!((code, out["outcome"].as_str()), (0, Some("added")), "{out}");
    assert_eq!(out["detail"]["cases"], 1);
    let parsed = cortex_cli::eval::parse_set(&std::fs::read_to_string(&set).unwrap())
        .expect("the new set parses");
    assert_eq!(parsed.cases.len(), 1);
    assert_eq!(parsed.cases[0].question, "Who develops the Widget engine?");
    assert_eq!(parsed.cases[0].expect_nodes, ["node-widget"]);
}
