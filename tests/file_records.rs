//! A `files` source reads each line of a JSON-lines export, or each `##` section of a markdown
//! file, as its own document, and delivers a record again as soon as its text changes
//! (`story:file-records`).

mod common;

use std::path::{Path, PathBuf};

use common::{ekr, World};
use serde_json::Value;

/// Five chat messages; the fourth is in the `dm` channel, which the source excludes.
const EXPORT: &str = r#"{"ts":"100.1","channel":"eng","user":"ana","time":"2026-10-01T09:00:00Z","text":"Example Labs ships the Widget engine on Friday."}
{"ts":"100.2","channel":"eng","user":"ben","time":"2026-10-01T09:01:00Z","text":"The Widget engine release notes are ready.","thread_ts":"100.1"}
{"ts":"100.3","channel":"ops","user":"cem","time":"2026-10-01T09:02:00Z","text":"Example Labs runs the Gadget runtime in two regions."}

{"ts":"100.4","channel":"dm","user":"ana","time":"2026-10-01T09:03:00Z","text":"A private note."}
{"ts":"100.5","channel":"ops","user":"ben","time":"2026-10-01T09:04:00Z","text":"Example Labs also develops the Widget engine."}
"#;

/// Three `##` sections. The `##` inside the code block is not a heading.
const NOTES: &str =
    "## Widget\n\nExample Labs develops the Widget engine.\n\n## Gadget\n\nExample Labs \
                     develops the Gadget runtime.\n\n```\n## not a heading\n```\n\n## Sprocket\n\n\
                     Example Labs sells the Sprocket kit.\n";

fn spec(w: &World) -> PathBuf {
    let path = w.root.join("records.yaml");
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: records
description: Test brain.
ekr: {{version: "0.0.30", bin: "{ekr}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
  - name: chat
    schedule: daily
    settings:
      kind: files
      value:
        paths: [chat]
        glob: "*.jsonl"
        records:
          format: JsonLines
          id: ts
          time: time
          author: user
          text: ["{{text}}"]
          thread: thread_ts
          filters: [{{field: channel, values: [dm], include: false}}]
    policy: {{refresh_after_days: 7, change: ContentHash, max_documents_per_run: 50, max_chars_per_document: 5000}}
  - name: notes
    schedule: daily
    settings:
      kind: files
      value:
        paths: [notes]
        glob: "*.md"
        records: {{format: MarkdownSections, id: heading, text: ["{{body}}"], filters: []}}
    policy: {{refresh_after_days: 7, change: ContentHash, max_documents_per_run: 50, max_chars_per_document: 5000}}
serve: {{view_port: 18993}}
"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    path
}

/// A world with the chat export and the notes file, and the instance `records` created on them.
fn world() -> World {
    let w = World::new();
    std::fs::create_dir_all(w.root.join("chat")).unwrap();
    std::fs::create_dir_all(w.root.join("notes")).unwrap();
    std::fs::write(w.root.join("chat/export.jsonl"), EXPORT).unwrap();
    std::fs::write(w.root.join("notes/handbook.md"), NOTES).unwrap();
    let path = spec(&w);
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    w
}

fn run(w: &World, source: &str) -> Value {
    let (code, ran) = w.cortex(&["run", &format!("records/{source}")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    ran
}

/// The document keys a source's state holds, with their content hashes.
fn seen(w: &World, source: &str) -> Vec<(String, String)> {
    let path = w
        .home
        .join("instances/records/state")
        .join(format!("{source}.json"));
    let state: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    state["documents"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v["hash"].as_str().unwrap().to_string()))
        .collect()
}

fn keys(w: &World, source: &str) -> Vec<String> {
    seen(w, source).into_iter().map(|(k, _)| k).collect()
}

/// The key a record of `file` (relative to the spec's directory) with id `id` is stored under.
fn key(root: &Path, file: &str, id: &str) -> String {
    format!("{}#{id}", root.join(file).display())
}

/// Every document text the runs of the instance showed the model, newest run last.
fn prompted(w: &World) -> Vec<String> {
    let mut runs: Vec<_> = std::fs::read_dir(w.home.join("instances/records/runs"))
        .unwrap()
        .map(|r| r.unwrap().path())
        .collect();
    runs.sort();
    let mut docs = Vec::new();
    for run in runs {
        for batch in std::fs::read_dir(run).unwrap() {
            let Ok(prompt) = std::fs::read_to_string(batch.unwrap().path().join("prompt.txt"))
            else {
                continue;
            };
            docs.extend(
                prompt
                    .split("\n=== Document — evidence id ")
                    .skip(1)
                    .map(str::to_string),
            );
        }
    }
    docs
}

#[test]
fn five_json_lines_one_filtered_and_three_markdown_sections_are_seven_documents() {
    let w = world();
    let root = std::fs::canonicalize(&w.root).unwrap();

    let chat = run(&w, "chat");
    assert_eq!(chat["detail"]["documents_new"], 4, "{chat}");
    assert_eq!(chat["detail"]["documents_applied"], 4, "{chat}");
    let notes = run(&w, "notes");
    assert_eq!(notes["detail"]["documents_new"], 3, "{notes}");
    assert_eq!(notes["detail"]["documents_applied"], 3, "{notes}");

    let chat_file = "chat/export.jsonl";
    assert_eq!(
        keys(&w, "chat"),
        ["100.1", "100.2", "100.3", "100.5"].map(|id| key(&root, chat_file, id))
    );
    let notes_file = "notes/handbook.md";
    assert_eq!(
        keys(&w, "notes"),
        ["Gadget", "Sprocket", "Widget"].map(|id| key(&root, notes_file, id))
    );

    let docs = prompted(&w);
    assert_eq!(docs.len(), 7, "{docs:#?}");
    let doc = |k: &str| {
        let source = format!("Source: file:{k}\n");
        docs.iter()
            .find(|d| d.contains(&source))
            .unwrap_or_else(|| panic!("no document {k} in {docs:#?}"))
            .clone()
    };
    // A record's text is its author and its rendered text.
    let first = doc(&key(&root, chat_file, "100.1"));
    assert!(
        first.contains("ana: Example Labs ships the Widget engine on Friday."),
        "{first}"
    );
    // The reply carries the message it answers as context.
    let reply = doc(&key(&root, chat_file, "100.2"));
    assert!(
        reply.contains("ben: The Widget engine release notes are ready."),
        "{reply}"
    );
    assert!(
        reply.contains("[context] ana: Example Labs ships the Widget engine on Friday."),
        "{reply}"
    );
    assert!(
        !docs.iter().any(|d| d.contains("A private note.")),
        "{docs:#?}"
    );
    // A section is its body; the code block stays inside its section.
    let gadget = doc(&key(&root, notes_file, "Gadget"));
    assert!(
        gadget.contains("Example Labs develops the Gadget runtime."),
        "{gadget}"
    );
    assert!(gadget.contains("## not a heading"), "{gadget}");
    assert!(!gadget.contains("Widget engine"), "{gadget}");
}

#[test]
fn an_edited_section_is_delivered_again_inside_the_refresh_window_and_alone() {
    let w = world();
    let root = std::fs::canonicalize(&w.root).unwrap();
    let ran = run(&w, "notes");
    assert_eq!(ran["detail"]["documents_applied"], 3, "{ran}");
    let before = seen(&w, "notes");

    // Nothing changed: nothing is applied.
    let ran = run(&w, "notes");
    assert_eq!(ran["detail"]["documents_new"], 0, "{ran}");

    std::fs::write(
        w.root.join("notes/handbook.md"),
        NOTES.replace(
            "sells the Sprocket kit",
            "sells the Sprocket kit and the Flange kit",
        ),
    )
    .unwrap();
    let calls = w.lines("claude-calls.log").len();
    let ran = run(&w, "notes");
    assert_eq!(ran["detail"]["documents_new"], 1, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    assert_eq!(w.lines("claude-calls.log").len(), calls + 1);

    let last = prompted(&w).pop().unwrap();
    let sprocket = key(&root, "notes/handbook.md", "Sprocket");
    assert!(
        last.contains(&format!("Source: file:{sprocket}\n")),
        "{last}"
    );
    assert!(last.contains("the Flange kit"), "{last}");

    let after = seen(&w, "notes");
    let changed: Vec<_> = before
        .iter()
        .zip(&after)
        .filter(|(b, a)| b != a)
        .map(|(_, (k, _))| k.clone())
        .collect();
    assert_eq!(changed, [sprocket]);
}

/// Edge cases of reading files as records: threads, redaction, fences, byte-order marks, repeated
/// and setext headings, and files or lines that cannot be read.
mod edges {

    use std::path::{Path, PathBuf};

    use super::common::{ekr, executable, World};
    use serde_json::Value;

    const POLICY: &str = "policy: {refresh_after_days: 7, change: ContentHash, max_documents_per_run: 50, max_chars_per_document: 5000}";

    /// The stand-in `claude` wrapped so every prompt is copied to `prompt.log`, with or without a
    /// redaction policy (with one, the run writes no `prompt.txt`).
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

    /// A `files` source `name` reading `dir` with `glob`, its `records` block being `records`.
    fn source(name: &str, dir: &str, glob: &str, records: &str) -> String {
        format!(
        "  - name: {name}\n    schedule: daily\n    settings:\n      kind: files\n      value:\n        paths: [{dir}]\n        glob: \"{glob}\"\n        records: {records}\n    {POLICY}\n"
    )
    }

    /// A world with `files` (relative path, bytes) written under its root and the instance `adv`
    /// created from `sources`, with `extra` appended to the spec (a `redaction` block).
    fn world(files: &[(&str, &[u8])], sources: &str, extra: &str) -> World {
        let w = World::new();
        wrap_claude(&w);
        for (rel, bytes) in files {
            let path = w.root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        }
        let spec = w.root.join("adv.yaml");
        std::fs::write(
        &spec,
        format!(
            "format: cortex.instance/1\nname: adv\ndescription: Test brain.\nekr: {{version: \"0.0.30\", bin: \"{ekr}\"}}\nseed: {{documents: []}}\nmodel: {{model: claude-haiku-4-5-20251001, budget_usd: \"1\", timeout_s: 60}}\nsources:\n{sources}serve: {{view_port: 18992}}\n{extra}",
            ekr = ekr().display()
        ),
    )
    .unwrap();
        let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
        assert_eq!(code, 0, "{created}");
        w
    }

    fn run(w: &World, source: &str) -> Value {
        let (code, ran) = w.cortex(&["run", &format!("adv/{source}")]);
        assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
        ran
    }

    /// Every document of every prompt the stand-in model received, in order.
    fn shown(w: &World) -> Vec<String> {
        std::fs::read_to_string(w.root.join("prompt.log"))
            .unwrap_or_default()
            .split("\n=== Document — evidence id ")
            .skip(1)
            .map(|d| d.split("=== end of prompt ===").next().unwrap().to_string())
            .collect()
    }

    /// The document keys a source's state holds.
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

    /// The document shown to the model under `key`.
    fn shown_doc(w: &World, key: &str) -> String {
        let docs = shown(w);
        let source = format!("Source: file:{key}\n");
        docs.iter()
            .find(|d| d.contains(&source))
            .unwrap_or_else(|| panic!("no document {key} in {docs:#?}"))
            .clone()
    }

    const MD: &str = "{format: MarkdownSections, id: heading, text: [\"{body}\"], filters: []}";
    const CHAT: &str =
        "{format: JsonLines, id: id, text: [\"{text}\"], thread: thread, filters: []}";

    /// Doc: "a record whose field is absent opens the thread named by its own id, and a record whose
    /// field names a thread carries the text of every earlier record of that thread". A record with
    /// no thread field opens a thread; it must carry no `[context]`. The code looks up the thread named
    /// by its id and appends whatever is there, so with per-file ids, a record of `b.jsonl` carries the
    /// text of `a.jsonl`'s thread `1` as its own.
    #[test]
    fn a_record_without_a_thread_field_carries_no_other_records_text() {
        let a = concat!(
            r#"{"id":"1","text":"Alpha opens a thread about the Widget engine."}"#,
            "\n",
            r#"{"id":"2","thread":"1","text":"Alpha replies in that thread."}"#,
            "\n"
        );
        let b = concat!(
            r#"{"id":"1","text":"Bravo writes a standalone note."}"#,
            "\n"
        );
        let w = world(
            &[
                ("chat/a.jsonl", a.as_bytes()),
                ("chat/b.jsonl", b.as_bytes()),
            ],
            &source("chat", "chat", "*.jsonl", CHAT),
            "",
        );
        let ran = run(&w, "chat");
        assert_eq!(ran["detail"]["documents_new"], 3, "{ran}");
        let bravo = shown_doc(&w, &key(&root(&w), "chat/b.jsonl", "1"));
        assert!(
            !bravo.contains("[context]") && !bravo.contains("Alpha"),
            "b.jsonl#1 opens its own thread, yet carries a.jsonl's records:\n{bravo}"
        );
    }

    /// `RareName` replaces a capitalised word the batch's texts hold at most `rare_limit` (1) times.
    /// A name written once in a chat export is rare and is replaced when records carry no context.
    /// With `thread`, the same message is repeated as `[context]` in its reply, the batch counts the
    /// name twice, and the name reaches the model unreplaced.
    #[test]
    fn a_name_written_once_stays_pseudonymised_when_its_message_is_thread_context() {
        let chat = concat!(
            r#"{"id":"1","text":"Ask Zorblat about the Widget engine release."}"#,
            "\n",
            r#"{"id":"2","thread":"1","text":"will do, thanks."}"#,
            "\n"
        );
        let plain = "{format: JsonLines, id: id, text: [\"{text}\"], filters: []}";
        let sources = format!(
            "{}{}",
            source("plain", "chat", "*.jsonl", plain),
            source("threaded", "chat", "*.jsonl", CHAT)
        );
        let w = world(
            &[("chat/export.jsonl", chat.as_bytes())],
            &sources,
            "redaction:\n  classes: [RareName]\n  rules: []\n  rare_limit: 1\n",
        );
        // Control: without thread context the name is replaced.
        run(&w, "plain");
        let before = shown(&w);
        assert_eq!(before.len(), 2, "{before:#?}");
        assert!(before.iter().all(|d| !d.contains("Zorblat")), "{before:#?}");
        run(&w, "threaded");
        let after = shown(&w);
        assert_eq!(after.len(), 4, "{after:#?}");
        let leaked: Vec<_> = after.iter().filter(|d| d.contains("Zorblat")).collect();
        assert!(
            leaked.is_empty(),
            "a name written once reached the model because thread context repeats it:\n{leaked:#?}"
        );
    }

    /// A `~~~` line inside a ```` ``` ```` fence does not close it (CommonMark: a fence closes only
    /// on its own character). The code toggles on either, so the `##` line inside the fence becomes a
    /// heading, and the real `## Usage` after the fence is read as fenced and lost.
    #[test]
    fn a_fence_is_not_closed_by_the_other_fence_character() {
        let md = "## Setup\n\nWrite the file:\n\n```sh\ncat > notes.md <<'EOF'\n~~~\n## Not a heading\nEOF\n```\n\n## Usage\n\nRun the Widget engine.\n";
        let w = world(
            &[("notes/guide.md", md.as_bytes())],
            &source("notes", "notes", "*.md", MD),
            "",
        );
        run(&w, "notes");
        let r = root(&w);
        let mut expected = vec![
            key(&r, "notes/guide.md", "Setup"),
            key(&r, "notes/guide.md", "Usage"),
        ];
        expected.sort();
        assert_eq!(keys(&w, "notes"), expected);
    }

    /// A UTF-8 byte order mark (as editors on one platform write) before the first `## ` hides that
    /// heading: the first section is dropped silently, and named nowhere.
    #[test]
    fn a_byte_order_mark_does_not_hide_the_first_section() {
        let md = "\u{feff}## First\n\nAlpha section.\n\n## Second\n\nBravo section.\n";
        let w = world(
            &[("notes/bom.md", md.as_bytes())],
            &source("notes", "notes", "*.md", MD),
            "",
        );
        let ran = run(&w, "notes");
        let r = root(&w);
        assert_eq!(
            keys(&w, "notes"),
            [
                key(&r, "notes/bom.md", "First"),
                key(&r, "notes/bom.md", "Second")
            ],
            "{ran}"
        );
    }

    /// The same with a JSON-lines file: the first record is lost (it is at least named in `skipped`).
    #[test]
    fn a_byte_order_mark_does_not_lose_the_first_json_line() {
        let chat = format!(
            "\u{feff}{}\n{}\n",
            r#"{"id":"1","text":"First record."}"#, r#"{"id":"2","text":"Second record."}"#
        );
        let w = world(
            &[("chat/export.jsonl", chat.as_bytes())],
            &source("chat", "chat", "*.jsonl", CHAT),
            "",
        );
        let ran = run(&w, "chat");
        let r = root(&w);
        assert_eq!(
            keys(&w, "chat"),
            [
                key(&r, "chat/export.jsonl", "1"),
                key(&r, "chat/export.jsonl", "2")
            ],
            "{ran}"
        );
        assert!(ran["detail"]["skipped"].is_null(), "{ran}");
    }

    /// Two `## Notes` sections in one file share a key; the second is left out and named nowhere,
    /// so its text never reaches the model and no run says it was not read.
    #[test]
    fn a_repeated_heading_loses_no_section_silently() {
        let md = "## Notes\n\nAlpha section.\n\n## Other\n\nBravo section.\n\n## Notes\n\nCharlie section.\n";
        let w = world(
            &[("notes/dup.md", md.as_bytes())],
            &source("notes", "notes", "*.md", MD),
            "",
        );
        let ran = run(&w, "notes");
        let delivered = shown(&w).iter().any(|d| d.contains("Charlie section."));
        let named = ran["detail"]["skipped"]
            .as_array()
            .is_some_and(|s| s.iter().any(|l| l.as_str().unwrap_or("").contains("Notes")));
        assert!(
            delivered || named,
            "the second `## Notes` section was neither read nor named in skipped: {ran}"
        );
        // A repeated heading's later sections are numbered in file order, so none is lost.
        assert!(delivered, "{ran}");
        let r = root(&w);
        assert_eq!(
            keys(&w, "notes"),
            [
                key(&r, "notes/dup.md", "Notes"),
                key(&r, "notes/dup.md", "Notes-2"),
                key(&r, "notes/dup.md", "Other")
            ],
            "{ran}"
        );
        let second = shown_doc(&w, &key(&r, "notes/dup.md", "Notes-2"));
        assert!(second.contains("Charlie section."), "{second}");
    }

    /// A markdown file that is not UTF-8 text is named in `skipped` with the reason.
    #[test]
    fn a_markdown_file_that_is_not_utf8_is_named_in_skipped() {
        let mut md = b"## First\n\nAlpha ".to_vec();
        md.push(0xff);
        md.extend_from_slice(b" section.\n");
        let w = world(
            &[("notes/latin.md", &md)],
            &source("notes", "notes", "*.md", MD),
            "",
        );
        let ran = run(&w, "notes");
        let skipped = ran["detail"]["skipped"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert!(
            skipped.iter().any(|l| {
                let l = l.as_str().unwrap_or("");
                l.contains("latin.md") && l.contains("not UTF-8")
            }),
            "{ran}"
        );
        assert!(keys(&w, "notes").is_empty(), "{ran}");
    }

    /// A markdown file with no `##` heading yields no record and is named nowhere: a `**/*.md`
    /// source silently drops every such file.
    #[test]
    fn a_markdown_file_without_a_level_two_heading_is_not_lost_silently() {
        let md = "# Runbook\n\nRestart the Widget engine before the Gadget runtime.\n";
        let w = world(
            &[("notes/runbook.md", md.as_bytes())],
            &source("notes", "notes", "*.md", MD),
            "",
        );
        let ran = run(&w, "notes");
        let delivered = shown(&w)
            .iter()
            .any(|d| d.contains("Restart the Widget engine"));
        let named = ran["detail"]["skipped"].as_array().is_some_and(|s| {
            s.iter()
                .any(|l| l.as_str().unwrap_or("").contains("runbook.md"))
        });
        assert!(
            delivered || named,
            "runbook.md was neither read nor named in skipped: {ran}"
        );
        // The file is named in `skipped`, with the reason.
        assert!(named, "{ran}");
        assert!(
            ran["detail"]["skipped"]
                .as_array()
                .unwrap()
                .iter()
                .any(|l| l.as_str().unwrap_or("").contains("no ## section")),
            "{ran}"
        );
    }

    /// A setext level-two heading (`Title` over `---`) is a `##` section in markdown; it is not split.
    #[test]
    fn a_setext_level_two_heading_opens_a_section() {
        let md =
        "Setup\n-----\n\nInstall the Widget engine.\n\nUsage\n-----\n\nRun the Widget engine.\n";
        let w = world(
            &[("notes/setext.md", md.as_bytes())],
            &source("notes", "notes", "*.md", MD),
            "",
        );
        let ran = run(&w, "notes");
        let r = root(&w);
        assert_eq!(
            keys(&w, "notes"),
            [
                key(&r, "notes/setext.md", "Setup"),
                key(&r, "notes/setext.md", "Usage")
            ],
            "{ran}"
        );
    }

    /// One byte that is not UTF-8 in one line of a JSON-lines export drops every record of the file,
    /// and the run names nothing in `skipped`.
    #[test]
    fn one_invalid_byte_does_not_lose_a_whole_json_lines_file_silently() {
        let mut chat = Vec::new();
        chat.extend_from_slice(br#"{"id":"1","text":"First record."}"#);
        chat.push(b'\n');
        chat.extend_from_slice(br#"{"id":"2","text":"Broken "#);
        chat.push(0xff);
        chat.extend_from_slice(b"\"}\n");
        chat.extend_from_slice(br#"{"id":"3","text":"Third record."}"#);
        chat.push(b'\n');
        let w = world(
            &[("chat/export.jsonl", &chat)],
            &source("chat", "chat", "*.jsonl", CHAT),
            "",
        );
        let ran = run(&w, "chat");
        let named = ran["detail"]["skipped"].as_array().is_some_and(|s| {
            s.iter()
                .any(|l| l.as_str().unwrap_or("").contains("export.jsonl"))
        });
        assert!(
            keys(&w, "chat").len() == 2 || named,
            "records 1 and 3 were not read and nothing was named in skipped: {ran}"
        );
        // Only the bad line is left out, and it is named.
        let r = root(&w);
        assert_eq!(
            keys(&w, "chat"),
            [
                key(&r, "chat/export.jsonl", "1"),
                key(&r, "chat/export.jsonl", "3")
            ],
            "{ran}"
        );
        assert_eq!(
            ran["detail"]["skipped"],
            serde_json::json!([format!(
                "{}: line 2 is not UTF-8; skipped",
                r.join("chat/export.jsonl").display()
            )]),
            "{ran}"
        );
    }

    /// Held: an unchanged threaded export, run twice inside the refresh window, applies nothing the
    /// second time (the refresh hold is skipped for records, so a hash that drifted would re-extract).
    #[test]
    fn an_unchanged_threaded_export_is_not_extracted_again() {
        let chat = concat!(
            r#"{"id":"1","text":"Root about the Widget engine."}"#,
            "\n",
            r#"{"id":"2","thread":"1","text":"Reply one."}"#,
            "\r\n",
            r#"{"id":"3","thread":"1","text":"Reply two."}"#
        );
        let w = world(
            &[("chat/export.jsonl", chat.as_bytes())],
            &source("chat", "chat", "*.jsonl", CHAT),
            "redaction:\n  classes: [Email, RareName]\n  rules: []\n",
        );
        let first = run(&w, "chat");
        assert_eq!(first["detail"]["documents_new"], 3, "{first}");
        let second = run(&w, "chat");
        assert_eq!(second["detail"]["documents_new"], 0, "{second}");
    }

    /// Held: a credential in a heading is masked in the key the state holds and the model is shown.
    #[test]
    fn a_credential_in_a_heading_is_masked_in_the_key() {
        let token = format!("glpat-{}", "A1b2C3d4E5f6G7h8I9j0");
        let md = format!("## Rotate {token}\n\nRotate the Widget engine token.\n");
        let w = world(
            &[("notes/keys.md", md.as_bytes())],
            &source("notes", "notes", "*.md", MD),
            "",
        );
        run(&w, "notes");
        let keys = keys(&w, "notes");
        assert_eq!(keys.len(), 1, "{keys:?}");
        assert!(!keys[0].contains(&token), "{keys:?}");
        assert!(shown(&w).iter().all(|d| !d.contains(&token)));
    }
}
