//! Adversary cases for unit `redact` of wave 20261030a (`story:redaction-patterns-hold`,
//! `story:mask-quoted-credentials`), run through the public `cortex_cli::redact`,
//! `cortex_cli::mask` and `cortex_cli::state` APIs a run uses. Each breaking case is marked
//! `#[ignore = "adversary: …"]` so the branch stays green; run them with `--ignored`.
//! Credential-shaped values are assembled at runtime so no such literal sits in the source.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use cortex_cli::mask::{mask, mask_key};
use cortex_cli::redact::{compile, Redactor};
use cortex_cli::state::{Seen, SeenState, FORMAT};
use cortex_model::instance as m;

/// A credential-shaped value, built at runtime.
fn secret() -> String {
    format!("Kd9{}", "xQ2".repeat(4))
}

fn redactor(classes: &[m::RedactionClass]) -> Redactor {
    let policy = m::RedactionPolicy {
        classes: classes.to_vec(),
        rules: Vec::new(),
        known_names: None,
        rare_limit: None,
        refuse_if_left: None,
    };
    compile(Some(&policy))
        .expect("a valid policy")
        .expect("a policy that redacts")
}

const BUILT_IN: [m::RedactionClass; 5] = [
    m::RedactionClass::Url,
    m::RedactionClass::Email,
    m::RedactionClass::PaymentCard,
    m::RedactionClass::IpAddress,
    m::RedactionClass::Phone,
];

/// `text` through one batch of a redactor for `classes`, and the counts it took.
fn run(classes: &[m::RedactionClass], text: &str) -> (String, BTreeMap<String, usize>) {
    let r = redactor(classes);
    let mut counts = r.counts();
    let out = r.redact(text, &mut counts);
    (out, counts)
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

// ---- Phone: `/` after an international area code -------------------------------------------

/// The unit's CHANGELOG says "a date after `+`, with two [`/`], is still not a number". An ISO
/// date written year first after a `+1` vote is: `+1 2024/10` has seven digits, and the `/30`
/// after it stays in the text. Before the change, `+1 2024` (five digits) was refused.
#[test]
#[ignore = "adversary: an ISO date after +1 is replaced as a phone number (+1 2024/10/30)"]
fn an_iso_date_after_a_plus_one_is_not_a_phone_number() {
    let text = "Votes: +1 2024/10/30 so far.";
    let (out, counts) = run(&BUILT_IN, text);
    assert_eq!(out, text);
    assert_eq!(counts["Phone"], 0, "{out}");
}

// ---- Url: square brackets inside a link -----------------------------------------------------

/// A citation after a link in parentheses is not part of the link: before the change the link
/// ended at `[` and its unbalanced `)` was trimmed; now the `)` and `[1]` are swallowed.
#[test]
#[ignore = "adversary: a [1] citation after a link swallows the closing parenthesis and the citation"]
fn a_citation_after_a_parenthesised_link_stays_outside_it() {
    for (text, replaced) in [
        (
            "Read it (see https://example.org/a)[1] today.",
            "Read it (see [Url-1])[1] today.",
        ),
        (
            "Read https://example.org/a.[2] Then stop.",
            "Read [Url-1].[2] Then stop.",
        ),
    ] {
        let (out, counts) = run(&[m::RedactionClass::Url], text);
        assert_eq!(out, replaced, "{text}");
        assert_eq!(counts["Url"], 1, "{text}");
    }
}

/// Two Markdown links joined by a comma, with no space, are two links. Now the first link runs
/// through `),[b](` into the second, and the label `b` is hidden inside one placeholder.
#[test]
#[ignore = "adversary: two Markdown links without a space between them become one link"]
fn two_markdown_links_joined_without_a_space_are_two_links() {
    let text = "Sources: [a](https://example.org/1),[b](https://example.org/2).";
    let (out, counts) = run(&[m::RedactionClass::Url], text);
    assert_eq!(out, "Sources: [a]([Url-1]),[b]([Url-2]).");
    assert_eq!(counts["Url"], 2, "{out}");
}

/// Links written in brackets, back to back with no space, are found in time linear in the
/// text: each match now runs to the end of the run of non-space text and `link` cuts it at `]`,
/// so every link scans the rest of the text again. Before the change a match ended at `[`.
#[test]
#[ignore = "adversary: bracketed links back to back are found in quadratic time"]
fn bracketed_links_back_to_back_are_found_in_linear_time() {
    let r = redactor(&[m::RedactionClass::Url]);
    let unit = "[https://a.example/b]";
    let small = unit.repeat(1 << 9);
    let large = unit.repeat(1 << 11);
    let _ = r.redact(unit, &mut r.counts());
    let t_small = fastest(|| {
        let _ = r.redact(&small, &mut r.counts());
    });
    let t_large = fastest(|| {
        let _ = r.redact(&large, &mut r.counts());
    });
    assert!(
        t_large < t_small * 8,
        "4x the text took {:.1}x the time ({t_small:?} -> {t_large:?})",
        t_large.as_secs_f64() / t_small.as_secs_f64()
    );
}

/// No input panics the built-in classes: a sweep over generated strings with multi-byte
/// characters at every position next to the link, phone and bracket shapes.
#[test]
fn no_generated_text_panics_the_built_in_classes() {
    let pieces = [
        "é",
        "中",
        "😀",
        "\u{a0}",
        "café.fr/",
        "中国.cn/",
        "www.",
        "ww",
        "w",
        "https://",
        "://",
        "[",
        "]",
        "(",
        ")",
        "[Url-1]",
        "[masked:x]",
        "+49 30/",
        "1234567",
        "/",
        ".",
        "@",
        "a",
        "0",
        " ",
        "?q[a]=",
        "ü.de/x",
        "+1 2/3/",
        "00",
    ];
    let r = redactor(&BUILT_IN);
    let mut x: u64 = 0x2545_f491_4f6c_dd1d;
    for _ in 0..20_000 {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let len = 1 + (x % 9) as usize;
        let mut y = x;
        let text: String = (0..len)
            .map(|_| {
                y = y
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                pieces[(y >> 33) as usize % pieces.len()]
            })
            .collect();
        let out = std::panic::catch_unwind(|| {
            let _ = r.redact(&text, &mut r.counts());
            let _ = mask(&text);
            let _ = mask_key(&text);
        });
        assert!(out.is_ok(), "panicked on {text:?}");
    }
}

// ---- mask: `url-password` runs to the last `@` --------------------------------------------

/// The password runs to the last `@` before the path, and a URL without a path ends at no `/`:
/// in compact JSON the rule now runs through the host, the closing quote and the next key to
/// the `@` of an address after it. Before the change only `pw` was masked.
#[test]
#[ignore = "adversary: url-password runs past the host to a later @ and masks the host and the next JSON key"]
fn a_url_password_does_not_run_past_its_host_to_a_later_at_sign() {
    let pw = secret();
    let text = format!(r#"{{"dsn":"postgres://app:{pw}@db:5432","owner":"ops@corp.example"}}"#);
    let (out, n) = mask(&text);
    assert!(!out.contains(&pw), "{out}");
    assert_eq!(n, 1, "{out}");
    assert!(out.contains("@db:5432\""), "the host is masked: {out}");
    assert!(out.contains("\"owner\""), "the next key is masked: {out}");
}

/// A masking pass costs time linear in the text: a text of many `BEGIN … PRIVATE KEY` lines
/// with no `END` line (4x the text) took 16x the time; measured in release on 1 MiB, 24.4 s
/// against 6.5 ms before the change. The `BEGIN … END` alternative before the cut-key one
/// scans to the end of the text for every `BEGIN`.
#[test]
#[ignore = "adversary: private-key masking is quadratic in BEGIN lines without an END line"]
fn begin_lines_without_an_end_line_are_masked_in_linear_time() {
    let begin = format!("-----BEGIN PRIVATE {}-----\n", "KEY");
    let small = begin.repeat(1 << 9);
    let large = begin.repeat(1 << 11);
    let _ = mask(&begin);
    let t_small = fastest(|| {
        let _ = mask(&small);
    });
    let t_large = fastest(|| {
        let _ = mask(&large);
    });
    assert!(
        t_large < t_small * 8,
        "4x the text took {:.1}x the time ({t_small:?} -> {t_large:?})",
        t_large.as_secs_f64() / t_small.as_secs_f64()
    );
}

/// A key a run stored before this change kept the part of a credential the old mask missed
/// (`[masked:url-password]@ss-<rest>` for a raw `@`, `[masked:secret-assignment]%zz<rest>` for a
/// raw `%`). The CHANGELOG says such an old key "is dropped from the source's state". It is not:
/// the masked value starts with `[masked:`, so `mask_key` counts nothing and `SeenState::load`
/// keeps the key, and the rest of the credential, for good.
#[test]
#[ignore = "adversary: a key stored before the change keeps the rest of the password in the state file"]
fn a_stored_key_holding_the_rest_of_a_credential_is_dropped_on_load() {
    let rest = secret();
    let old_keys = [
        format!("postgres://app:[masked:url-password]@ss-{rest}@db.internal:5432/x"),
        format!("https://example.org/a?access_token=[masked:secret-assignment]%zz{rest}&page=2"),
    ];
    let mut state = SeenState {
        format: FORMAT.into(),
        ..SeenState::default()
    };
    for k in &old_keys {
        state.documents.insert(
            k.clone(),
            Seen {
                hash: None,
                applied_at: 1,
                unlinked_hash: None,
            },
        );
        state.child_failures.insert(k.clone(), 2);
    }
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("seen.json");
    state.save(&path).expect("saved");
    let loaded = SeenState::load(&path).expect("loaded");
    let kept: Vec<&String> = loaded
        .documents
        .keys()
        .chain(loaded.child_failures.keys())
        .filter(|k| k.contains(&rest))
        .collect();
    assert!(kept.is_empty(), "kept after load: {kept:?}");
}

/// In a key, a `%` that starts no escape is part of the value, and `%%26` reads as one: the
/// encoded `&` after it no longer ends the value, so two URLs that differ after it become one
/// key and the second document is left out as a repeat.
#[test]
#[ignore = "adversary: a raw % before %26 swallows the rest of the query into the masked value"]
fn a_raw_percent_before_an_encoded_ampersand_keeps_two_keys_apart() {
    let t = secret();
    let a = mask_key(&format!("https://example.org/a?access_token={t}%%26page=2"));
    let b = mask_key(&format!("https://example.org/a?access_token={t}%%26page=3"));
    assert_ne!(a.0, b.0);
}

/// A stored key is `mask_key` of the fetched key, and `SeenState::load` drops a key `mask_key`
/// still changes; so a key whose masked form `mask_key` changes again is dropped on every load,
/// its document is read again on every run and its child-failure count restarts every run. The
/// `[masked:secret-assignment]` token holds a `:` that `url-password` reads as the user's end;
/// `%3A` as a separator makes a key that was left alone before the change one such key.
#[test]
#[ignore = "adversary: mask_key is not idempotent on a %3A-separated key after a scheme (unreached shape)"]
fn a_masked_key_is_not_masked_again() {
    let v = "abcdefgh";
    let key = format!("x://password%3a{v}&u@h");
    let (once, _) = mask_key(&key);
    assert_eq!(mask_key(&once), (once.clone(), 0), "{key}");
}

// ---- mask: shapes next to the ones fixed ----------------------------------------------------

/// A JSON object written inside a JSON string, `{\"password\": \"…\"}`, as a log line or a code
/// sample on a page holds it. The cut private key accepts `\n` "as in a JSON string"; the quoted
/// name does not accept `\"`.
#[test]
#[ignore = "adversary: an escaped JSON-quoted password is not masked (pre-existing)"]
fn an_escaped_json_quoted_password_is_masked() {
    let t = secret();
    let text = format!(r#"Log: {{\"password\": \"{t}\"}}"#);
    let (out, _) = mask(&text);
    assert!(!out.contains(&t), "{out}");
}

/// `%253D`, `=` encoded twice, as a URL inside another URL's query carries it.
#[test]
#[ignore = "adversary: a twice percent-encoded separator is not masked (pre-existing)"]
fn a_twice_encoded_separator_is_masked() {
    let t = secret();
    let key = format!("https://example.org/r?next=%252Fapi%253Faccess_token%253D{t}");
    assert!(!mask_key(&key).0.contains(&t), "{key}");
    assert!(!mask(&key).0.contains(&t), "{key}");
}

/// A user part holding a raw `@`, as an address used as the user name is often written.
#[test]
#[ignore = "adversary: a URL password after a user name with a raw @ is not masked (pre-existing)"]
fn a_url_password_after_a_user_with_an_at_sign_is_masked() {
    let t = secret();
    let url = format!("ftp://jane@corp.example:{t}@files.example/x");
    assert!(!mask(&url).0.contains(&t), "{url}");
    assert!(!mask_key(&url).0.contains(&t), "{url}");
}
