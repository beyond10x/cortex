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
