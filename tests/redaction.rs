//! Personal data is pseudonymised before the model sees a document (`story:redaction-before-model`):
//! a planted email address, phone number and card number never reach the stand-in model's
//! prompt, which carries `[Email-1]`-style placeholders instead; the placeholders in its answer are
//! restored before the store is written, so the store and the evidence hold the originals; and the
//! run's log line counts what was replaced in the prompts sent.
//!
//! The adversary's end-to-end cases of passes 1 and 2 are folded in here. Under the operator's
//! design change, the two that asserted the store holds no personal data now assert the opposite:
//! the store holds the original and the prompt does not. A rule with a `replacement` is the
//! exception: it is irreversible, so its matches are neither sent nor stored.

mod common;

use std::path::Path;
use std::process::Command;

use common::{answer, ekr, executable, World, PAGES};
use serde_json::Value;

const EMAIL: &str = "jane.doe@example.com";
const PHONE: &str = "+44 20 7946 0958";
const CARD: &str = "4111 1111 1111 1111";
const POLICY: &str = "redaction:\n  classes: [Email, Phone, IpAddress, PaymentCard]\n  rules: []\n";
const SERVE: &str = "serve: {view_port: 18999}\n";

/// The stand-in `claude` wrapped: it copies each prompt to `prompt.log` (prompts separated by
/// `=== end of prompt ===`), then answers as `tests/common`'s stand-in does, with the fact objects
/// in the file `inject-facts` (`@ID@` replaced by the first evidence id of the prompt) and the
/// entity objects in `inject-entities` put first in its lists.
const WRAPPER: &str = r#"R="@ROOT@"
cat > "$R/prompt.cur"
cat "$R/prompt.cur" >> "$R/prompt.log"
echo "=== end of prompt ===" >> "$R/prompt.log"
id=$(grep -oE 'evidence id [0-9a-f-]{36}' "$R/prompt.cur" | head -1 | cut -d' ' -f3)
facts=""
ents=""
if [ -e "$R/inject-facts" ]; then facts=$(sed "s/@ID@/$id/g" "$R/inject-facts"); fi
if [ -e "$R/inject-entities" ]; then ents=$(cat "$R/inject-entities"); fi
"@INNER@" "$@" < "$R/prompt.cur" | awk -v f="$facts" -v e="$ents" '{
  i = index($0, "\"facts\":["); if (i && f != "") $0 = substr($0, 1, i + 8) f substr($0, i + 9)
  j = index($0, "\"entities\":["); if (j && e != "") $0 = substr($0, 1, j + 11) e substr($0, j + 12)
  print }'
"#;

fn wrap_claude(w: &World) {
    let claude = w.bin.join("claude");
    let inner = w.bin.join("claude-inner");
    std::fs::rename(&claude, &inner).unwrap();
    executable(
        &claude,
        &WRAPPER
            .replace("@ROOT@", &w.root.display().to_string())
            .replace("@INNER@", &inner.display().to_string()),
    );
}

/// A world whose stand-in search answers `pages`, an instance `r` created from a spec with the
/// full redaction policy and `max_chars` characters per document, and the wrapped `claude`.
fn world(pages: &str, max_chars: usize) -> World {
    world_with(pages, max_chars, POLICY)
}

/// As `world`, with `policy` as the spec's `redaction` block.
fn world_with(pages: &str, max_chars: usize, policy: &str) -> World {
    let w = World::new();
    std::fs::write(w.root.join("answer.json"), answer(pages)).unwrap();
    wrap_claude(&w);
    let spec = w.spec("r", "conn_test");
    let text = std::fs::read_to_string(&spec).unwrap();
    assert!(text.contains(SERVE), "{text}");
    let chars = "max_chars_per_document: 5000";
    assert!(text.contains(chars), "{text}");
    let text = text
        .replacen(SERVE, &format!("{SERVE}{policy}"), 1)
        .replacen(chars, &format!("max_chars_per_document: {max_chars}"), 1);
    std::fs::write(&spec, text).unwrap();
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    w
}

/// `PAGES` with an email address and a card number in one page's text and a phone number in the
/// other page's search snippet.
fn planted_pages() -> String {
    let pages = PAGES.replacen(
        "Its website is example.org.",
        &format!("Its website is example.org. Write to {EMAIL}; card {CARD} paid the licence."),
        1,
    );
    let pages = pages.replacen(
        r#""description":"Gadget news.""#,
        &format!(r#""description":"Gadget news. Call {PHONE}.""#),
        1,
    );
    assert_ne!(pages, PAGES);
    pages
}

/// Every prompt the stand-in model received, in order.
fn prompts(w: &World) -> Vec<String> {
    std::fs::read_to_string(w.root.join("prompt.log"))
        .unwrap_or_default()
        .split("=== end of prompt ===\n")
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect()
}

fn log_lines(w: &World) -> Vec<Value> {
    let log = w.home.join("instances/r/cortex.log");
    std::fs::read_to_string(log)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
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

/// The store's facts, read back through `ekr sample` (`ekr.fact-sample/1`).
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

/// Whether some string in `v` equals `s` exactly.
fn has_string(v: &Value, s: &str) -> bool {
    match v {
        Value::String(x) => x == s,
        Value::Array(a) => a.iter().any(|x| has_string(x, s)),
        Value::Object(o) => o.values().any(|x| has_string(x, s)),
        _ => false,
    }
}

/// A fact whose value the model wrote as `placeholder`, citing the prompt's first evidence id.
fn website_fact(placeholder: &str) -> String {
    format!(
        r#"{{"!Property":{{"subject":{{"node_type":"Organization","aliases":["Example Labs"]}},"property":"website","value":{{"value_kind":"String","value":"{placeholder}"}},"evidence":["@ID@"]}}}},"#
    )
}

#[test]
fn planted_personal_data_never_reaches_the_model_and_is_restored_in_the_store() {
    let w = world(&planted_pages(), 5000);
    std::fs::write(w.root.join("inject-facts"), website_fact("[Email-1]")).unwrap();
    let (code, ran) = w.cortex(&["run", "r/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
    assert_eq!(ran["detail"]["parts_rejected"], 0, "{ran}");

    let prompts = prompts(&w);
    assert_eq!(prompts.len(), 1);
    let prompt = &prompts[0];
    assert!(
        prompt.contains("Example Labs develops the Widget engine."),
        "the stand-in model read the prompt: {prompt}"
    );
    for planted in [EMAIL, PHONE, CARD] {
        assert!(
            !prompt.contains(planted),
            "{planted} reached the model: {prompt}"
        );
    }
    assert!(
        prompt.contains("Write to [Email-1]; card [Card-1] paid"),
        "{prompt}"
    );
    assert!(
        prompt.contains("Description: Gadget news. Call [Phone-1]."),
        "{prompt}"
    );

    // The answer's `[Email-1]` is stored as the address, and no placeholder is stored.
    let facts = stored_facts(&w);
    assert!(
        has_string(&facts, EMAIL),
        "the restored fact is stored: {facts}"
    );
    let instance = w.home.join("instances/r");
    let store = std::fs::read(instance.join("store.sqlite")).unwrap();
    assert!(!contains(&store, "[Email-1]"), "a placeholder is stored");
    // The evidence payload is the original text.
    assert!(contains(
        &store,
        &format!("Write to {EMAIL}; card {CARD} paid the licence.")
    ));
    assert!(contains(&store, &format!("Gadget news. Call {PHONE}.")));
    // No file holds the mapping: none carries both a placeholder and the value it stands for.
    for (path, bytes) in files_under(&instance) {
        for (placeholder, value) in [
            ("[Email-1]", EMAIL),
            ("[Card-1]", CARD),
            ("[Phone-1]", PHONE),
        ] {
            assert!(
                !(contains(&bytes, placeholder) && contains(&bytes, value)),
                "{path} holds {placeholder} and {value}"
            );
        }
    }

    let line = log_lines(&w).pop().unwrap();
    assert_eq!(
        line["redacted"],
        serde_json::json!({"Email": 1, "Phone": 1, "IpAddress": 0, "PaymentCard": 1}),
        "{line}"
    );
    let total: u64 = line["redacted"]
        .as_object()
        .unwrap()
        .values()
        .map(|n| n.as_u64().unwrap())
        .sum();
    assert_eq!(total, 3, "{line}");
    assert_eq!(line["unrestored"], 0, "{line}");
}

#[test]
fn an_invalid_rule_pattern_fails_the_run_before_the_model_is_asked() {
    let w = World::new();
    let spec = w.spec("v", "conn_test");
    let text = std::fs::read_to_string(&spec).unwrap();
    std::fs::write(
        &spec,
        text.replacen(
            SERVE,
            &format!(
                "{SERVE}redaction:\n  classes: []\n  rules:\n    - {{name: broken, pattern: \"(unclosed\", replacement: \"[x]\"}}\n"
            ),
            1,
        ),
    )
    .unwrap();
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let (code, ran) = w.cortex(&["run", "v/news"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("extraction-failed")),
        "{ran}"
    );
    assert!(
        ran["detail"]["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("broken"),
        "{ran}"
    );
    assert!(
        w.lines("claude-calls.log").is_empty(),
        "claude was not asked"
    );
}

/// An email address in a page's URL reaches neither the prompt's `Source:` line nor the model;
/// the evidence identity in the store keeps the original URL.
#[test]
fn an_email_address_in_a_document_url_never_reaches_the_model() {
    let url = format!("https://example.org/team/{EMAIL}");
    let pages = PAGES.replacen("https://example.org/a", &url, 1);
    assert_ne!(pages, PAGES);
    let w = world(&pages, 5000);
    let (code, ran) = w.cortex(&["run", "r/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let prompt = prompts(&w).concat();
    assert!(prompt.contains("Example Labs develops"), "{prompt}");
    assert!(
        !prompt.contains(EMAIL),
        "the address reached the model: {prompt}"
    );
    assert!(
        prompt.contains("Source: https://example.org/team/[Email-1]\n"),
        "{prompt}"
    );
    let store = std::fs::read(w.home.join("instances/r/store.sqlite")).unwrap();
    assert!(contains(&store, &url), "the evidence keeps its identity");
    assert_eq!(log_lines(&w).pop().unwrap()["redacted"]["Email"], 1);
}

/// An email address in a page's title.
#[test]
fn an_email_address_in_a_title_never_reaches_the_model() {
    let pages = PAGES.replacen(r#""title":"A""#, &format!(r#""title":"A, by {EMAIL}""#), 1);
    assert_ne!(pages, PAGES);
    let w = world(&pages, 5000);
    let (code, ran) = w.cortex(&["run", "r/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let prompt = prompts(&w).concat();
    assert!(prompt.contains("Title: A, by [Email-1]"), "{prompt}");
    assert!(!prompt.contains(EMAIL), "{prompt}");
    let line = log_lines(&w).pop().unwrap();
    assert_eq!(line["redacted"]["Email"], 1, "{line}");
}

/// The store's own file holds the original evidence payload, and the prompt does not.
#[test]
fn the_store_file_itself_holds_the_original_payload() {
    let pages = PAGES.replacen(
        "Its website is example.org.",
        &format!("Its website is example.org. Write to {EMAIL}."),
        1,
    );
    let w = world(&pages, 5000);
    let (code, ran) = w.cortex(&["run", "r/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let store = std::fs::read(w.home.join("instances/r/store.sqlite")).unwrap();
    assert!(
        contains(&store, &format!("Write to {EMAIL}.")),
        "the store file holds the original payload"
    );
    assert!(!contains(&store, "[Email-1]"));
    assert!(prompts(&w).concat().contains("Write to [Email-1]."));
}

/// A run that sends nothing to the model has replaced nothing anyone sees.
#[test]
fn a_run_that_applies_nothing_counts_no_redactions() {
    let pages = PAGES.replacen(
        "Its website is example.org.",
        &format!("Its website is example.org. Write to {EMAIL}."),
        1,
    );
    let w = world(&pages, 5000);
    let (code, ran) = w.cortex(&["run", "r/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let (code, ran) = w.cortex(&["run", "r/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let line = log_lines(&w).pop().unwrap();
    assert_eq!(line["documents_new"], 0, "{line}");
    assert_eq!(
        line["redacted"]["Email"], 0,
        "a run that sent no document counted a redaction: {line}"
    );
    assert_eq!(line["unrestored"], 0, "{line}");
}

/// The story's acceptance: "the run report counts 3 redactions". `cortex run` prints the report.
/// Red in this unit's tree until the coordinator applies `run-report-detail.patch` to
/// `src/main.rs` (story:store-backend-per-instance's file).
#[test]
fn the_run_report_printed_by_cortex_run_counts_redactions() {
    let pages = PAGES.replacen(
        "Its website is example.org.",
        &format!("Its website is example.org. Write to {EMAIL}; card {CARD}; call {PHONE}."),
        1,
    );
    let w = world(&pages, 5000);
    let (code, ran) = w.cortex(&["run", "r/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(
        ran["detail"]["redacted"],
        serde_json::json!({"Email": 1, "Phone": 1, "IpAddress": 0, "PaymentCard": 1}),
        "{ran}"
    );
    assert_eq!(ran["detail"]["unrestored"], 0, "{ran}");
}

/// Only the text kept after `max_chars_per_document` is counted, and a value the cut splits is
/// replaced in what is kept.
#[test]
fn only_the_text_kept_after_truncation_is_pseudonymised_and_counted() {
    let pages = PAGES
        .replacen(
            "Example Labs develops the Widget engine. Its website is example.org.",
            &format!("Mail {EMAIL} now. Example Labs develops the Widget engine. Or ops.desk@example.org."),
            1,
        )
        .replacen(
            "Example Labs also develops the Gadget runtime.",
            &format!("The licence for the team was paid by {CARD}."),
            1,
        );
    let w = world(&pages, 40);
    let (code, ran) = w.cortex(&["run", "r/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let prompt = prompts(&w).concat();
    assert!(prompt.contains("Mail [Email-1] now. Example L"), "{prompt}");
    assert!(prompt.contains("was paid by [Card-1]\n"), "{prompt}");
    assert!(!prompt.contains("paid by 411"), "{prompt}");
    let line = log_lines(&w).pop().unwrap();
    assert_eq!(
        line["redacted"]["Email"], 1,
        "the cut-off address is not counted: {line}"
    );
    assert_eq!(line["redacted"]["PaymentCard"], 1, "{line}");
}

/// A name the model gave as a placeholder is restored, remembered as a known entity, and
/// pseudonymised again when the next prompt lists known entities; a placeholder with no mapping
/// is left as it is and counted as unrestored.
#[test]
fn a_restored_known_entity_is_pseudonymised_in_the_next_prompt() {
    let pages = PAGES.replacen(
        "Its website is example.org.",
        &format!("Its website is example.org. Write to {EMAIL}."),
        1,
    );
    let w = world(&pages, 5000);
    std::fs::write(
        w.root.join("inject-entities"),
        r#"{"node_type":"Organization","aliases":["[Email-1]"]},{"node_type":"Product","aliases":["[Phone-9]"]},"#,
    )
    .unwrap();
    let (code, ran) = w.cortex(&["run", "r/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let line = log_lines(&w).pop().unwrap();
    assert_eq!(line["unrestored"], 1, "[Phone-9] has no mapping: {line}");
    let entities = std::fs::read_to_string(w.home.join("instances/r/state/entities.json")).unwrap();
    assert!(entities.contains(EMAIL), "{entities}");

    std::fs::remove_file(w.root.join("inject-entities")).unwrap();
    std::fs::write(
        w.root.join("answer.json"),
        answer(&PAGES.replace(
            "also develops the Gadget runtime",
            "now develops the Gizmo runtime",
        )),
    )
    .unwrap();
    let (code, ran) = w.cortex(&["run", "r/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let prompts = prompts(&w);
    assert_eq!(prompts.len(), 2);
    assert!(prompts[1].contains("Known entities:"), "{}", prompts[1]);
    assert!(!prompts[1].contains(EMAIL), "{}", prompts[1]);
    assert!(prompts[1].contains("[Email-1]"), "{}", prompts[1]);
    assert_eq!(log_lines(&w).pop().unwrap()["redacted"]["Email"], 1);
}

/// A search result URL with a percent-encoded address goes to the `Source:` line.
#[test]
fn a_url_encoded_email_in_a_url_never_reaches_the_model() {
    let url = "https://example.org/unsubscribe?email=jane.doe%40example.com";
    let pages = PAGES.replacen("https://example.org/a", url, 1);
    assert_ne!(pages, PAGES);
    let w = world(&pages, 5000);
    let (code, ran) = w.cortex(&["run", "r/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let p = prompts(&w).concat();
    assert!(p.contains("Example Labs develops"), "{p}");
    assert!(
        !p.contains("jane.doe"),
        "the address reached the model: {p}"
    );
    assert!(
        p.contains("Source: https://example.org/unsubscribe?email=[Email-1]\n"),
        "{p}"
    );
    let store = std::fs::read(w.home.join("instances/r/store.sqlite")).unwrap();
    assert!(contains(&store, url), "the evidence keeps its identity");
}

/// `cortex create` runs the seed documents through the same pipeline and prints that run's
/// report; it must count what `cortex.log` counts for the same run. Red in this unit's tree until
/// the coordinator applies `seed-report.patch` to `src/main.rs`.
#[test]
fn the_seed_report_printed_by_create_counts_redactions_as_the_log_does() {
    let w = World::new();
    wrap_claude(&w);
    std::fs::write(
        w.root.join("seed.md"),
        format!("Example Labs develops the Widget engine. Write to {EMAIL}.\n"),
    )
    .unwrap();
    let spec = w.spec("s", "conn_test");
    let text = std::fs::read_to_string(&spec).unwrap();
    let text = text
        .replacen(SERVE, &format!("{SERVE}{POLICY}"), 1)
        .replacen("seed: {documents: []}", "seed: {documents: [seed.md]}", 1);
    std::fs::write(&spec, text).unwrap();
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let p = prompts(&w).concat();
    assert!(
        p.contains("Example Labs develops"),
        "the seed was extracted: {p}"
    );
    assert!(!p.contains(EMAIL), "{p}");
    let log = w.home.join("instances/s/cortex.log");
    let line: Value = serde_json::from_str(
        std::fs::read_to_string(log)
            .unwrap()
            .lines()
            .last()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(line["source"], "seed", "{line}");
    assert_eq!(line["redacted"]["Email"], 1, "{line}");
    assert_eq!(
        created["detail"]["seed"]["redacted"], line["redacted"],
        "create's report: {created}"
    );
    assert_eq!(created["detail"]["seed"]["unrestored"], 0, "{created}");
}

/// The operator's decision: a rule with a `replacement` is irreversible, like a credential. Its
/// matches are neither sent nor stored, and the replacement in an answer stays as written. A rule
/// with an empty `replacement` is a reversible placeholder, restored in the store.
#[test]
fn a_rule_with_a_replacement_is_neither_sent_nor_stored_and_one_without_is_restored() {
    let secret = "AK-0123ABCD";
    let policy = concat!(
        "redaction:\n  classes: [Email]\n  rules:\n",
        "    - {name: api-key, pattern: \"AK-[0-9A-F]{8}\", replacement: \"[api key]\"}\n",
        "    - {name: ticket, pattern: \"T-[0-9]{4}\", replacement: \"\"}\n",
    );
    let pages = PAGES.replacen(
        "Its website is example.org.",
        &format!("Its website is example.org. Key {secret}, ticket T-4711."),
        1,
    );
    let w = world_with(&pages, 5000, policy);
    std::fs::write(
        w.root.join("inject-facts"),
        website_fact("[api key] and [ticket-1]"),
    )
    .unwrap();
    let (code, ran) = w.cortex(&["run", "r/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let p = prompts(&w).concat();
    assert!(p.contains("Key [api key], ticket [ticket-1]."), "{p}");
    assert!(!p.contains(secret) && !p.contains("T-4711"), "{p}");
    let facts = stored_facts(&w);
    assert!(
        has_string(&facts, "[api key] and T-4711"),
        "the reversible rule is restored, the irreversible one is not: {facts}"
    );
    for (path, bytes) in files_under(&w.home.join("instances/r")) {
        assert!(!contains(&bytes, secret), "{path} holds the secret");
    }
    let store = std::fs::read(w.home.join("instances/r/store.sqlite")).unwrap();
    assert!(
        contains(&store, "Key [api key], ticket T-4711."),
        "the stored payload"
    );
    let line = log_lines(&w).pop().unwrap();
    assert_eq!(line["redacted"]["api-key"], 1, "{line}");
    assert_eq!(line["redacted"]["ticket"], 1, "{line}");
    assert_eq!(line["unrestored"], 0, "{line}");
}

/// The operator's decision: a batch directory keeps only the restored `extraction.yaml`, so no
/// file on disk pairs a placeholder with its value.
#[test]
fn no_file_in_a_batch_directory_holds_a_placeholder() {
    let w = world(&planted_pages(), 5000);
    std::fs::write(w.root.join("inject-facts"), website_fact("[Email-1]")).unwrap();
    let (code, ran) = w.cortex(&["run", "r/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let runs = w.home.join("instances/r/runs");
    let files = files_under(&runs);
    assert!(
        files
            .iter()
            .any(|(p, _)| p.ends_with("/batch-0/extraction.yaml")),
        "{:?}",
        files.iter().map(|(p, _)| p).collect::<Vec<_>>()
    );
    for (path, bytes) in &files {
        for placeholder in ["[Email-1]", "[Card-1]", "[Phone-1]"] {
            assert!(!contains(bytes, placeholder), "{path} holds {placeholder}");
        }
    }
    for entry in std::fs::read_dir(
        runs.join(
            std::fs::read_dir(&runs)
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .file_name(),
        )
        .join("batch-0"),
    )
    .unwrap()
    {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            assert_eq!(
                entry.file_name(),
                "extraction.yaml",
                "only the restored extraction"
            );
        }
    }
}
