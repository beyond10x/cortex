//! Masking credential-shaped text before it is stored as evidence or sent to a model. This is not
//! PII redaction, and it is not a guarantee: it removes the shapes listed here.
//!
//! [`mask`] is for text and [`mask_key`] for a document key (a URL, a file path or a record key).
//! They differ in one place: in a key, an assigned value ends at `&`, `#` and their
//! percent-encoded forms, so the query parameters after a masked token are kept and two URLs that
//! differ only after it stay two keys. Masking is idempotent: a `[masked:<shape>]` value is left
//! as it is and not counted again.

use std::sync::OnceLock;

use regex::Regex;

/// What a rule replaces.
#[derive(Clone, Copy)]
enum Keep {
    /// The whole match.
    Nothing,
    /// The value only: groups 1 and 2 (the name and the separator) are kept, group 3 is the value.
    Name,
    /// The password of a URL's userinfo: group 1 (scheme, user and `:`) is kept, group 2 is the
    /// password, and the `@` after it is kept.
    User,
}

/// A credential's name, with any `<word>_` prefix (`DB_PASSWORD`, `client_secret`), starting at a
/// word boundary or after a percent-encoded byte (`%3Faccess_token`).
const NAME: &str = r"((?:\b|%[0-9a-f]{2})(?:[a-z0-9]+_)*(?:password|passwd|secret|api[_-]?key|access[_-]?token|auth[_-]?token))";
/// `=` or `:`, or `=` percent-encoded.
const SEPARATOR: &str = r"(\s*(?:[:=]|%3d)\s*)";
/// An assigned value in text: up to the next space or quote.
const TEXT_VALUE: &str = r#"([^\s"']{6,})"#;
/// An assigned value in a key: also up to `&` or `#`, raw or percent-encoded.
const KEY_VALUE: &str = r#"((?:[^\s"'&#%]|%(?:[013-9a-f][0-9a-f]|2[0-24-57-9a-f])){6,})"#;

fn build(value: &str) -> Vec<(&'static str, Keep, Regex)> {
    let assignment = format!(r#"(?i){NAME}{SEPARATOR}["']?{value}"#);
    [
        (
            "private-key",
            Keep::Nothing,
            r"-----BEGIN [A-Z ]*PRIVATE KEY-----[\s\S]*?-----END [A-Z ]*PRIVATE KEY-----"
                .to_string(),
        ),
        (
            "aws-access-key",
            Keep::Nothing,
            r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b".into(),
        ),
        (
            "gitlab-token",
            Keep::Nothing,
            r"\bglpat-[A-Za-z0-9_\-]{20,}".into(),
        ),
        (
            "github-token",
            Keep::Nothing,
            r"\b(?:gh[pousr]_[A-Za-z0-9]{36,}|github_pat_[A-Za-z0-9_]{40,})".into(),
        ),
        (
            "slack-token",
            Keep::Nothing,
            r"\bxox[abprs]-[A-Za-z0-9\-]{10,}".into(),
        ),
        (
            "tavily-key",
            Keep::Nothing,
            r"\btvly-[A-Za-z0-9_\-]{16,}".into(),
        ),
        ("api-key", Keep::Nothing, r"\bsk-[A-Za-z0-9_\-]{20,}".into()),
        (
            "jwt",
            Keep::Nothing,
            r"\beyJ[A-Za-z0-9_\-]{8,}\.eyJ[A-Za-z0-9_\-]{8,}\.[A-Za-z0-9_\-]{8,}".into(),
        ),
        (
            "url-password",
            Keep::User,
            r"(?i)\b([a-z][a-z0-9+.\-]*://[^\s/?#@:]*:)([^\s/?#@]+)@".into(),
        ),
        ("secret-assignment", Keep::Name, assignment),
    ]
    .into_iter()
    .map(|(name, keep, pattern)| (name, keep, Regex::new(&pattern).expect("a valid pattern")))
    .collect()
}

fn text_rules() -> &'static [(&'static str, Keep, Regex)] {
    static RULES: OnceLock<Vec<(&'static str, Keep, Regex)>> = OnceLock::new();
    RULES.get_or_init(|| build(TEXT_VALUE))
}

fn key_rules() -> &'static [(&'static str, Keep, Regex)] {
    static RULES: OnceLock<Vec<(&'static str, Keep, Regex)>> = OnceLock::new();
    RULES.get_or_init(|| build(KEY_VALUE))
}

fn apply(rules: &[(&'static str, Keep, Regex)], text: &str) -> (String, usize) {
    let mut out = text.to_string();
    let mut count = 0;
    for (name, keep, rule) in rules {
        let replaced = rule.replace_all(&out, |c: &regex::Captures| {
            let value = match keep {
                Keep::Nothing => &c[0],
                Keep::Name => &c[3],
                Keep::User => &c[2],
            };
            if value.starts_with("[masked:") {
                return c[0].to_string();
            }
            count += 1;
            match keep {
                Keep::Nothing => format!("[masked:{name}]"),
                Keep::Name => format!("{}{}[masked:{name}]", &c[1], &c[2]),
                Keep::User => format!("{}[masked:{name}]@", &c[1]),
            }
        });
        out = replaced.into_owned();
    }
    (out, count)
}

/// `text` with every listed credential shape replaced by `[masked:<shape>]`, and how many were.
pub fn mask(text: &str) -> (String, usize) {
    apply(text_rules(), text)
}

/// A document key masked as [`mask`] masks text, except that an assigned value ends at `&`, `#`,
/// `%26` and `%23`, so the rest of a URL's query and its fragment are kept.
pub fn mask_key(key: &str) -> (String, usize) {
    apply(key_rules(), key)
}

#[cfg(test)]
mod tests {
    use super::{mask, mask_key};

    #[test]
    fn credential_shapes_are_masked_and_prose_is_kept() {
        // Assembled at runtime so no credential-shaped literal sits in the source.
        let token = format!("glpat-{}", "a".repeat(24));
        let tavily = format!("tvly-{}", "B".repeat(20));
        let text =
            format!("Use {token} or {tavily}. password = hunter2hunter2. The release ships.");
        let (masked, count) = mask(&text);
        assert_eq!(count, 3);
        assert!(!masked.contains(&token));
        assert!(!masked.contains(&tavily));
        assert!(!masked.contains("hunter2"));
        assert!(masked.contains("password = [masked:secret-assignment]"));
        assert!(masked.contains("The release ships."));
    }

    fn secret() -> String {
        format!("Zr4{}", "p0".repeat(8))
    }

    #[test]
    fn a_key_keeps_the_parameters_after_a_masked_value() {
        let t = secret();
        let key = format!("https://example.org/doc?access_token={t}&page=2#top");
        assert_eq!(
            mask_key(&key),
            (
                "https://example.org/doc?access_token=[masked:secret-assignment]&page=2#top".into(),
                1
            )
        );
        // The same value in text runs to the next space, as before.
        let (text, n) = mask(&format!("see {key} now"));
        assert_eq!(
            (text.as_str(), n),
            (
                "see https://example.org/doc?access_token=[masked:secret-assignment] now",
                1
            )
        );
    }

    #[test]
    fn a_percent_encoded_assignment_is_masked_up_to_the_encoded_ampersand() {
        let t = secret();
        let key = format!("https://example.org/a?next=%2Fapi%3Faccess_token%3D{t}%26page%3D2");
        assert_eq!(
            mask_key(&key),
            (
                "https://example.org/a?next=%2Fapi%3Faccess_token%3D[masked:secret-assignment]%26page%3D2"
                    .into(),
                1
            )
        );
        let (text, n) = mask(&format!("go to {key}"));
        assert!(!text.contains(&t), "{text}");
        assert_eq!(n, 1);
    }

    #[test]
    fn a_userinfo_password_is_masked_and_the_user_kept() {
        let t = secret();
        for (input, n) in [
            (format!("https://alice:{t}@example.org/a"), 1),
            (format!("db is postgres://app:{t}@db.internal:5432/x."), 1),
            ("https://example.org:8443/a?x=1".to_string(), 0),
        ] {
            let (out, count) = mask_key(&input);
            assert_eq!(count, n, "{input} -> {out}");
            assert!(!out.contains(&t), "{out}");
            let (text, count) = mask(&input);
            assert_eq!((text.as_str(), count), (out.as_str(), n), "{input}");
        }
        assert_eq!(
            mask(&format!("https://alice:{t}@example.org/a")).0,
            "https://alice:[masked:url-password]@example.org/a"
        );
    }

    #[test]
    fn a_prefixed_name_is_masked_and_ordinary_words_are_not() {
        let t = secret();
        for (input, kept) in [
            (format!("DB_PASSWORD={t}"), "DB_PASSWORD="),
            (format!("client_secret: {t}"), "client_secret: "),
            (format!("x_api_key={t}"), "x_api_key="),
            (format!("MY_APP_DB_PASSWORD={t}"), "MY_APP_DB_PASSWORD="),
        ] {
            let (out, n) = mask(&input);
            assert_eq!(
                (out.as_str(), n),
                (format!("{kept}[masked:secret-assignment]").as_str(), 1),
                "{input}"
            );
        }
        let (key, n) = mask_key(&format!(
            "https://example.org/a?client_id=app&client_secret={t}"
        ));
        assert_eq!(
            (key.as_str(), n),
            (
                "https://example.org/a?client_id=app&client_secret=[masked:secret-assignment]",
                1
            )
        );
        for prose in [
            "Read my_password_policy before you start.",
            "The password_reset flow and the secret_santa list are documented.",
            "A secretary keeps the passwords file offline.",
        ] {
            assert_eq!(mask(prose), (prose.to_string(), 0), "{prose}");
            assert_eq!(mask_key(prose), (prose.to_string(), 0), "{prose}");
        }
    }

    #[test]
    fn masking_twice_changes_and_counts_nothing_more() {
        let t = secret();
        for input in [
            format!("https://alice:{t}@example.org/a?access_token={t}&p=1"),
            format!("DB_PASSWORD={t} and glpat-{}", "a".repeat(24)),
        ] {
            let (once, n) = mask_key(&input);
            assert!(n >= 2, "{once}");
            assert_eq!(mask_key(&once), (once.clone(), 0));
            let (once, _) = mask(&input);
            assert_eq!(mask(&once), (once.clone(), 0));
        }
    }
}
