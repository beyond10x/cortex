//! Names, links and credentials are masked, and a run refuses what survives
//! (`story:redaction-names-and-gate`).
//!
//! Under the operator's decision, personal data stays out of the model, not out of the store: a
//! known name, a rare name and a link reach the stand-in model's prompt only as placeholders, and
//! the placeholders in its answer are restored before the store is written. A credential is
//! masked irreversibly: it is neither sent nor stored. A class `refuse_if_left` names that is
//! still detected in what the model would be shown fails the run before any model call or store
//! write.

mod common;

use std::path::Path;
use std::process::Command;

use common::{answer, ekr, executable, World, PAGES};
use serde_json::Value;

const SERVE: &str = "serve: {view_port: 18999}\n";

/// The stand-in `claude` wrapped: it copies each prompt to `prompt.log`, then answers as
/// `tests/common`'s stand-in does, with the fact objects in the file `inject-facts` (`@ID@`
/// replaced by the first evidence id of the prompt), the entity objects in `inject-entities` and
/// the node types in `inject-types` put first in their lists.
const WRAPPER: &str = r#"R="@ROOT@"
cat > "$R/prompt.cur"
cat "$R/prompt.cur" >> "$R/prompt.log"
echo "=== end of prompt ===" >> "$R/prompt.log"
id=$(grep -oE 'evidence id [0-9a-f-]{36}' "$R/prompt.cur" | head -1 | cut -d' ' -f3)
facts=""
ents=""
types=""
if [ -e "$R/inject-facts" ]; then facts=$(sed "s/@ID@/$id/g" "$R/inject-facts"); fi
if [ -e "$R/inject-entities" ]; then ents=$(cat "$R/inject-entities"); fi
if [ -e "$R/inject-types" ]; then types=$(cat "$R/inject-types"); fi
"@INNER@" "$@" < "$R/prompt.cur" | awk -v f="$facts" -v e="$ents" -v t="$types" '{
  i = index($0, "\"facts\":["); if (i && f != "") $0 = substr($0, 1, i + 8) f substr($0, i + 9)
  j = index($0, "\"entities\":["); if (j && e != "") $0 = substr($0, 1, j + 11) e substr($0, j + 12)
  k = index($0, "\"node_types\":["); if (k && t != "") $0 = substr($0, 1, k + 13) t substr($0, k + 14)
  print }'
"#;

/// A world whose stand-in search answers `pages`, and an instance `r` created from a spec with
/// `policy` as its `redaction` block.
fn world(pages: &str, policy: &str) -> World {
    let w = World::new();
    std::fs::write(w.root.join("answer.json"), answer(pages)).unwrap();
    let claude = w.bin.join("claude");
    let inner = w.bin.join("claude-inner");
    std::fs::rename(&claude, &inner).unwrap();
    executable(
        &claude,
        &WRAPPER
            .replace("@ROOT@", &w.root.display().to_string())
            .replace("@INNER@", &inner.display().to_string()),
    );
    let spec = w.spec("r", "conn_test");
    let text = std::fs::read_to_string(&spec).unwrap();
    assert!(text.contains(SERVE), "{text}");
    std::fs::write(&spec, text.replacen(SERVE, &format!("{SERVE}{policy}"), 1)).unwrap();
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    w
}

/// `PAGES` with `planted` added to the first page's text.
fn planted(planted: &str) -> String {
    let pages = PAGES.replacen(
        "Its website is example.org.",
        &format!("Its website is example.org. {planted}"),
        1,
    );
    assert_ne!(pages, PAGES);
    pages
}

fn prompts(w: &World) -> String {
    std::fs::read_to_string(w.root.join("prompt.log")).unwrap_or_default()
}

fn last_log_line(w: &World) -> Value {
    let log = w.home.join("instances/r/cortex.log");
    serde_json::from_str(
        std::fs::read_to_string(log)
            .unwrap()
            .lines()
            .last()
            .unwrap(),
    )
    .unwrap()
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|w| w == needle.as_bytes())
}

/// Every file under `dir`, by path, with its bytes.
fn files_under(dir: &Path) -> Vec<(String, Vec<u8>)> {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .map(|e| e.unwrap())
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            (
                e.path().display().to_string(),
                std::fs::read(e.path()).unwrap(),
            )
        })
        .collect()
}

/// The store's facts, read back through `ekr sample`.
fn stored_facts(w: &World) -> Value {
    let dir = w.home.join("instances/r");
    let out = Command::new(ekr())
        .args(["sample", "--seed", "1", "--size", "1000"])
        .env("EKR_HOST", dir.join("host.json"))
        .env("EKR_BACKEND", "sqlite")
        .env("EKR_STORE", dir.join("store.sqlite"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

fn has_string(v: &Value, s: &str) -> bool {
    match v {
        Value::String(x) => x == s,
        Value::Array(a) => a.iter().any(|x| has_string(x, s)),
        Value::Object(o) => o.values().any(|x| has_string(x, s)),
        _ => false,
    }
}

/// A fact whose value the model wrote as `value`, citing the prompt's first evidence id.
fn website_fact(value: &str) -> String {
    format!(
        r#"{{"!Property":{{"subject":{{"node_type":"Organization","aliases":["Example Labs"]}},"property":"website","value":{{"value_kind":"String","value":"{value}"}},"evidence":["@ID@"]}}}},"#
    )
}

/// The story's acceptance: a phone number with person context that masking does not remove,
/// under `refuse_if_left: [Phone]`, fails the run with the class named, makes no model call, and
/// leaves `ekr head` at the revision from before the run.
#[test]
fn a_phone_number_left_after_masking_fails_the_run_before_the_model_and_the_store() {
    let policy = "redaction:\n  classes: [Phone]\n  rules: []\n  refuse_if_left: [Phone]\n";
    let w = world(
        &planted("Jane's mobile is 7946 0958, call her about the licence."),
        policy,
    );
    let before = w.head("r");
    let (code, ran) = w.cortex(&["run", "r/news"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("extraction-failed")),
        "{ran}"
    );
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("Phone"), "the class is named: {ran}");
    assert!(!reason.contains("7946"), "the value is not: {ran}");
    assert!(
        w.lines("claude-calls.log").is_empty(),
        "no model call was made"
    );
    assert!(prompts(&w).is_empty(), "no prompt was sent");
    assert_eq!(w.head("r"), before, "nothing was written to the store");
}

/// The gate passes a batch whose phone number masking replaced; the run applies as before.
#[test]
fn a_run_with_nothing_left_after_masking_is_applied() {
    let policy = "redaction:\n  classes: [Phone]\n  rules: []\n  refuse_if_left: [Phone]\n";
    let w = world(&planted("Call +44 20 7946 0958 for the licence."), policy);
    let before = w.head("r");
    let (code, ran) = w.cortex(&["run", "r/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
    assert!(w.head("r") > before);
    let p = prompts(&w);
    assert!(p.contains("Call [Phone-1] for the licence."), "{p}");
}

/// A known name, rare names and a link never reach the model; the answer's placeholders are
/// restored, so the store holds the originals and the evidence is the original text.
#[test]
fn names_and_links_never_reach_the_model_and_are_restored_in_the_store() {
    let policy = concat!(
        "redaction:\n  classes: [Url, RareName]\n  rules: []\n",
        "  known_names: [Jane Doe]\n  rare_limit: 1\n",
        "  refuse_if_left: [Url, RareName]\n",
    );
    let text =
        "Jane Doe wrote it. Rowan Pell reviewed it; see https://intranet.example.org/people/rpell.";
    let w = world(&planted(text), policy);
    std::fs::write(
        w.root.join("inject-facts"),
        website_fact("[Name-1] at [Url-2]"),
    )
    .unwrap();
    let (code, ran) = w.cortex(&["run", "r/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");

    let p = prompts(&w);
    assert!(
        p.contains("Example Labs develops the Widget engine."),
        "{p}"
    );
    for planted in [
        "Jane",
        "Jane Doe",
        "Rowan",
        "Pell",
        "intranet.example.org",
        "https://example.org/a",
    ] {
        assert!(!p.contains(planted), "{planted} reached the model: {p}");
    }
    assert!(
        p.contains("[Name-1] wrote it. [Name-2] [Name-3] reviewed it; see [Url-2]."),
        "{p}"
    );
    assert!(p.contains("Source: [Url-1]\n"), "{p}");

    let facts = stored_facts(&w);
    assert!(
        has_string(
            &facts,
            "Jane Doe at https://intranet.example.org/people/rpell"
        ),
        "the restored fact is stored: {facts}"
    );
    let instance = w.home.join("instances/r");
    let store = std::fs::read(instance.join("store.sqlite")).unwrap();
    assert!(contains(&store, text), "the evidence is the original text");
    assert!(
        contains(&store, "https://example.org/a"),
        "the evidence keeps its identity"
    );
    for placeholder in ["[Name-1]", "[Url-1]"] {
        assert!(!contains(&store, placeholder), "{placeholder} is stored");
    }
    for (path, bytes) in files_under(&instance) {
        assert!(
            !(contains(&bytes, "[Name-1]") && contains(&bytes, "Jane Doe")),
            "{path} holds the mapping"
        );
    }

    let line = last_log_line(&w);
    assert_eq!(
        line["redacted"],
        serde_json::json!({"RareName": 2, "Url": 3, "known_names": 1}),
        "{line}"
    );
    assert_eq!(line["unrestored"], 0, "{line}");
}

/// A credential of a shape only the `Credential` class covers is neither sent nor stored, and
/// nothing in the answer is restored to it.
#[test]
fn a_credential_is_neither_sent_nor_stored() {
    let token = format!("{}{}", "aB3dE5gH7jK9", "mN1pQ3sT5v");
    let policy = "redaction:\n  classes: [Credential]\n  rules: []\n";
    let w = world(
        &planted(&format!(
            "Send Authorization: Bearer {token} with each call."
        )),
        policy,
    );
    let (code, ran) = w.cortex(&["run", "r/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let p = prompts(&w);
    assert!(p.contains("Example Labs develops"), "{p}");
    assert!(!p.contains(&token), "the credential reached the model: {p}");
    assert!(p.contains("Authorization: Bearer [masked:"), "{p}");
    for (path, bytes) in files_under(&w.home.join("instances/r")) {
        assert!(!contains(&bytes, &token), "{path} holds the credential");
    }
    let store = std::fs::read(w.home.join("instances/r/store.sqlite")).unwrap();
    assert!(
        contains(&store, "Send Authorization: Bearer [masked:"),
        "the stored payload is masked"
    );
    assert_eq!(last_log_line(&w)["redacted"]["Credential"], 1);
}

/// Every prompt the stand-in model received, in order.
fn prompt_list(w: &World) -> Vec<String> {
    prompts(w)
        .split("=== end of prompt ===\n")
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect()
}

/// Run 1 hides the rare name `Rowan Pell` and the model names the person by its placeholders; the
/// restored name is kept among the known entities. Run 2's page mentions the person once. Rarity
/// counts only the batch's document texts, so the known-entity line is no second sighting: the
/// name is hidden in the known entities and in the text.
#[test]
fn a_person_the_store_already_knows_is_not_shown_to_the_model_by_name() {
    let policy =
        "redaction:\n  classes: [RareName]\n  rules: []\n  rare_limit: 1\n  refuse_if_left: [RareName]\n";
    let w = world(&planted("Rowan Pell reviewed it."), policy);
    std::fs::write(
        w.root.join("inject-types"),
        r#"{"name":"Person","parents":[],"abstract_type":false,"properties":[]},"#,
    )
    .unwrap();
    std::fs::write(
        w.root.join("inject-entities"),
        r#"{"node_type":"Person","aliases":["[Name-1] [Name-2]"]},"#,
    )
    .unwrap();
    let (code, ran) = w.cortex(&["run", "r/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let first = prompt_list(&w);
    assert_eq!(first.len(), 1, "{first:?}");
    assert!(
        first[0].contains("[Name-1] [Name-2] reviewed it.") && !first[0].contains("Rowan"),
        "run 1 hides the name: {}",
        first[0]
    );
    let entities =
        std::fs::read_to_string(w.home.join("instances/r/state/entities.json")).unwrap_or_default();
    assert!(
        entities.contains("Rowan Pell"),
        "run 1 keeps the restored name among the known entities: {entities}"
    );

    std::fs::remove_file(w.root.join("inject-entities")).unwrap();
    std::fs::write(
        w.root.join("answer.json"),
        answer(&planted("Rowan Pell called again.")),
    )
    .unwrap();
    let (code, ran) = w.cortex(&["run", "r/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let all = prompt_list(&w);
    assert_eq!(all.len(), 2, "run 2 sent a prompt: {ran}");
    assert!(
        !all[1].contains("Rowan"),
        "a person's name reached the model in run 2: {}",
        all[1]
    );
}
