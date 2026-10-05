//! A credential in a fetched document's title, description or key (its URL) is masked like one in
//! its text (`story:credential-mask-covers-titles`): the stand-in model's prompt, every file under
//! the instance and the evidence in the store hold none of them, and the run's log line counts
//! each mask. Masking is irreversible, so unlike a redaction placeholder it is never restored.
//! The first case runs once without a `redaction` policy and once with one, the two paths
//! `run.rs` takes. The cases from `two_urls_that_differ_after_the_token_stay_two_documents` on came
//! from the adversary review: key masking keeps the rest of a query, masked keys are deduplicated,
//! percent-encoded, userinfo and prefixed credentials are masked, and a record key is masked in
//! the run report, `cortex.log` and the source's state file.

mod common;

use std::path::Path;

use common::{answer, ekr, executable, World, PAGES};
use serde_json::Value;

/// The stand-in `claude` wrapped: it appends each prompt to `prompt.log`, then answers as
/// `tests/common`'s stand-in does.
const WRAPPER: &str = r#"R="@ROOT@"
cat > "$R/prompt.cur"
cat "$R/prompt.cur" >> "$R/prompt.log"
"@INNER@" "$@" < "$R/prompt.cur"
"#;

const POLICY: &str = "redaction:\n  classes: [Email, Phone, IpAddress, PaymentCard]\n  rules: []\n";
const SERVE: &str = "serve: {view_port: 18999}\n";

/// Credential-shaped tokens, assembled at runtime so none sits in the source as a literal.
struct Planted {
    /// In the first page's title.
    title: String,
    /// In the second page's description.
    description: String,
    /// The value of an `access_token=` query parameter in the first page's URL.
    url: String,
}

fn planted() -> Planted {
    Planted {
        title: format!("glpat-{}", "Tq7".repeat(8)),
        description: format!("tvly-{}", "Kd9x".repeat(5)),
        url: format!("Zr4{}", "p0".repeat(8)),
    }
}

/// `PAGES` with `p`'s tokens in the first title, the second description and the first URL.
fn planted_pages(p: &Planted) -> String {
    let pages = PAGES
        .replacen(r#""title":"A""#, &format!(r#""title":"A {}""#, p.title), 1)
        .replacen(
            r#""description":"Gadget news.""#,
            &format!(r#""description":"Gadget news. Key {}.""#, p.description),
            1,
        )
        .replacen(
            r#""url":"https://example.org/a""#,
            &format!(r#""url":"https://example.org/a?access_token={}""#, p.url),
            1,
        );
    assert_eq!(pages.matches("Tq7").count(), 8, "{pages}");
    assert!(pages.contains(&p.description) && pages.contains(&p.url));
    pages
}

/// A world answering `pages`, an instance `r` whose spec carries `policy` (nothing when empty),
/// and the wrapped `claude`.
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

fn last_log_line(w: &World) -> Value {
    let log = std::fs::read_to_string(w.home.join("instances/r/cortex.log")).unwrap();
    serde_json::from_str(log.lines().last().unwrap()).unwrap()
}

fn credentials_in_title_description_and_url_are_masked(policy: &str) {
    let p = planted();
    let w = world(&planted_pages(&p), policy);
    let (code, ran) = w.cortex(&["run", "r/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");

    let prompt = std::fs::read_to_string(w.root.join("prompt.log")).unwrap();
    assert!(
        prompt.contains("Example Labs develops the Widget engine."),
        "the stand-in model read the prompt: {prompt}"
    );
    for token in [&p.title, &p.description, &p.url] {
        assert!(
            !prompt.contains(token.as_str()),
            "{token} reached the model: {prompt}"
        );
    }
    assert!(
        prompt.contains("Title: A [masked:gitlab-token]"),
        "{prompt}"
    );
    assert!(
        prompt.contains("Description: Gadget news. Key [masked:tavily-key]."),
        "{prompt}"
    );
    assert!(
        prompt.contains("Source: https://example.org/a?access_token=[masked:secret-assignment]"),
        "{prompt}"
    );

    let instance = w.home.join("instances/r");
    let files = files_under(&instance);
    assert!(
        files.iter().any(|(path, _)| path.ends_with("store.sqlite")),
        "the store is under the instance"
    );
    for (path, bytes) in &files {
        for token in [&p.title, &p.description, &p.url] {
            assert!(!contains(bytes, token), "{path} holds {token}");
        }
    }
    // The evidence carries the masked identity and header lines.
    let store = std::fs::read(instance.join("store.sqlite")).unwrap();
    assert!(
        contains(
            &store,
            "https://example.org/a?access_token=[masked:secret-assignment]"
        ),
        "the evidence identity is the masked key"
    );
    assert!(contains(&store, "Title: A [masked:gitlab-token]"));

    let line = last_log_line(&w);
    assert_eq!(line["masked"], 3, "{line}");
    assert_eq!(ran["detail"]["masked"], 3, "{ran}");
}

#[test]
fn credentials_in_title_description_and_url_are_masked_without_a_redaction_policy() {
    credentials_in_title_description_and_url_are_masked("");
}

#[test]
fn credentials_in_title_description_and_url_are_masked_with_a_redaction_policy() {
    credentials_in_title_description_and_url_are_masked(POLICY);
}

/// A key without a credential keeps its evidence identity, and a second run of the same masked
/// key finds it already applied rather than as a new document.
#[test]
fn a_masked_key_is_stable_across_runs_and_a_clean_key_is_unchanged() {
    let p = planted();
    let w = world(&planted_pages(&p), "");
    let (code, ran) = w.cortex(&["run", "r/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let store = std::fs::read(w.home.join("instances/r/store.sqlite")).unwrap();
    assert!(
        contains(&store, "Source: https://example.org/b\n"),
        "a clean key is unchanged"
    );
    let (code, ran) = w.cortex(&["run", "r/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let line = last_log_line(&w);
    assert_eq!(line["documents_new"], 0, "{line}");
}

/// A token no rule would call prose, assembled at runtime.
fn token(seed: &str) -> String {
    format!("{seed}{}", "p0".repeat(8))
}

/// `PAGES` with the first page's URL replaced by `a` and the second's by `b`.
fn pages_with_urls(a: &str, b: &str) -> String {
    let pages = PAGES
        .replacen(
            r#""url":"https://example.org/a""#,
            &format!(r#""url":"{a}""#),
            1,
        )
        .replacen(
            r#""url":"https://example.org/b""#,
            &format!(r#""url":"{b}""#),
            1,
        );
    assert!(pages.contains(a) && pages.contains(b), "{pages}");
    pages
}

/// A world answering `pages` with no `redaction` policy.
fn plain(pages: &str) -> World {
    world(pages, "")
}

/// The paths of the files under `dir` that hold `needle`.
fn holders(dir: &Path, needle: &str) -> Vec<String> {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .map(|e| e.unwrap())
        .filter(|e| e.file_type().is_file())
        .filter(|e| contains(&std::fs::read(e.path()).unwrap(), needle))
        .map(|e| e.path().display().to_string())
        .collect()
}

fn run(w: &World) -> Value {
    let (code, ran) = w.cortex(&["run", "r/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    ran
}

/// Asserts `secret` reached neither the model nor any file under the instance.
fn assert_nowhere(w: &World, secret: &str) {
    let prompt = std::fs::read_to_string(w.root.join("prompt.log")).unwrap_or_default();
    assert!(
        prompt.contains("=== Document"),
        "the stand-in model was asked: {prompt}"
    );
    assert!(
        !prompt.contains(secret),
        "{secret} reached the model:\n{prompt}"
    );
    let held = holders(&w.home.join("instances/r"), secret);
    assert!(held.is_empty(), "{secret} is stored in {held:?}");
}

/// Two search results on one page whose URLs carry a token and then differ in a later query
/// parameter are two documents. Masking the token must not swallow the parameters after it: the
/// state file keeps two keys, and a second run of the same, unchanged results finds nothing new.
#[test]
fn two_urls_that_differ_after_the_token_stay_two_documents() {
    let t = token("Zr4");
    let w = plain(&pages_with_urls(
        &format!("https://example.org/doc?access_token={t}&page=1"),
        &format!("https://example.org/doc?access_token={t}&page=2"),
    ));
    let ran = run(&w);
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
    let state: Value =
        serde_json::from_slice(&std::fs::read(w.home.join("instances/r/state/news.json")).unwrap())
            .unwrap();
    let keys: Vec<&String> = state["documents"].as_object().unwrap().keys().collect();
    assert_eq!(keys.len(), 2, "two documents, one key: {keys:?}");
    let ran = run(&w);
    assert_eq!(
        ran["detail"]["documents_new"], 0,
        "unchanged results were extracted again: {ran}"
    );
}

/// The same page returned twice with two different tokens is one document once the tokens are
/// masked: it is extracted and cited once, not twice under one identity.
#[test]
fn two_urls_that_differ_only_in_the_token_are_one_document() {
    let pages = pages_with_urls(
        &format!("https://example.org/doc?access_token={}", token("Zr4")),
        &format!("https://example.org/doc?access_token={}", token("Qy8")),
    )
    .replacen(
        "Example Labs also develops the Gadget runtime.",
        "Example Labs develops the Widget engine. Its website is example.org.",
        1,
    );
    let w = plain(&pages);
    let ran = run(&w);
    assert_eq!(
        ran["detail"]["documents_new"], 1,
        "one masked key, two documents: {ran}"
    );
}

/// An `access_token=` assignment percent-encoded inside a URL (a `redirect`/`next` parameter) is
/// the same credential: the redaction policy already decodes `%40` for e-mail addresses in URLs.
#[test]
fn a_percent_encoded_access_token_in_a_url_is_masked() {
    let t = token("Zr4");
    let w = plain(&pages_with_urls(
        &format!("https://example.org/a?next=%2Fapi%3Faccess_token%3D{t}"),
        "https://example.org/b",
    ));
    run(&w);
    assert_nowhere(&w, &t);
}

/// The password of a URL's userinfo (`https://user:password@host`) is the credential a URL
/// carries by its own syntax.
#[test]
fn a_userinfo_password_in_a_url_is_masked() {
    let t = token("Zr4");
    let w = plain(&pages_with_urls(
        &format!("https://alice:{t}@example.org/a"),
        "https://example.org/b",
    ));
    run(&w);
    assert_nowhere(&w, &t);
}

/// `client_secret=` is a `secret` assignment: a name with a `<word>_` prefix is masked in a URL.
#[test]
fn a_client_secret_in_a_url_is_masked() {
    let t = token("Zr4");
    let w = plain(&pages_with_urls(
        &format!("https://example.org/a?client_id=app&client_secret={t}"),
        "https://example.org/b",
    ));
    run(&w);
    assert_nowhere(&w, &t);
}

/// An environment-file line `DB_PASSWORD=...` in a fetched text is a `password=` assignment.
#[test]
fn an_env_style_password_in_a_text_is_masked() {
    let t = token("Zr4");
    let pages = PAGES.replacen(
        "Example Labs also develops the Gadget runtime.",
        &format!("Example Labs also develops the Gadget runtime. Set DB_PASSWORD={t} to run it."),
        1,
    );
    let w = plain(&pages);
    run(&w);
    assert_nowhere(&w, &t);
}

/// A stand-in `connectors` for a `tracker` source whose one record's id is a URL with a token,
/// and whose child call fails for every record.
const RECORD_CONNECTORS: &str = r#"R="@ROOT@"
case "$*" in
  *"connections list"*) echo '{"ok":true,"result":{"connections":[{"adapter":"tracker","connection":"conn_test","state":"ready","revision":"rev1"}]}}' ;;
  *"connections revalidate"*) echo '{"ok":true,"result":{}}' ;;
  *"operations describe"*) echo '{"ok":true,"result":{"schema":"s","revision":"r"}}' ;;
  *"--operation hooks.list"*)
    printf '{"ok":true,"result":{"adapter":"tracker","operation":"hooks.list","revision":"r","result":{"hooks":[{"url":"https://hooks.example.org/in?access_token=@TOKEN@","summary":"Inbound hook"}]}}}\n' ;;
  *"--operation deliveries.list"*)
    echo '{"ok":false,"error":{"code":"failure","data":{"code":"unavailable","stage":"execution"}}}'; exit 1 ;;
  *) echo '{"ok":false}'; exit 2 ;;
esac
"#;

/// A world whose instance `r` has the `tracker` source of [`RECORD_CONNECTORS`], with token `t`.
fn record_world(t: &str) -> World {
    let w = World::new();
    executable(
        &w.bin.join("connectors"),
        &RECORD_CONNECTORS
            .replace("@ROOT@", &w.root.display().to_string())
            .replace("@TOKEN@", t),
    );
    let path = w.root.join("hooks.yaml");
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: r
description: Test brain.
ekr: {{version: "0.0.30", bin: "{ekr}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
  - name: tracker
    schedule: daily
    settings:
      kind: connectors
      value:
        adapter: tracker
        connection: conn_test
        operation: hooks.list
        inputs: [{{}}]
        records: hooks
        id: url
        text: ["{{url}}: {{summary}}"]
        child: {{operation: deliveries.list, input: {{hook: "{{url}}"}}, records: deliveries}}
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
serve: {{view_port: 18995}}
"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    w
}

/// A record's key is its document key: a token in it must not reach the run report when the
/// record's child call fails (`stopped`) or, from the third failing run, when it is skipped
/// (`skipped`).
#[test]
fn a_token_in_a_record_key_stays_out_of_the_run_report() {
    let t = token("Zr4");
    let w = record_world(&t);
    let mut leaked = Vec::new();
    for n in 1..=3 {
        let (code, ran) = w.cortex(&["run", "r/tracker"]);
        assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
        if ran.to_string().contains(&t) {
            leaked.push(format!("run {n}: {}", ran["detail"]));
        }
    }
    assert!(
        leaked.is_empty(),
        "the run report holds the token:\n{leaked:#?}"
    );
}

/// The same token must not reach `cortex.log` or the source's state file (`child_failures`).
#[test]
fn a_token_in_a_record_key_stays_out_of_the_log_and_the_state() {
    let t = token("Zr4");
    let w = record_world(&t);
    let mut held = Vec::new();
    for n in 1..=3 {
        let (code, ran) = w.cortex(&["run", "r/tracker"]);
        assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
        for path in holders(&w.home.join("instances/r"), &t) {
            held.push(format!("run {n}: {path}"));
        }
    }
    assert!(held.is_empty(), "the token is stored in:\n{held:#?}");
}

/// Masking a record key keeps the child-failure count working: the parent whose child call fails
/// on three runs is skipped on the third, named by its masked key.
#[test]
fn a_record_key_with_a_token_is_still_skipped_on_its_third_failing_run() {
    let t = token("Zr4");
    let w = record_world(&t);
    let mut skipped = Vec::new();
    for _ in 1..=3 {
        let (code, ran) = w.cortex(&["run", "r/tracker"]);
        assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
        skipped.push(ran["detail"]["skipped"].clone());
    }
    let none = Value::Array(Vec::new());
    assert!(
        skipped[..2].iter().all(|s| s.is_null() || *s == none),
        "{skipped:?}"
    );
    let third = skipped[2].to_string();
    assert!(
        third.contains(
            "child call failed for tracker:hooks.list:https://hooks.example.org/in?access_token=[masked:secret-assignment] on 3 runs; skipped"
        ),
        "{skipped:?}"
    );
    let state: Value = serde_json::from_slice(
        &std::fs::read(w.home.join("instances/r/state/tracker.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        state["child_failures"]
            ["tracker:hooks.list:https://hooks.example.org/in?access_token=[masked:secret-assignment]"],
        3,
        "{state}"
    );
}
