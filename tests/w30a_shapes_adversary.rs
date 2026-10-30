//! Adversary pass 1 on unit `shapes` of wave 20261030a (`story:record-key-template`,
//! `story:structured-children-follow-rule`, `story:child-text-and-filter`): a `connectors`
//! source's `key` template that is no template, the re-key refusal it slips past, and a `follow`
//! rule's `equals` on numbers.

mod common;

use common::{ekr, executable, World};
use cortex_model::instance as m;
use serde_json::Value;

/// The connection part of a stand-in `connectors`; `messages.list` answers two messages for the
/// `channel` it is given, each with its own `ts`.
const CONNECTORS: &str = r#"R="@ROOT@"
IN=""; if [ "$1" = invoke ]; then IN=$(cat); fi; A="$* $IN"
case "$A" in
  *"connections list"*) echo '{"ok":true,"result":{"connections":[{"adapter":"tracker","connector":"tracker","instance":"main","connection":"conn_test","state":"ready","revision":"rev1"}]}}' ;;
  *"connections revalidate"*) echo '{"ok":true,"result":{}}' ;;
  "invoke "*" messages.list "*)
    echo "$A" >> "$R/messages.log"
    C=$(printf '%s' "$A" | sed -n 's/.*"channel":"\([^"]*\)".*/\1/p')
    printf '{"ok":true,"result":{"connector":"tracker","instance":"main","operation":"messages.list","revision":"r","result":{"messages":[{"ts":"1767312000.000100","text":"deploy done in %s"},{"ts":"1767312060.000200","text":"thanks from %s"}]}}}\n' "$C" "$C" ;;
  *) echo '{"ok":false}'; exit 2 ;;
esac
"#;

fn world() -> World {
    let w = World::new();
    executable(
        &w.bin.join("connectors"),
        &CONNECTORS.replace("@ROOT@", &w.root.display().to_string()),
    );
    w
}

/// The spec of instance `name`: one `connectors` source, `chat`, reading channel C1 keyed by `ts`,
/// with `key_line` (a whole `key: …` line, or nothing) after its `id`.
fn spec_text(name: &str, key_line: &str) -> String {
    format!(
        r#"format: cortex.instance/1
name: {name}
description: Test brain.
ekr: {{version: "0.0.30", bin: "{ekr}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
  - name: chat
    schedule: daily
    settings:
      kind: connectors
      value:
        adapter: tracker
        connection: conn_test
        operation: messages.list
        inputs: [{{channel: C1}}]
        records: messages
        id: ts
{key_line}        text: ["{{text}}"]
    policy: {{refresh_after_days: 3, change: ContentHash, max_documents_per_run: 50, max_chars_per_document: 5000}}
serve: {{view_port: 18997}}
"#,
        ekr = ekr().display()
    )
}

/// `cortex` with the world's variables, its exit status and its output, without the shared
/// helper's assertion that the status is not 2: a spec refused when it is read exits 2.
fn status(w: &World, args: &[&str]) -> (i32, String) {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(args)
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .env("ANTHROPIC_API_KEY", "must-not-reach-claude")
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

fn run(w: &World, name: &str) -> Value {
    let (code, ran) = w.cortex(&["run", &format!("{name}/chat")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    ran
}

/// Adding `key: ""` to a source that has seen documents re-keys it as surely as any other
/// template: no record renders a key from an empty template, so every record the source reads is
/// skipped from then on and none of its seen documents is refreshed again. `keyed_by` compares
/// `c.key.clone().unwrap_or_default()`, so an absent `key` and `key: ""` are the same field and
/// `cortex update` lets the change through. Either `update` refuses it (`rekey-refused`), or the
/// spec is refused as one whose `key` is no template.
#[test]
fn adding_an_empty_key_to_a_source_with_seen_documents_is_refused() {
    let w = world();
    let path = w.root.join("empty.yaml");
    std::fs::write(&path, spec_text("empty", "")).unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let first = run(&w, "empty");
    assert_eq!(first["detail"]["documents_applied"], 2, "{first}");

    std::fs::write(&path, spec_text("empty", "        key: \"\"\n")).unwrap();
    let (code, updated) = status(
        &w,
        &[
            "update",
            "empty",
            "--spec",
            path.to_str().unwrap(),
            "--no-units",
        ],
    );
    let after = if code == 0 {
        w.cortex(&["run", "empty/chat"]).1
    } else {
        Value::Null
    };
    assert_ne!(
        code, 0,
        "update accepted `key: \"\"` on a source with seen documents: {updated}\nthe next run: {after}"
    );
    assert!(
        updated.contains(".key:"),
        "the refusal names the key field: {updated}"
    );
}

/// A `key` template is rendered by `structured::render_template`, whose grammar refuses an
/// unclosed `{`, a `}` that closes nothing and a template without a placeholder, and an alias
/// template with any of those is refused when the spec is read (`spec::alias_template_refusal`).
/// A `key` is not checked at all: the spec is created, and every run then skips every record as
/// one the template "renders nothing for", blaming the records for a typo in the spec.
#[test]
fn a_key_that_is_no_template_is_refused_when_the_spec_is_read() {
    for (n, key) in ["{input.channel}:{ts", "{ts}}", "channel"]
        .iter()
        .enumerate()
    {
        let w = world();
        let name = format!("typo{n}");
        let path = w.root.join(format!("{name}.yaml"));
        std::fs::write(
            &path,
            spec_text(&name, &format!("        key: \"{key}\"\n")),
        )
        .unwrap();
        let (code, created) = status(
            &w,
            &["create", "--spec", path.to_str().unwrap(), "--no-units"],
        );
        let ran = if code == 0 {
            w.cortex(&["run", &format!("{name}/chat")]).1
        } else {
            Value::Null
        };
        assert_ne!(
            code, 0,
            "key {key:?} was accepted: {created}\nits first run: {ran}"
        );
        assert!(
            created.contains(".key:"),
            "the refusal of key {key:?} names the key field: {created}"
        );
    }
}

/// `equals`: "numbers compare by value" (`cortex.instance.FollowRule`, `spec-file.md`).
/// `follows` compares two numbers as `f64`, so two integers above 2^53 that differ compare equal:
/// a rule meant for one id follows a parent with another.
#[test]
fn an_equals_rule_on_a_large_integer_does_not_follow_a_parent_with_a_neighbouring_one() {
    const START: i64 = 1_793_318_400_000;
    let rule = m::FollowRule::Equals(m::FollowEquals {
        field: "owner_id".into(),
        value: cortex_model::json::Value::Number("9007199254740993".into()),
    });
    let same: Value = serde_json::from_str(r#"{"owner_id": 9007199254740993}"#).unwrap();
    assert!(cortex_cli::sources::follows(Some(&rule), &same, START));
    let other: Value = serde_json::from_str(r#"{"owner_id": 9007199254740992}"#).unwrap();
    assert!(
        !cortex_cli::sources::follows(Some(&rule), &other, START),
        "equals 9007199254740993 followed {other}"
    );
}
