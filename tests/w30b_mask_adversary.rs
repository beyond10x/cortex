//! Adversary cases for unit `mask` of wave 20261030b (`story:mask-percent-before-encoded-amp`),
//! run through the public `cortex_cli::mask` and `cortex_cli::state` APIs a run uses. The two
//! cases that broke were committed `#[ignore]`d and run since their fix in the same story.
//! Credential-shaped values are assembled at runtime so no such literal sits in the source.

use std::time::{Duration, Instant};

use cortex_cli::mask::{mask, mask_key};
use cortex_cli::sources::{Document, Origin};
use cortex_cli::state::{Seen, SeenState, FORMAT};

/// The first part of a credential, built at runtime.
fn head() -> String {
    format!("Kd9{}", "xQ2".repeat(2))
}

/// The rest of a credential, after a `%` in it, built at runtime.
fn tail() -> String {
    format!("Rt7{}", "wZ4".repeat(3))
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

/// A raw `%` and one hex digit in front of another `%` (`%2%41`, `%a%41`, `%2%x`, `%252%41`) is
/// no encoded `&` or `#`, yet `key_unit!` now reads no unit at `%2%` (its last alternative
/// excludes `%` after the digit), so the value ends there and the rest of the credential is
/// stored in the key, in clear. Before the change (`7a94960c`) `%2%` was one unit and the whole
/// value was masked. A masked key shown in a log line keeps it too: `mask` takes the token
/// followed by `%2%…` as already masked.
#[test]
fn a_percent_and_one_hex_digit_before_a_percent_stays_in_a_key_value() {
    let (h, t) = (head(), tail());
    for mid in ["%2%41", "%a%41", "%2%x", "%252%41", "%2%%41"] {
        let key = format!("https://example.org/a?access_token={h}{mid}{t}&page=2");
        let (masked, _) = mask_key(&key);
        assert!(!masked.contains(&t), "stored key keeps the rest: {masked}");
        let line = format!("stopped: {masked}");
        let (logged, _) = mask(&line);
        assert!(!logged.contains(&t), "log line keeps the rest: {logged}");
    }
}

/// A URL with a user name and a port, followed without a space by `,` or `;` and an address, as
/// a CSV line or a header list writes it. The user name may now hold a raw `@`, so the port and
/// the next address's local part read as a password and are masked. Before the change neither
/// was.
#[test]
fn a_url_with_a_user_and_a_port_before_a_comma_and_an_address_is_kept() {
    for kept in [
        "smtp://ops@mail.example:587,admin@example.org",
        "see https://jane@corp.example:8443;bob@example.org",
    ] {
        assert_eq!(mask(kept), (kept.to_string(), 0), "{kept}");
        assert_eq!(mask_key(kept), (kept.to_string(), 0), "{kept}");
    }
}

/// The re-key measured. A 0.7.0 run stored `access_token=<t>%%26page=2` as
/// `access_token=[masked:secret-assignment]` (the `%%26page=2` inside the masked value), one key
/// for every page. After the change that stored key is kept on load (it is masked, so nothing
/// drops it), matches no fetched key, and each page's document is wanted again, under its own
/// key. The old key stays in the state file, matched by nothing.
#[test]
fn a_key_stored_by_0_7_0_for_a_percent_before_an_encoded_ampersand_is_kept_and_matches_nothing() {
    let t = head() + &tail();
    let base = "https://example.org/a?access_token=";
    let old = format!("{base}[masked:secret-assignment]");
    let mut state = SeenState {
        format: FORMAT.into(),
        ..SeenState::default()
    };
    state.documents.insert(
        old.clone(),
        Seen {
            hash: Some("h".into()),
            applied_at: 1,
            unlinked_hash: None,
        },
    );
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("seen.json");
    state.save(&path).expect("saved");
    let loaded = SeenState::load(&path).expect("loaded");
    assert!(loaded.documents.contains_key(&old), "the old key is kept");
    let mut keys = Vec::new();
    for page in ["2", "3"] {
        let (key, n) = mask_key(&format!("{base}{t}%%26page={page}"));
        assert_eq!(n, 1);
        assert_eq!(
            key,
            format!("{base}[masked:secret-assignment]%%26page={page}")
        );
        let doc = Document {
            key: key.clone(),
            origin: Origin::Url,
            title: None,
            description: None,
            published: None,
            text: "text".into(),
            hash: None,
        };
        assert!(!loaded.documents.contains_key(&key));
        assert!(loaded.wants(&doc, 2, 30), "{key} is read again");
        keys.push(key);
    }
    assert_ne!(keys[0], keys[1]);
}

/// Masking is idempotent over generated keys and texts made of the shapes the change touched
/// (`%`, `%%`, `%25`, `%2`, `%26`, `%2526`, `%253d`, `\"`, a user name with `@`), with a fixed
/// seed: a key masked once is masked to itself, as a key and in a text, and a text likewise.
#[test]
fn masking_is_idempotent_over_generated_percent_and_quote_shapes() {
    let s = head() + &tail();
    let frags: Vec<&str> = vec![
        "access_token",
        "password",
        "api_key",
        "secret",
        "=",
        ":",
        "%3a",
        "%3d",
        "%253d",
        "%253a",
        "%",
        "%%",
        "%25",
        "%26",
        "%2526",
        "%23",
        "%2523",
        "%2",
        "%a",
        "%41",
        "%21",
        "&",
        "#",
        "@",
        "x://",
        "https://",
        "u",
        "abcdefgh",
        "\\\"",
        "\"",
        "'",
        " ",
        "/",
        "?",
        "jane@corp",
        "41",
        "zz",
        "page=2",
        &s,
        &s,
        "%%26",
        "%%41",
        "%2%26",
        "=>",
        "\\",
        ";",
    ];
    let mut x: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    for _ in 0..20_000 {
        let n = 2 + (next() % 9) as usize;
        let input: String = (0..n)
            .map(|_| frags[(next() % frags.len() as u64) as usize])
            .collect();
        let (k, _) = mask_key(&input);
        assert_eq!(mask_key(&k), (k.clone(), 0), "key {input:?}");
        let (t, _) = mask(&input);
        assert_eq!(mask(&t), (t.clone(), 0), "text {input:?}");
    }
}

/// Long runs of `%25` before a name, and of `a@:` after a scheme, the two shapes the change made
/// slower: linear, 4x the input takes well under 8x the time.
#[test]
fn long_percent_and_user_runs_take_linear_time() {
    let shapes: [fn(usize) -> String; 3] = [
        |n| format!("%{}3fpassword=abcdefgh", "25".repeat(n / 2)),
        |n| format!("x://{}", "a@:".repeat(n / 3)),
        |n| format!("password=abcdefgh{}", "%%".repeat(n / 2)),
    ];
    for shape in shapes {
        let (small, large) = (shape(1 << 16), shape(1 << 18));
        for f in [mask_key as fn(&str) -> (String, usize), mask] {
            let t_small = fastest(|| {
                f(&small);
            });
            let t_large = fastest(|| {
                f(&large);
            });
            assert!(
                t_large < t_small * 8,
                "4x the text took {:.1}x the time ({t_small:?} -> {t_large:?}): {}",
                t_large.as_secs_f64() / t_small.as_secs_f64(),
                &small[..24]
            );
        }
    }
}

// Pass 2: the stop rule of `bb4835c7`, its second `url-password` rule and `access[_-]?key`.

/// The pieces a stop sequence is built from: a raw `%`, hex digits, `&`, `#`, `;` and `amp`.
const STOP_ALPHABET: [&str; 10] = ["%", "2", "6", "a", "4", "1", "#", "&", ";", "amp"];

/// Every sequence of up to `max` pieces of `STOP_ALPHABET`, the empty one first.
fn stop_sequences(max: usize) -> Vec<String> {
    let mut all = vec![String::new()];
    let mut layer = vec![String::new()];
    for _ in 0..max {
        layer = layer
            .iter()
            .flat_map(|s| STOP_ALPHABET.iter().map(move |a| format!("{s}{a}")))
            .collect();
        all.extend(layer.iter().cloned());
    }
    all
}

/// Whether `seq` holds an encoded `&` (`%26`), the only encoded separator the alphabet builds.
fn holds_an_encoded_separator(seq: &str) -> bool {
    seq.to_ascii_lowercase().contains("%26")
}

/// Whether `seq` holds a raw `&` or `#`.
fn holds_a_raw_separator(seq: &str) -> bool {
    seq.contains('&') || seq.contains('#')
}

/// The first five of `found`, for a failure message.
fn first_five(found: &[String]) -> &[String] {
    &found[..found.len().min(5)]
}

/// A stop sequence of up to five pieces that holds no `&`, `#` or `%26`, inside a credential or in
/// front of it, ends nothing: the whole value is masked in a key, and in a text, where the value
/// runs to the next space.
#[test]
fn a_stop_sequence_without_a_separator_is_masked_with_its_credential() {
    let (h, t) = (head(), tail());
    let base = "https://example.org/a?access_token=";
    let token = format!("{base}[masked:secret-assignment]");
    let mut leaks = Vec::new();
    for seq in stop_sequences(5) {
        if holds_an_encoded_separator(&seq) || holds_a_raw_separator(&seq) {
            continue;
        }
        for value in [format!("{h}{seq}{t}"), format!("{seq}{h}{t}")] {
            let key = format!("{base}{value}&page=2");
            let as_key = mask_key(&key);
            let as_text = mask(&key);
            if as_key != (format!("{token}&page=2"), 1) || as_text != (token.clone(), 1) {
                leaks.push(format!("{seq:?}: {} | {}", as_key.0, as_text.0));
            }
        }
    }
    assert!(
        leaks.is_empty(),
        "{} values leak: {:#?}",
        leaks.len(),
        first_five(&leaks)
    );
}

/// A stop sequence that holds `%26` ends the value in a key. What the key keeps after the masked
/// token starts at the separator, after at most a run of bare `%`s (`%%26`, as `src/mask.rs`
/// documents): no hex digit or letter of the credential before the separator is kept in clear.
/// `%2%26` and `%a%26` keep `%2` and `%a` of the credential.
#[test]
#[ignore = "story:ignored-adversary-limits: %2%26 keeps %2 of the credential in a key"]
fn a_stop_sequence_before_an_encoded_ampersand_keeps_no_hex_digit_of_the_credential() {
    let (h, t) = (head(), tail());
    let base = "https://example.org/a?access_token=";
    let token = format!("{base}[masked:secret-assignment]");
    let mut leaks = Vec::new();
    for seq in stop_sequences(5) {
        if !holds_an_encoded_separator(&seq) || holds_a_raw_separator(&seq) {
            continue;
        }
        let (got, _) = mask_key(&format!("{base}{h}{seq}{t}&page=2"));
        let at_separator = got.strip_prefix(&token).is_some_and(|rest| {
            rest.starts_with('%')
                && rest
                    .trim_start_matches('%')
                    .to_ascii_lowercase()
                    .starts_with("26")
        });
        if !at_separator || got.contains(&h) {
            leaks.push(format!("{seq:?}: {got}"));
        }
    }
    assert!(
        leaks.is_empty(),
        "{} keys keep part of the credential: {:#?}",
        leaks.len(),
        first_five(&leaks)
    );
}

/// The same for a raw `&` or `#`: the key keeps nothing of the credential before it but a run of
/// bare `%`s. `%a&` and `%2#` keep `%a` and `%2`.
#[test]
#[ignore = "story:ignored-adversary-limits: %2& keeps %2 of the credential in a key (also at 7a94960c)"]
fn a_stop_sequence_before_a_raw_separator_keeps_no_hex_digit_of_the_credential() {
    let (h, t) = (head(), tail());
    let base = "https://example.org/a?access_token=";
    let token = format!("{base}[masked:secret-assignment]");
    let mut leaks = Vec::new();
    for seq in stop_sequences(5) {
        if holds_an_encoded_separator(&seq) || !holds_a_raw_separator(&seq) {
            continue;
        }
        let (got, _) = mask_key(&format!("{base}{h}{seq}{t}&page=2"));
        let at_separator = got.strip_prefix(&token).is_some_and(|rest| {
            let stop = rest.trim_start_matches('%');
            stop.starts_with('&') || stop.starts_with('#')
        });
        if !at_separator || got.contains(&h) {
            leaks.push(format!("{seq:?}: {got}"));
        }
    }
    assert!(
        leaks.is_empty(),
        "{} keys keep part of the credential: {:#?}",
        leaks.len(),
        first_five(&leaks)
    );
}

/// A user name with a raw `@` and a password holding `,` or `;`: the second `url-password` rule
/// refuses the password, the first refuses the user name, and the password is kept in clear.
#[test]
#[ignore = "story:credential-key-spellings: user@host:<pw with , or ;>@host keeps the password in clear"]
fn a_password_with_a_comma_after_a_user_name_with_an_at_sign_is_masked() {
    let (h, t) = (head(), tail());
    for sep in [",", ";"] {
        let url = format!("ftp://jane@corp.example:{h}{sep}{t}@files.example/x");
        for (f, what) in [
            (mask as fn(&str) -> (String, usize), "text"),
            (mask_key, "key"),
        ] {
            let (got, n) = f(&url);
            assert!(
                !got.contains(&h) && !got.contains(&t) && n == 1,
                "{what}: {got} ({n})"
            );
        }
    }
}

/// A URL with a user name and a port, followed by a quote and a quoted address, as a list in code
/// or an SQL row writes it. The second `url-password` rule reads `587','admin` as a password.
#[test]
#[ignore = "story:ignored-adversary-limits: (smtp://ops@mail.example:587)admin@example.org masks 587)admin"]
fn a_quoted_url_with_a_user_and_a_port_before_a_quoted_address_is_kept() {
    for kept in [
        "('smtp://ops@mail.example:587','admin@example.org')",
        "['smtp://ops@mail.example:587', 'x'] and ('a','admin@example.org')",
        "(smtp://ops@mail.example:587)admin@example.org",
    ] {
        assert_eq!(mask(kept), (kept.to_string(), 0), "{kept}");
        assert_eq!(mask_key(kept), (kept.to_string(), 0), "{kept}");
    }
}

/// The two `url-password` rules mask one password once, whichever reads it, and a masked URL is
/// masked to itself: user names with and without `@`, passwords with `@` and `:`.
#[test]
fn the_two_url_password_rules_mask_one_password_once() {
    let (h, t) = (head(), tail());
    let users = ["u", "jane@corp.example", "a@b@c", "j.doe+x@corp", "[masked"];
    let passwords = [
        format!("{h}{t}"),
        format!("{h}@{t}"),
        format!("{h}:{t}"),
        format!("{h}@{t}:{h}"),
        format!("{h}%40{t}"),
    ];
    let tails = ["", "/x", ":22/x", "?q=1", " and x://v:w@y"];
    for user in users {
        for pw in &passwords {
            for end in tails {
                let url = format!("https://{user}:{pw}@host.example{end}");
                for f in [mask as fn(&str) -> (String, usize), mask_key] {
                    let (got, n) = f(&url);
                    let want = 1 + usize::from(end.contains("x://"));
                    assert!(
                        !got.contains(&h) && !got.contains(&t) && n == want,
                        "{url} -> {got} ({n})"
                    );
                    assert_eq!(f(&got), (got.clone(), 0), "{url}");
                }
            }
        }
    }
}

/// Names that hold `access_key` and are no credential, with a value of six or more characters,
/// are kept; an AWS key id under `aws_access_key_id` is masked once, by its own rule.
#[test]
fn access_key_words_that_are_no_credential_are_kept() {
    for kept in [
        "aws_access_key_id = rotated-monthly",
        "has_access_keys = enabled",
        "access-key-rotation: monthly",
        "access_key_count=123456",
        "accessKeyId: string",
        "the access key rotation runs monthly",
        "access_keys: [primary, backup]",
    ] {
        assert_eq!(mask(kept), (kept.to_string(), 0), "{kept}");
        assert_eq!(mask_key(kept), (kept.to_string(), 0), "{kept}");
    }
    let id = format!("AKIA{}", "Q7".repeat(8));
    for line in [
        format!("aws_access_key_id = {id}"),
        format!("access_key = {id}"),
    ] {
        let (got, n) = mask(&line);
        assert!(!got.contains(&id) && n == 1, "{line} -> {got} ({n})");
    }
}

/// The pair an S3-compatible client's config file writes (`access_key`, `secret_key`) and a
/// framework's `SECRET_KEY`: `access_key` masks the key id, which is no secret, and the secret
/// beside it is kept in clear, because `secret` must be the last word of a name.
#[test]
#[ignore = "story:credential-key-spellings: secret_key = <secret> is kept in clear (also at 7a94960c)"]
fn the_secret_half_of_an_access_key_pair_is_masked() {
    let s = head() + &tail();
    for line in [
        format!("secret_key = {s}"),
        format!("SECRET_KEY = '{s}'"),
        format!("aws_secret_key: {s}"),
    ] {
        let (got, n) = mask(&line);
        assert!(!got.contains(&s) && n == 1, "{line} -> {got} ({n})");
    }
}

/// Masking is idempotent over generated keys and texts made of the shapes pass 2 touches (`%2%`,
/// `%a%`, `,`, `;`, `'`, a user name with `@` and a port, `access_key`), with a fixed seed.
#[test]
fn masking_is_idempotent_over_generated_stop_runs_user_lists_and_access_keys() {
    let s = head() + &tail();
    let frags: Vec<&str> = vec![
        "access_token",
        "access_key",
        "access-key",
        "aws_secret_access_key",
        "access_key_id",
        "password",
        "=",
        ":",
        "%3d",
        "%",
        "%%",
        "%2",
        "%a",
        "%2%",
        "%a%",
        "%26",
        "%%26",
        "%2%26",
        "%2526",
        "&",
        "#",
        ";",
        ",",
        "'",
        ")",
        "@",
        "smtp://",
        "x://",
        "ops@mail.example:587",
        "jane@corp.example:",
        "admin@example.org",
        "[masked",
        "]",
        "amp",
        "41",
        "26",
        " ",
        "/",
        &s,
        &s,
    ];
    let mut x: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    for _ in 0..20_000 {
        let n = 2 + (next() % 9) as usize;
        let input: String = (0..n)
            .map(|_| frags[(next() % frags.len() as u64) as usize])
            .collect();
        let (k, _) = mask_key(&input);
        assert_eq!(mask_key(&k), (k.clone(), 0), "key {input:?}");
        let (t, _) = mask(&input);
        assert_eq!(mask(&t), (t.clone(), 0), "text {input:?}");
    }
}

/// The shapes pass 2 added, long: a run of `%2%` in a key value, with and without `%26` after
/// it; many `@` in a user name and in a password for the second `url-password` rule; many
/// `access_key` assignments. Linear: 4x the input takes well under 8x the time.
#[test]
fn long_stop_runs_and_user_names_take_linear_time() {
    let shapes: [fn(usize) -> String; 6] = [
        |n| format!("access_token=abcdef{}", "%2".repeat(n / 2)),
        |n| format!("access_token=abcdef{}26", "%2".repeat(n / 2)),
        |n| format!("x://{}:p@", "a@".repeat(n / 2)),
        |n| format!("x://a@b:{}", "p@".repeat(n / 2)),
        |n| format!("x://{}", "a@b:,".repeat(n / 5)),
        |n| "access_key=abcdefgh ".repeat(n / 20),
    ];
    for shape in shapes {
        let (small, large) = (shape(1 << 16), shape(1 << 18));
        for f in [mask_key as fn(&str) -> (String, usize), mask] {
            let t_small = fastest(|| {
                f(&small);
            });
            let t_large = fastest(|| {
                f(&large);
            });
            assert!(
                t_large < t_small * 8,
                "4x the text took {:.1}x the time ({t_small:?} -> {t_large:?}): {}",
                t_large.as_secs_f64() / t_small.as_secs_f64(),
                &small[..24]
            );
        }
    }
}
