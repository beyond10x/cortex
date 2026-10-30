//! Adversary pass 2 on unit `shapes` of wave 20261030a, against the correction 19d7a75a (D24-D26):
//! the re-key refusal on `inputs` under an `{input.…}` key template refuses changes that re-key
//! nothing, the load check of a `key` passes a template whose `{input.…}` names no input field,
//! and `follow` compares a whole number written with a fraction through `f64`.

mod common;

use common::{ekr, executable, World};
use cortex_model::instance as m;
use serde_json::Value;

/// The connection part of a stand-in `connectors`; `messages.list` answers two messages for the
/// `channel` it is given, each with its own `ts`, the same two `ts` in every channel.
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

/// The spec of instance `name`: one `connectors` source, `chat`, reading `inputs` (a YAML flow
/// list) keyed by the template `key`.
fn spec_text(name: &str, inputs: &str, key: &str) -> String {
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
        inputs: {inputs}
        records: messages
        id: ts
        key: "{key}"
        text: ["{{text}}"]
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

/// Creates instance `name` reading `inputs` under `key` and runs it once, so its source has seen
/// documents; returns the spec's path.
fn created_and_run(w: &World, name: &str, inputs: &str, key: &str) -> std::path::PathBuf {
    let path = w.root.join(format!("{name}.yaml"));
    std::fs::write(&path, spec_text(name, inputs, key)).unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(
        (code, created["outcome"].as_str()),
        (0, Some("created")),
        "{created}"
    );
    let first = run(w, name);
    assert_eq!(first["detail"]["documents_applied"], 2, "{first}");
    path
}

/// D26 refuses every change of `inputs` while `key` names an `{input.…}`. Adding a channel
/// changes no seen document's key: C1's records render `C1:<ts>` before and after, which is the
/// point of keying by `{input.channel}` (`story:record-key-template`, two channels). The refusal
/// has no override, so an instance keyed by channel can never be given another channel.
#[test]
fn adding_a_channel_under_an_input_channel_key_is_not_a_rekey() {
    let w = world();
    let key = "{input.channel}:{ts}";
    let path = created_and_run(&w, "addchan", "[{channel: C1}]", key);
    std::fs::write(
        &path,
        spec_text("addchan", "[{channel: C1}, {channel: C2}]", key),
    )
    .unwrap();
    let (code, updated) = w.cortex(&[
        "update",
        "addchan",
        "--spec",
        path.to_str().unwrap(),
        "--no-units",
    ]);
    assert_eq!(
        (code, updated["outcome"].as_str()),
        (0, Some("updated")),
        "adding input C2 re-keys no document of C1: {updated}"
    );
    let next = run(&w, "addchan");
    assert_eq!(
        next["detail"]["documents_applied"], 2,
        "only C2's two messages are new: {next}"
    );
}

/// D26 compares the whole `inputs` list, so a change of an input field the key does not name
/// (`limit`) is refused as a re-key although every document keeps its key.
#[test]
fn changing_an_input_field_the_key_does_not_name_is_not_a_rekey() {
    let w = world();
    let key = "{input.channel}:{ts}";
    let path = created_and_run(&w, "limit", "[{channel: C1, limit: 10}]", key);
    std::fs::write(&path, spec_text("limit", "[{channel: C1, limit: 20}]", key)).unwrap();
    let (code, updated) = w.cortex(&[
        "update",
        "limit",
        "--spec",
        path.to_str().unwrap(),
        "--no-units",
    ]);
    assert_eq!(
        (code, updated["outcome"].as_str()),
        (0, Some("updated")),
        "`limit` is in no key: {updated}"
    );
}

/// D24 refuses a `key` that is no template by its grammar, so a typo in a placeholder's name is
/// not refused. A placeholder under `input` is read from the spec's own `inputs` (only their
/// `{since}`-style values are filled in at run time), so a name no input holds is known when the
/// spec is read, and every record of such an input is skipped on every run.
#[test]
fn a_key_naming_an_input_field_no_input_holds_is_refused_when_the_spec_is_read() {
    let w = world();
    let path = w.root.join("typo.yaml");
    std::fs::write(
        &path,
        spec_text("typo", "[{channel: C1}]", "{input.chanel}:{ts}"),
    )
    .unwrap();
    let (code, created) = status(
        &w,
        &["create", "--spec", path.to_str().unwrap(), "--no-units"],
    );
    let ran = if code == 0 {
        w.cortex(&["run", "typo/chat"]).1.to_string()
    } else {
        String::new()
    };
    assert_ne!(
        code, 0,
        "key `{{input.chanel}}` over inputs holding only `channel` was accepted: {created}\n\
         its first run: {ran}"
    );
    assert!(
        created.contains(".key:"),
        "the refusal names the key field: {created}"
    );
}

/// D25 compares two integers exactly but an integer with a number written with a fraction as
/// `f64`, so `9007199254740993` is cast to `9007199254740992.0`. `9007199254740992.0` is exact in
/// `f64` and is less than, and not equal to, `9007199254740993`; the docs say "numbers compare by
/// value, whole numbers exactly".
#[test]
fn a_whole_number_written_with_a_fraction_compares_exactly_with_a_large_integer() {
    const START: i64 = 1_793_318_400_000;
    let parent: Value = serde_json::from_str(r#"{"n": 9007199254740992.0}"#).unwrap();
    assert_eq!(parent["n"].as_f64(), Some(9_007_199_254_740_992.0));
    let equals = m::FollowRule::Equals(m::FollowEquals {
        field: "n".into(),
        value: cortex_model::json::Value::Number("9007199254740993".into()),
    });
    let at_least = m::FollowRule::AtLeast(m::FollowAtLeast {
        field: "n".into(),
        number: 9_007_199_254_740_993,
    });
    let followed = (
        cortex_cli::sources::follows(Some(&equals), &parent, START),
        cortex_cli::sources::follows(Some(&at_least), &parent, START),
    );
    assert_eq!(
        followed,
        (false, false),
        "(equals 9007199254740993, at_least 9007199254740993) on {parent}"
    );
}
