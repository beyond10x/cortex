//! `story:connectors-invoke-0-60`: a Connectors source runs against Connectors 0.60.0, which
//! removed the `operations` form of an invocation (now a parse refusal, exit 2) for
//! `invoke <connector>[/<instance>] <operation>`. The stand-in `connectors` here answers as 0.60.0
//! does: `invoke` answers an `InvokeResult` (`connector`, `instance`, `operation`, `revision`,
//! `result` as a JSON value), and the input is read from standard input only. It refuses every
//! `operations` command with exit 2: the invoke as 0.60.0 does, and `operations describe`, which
//! 0.60.0 still has, so that a call that reads a description first is seen.

mod common;

use common::{executable, World, PAGES};
use serde_json::{json, Value};

/// The world with a Connectors 0.60.0 stand-in. `connections list` answers one ready `tavily`
/// connection of connector `tavily`, instance `main`, or the entry in the file `entry` when it
/// exists. Every call's argv goes to `calls.log`; an `invoke` reads its input from standard input
/// into `inputs.log` and answers `answer.json`, refusing any argv that is not
/// `invoke tavily/main <op> --connection conn_test --input-stdin --output json`.
fn world() -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    let entry = json!({"adapter": "tavily", "connector": "tavily", "instance": "main",
        "instance_id": "i1", "connection": "conn_test", "profile": "tavily.api-key",
        "revision": "rev1", "state": "ready"});
    std::fs::write(w.root.join("default-entry"), entry.to_string()).unwrap();
    executable(
        &w.bin.join("connectors"),
        &format!(
            r#"R="{root}"
echo "$*" >> "$R/calls.log"
REFUSED='{{"ok":false,"error":{{"code":"failure","data":{{"code":"cli_parse","kind":"usage","stage":"arguments","next_action":"none"}}}}}}'
case "$*" in
  "connections list "*)
    E="$R/default-entry"; if [ -e "$R/entry" ]; then E="$R/entry"; fi
    printf '{{"ok":true,"result":{{"connections":[%s]}}}}' "$(cat "$E")" ;;
  "operations "*) echo "$*" >> "$R/refused.log"; echo "$REFUSED" >&2; exit 2 ;;
  "invoke tavily/main "*" --connection conn_test --input-stdin --output json")
    echo "$*" >> "$R/invocations.log"
    cat >> "$R/inputs.log"; echo >> "$R/inputs.log"
    cat "$R/answer.json" ;;
  *) echo "$*" >> "$R/refused.log"; echo "$REFUSED" >&2; exit 2 ;;
esac
"#
        ),
    );
    w
}

/// The 0.60.0 `InvokeResult` envelope around `result`.
fn invoke_result(result: Value) -> String {
    json!({"ok": true, "result": {"connector": "tavily", "instance": "main",
        "operation": "websearch.search", "revision": "r", "result": result}})
    .to_string()
}

fn created(w: &World, name: &str) {
    let spec = w.spec(name, "conn_test");
    let (code, out) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{out}");
    let pages: Value = serde_json::from_str(PAGES).unwrap();
    std::fs::write(w.root.join("answer.json"), invoke_result(pages)).unwrap();
}

/// Acceptance 1 and 3: the run ends `ran` against a `connectors` that refuses `operations …`, and
/// each call is one `invoke` spawn: no description is read before it.
#[test]
fn a_connectors_source_runs_against_the_0_60_invoke_form_with_one_spawn_per_call() {
    let w = world();
    created(&w, "a");
    let (code, ran) = w.cortex(&["run", "a/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
    assert!(
        w.lines("refused.log").is_empty(),
        "a refused command line was sent: {:?}",
        w.lines("refused.log")
    );
    let calls = w.lines("calls.log");
    let besides_readiness: Vec<&String> = calls
        .iter()
        .filter(|c| !c.starts_with("connections "))
        .collect();
    assert_eq!(
        besides_readiness,
        [&"invoke tavily/main websearch.search --connection conn_test --input-stdin --output json"
            .to_string()],
        "one invoke spawn for the one query, nothing else: {calls:?}"
    );
}

/// Acceptance 2: the input reaches the child on standard input; no field of it is in the argv.
#[test]
fn the_input_reaches_connectors_on_standard_input_and_never_in_its_argv() {
    let w = world();
    created(&w, "b");
    let (code, ran) = w.cortex(&["run", "b/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let inputs = w.lines("inputs.log");
    assert_eq!(inputs.len(), 1, "{inputs:?}");
    let input: Value = serde_json::from_str(&inputs[0]).expect("one JSON document on stdin");
    assert_eq!(input["query"], "widget engine", "{input}");
    let argv = w.lines("calls.log").join("\n");
    for field in ["widget engine", "query", "content", "{", "news"] {
        assert!(!argv.contains(field), "{field:?} in the argv: {argv}");
    }
}

/// The 2026-10-30 decision: a `connections list` entry without `connector` or `instance` (a
/// Connectors older than 0.60.0) is refused naming the entry and the version; nothing is invoked.
#[test]
fn an_entry_without_connector_and_instance_is_refused_naming_connectors_0_60() {
    let w = world();
    created(&w, "c");
    std::fs::write(
        w.root.join("entry"),
        json!({"adapter": "tavily", "connection": "conn_test", "profile": "tavily.api-key",
            "revision": "rev1", "state": "ready"})
        .to_string(),
    )
    .unwrap();
    let (code, ran) = w.cortex(&["run", "c/news"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("fetch-failed")),
        "{ran}"
    );
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    for part in ["conn_test", "Connectors 0.60.0 or newer"] {
        assert!(reason.contains(part), "{part:?} in {reason}");
    }
    assert!(w.lines("invocations.log").is_empty());
    assert!(w.lines("refused.log").is_empty());
}

/// Acceptance 5 (R3-10): a `result` that is no object is refused with its JSON type named.
#[test]
fn a_result_that_is_no_object_is_refused_naming_its_json_type() {
    let w = world();
    created(&w, "d");
    std::fs::write(
        w.root.join("answer.json"),
        invoke_result(json!([{"url": "https://example.org/a"}])),
    )
    .unwrap();
    let (code, ran) = w.cortex(&["run", "d/news"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("fetch-failed")),
        "{ran}"
    );
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("tavily.websearch.search") && reason.contains("an array"),
        "{reason}"
    );
    assert!(!reason.contains("no array at"), "{reason}");
}
