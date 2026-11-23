//! `story:quality-missing-verdicts`: a fact the judge's answer gives no verdict is asked once more
//! in a later judge call; one the second answer gives none either is `unjudged`, outside the rate,
//! whose denominator `judged` names. Each judge call keeps its request and the answer before any
//! placeholder is restored under `quality/<stamp>/batch-<n>/`. The judge is a stand-in; the facts
//! are invented products.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use common::{executable, World};
use serde_json::{json, Value};

/// An extraction stand-in: `n` products, each with its own `version`, all citing the first
/// evidence id of the batch.
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

/// A judge stand-in. Call `n` (from 0) keeps its request as `judge-prompt-<n>.txt` and its
/// structured answer as `judge-answer-<n>.json`. Call 0 gives no verdict for the first `omit`
/// facts it is shown and `first` for each other; every later call gives `later` for each fact, or
/// no verdict at all when `later` is empty. Each reason names the call and the first email
/// placeholder the request holds. Every call costs 0.02 USD.
fn judge(w: &World, omit: usize, first: &str, later: &str) {
    executable(
        &w.bin.join("claude"),
        &format!(
            r#"R="{root}"
n=$(cat "$R/judge-calls.log" 2>/dev/null | wc -l)
echo call >> "$R/judge-calls.log"
prompt=$(cat)
printf '%s\n' "$prompt" > "$R/judge-prompt-$n.txt"
mark=$(printf '%s\n' "$prompt" | grep -oE '\[Email-[0-9]+\]' | head -1)
verdicts=""
k=0
for id in $(printf '%s\n' "$prompt" | grep -oE '^=== Fact [0-9a-f-]{{36}}' | cut -d' ' -f3); do
  if [ "$n" -eq 0 ]; then
    if [ $k -ge {omit} ]; then v={first}; else v=; fi
  else
    v={later}
  fi
  if [ -n "$v" ]; then
    verdicts="$verdicts{{\"fact\":\"$id\",\"verdict\":\"$v\",\"reason\":\"call $n says $v $mark\"}},"
  fi
  k=$((k+1))
done
verdicts=${{verdicts%,}}
out="{{\"verdicts\":[$verdicts]}}"
printf '%s' "$out" > "$R/judge-answer-$n.json"
echo "{{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"total_cost_usd\":0.02,\"structured_output\":$out}}"
"#,
            root = w.root.display()
        ),
    );
}

/// An instance `name` whose store holds `products` product facts besides the web pages' facts.
/// `extra` is appended to its spec.
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
    w
}

/// The one measurement directory of the instance.
fn measurement(w: &World, name: &str) -> PathBuf {
    let dir = w.home.join("instances").join(name).join("quality");
    let dirs: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(dirs.len(), 1, "{dirs:?}");
    dirs.into_iter().next().unwrap()
}

fn verdicts(dir: &Path) -> Vec<Value> {
    std::fs::read_to_string(dir.join("verdicts.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap())
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The fact ids a judge request shows, in its order.
fn facts_shown(prompt: &str) -> Vec<String> {
    prompt
        .lines()
        .filter_map(|l| l.strip_prefix("=== Fact "))
        .filter_map(|l| l.strip_suffix(" ==="))
        .map(str::to_string)
        .collect()
}

/// Each judge call `n` left `batch-<n>/prompt.txt`, the request the judge read, and
/// `batch-<n>/answer.json`, the structured answer it gave.
fn assert_calls_kept(w: &World, dir: &Path, calls: usize) {
    for n in 0..calls {
        let batch = dir.join(format!("batch-{n}"));
        let kept = std::fs::read_to_string(batch.join("prompt.txt"))
            .unwrap_or_else(|e| panic!("{}: {e}", batch.display()));
        let read = std::fs::read_to_string(w.root.join(format!("judge-prompt-{n}.txt"))).unwrap();
        assert_eq!(kept.trim_end(), read.trim_end(), "call {n}");
        assert_eq!(
            read_json(&batch.join("answer.json")),
            read_json(&w.root.join(format!("judge-answer-{n}.json"))),
            "call {n}"
        );
    }
    assert!(!dir.join(format!("batch-{calls}")).exists());
}

/// The acceptance's first case: the judge gives no verdict for 3 of 6 facts in its first call and
/// `no` for each of them in the second. The rate is over all 6 facts (3 `yes` of 6), nothing is
/// `unjudged`, and the second call was shown exactly the 3 facts the first answer left out.
#[test]
fn a_fact_without_a_verdict_is_asked_again_and_the_rate_is_over_every_fact() {
    let w = instance("again", 30, "");
    judge(&w, 3, "yes", "no");
    let (code, out) = w.cortex(&["quality", "again", "--sample", "6"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("measured")),
        "{out}"
    );
    assert_eq!(w.lines("judge-calls.log").len(), 2, "{out}");
    let first = facts_shown(&std::fs::read_to_string(w.root.join("judge-prompt-0.txt")).unwrap());
    let second = facts_shown(&std::fs::read_to_string(w.root.join("judge-prompt-1.txt")).unwrap());
    assert_eq!(first.len(), 6);
    assert_eq!(second, first[..3], "the later call asks the left-out facts");

    let d = &out["detail"];
    assert_eq!(
        [&d["judged"], &d["passed"], &d["unclear"], &d["unjudged"]],
        [&json!(6), &json!(3), &json!(0), &json!(0)],
        "{out}"
    );
    assert_eq!(d["rate"], json!(0.5), "{out}");
    assert_eq!(d["cost_usd"], "0.0400", "{out}");

    let dir = measurement(&w, "again");
    let report = read_json(&dir.join("fact-quality.json"));
    assert_eq!(
        [&report["judged"], &report["passed"], &report["rate"]],
        [&json!(6), &json!(3), &json!(0.5)],
        "{report}"
    );
    let lines = verdicts(&dir);
    let facts: BTreeSet<&str> = lines.iter().filter_map(|v| v["fact"].as_str()).collect();
    assert_eq!(
        (lines.len(), facts.len()),
        (6, 6),
        "every fact once: {lines:?}"
    );
    for v in &lines {
        let fact = v["fact"].as_str().unwrap();
        let (verdict, call) = if second.iter().any(|f| f == fact) {
            ("no", "call 1")
        } else {
            ("yes", "call 0")
        };
        assert_eq!(v["verdict"], verdict, "{v}");
        assert!(v["reason"].as_str().unwrap().starts_with(call), "{v}");
    }
    assert_calls_kept(&w, &dir, 2);
}

/// The acceptance's second case: the judge gives no verdict for 2 of 5 facts in its first call
/// and none in the second. Those 2 are `unjudged`: written so in `verdicts.jsonl`, not sent to
/// `ekr fact-quality`, and outside the rate, which is 1 `yes` of the 3 judged.
#[test]
fn a_fact_never_given_a_verdict_is_unjudged_and_outside_the_rate() {
    let w = instance("never", 30, "");
    // Call 0: no verdict for facts 1 and 2, `yes` for fact 3, then `no`: the stand-in answers by
    // position, so fact 3 is given `yes` through a third value of `first`.
    judge(&w, 2, "\"$([ $k -eq 2 ] && echo yes || echo no)\"", "");
    let (code, out) = w.cortex(&["quality", "never", "--sample", "5"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("measured")),
        "{out}"
    );
    assert_eq!(w.lines("judge-calls.log").len(), 2, "{out}");
    let first = facts_shown(&std::fs::read_to_string(w.root.join("judge-prompt-0.txt")).unwrap());
    let second = facts_shown(&std::fs::read_to_string(w.root.join("judge-prompt-1.txt")).unwrap());
    assert_eq!(second, first[..2], "the later call asks the left-out facts");

    let d = &out["detail"];
    assert_eq!(
        [&d["judged"], &d["passed"], &d["unclear"], &d["unjudged"]],
        [&json!(3), &json!(1), &json!(0), &json!(2)],
        "{out}"
    );
    assert_eq!(d["rate"].to_string(), "0.3333333333333333", "{out}");

    let dir = measurement(&w, "never");
    let report = read_json(&dir.join("fact-quality.json"));
    assert_eq!(
        [&report["judged"], &report["passed"], &report["failed"]],
        [&json!(3), &json!(1), &json!(2)],
        "{report}"
    );
    assert_eq!(
        d["rate"].to_string(),
        report["rate"].to_string(),
        "{report}"
    );
    let lines = verdicts(&dir);
    assert_eq!(lines.len(), 5, "{lines:?}");
    let unjudged: Vec<&str> = lines
        .iter()
        .filter(|v| v["verdict"] == "unjudged")
        .filter_map(|v| v["fact"].as_str())
        .collect();
    assert_eq!(unjudged, second, "{lines:?}");
    for v in lines.iter().filter(|v| v["verdict"] == "unjudged") {
        assert_eq!(v["format"], "cortex.quality-verdict/1", "{v}");
        assert_eq!(
            v["reason"], "the judge gave no verdict for this fact in two calls",
            "{v}"
        );
    }
    assert_calls_kept(&w, &dir, 2);
}

/// A judge that never gives a verdict: 25 facts are asked in two first calls (20 and 5) and two
/// later ones (20 and 5), every fact is `unjudged`, nothing is judged and the rate is null with
/// the interval 0 to 1, as for a store with no fact to draw.
#[test]
fn every_fact_unjudged_is_a_null_rate_and_is_asked_again_in_batches_of_twenty() {
    let w = instance("silent", 30, "");
    judge(&w, 20, "", "");
    let (code, out) = w.cortex(&["quality", "silent", "--sample", "25"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("measured")),
        "{out}"
    );
    assert_eq!(w.lines("judge-calls.log").len(), 4, "{out}");
    let shown: Vec<usize> = (0..4)
        .map(|n| {
            facts_shown(
                &std::fs::read_to_string(w.root.join(format!("judge-prompt-{n}.txt"))).unwrap(),
            )
            .len()
        })
        .collect();
    assert_eq!(shown, [20, 5, 20, 5], "{out}");
    let d = &out["detail"];
    assert_eq!(
        [&d["judged"], &d["passed"], &d["unjudged"]],
        [&json!(0), &json!(0), &json!(25)],
        "{out}"
    );
    assert_eq!(d.get("rate"), Some(&Value::Null), "{out}");
    assert_eq!(
        (d["lower"].as_f64(), d["upper"].as_f64()),
        (Some(0.0), Some(1.0)),
        "{out}"
    );
    let dir = measurement(&w, "silent");
    let lines = verdicts(&dir);
    assert_eq!(lines.len(), 25);
    assert!(
        lines.iter().all(|v| v["verdict"] == "unjudged"),
        "{lines:?}"
    );
    assert_calls_kept(&w, &dir, 4);
}

/// Under a redaction policy the kept request and answer hold the placeholders the judge was
/// shown and wrote, never the value; `verdicts.jsonl` holds the restored reason.
#[test]
fn the_kept_request_and_answer_hold_placeholders_under_a_redaction_policy() {
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
    judge(&w, 1, "yes", "yes");
    // Every fact of the store, so the first call is shown the page that holds the address.
    let (code, out) = w.cortex(&["quality", "private", "--sample", "20"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (0, Some("measured")),
        "{out}"
    );
    assert_eq!(w.lines("judge-calls.log").len(), 2, "{out}");
    assert_eq!(out["detail"]["unjudged"], 0, "{out}");
    let dir = measurement(&w, "private");
    assert_calls_kept(&w, &dir, 2);
    for n in 0..2 {
        for file in ["prompt.txt", "answer.json"] {
            let text = std::fs::read_to_string(dir.join(format!("batch-{n}")).join(file)).unwrap();
            assert!(!text.contains(&email), "batch-{n}/{file}: {text}");
            if n == 0 {
                assert!(text.contains("[Email-1]"), "batch-{n}/{file}: {text}");
            }
        }
    }
    let lines = verdicts(&dir);
    let first: Vec<&str> = lines
        .iter()
        .filter_map(|v| v["reason"].as_str())
        .filter(|r| r.starts_with("call 0"))
        .collect();
    assert!(!first.is_empty(), "{lines:?}");
    assert!(
        first.iter().all(|r| r.ends_with(&email)),
        "the reasons are restored: {lines:?}"
    );
}
