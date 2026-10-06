//! Adversary cases for `story:record-field-lookups`: a `records` lookup file maps author ids and
//! `<@id>` mentions to names, and `fallback_text` stands in for an empty `text`.

mod common;

use std::path::{Path, PathBuf};

use common::{ekr, executable, World};
use serde_json::Value;

const POLICY: &str = "policy: {refresh_after_days: 7, change: ContentHash, max_documents_per_run: 50, max_chars_per_document: 5000}";

/// The stand-in `claude` wrapped so every prompt is copied to `prompt.log`.
const WRAPPER: &str = r#"R="@ROOT@"
cat > "$R/prompt.cur"
cat "$R/prompt.cur" >> "$R/prompt.log"
echo "=== end of prompt ===" >> "$R/prompt.log"
"@INNER@" "$@" < "$R/prompt.cur"
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

fn source(name: &str, dir: &str, glob: &str, records: &str) -> String {
    format!(
        "  - name: {name}\n    schedule: daily\n    settings:\n      kind: files\n      value:\n        paths: [{dir}]\n        glob: \"{glob}\"\n        records: {records}\n    {POLICY}\n"
    )
}

/// A world with `files` written under its root, `links` (link, target) as symlinks under it, and
/// the instance `adv` created from `sources` with `extra` appended to the spec.
fn world_with(
    files: &[(&str, &[u8])],
    links: &[(&str, &str)],
    sources: impl Fn(&Path) -> String,
    extra: &str,
) -> World {
    let w = World::new();
    wrap_claude(&w);
    for (rel, bytes) in files {
        let path = w.root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    for (link, target) in links {
        std::os::unix::fs::symlink(w.root.join(target), w.root.join(link)).unwrap();
    }
    let spec = w.root.join("adv.yaml");
    std::fs::write(
        &spec,
        format!(
            "format: cortex.instance/1\nname: adv\ndescription: Test brain.\nekr: {{version: \"0.0.30\", bin: \"{ekr}\"}}\nseed: {{documents: []}}\nmodel: {{model: claude-haiku-4-5-20251001, budget_usd: \"1\", timeout_s: 60}}\nsources:\n{sources}serve: {{view_port: 18991}}\n{extra}",
            ekr = ekr().display(),
            sources = sources(&w.root),
        ),
    )
    .unwrap();
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    w
}

fn world(files: &[(&str, &[u8])], sources: &str, extra: &str) -> World {
    world_with(files, &[], |_| sources.to_string(), extra)
}

fn run(w: &World, source: &str) -> Value {
    let (code, ran) = w.cortex(&["run", &format!("adv/{source}")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    ran
}

fn shown(w: &World) -> Vec<String> {
    std::fs::read_to_string(w.root.join("prompt.log"))
        .unwrap_or_default()
        .split("\n=== Document — evidence id ")
        .skip(1)
        .map(|d| d.split("=== end of prompt ===").next().unwrap().to_string())
        .collect()
}

fn keys(w: &World, source: &str) -> Vec<String> {
    let path = w
        .home
        .join("instances/adv/state")
        .join(format!("{source}.json"));
    let Ok(bytes) = std::fs::read(path) else {
        return Vec::new();
    };
    let state: Value = serde_json::from_slice(&bytes).unwrap();
    state["documents"]
        .as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default()
}

fn root(w: &World) -> PathBuf {
    std::fs::canonicalize(&w.root).unwrap()
}

fn key(root: &Path, file: &str, id: &str) -> String {
    format!("{}#{id}", root.join(file).display())
}

fn shown_doc(w: &World, key: &str) -> String {
    let docs = shown(w);
    let source = format!("Source: file:{key}\n");
    docs.iter()
        .find(|d| d.contains(&source))
        .unwrap_or_else(|| panic!("no document {key} in {docs:#?}"))
        .clone()
}

const PEOPLE: &str = r#"{"U1": "Ana", "U2": "Ben"}"#;
const WITH_LOOKUP: &str = "{format: JsonLines, id: id, author: user, text: [\"{text}\"], lookup: people.json, filters: []}";

/// A Slack mention can carry a label after the id, `<@U2|ben>` (the form older exports write).
/// The story maps "every `<@id>`"; the id here is `U2`, which the lookup holds, yet `mentions`
/// looks up `U2|ben` and keeps the mention as written, so the model sees the raw id.
#[test]
fn a_labelled_mention_is_mapped_by_its_id() {
    let chat = concat!(r#"{"id":"1","user":"U1","text":"hi <@U2|ben>"}"#, "\n");
    let w = world(
        &[
            ("chat/export.jsonl", chat.as_bytes()),
            ("people.json", PEOPLE.as_bytes()),
        ],
        &source("chat", "chat", "*.jsonl", WITH_LOOKUP),
        "",
    );
    run(&w, "chat");
    let one = shown_doc(&w, &key(&root(&w), "chat/export.jsonl", "1"));
    assert!(one.contains("\nAna: hi @Ben\n"), "{one}");
}

/// The record files of a `files` source are read with a leading UTF-8 byte-order mark ignored
/// (`file_records` doc). A lookup file saved with one (a Windows editor, `Out-File -Encoding
/// utf8`) is the same JSON object, yet `read_lookup` hands the raw bytes to serde and the whole
/// source's run fails.
#[test]
fn a_lookup_file_with_a_byte_order_mark_is_read() {
    let chat = concat!(r#"{"id":"1","user":"U1","text":"hi <@U2>"}"#, "\n");
    let people = format!("\u{feff}{PEOPLE}");
    let w = world(
        &[
            ("chat/export.jsonl", chat.as_bytes()),
            ("people.json", people.as_bytes()),
        ],
        &source("chat", "chat", "*.jsonl", WITH_LOOKUP),
        "",
    );
    let (code, ran) = w.cortex(&["run", "adv/chat"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let one = shown_doc(&w, &key(&root(&w), "chat/export.jsonl", "1"));
    assert!(one.contains("\nAna: hi @Ben\n"), "{one}");
}

/// A blank author value is no author (`scalar`), so a record never reads `: <text>`. A lookup
/// that maps an id to a blank name (an account without a display name) erases the id instead:
/// the author prefix becomes `: ` and the mention `@  `, and the model can tell nobody apart.
/// A blank name should count as no name, keeping the id as an id the file does not hold is kept.
#[test]
fn a_blank_name_in_the_lookup_keeps_the_id() {
    let chat = concat!(r#"{"id":"1","user":"U1","text":"hi <@U2>"}"#, "\n");
    let people = r#"{"U1": "", "U2": "  "}"#;
    let w = world(
        &[
            ("chat/export.jsonl", chat.as_bytes()),
            ("people.json", people.as_bytes()),
        ],
        &source("chat", "chat", "*.jsonl", WITH_LOOKUP),
        "",
    );
    run(&w, "chat");
    let one = shown_doc(&w, &key(&root(&w), "chat/export.jsonl", "1"));
    assert!(one.contains("\nU1: hi <@U2>\n"), "{one}");
}

/// Held: a one-line lookup file inside the source's own `paths` and glob is not read as a record
/// (it has no `id`), so it yields no document.
#[test]
fn a_lookup_inside_the_sources_glob_is_not_a_document() {
    let chat = concat!(r#"{"id":"1","user":"U1","text":"hi <@U2>"}"#, "\n");
    let records = "{format: JsonLines, id: id, author: user, text: [\"{text}\"], lookup: chat/people.json, filters: []}";
    let w = world(
        &[
            ("chat/export.jsonl", chat.as_bytes()),
            ("chat/people.json", PEOPLE.as_bytes()),
        ],
        &source("chat", "chat", "*", records),
        "",
    );
    let ran = run(&w, "chat");
    assert_eq!(ran["detail"]["documents_new"], 1, "{ran}");
    assert_eq!(keys(&w, "chat"), [key(&root(&w), "chat/export.jsonl", "1")]);
    assert!(shown(&w).iter().all(|d| !d.contains("\"U1\"")));
}

/// Held: an absolute lookup path and a symlinked lookup file are both read.
#[test]
fn an_absolute_and_a_symlinked_lookup_are_read() {
    let chat = concat!(r#"{"id":"1","user":"U1","text":"hi <@U2>"}"#, "\n");
    let w = world_with(
        &[
            ("chat/export.jsonl", chat.as_bytes()),
            ("people.json", PEOPLE.as_bytes()),
        ],
        &[("link.json", "people.json")],
        |root| {
            let abs = root.join("people.json").display().to_string();
            format!(
                "{}{}",
                source(
                    "abs",
                    "chat",
                    "*.jsonl",
                    &format!("{{format: JsonLines, id: id, author: user, text: [\"{{text}}\"], lookup: \"{abs}\", filters: []}}")
                ),
                source(
                    "linked",
                    "chat",
                    "*.jsonl",
                    "{format: JsonLines, id: id, author: user, text: [\"{text}\"], lookup: link.json, filters: []}"
                )
            )
        },
        "",
    );
    run(&w, "abs");
    run(&w, "linked");
    let docs = shown(&w);
    assert_eq!(docs.len(), 2, "{docs:#?}");
    assert!(
        docs.iter().all(|d| d.contains("\nAna: hi @Ben\n")),
        "{docs:#?}"
    );
}

/// Held: a name holding mention or template syntax is inserted as written, never expanded again.
#[test]
fn a_name_with_mention_or_template_syntax_is_not_expanded_again() {
    let chat = concat!(r#"{"id":"1","user":"U1","text":"x <@U1>"}"#, "\n");
    let people = r#"{"U1": "<@U2> {text}", "U2": "Ben"}"#;
    let w = world(
        &[
            ("chat/export.jsonl", chat.as_bytes()),
            ("people.json", people.as_bytes()),
        ],
        &source("chat", "chat", "*.jsonl", WITH_LOOKUP),
        "",
    );
    run(&w, "chat");
    let one = shown_doc(&w, &key(&root(&w), "chat/export.jsonl", "1"));
    assert!(
        one.contains("\n<@U2> {text}: x @<@U2> {text}\n") && !one.contains("Ben"),
        "{one}"
    );
}

/// Held: a mapped name is a name in the text, and `RareName` replaces it as any other.
#[test]
fn a_mapped_name_is_redacted_like_any_other_name() {
    let chat = concat!(r#"{"id":"1","user":"U1","text":"hi <@U2>"}"#, "\n");
    let w = world(
        &[
            ("chat/export.jsonl", chat.as_bytes()),
            ("people.json", PEOPLE.as_bytes()),
        ],
        &source("chat", "chat", "*.jsonl", WITH_LOOKUP),
        "redaction:\n  classes: [RareName]\n  rules: []\n  rare_limit: 1\n",
    );
    run(&w, "chat");
    let docs = shown(&w);
    assert_eq!(docs.len(), 1, "{docs:#?}");
    assert!(
        !docs[0].contains("Ana") && !docs[0].contains("Ben"),
        "{docs:#?}"
    );
}

/// Pins "the first non-empty one is the document text": with two non-empty fallbacks and a `text`
/// that renders only whitespace, the text is the first fallback alone. The unit's own case has one
/// non-empty fallback, so taking the last, or joining every fallback as `text` does, passes it.
#[test]
fn only_the_first_non_empty_fallback_is_the_text() {
    let chat = concat!(
        r#"{"id":"1","text":"   ","a":"first fallback","b":"second fallback"}"#,
        "\n"
    );
    let records = "{format: JsonLines, id: id, text: [\"{text}\"], fallback_text: [\"{none}\", \"{a}\", \"{b}\"], filters: []}";
    let w = world(
        &[("chat/export.jsonl", chat.as_bytes())],
        &source("chat", "chat", "*.jsonl", records),
        "",
    );
    run(&w, "chat");
    let one = shown_doc(&w, &key(&root(&w), "chat/export.jsonl", "1"));
    assert!(
        one.contains("\nfirst fallback\n") && !one.contains("second fallback"),
        "{one}"
    );
}
