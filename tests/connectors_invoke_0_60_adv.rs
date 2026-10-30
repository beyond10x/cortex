//! Adversary cases for `story:connectors-invoke-0-60`: cortex against a Connectors older than
//! 0.60.0 that already lists `connector` and `instance` beside `adapter`.
//!
//! Connectors 0.54.0 to 0.59.3 list every connection with `connector` and `instance` beside
//! `adapter` (`connectors.cli.ConnectionSummary` in their `ess/domains/cli.yaml`), and none of
//! them knows `invoke <connector>/<instance> <operation>`: 0.59.3 refuses the command line cortex
//! now sends as the `ess-cli/1` parse refusal, `{"error":{"code":"cli_parse","data":{}},"ok":false}`
//! on standard error with exit 2. The stand-in here answers exactly that.

mod common;

use common::{executable, World, PAGES};
use serde_json::{json, Value};

/// The world with a Connectors 0.59.3 stand-in: `connections list` answers one ready `tavily`
/// connection named `tavily/main`, and every other command line is the parse refusal 0.59.3
/// writes for `invoke tavily/main websearch.search --connection conn_test --input-stdin --output
/// json`.
fn world() -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(
        &w.bin.join("connectors"),
        &format!(
            r#"R="{root}"
echo "$*" >> "$R/calls.log"
case "$*" in
  "connections list "*)
    printf '{{"ok":true,"result":{{"connections":[{{"adapter":"tavily","connector":"tavily","instance":"main","instance_id":"i1","connection":"conn_test","profile":"tavily.api-key","revision":"rev1","state":"ready"}}]}}}}' ;;
  *) echo '{{"error":{{"code":"cli_parse","data":{{}}}},"ok":false}}' >&2; exit 2 ;;
esac
"#
        ),
    );
    w
}

/// A Connectors older than 0.60.0 fails the run with a reason that says what is wrong: that
/// cortex needs Connectors 0.60.0 or newer (the CHANGELOG's promise), or at least the code the
/// CLI refused with. Measured: the reason is `connectors failed: tavily.websearch.search:
/// unknown at unknown`, because the version guard reads only an entry without `connector` and
/// `instance`, which no Connectors from 0.54.0 on writes, and `envelope` reads only
/// `error.data.code`, which a parse refusal leaves empty.
#[test]
#[ignore = "adversary: against Connectors 0.54.0-0.59.x the 0.60.0 guard never fires and the reason is `unknown at unknown`"]
fn adv_a_connectors_older_than_0_60_that_lists_connector_and_instance_is_named_in_the_reason() {
    let w = world();
    let spec = w.spec("old", "conn_test");
    let (code, out) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{out}");
    let pages: Value = serde_json::from_str(PAGES).unwrap();
    std::fs::write(
        w.root.join("answer.json"),
        json!({"ok": true, "result": {"connector": "tavily", "instance": "main",
            "operation": "websearch.search", "revision": "r", "result": pages}})
        .to_string(),
    )
    .unwrap();
    let (code, ran) = w.cortex(&["run", "old/news"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("fetch-failed")),
        "{ran}"
    );
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("0.60.0") || reason.contains("cli_parse"),
        "the reason names neither the Connectors version cortex needs nor the refusal: {reason}"
    );
}
