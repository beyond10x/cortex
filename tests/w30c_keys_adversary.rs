//! Adversary pass on `story:credential-key-spellings` (wave 20261030c, unit `keys`).
//!
//! Each case drives `mask::mask`, `mask::mask_key`, the `Credential` class of a redaction policy,
//! `state::SeenState` or `structured::Source` with a shape the unit's new classes read: a value
//! left partly in clear, content that is no credential masked, a stored key or a record identity
//! that changes, idempotence over mixed shapes, and time on long runs of the new shapes' parts.
//! A case that breaks is marked `#[ignore = "adversary: …"]` so the branch stays green; run it
//! with `--ignored`. Credential-shaped values and names are assembled at run time so no such
//! literal sits in the source.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use cortex_cli::ekr::Operation;
use cortex_cli::mask::{mask, mask_key};
use cortex_cli::redact::{compile, Redactor};
use cortex_cli::state::{Seen, SeenState, FORMAT};
use cortex_cli::structured::Source;
use cortex_model::instance as m;
use serde_json::json;

fn cat(parts: &[&str]) -> String {
    parts.concat()
}

/// `password`, assembled.
fn pw() -> String {
    cat(&["pass", "word"])
}

/// A password-shaped value, built at run time.
fn v() -> String {
    cat(&["Hq7adv2", "Wm4Lp", "9Zr3"])
}

/// The parts of `value` that must not survive: every run of five of its characters.
fn keeps_a_part(text: &str, value: &str) -> bool {
    let chars: Vec<char> = value.chars().filter(|c| !c.is_whitespace()).collect();
    chars
        .windows(5)
        .map(|w| w.iter().collect::<String>())
        .any(|p| text.contains(p.as_str()))
}

fn policy() -> Redactor {
    let policy = m::RedactionPolicy {
        classes: vec![m::RedactionClass::Credential],
        rules: Vec::new(),
        known_names: None,
        rare_limit: None,
        refuse_if_left: Some(vec![m::RedactionClass::Credential]),
    };
    compile(Some(&policy))
        .expect("a valid policy")
        .expect("a policy that redacts")
}

/// What the model is shown of `text` by the policy alone, and what `left` still finds.
fn policy_only(r: &Redactor, text: &str) -> (String, Vec<&'static str>) {
    let mut batch = r.batch();
    batch.reserve(text);
    let shown = batch.replace(text);
    let left = batch.left(&shown).into_iter().collect();
    (shown, left)
}

/// The fastest of three runs of `f`.
fn fastest(mut f: impl FnMut()) -> Duration {
    (0..3)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed()
        })
        .min()
        .expect("three runs")
}

// ---- values left partly in clear -------------------------------------------------------------

/// A flag's value in quotes with spaces (`--db-password "<four words>"`, as a shell writes a
/// passphrase): `FLAG_VALUE` stops at the first space, so `mask` keeps the last three words.
#[test]
#[ignore = "adversary: a quoted flag value with spaces keeps all but its first word in clear"]
fn a_quoted_flag_value_with_spaces_is_masked_whole() {
    let value = cat(&["Tr0ub4dor ", "horse9 ", "battery7 ", "staple3"]);
    let r = policy();
    let mut leaks = Vec::new();
    for q in ["\"", "'"] {
        let text = format!("psql --db-{} {q}{value}{q} -h db", pw());
        let (masked, _) = mask(&text);
        let (shown, left) = policy_only(&r, &text);
        if keeps_a_part(&masked, &value) {
            leaks.push(format!("mask: {masked}"));
        }
        if keeps_a_part(&shown, &value) && !left.contains(&"Credential") {
            leaks.push(format!("policy: {shown}"));
        }
    }
    assert!(leaks.is_empty(), "{leaks:#?}");
}

/// An XML password as a SOAP header (`<wsse:Password …>`) or a CDATA section writes it: the
/// element rule reads neither a namespace prefix nor a value that starts with `<`, so the whole
/// password is kept in clear.
#[test]
#[ignore = "adversary: <ns:Password> and a CDATA password element keep the whole value in clear"]
fn a_namespaced_or_cdata_password_element_is_masked() {
    let s = v();
    let p = cat(&["Pass", "word"]);
    let r = policy();
    let mut leaks = Vec::new();
    for text in [
        format!("<wsse:{p} Type=\"#{p}Text\">{s}</wsse:{p}>"),
        format!("<{p}><![CDATA[{s}]]></{p}>"),
    ] {
        let (masked, n) = mask(&text);
        let (shown, left) = policy_only(&r, &text);
        if keeps_a_part(&masked, &s) {
            leaks.push(format!("mask ({n}): {masked}"));
        }
        if keeps_a_part(&shown, &s) && !left.contains(&"Credential") {
            leaks.push(format!("policy: {shown}"));
        }
    }
    assert!(leaks.is_empty(), "{leaks:#?}");
}

// ---- content that is no credential -----------------------------------------------------------

/// A placeholder `<password>` in prose is kept only while no closing tag follows it in the
/// text. In HTML, any later `</…>` (`</p>`, `</li>`) closes the "element", and the prose up to it
/// is masked.
#[test]
#[ignore = "adversary: a <password> placeholder before any later closing tag masks the prose after it"]
fn a_placeholder_element_before_a_later_closing_tag_keeps_the_prose() {
    let mut changed = Vec::new();
    for text in [
        format!("<p>Type <{}> and press Enter.</p>", pw()),
        format!("<li>Run login <{}> to sign in</li>", pw()),
    ] {
        let (out, n) = mask(&text);
        if (out.as_str(), n) != (text.as_str(), 0) {
            changed.push(format!("{text} -> {out}"));
        }
    }
    assert!(changed.is_empty(), "{changed:#?}");
}

/// Code and settings whose name ends in `_pass` and that hold no credential: a render pass, a
/// compiler pass, a CI hook. `(?:[a-z0-9]+_)+pass` reads each as a password's name, and the code
/// or the setting after it is masked. They were kept at 6aaa4800.
#[test]
#[ignore = "adversary: `<word>_pass` masks render passes, compiler passes and CI hooks"]
fn pass_suffixed_names_that_hold_no_credential_are_kept() {
    let r = policy();
    let mut changed = Vec::new();
    for text in [
        "let shadow_pass = encoder.begin_render_pass(&desc);",
        "first_pass: tokenize_input",
        "on_pass: notify-team",
        "struct Config { render_pass: Option<RenderPass>, }",
    ] {
        let (out, n) = mask(text);
        if n != 0 {
            changed.push(format!("mask: {text} -> {out}"));
        }
        let (shown, _) = policy_only(&r, text);
        if shown != text {
            changed.push(format!("policy: {text} -> {shown}"));
        }
    }
    assert!(changed.is_empty(), "{changed:#?}");
}

/// Command-line help and commands that name a flag and hold no credential: a metavar
/// (`--password PASSWORD`, `--db-password <DB_PASSWORD>`), a variable (`--api-key $API_KEY`) and
/// a BuildKit secret mount (`--secret id=…,src=…`, which names a file). `mask` masks each; the
/// policy keeps the first three as placeholders.
#[test]
#[ignore = "adversary: flag metavars, $VARS and `docker build --secret id=…,src=…` are masked"]
fn flag_metavars_variables_and_secret_mounts_are_kept() {
    let p = pw();
    let up = p.to_uppercase();
    let mut changed = Vec::new();
    for text in [
        format!("  --{p} {up}   the {p} to use"),
        format!("  --db-{p} <DB_{up}>  The database {p}"),
        "curl --api-key $API_KEY https://api.example".to_string(),
        "docker build --secret id=npmrc,src=$HOME/.npmrc .".to_string(),
    ] {
        let (out, n) = mask(&text);
        if n != 0 {
            changed.push(format!("{text} -> {out}"));
        }
    }
    assert!(changed.is_empty(), "{changed:#?}");
}

/// `prose_word` reads only ASCII lower-case letters as prose, so a lower-case word with an
/// accent after a flag is masked as a value (`--password ändern`, German for "change").
#[test]
#[ignore = "adversary: a lower-case non-ASCII word after a flag is masked as a value"]
fn a_lower_case_word_with_an_accent_after_a_flag_is_prose() {
    let text = format!("Mit --{} ändern Sie das Kennwort.", pw());
    assert_eq!(mask(&text), (text.clone(), 0));
}

// ---- re-keying -------------------------------------------------------------------------------

/// Stored keys that hold no credential and that `mask_key` left alone at 6aaa4800: a search for
/// `render_pass=…`, a ticket's `season_pass` id. `SeenState::load` now drops each (the document
/// is read again under a new key, a second evidence document), and two such keys that differ
/// only in that value are one key (the second document is left out as a repeat).
#[test]
#[ignore = "adversary: non-credential `<word>_pass=` keys are dropped on load and merged"]
fn keys_that_hold_no_credential_keep_their_stored_form() {
    let keys = [
        "https://example.org/search?q=render_pass%3Dvulkan",
        "https://example.org/tickets?season_pass=SP-2026-0001&page=1",
        "https://example.org/tickets?season_pass=SP-2026-0002&page=1",
    ];
    let mut state = SeenState {
        format: FORMAT.into(),
        ..SeenState::default()
    };
    for k in keys {
        state.documents.insert(
            k.to_string(),
            Seen {
                hash: None,
                applied_at: 1,
                unlinked_hash: None,
            },
        );
    }
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("seen.json");
    state.save(&path).expect("saved");
    let loaded = SeenState::load(&path).expect("loaded");
    let dropped: Vec<&str> = keys
        .iter()
        .copied()
        .filter(|k| !loaded.documents.contains_key(*k))
        .collect();
    let distinct: BTreeSet<String> = keys.iter().map(|k| mask_key(k).0).collect();
    assert!(
        dropped.is_empty() && distinct.len() == keys.len(),
        "dropped on load: {dropped:?}; {} keys mask to {} distinct: {distinct:?}",
        keys.len(),
        distinct.len()
    );
}

const CHILD_OPERATION: &str = "storage.get_secret_key";

fn source_yaml(adapter: &str, operation: &str, child: bool) -> m::StructuredSource {
    let children = if child {
        format!(
            "            children:
              - operation: {CHILD_OPERATION}
                input: {{project: \"{{id}}\"}}
                records: \"$.keys\"
                parent: IN_PROJECT
                mapping:
                  node_type: Key
                  id: \"$.name\"
                  name: \"$.name\"
                  aliases: []
                  properties: [{{property: commit, path: \"$.commit\"}}]
                  relations: []
"
        )
    } else {
        String::new()
    };
    let yaml = format!(
        r#"format: cortex.instance/1
name: c
description: Test brain.
ekr: {{version: "0.0.32", bin: "ekr"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
  - name: people
    schedule: daily
    settings:
      kind: structured
      value:
        input:
          from: connectors
          value:
            adapter: "{adapter}"
            connection: conn_test
            operation: "{operation}"
            inputs: [{{}}]
{children}        records: "$.people"
        mapping:
          node_type: Person
          id: "$.id"
          name: "$.name"
          aliases: []
          properties: [{{property: commit, path: "$.commit"}}]
          relations: []
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
serve: {{view_port: 18995}}
"#
    );
    let spec = cortex_cli::spec::parse(&yaml).expect("a valid spec");
    match spec.sources.into_iter().next().unwrap().settings {
        m::SourceSettings::Structured(st) => st,
        _ => unreachable!(),
    }
}

/// `Source::ended` on one active value whose evidence names `old`, a record the run no longer
/// lists.
fn ended_for(s: &Source, old: &str) -> Vec<Operation> {
    let snapshot = json!({"graph": {"graph": {
        "assertions": {"a-old": {"id": "a-old", "subject": {"Node": "n1"},
            "predicate": {"Property": "p-commit"},
            "object": {"Value": {"value_kind": "String", "value": "c1"}},
            "evidence": ["e-1"], "lifecycle": "Active",
            "valid_time": {"from": 1, "to": null}}},
        "edges": {}, "nodes": {},
        "evidence": {"e-1": {"source": {"HumanStatement": {"identity": format!("record:{old}")}}}},
    }}});
    let ontology = json!({"node_types": [{"properties": [{"id": "p-commit", "name": "commit"}]}],
        "edge_types": []});
    s.ended(
        &snapshot,
        &ontology,
        &BTreeMap::new(),
        &BTreeSet::new(),
        &[],
    )
}

/// A child operation that now ends in a name (`…get_secret_key`) is written in hex in its
/// records' identities, while the source's own prefix is unchanged, so `Source::earlier` is
/// empty. A child value applied at 6aaa4800 cites `<prefix>/<operation>:<parent>:<id>` in
/// clear; no prefix the source holds owns it, so when the record is gone its value is never
/// ended, and the re-read record's value stands beside it.
#[test]
#[ignore = "adversary: a child operation newly in hex orphans the child values applied at 6aaa4800"]
fn a_child_value_applied_before_its_operation_turned_hex_still_ends() {
    let st = source_yaml("cloud", "projects.list", true);
    let s = Source::new(&st, None);
    let old = format!("cloud:projects.list/{CHILD_OPERATION}:P-1:k1");
    let now = s.child_identity(CHILD_OPERATION, "P-1", "k1");
    assert_ne!(now.as_deref(), Some(old.as_str()), "the identity changed");
    let ops = ended_for(&s, &old);
    assert!(
        matches!(&ops[..], [Operation::Retract { assertion, .. }] if assertion == "a-old"),
        "identity at 6aaa4800 {old}, now {now:?}; earlier {:?}; ended {ops:?}",
        s.earlier
    );
}

/// A prefix with one part in hex at 6aaa4800 (`vault.secret`) and one part newly in hex
/// (`onepassword`, glued): `Source::earlier` holds the prefix with no part in hex, not the one
/// 6aaa4800 wrote, so a value applied then is never ended.
#[test]
#[ignore = "adversary: a prefix partly in hex at 6aaa4800 is not among `earlier`; its values never end"]
fn a_value_applied_under_a_partly_hex_prefix_still_ends() {
    let st = source_yaml(&cat(&["one", "pass", "word"]), "vault.secret", false);
    let s = Source::new(&st, None);
    let hex_op = format!("%x{}", cortex_cli::state::hex(b"vault.secret"));
    let old = format!("{}:{hex_op}:P-1", cat(&["one", "pass", "word"]));
    let ops = ended_for(&s, &old);
    assert!(
        matches!(&ops[..], [Operation::Retract { assertion, .. }] if assertion == "a-old"),
        "identity at 6aaa4800 {old}, now {}; earlier {:?}; ended {ops:?}",
        s.identity("P-1"),
        s.earlier
    );
}

// ---- idempotence -----------------------------------------------------------------------------

/// Texts mixing the new shapes (glued and cut names, markup, elements, flags, URL passwords with
/// `,`), joined by separators, from a fixed seed: masking a masked text changes nothing more.
#[test]
fn masking_mixed_new_shapes_twice_changes_nothing_more() {
    let p = pw();
    let s = v();
    let frags: Vec<String> = vec![
        format!("PG{}", p.to_uppercase()),
        "DB_PASS".into(),
        "secret_key".into(),
        format!("_{p}"),
        format!("db-{p}"),
        "`".into(),
        "**".into(),
        "*".into(),
        "_".into(),
        "~".into(),
        ":".into(),
        "=".into(),
        " ".into(),
        "\\\"".into(),
        "'".into(),
        format!("<{p}>"),
        format!("</{p}>"),
        "<".into(),
        "</".into(),
        format!("--{p}"),
        "--api-key".into(),
        "-".into(),
        s.clone(),
        "%2B".into(),
        "&".into(),
        "#".into(),
        "@".into(),
        ",".into(),
        ";".into(),
        "https://ops@corp.example:".into(),
        "[masked:secret-assignment]".into(),
        "\n".into(),
        "option".into(),
        "é".into(),
    ];
    let mut seed: u64 = 0x5eed_c0de_2026_1030;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let mut broken = Vec::new();
    for _ in 0..20_000 {
        let n = 2 + (next() % 10) as usize;
        let text: String = (0..n)
            .map(|_| frags[(next() % frags.len() as u64) as usize].as_str())
            .collect();
        let (once, _) = mask(&text);
        let twice = mask(&once);
        if twice != (once.clone(), 0) {
            broken.push(format!("mask {text:?}: {once:?} -> {twice:?}"));
        }
        let (once, _) = mask_key(&text);
        let twice = mask_key(&once);
        if twice != (once.clone(), 0) {
            broken.push(format!("mask_key {text:?}: {once:?} -> {twice:?}"));
        }
        if broken.len() >= 5 {
            break;
        }
    }
    assert!(broken.is_empty(), "{broken:#?}");
}

// ---- time ------------------------------------------------------------------------------------

/// Runs of the new shapes' parts at 256 KiB and 1 MiB (green at 1 and 4 MiB too, 153 s in a
/// debug build): masking and the policy take time linear in
/// the text (4x the text, under 8x the time).
#[test]
fn runs_of_the_new_shapes_parts_take_linear_time() {
    let p = pw();
    let units: Vec<(&str, String)> = vec![
        ("-", "-".into()),
        ("_", "_".into()),
        ("*", "*".into()),
        ("`", "`".into()),
        ("<", "<".into()),
        ("flag", format!("--{p} ")),
        ("flag-prose", format!("--{p} option ")),
        ("dash-name", format!("a-{p}-")),
        ("open-element", format!("<{p}>")),
        ("element-value", format!("<{p}>x")),
        ("markup-name", format!("**_{p}_**")),
        ("cut-name", "a_pass_".into()),
    ];
    let r = policy();
    let mut slow = Vec::new();
    for (label, unit) in &units {
        let small = unit.repeat((1 << 18) / unit.len());
        let large = unit.repeat((1 << 20) / unit.len());
        for (what, f) in [
            ("mask", &(|t: &str| drop(mask(t))) as &dyn Fn(&str)),
            ("mask_key", &|t: &str| drop(mask_key(t))),
            ("policy", &|t: &str| drop(policy_only(&r, t))),
        ] {
            let t_small = fastest(|| f(&small));
            let t_large = fastest(|| f(&large));
            if t_large > t_small * 8 && t_large > Duration::from_millis(200) {
                slow.push(format!("{what} {label}: {t_small:?} -> {t_large:?}"));
            }
        }
    }
    assert!(slow.is_empty(), "{slow:#?}");
}
