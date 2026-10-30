//! Masking credential-shaped text before it is stored as evidence or sent to a model. This is not
//! PII redaction, and it is not a guarantee: it removes the shapes listed here.
//!
//! [`mask`] is for text and [`mask_key`] for a document key (a URL, a file path or a record key).
//! They differ in one place: in a key, an assigned value ends at `&`, `#` and their
//! percent-encoded forms (encoded once or more), so the query parameters after a masked token
//! are kept and two URLs that differ only after it stay two keys. Masking is idempotent: a
//! `[masked:<shape>]` value is left as it is and not counted again.

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

/// A credential's name, with any `<word>_` prefix (`DB_PASSWORD`, `client_secret`,
/// `AWS_SECRET_ACCESS_KEY`), starting at a word boundary or after a percent-encoded byte,
/// encoded once or more (`%3Faccess_token`, `%253Faccess_token`).
const NAME: &str = r"((?:\b|%(?:25)*[0-9a-f]{2})(?:[a-z0-9]+_)*(?:password|passwd|secret|api[_-]?key|access[_-]?key|access[_-]?token|auth[_-]?token))";
/// `=`, `:` or `=>`, raw or percent-encoded once or more, after the quote that closes a quoted
/// name (`"password": `, `\"password\": ` inside a JSON string), as the opt-in `Credential`
/// class of `crate::redact` reads it.
const SEPARATOR: &str = r#"((?:\\?["']|%(?:25)*2[27])?\s*(?:=>|[:=]|%(?:25)*3[ad])\s*)"#;
/// An assigned value in text: up to the next space or quote.
const TEXT_VALUE: &str = r#"([^\s"']{6,})"#;
/// One character of an assigned value in a key. A value ends at a space, a quote, `&` or `#`,
/// raw or percent-encoded once or more (`%26`, `%2526`). A `%` that starts no escape (`ab%zz`)
/// is part of the value, and so is a run of them, each alone, encoded or with one hex digit,
/// before an escape or another character (`%%41`, `%25%21`, `%2%41`, `%2%x`); a run before an
/// encoded `&` or `#` (`%%26`, `%2%26`) ends the value there, so the query after it is kept. The
/// run reads a hex digit only with a `%` after it, so it never splits `%26` into `%2` and `6`.
macro_rules! key_unit {
    () => {
        r#"(?:[^\s"'&#%]|%(?:%|25|[0-9a-f]%)*(?:[013-9a-f][0-9a-f]|2[0-247-9a-f]|[0-9a-f]?[^0-9a-f\s"'&#%]))"#
    };
}
/// An assigned value in a key: six or more of `key_unit!`.
const KEY_VALUE: &str = concat!("((?:", key_unit!(), "){6,})");
/// A private key's base64 lines after its `BEGIN` line when its `END` line is missing (a provider
/// cut the text after it): every line of 16 or more base64 characters, the last line of the text
/// whatever its length, and the `Proc-Type` and `DEK-Info` lines of an encrypted key. A line
/// break may be written `\n`, as in a JSON string.
const KEY_LINES: &str = r"(?:(?:\s|\\[rn])+(?:(?:Proc-Type|DEK-Info):[^\r\n\\]*|[A-Za-z0-9+/=]{16,}|[A-Za-z0-9+/=]+\s*\z))*";

fn build(value: &str) -> Vec<(&'static str, Keep, Regex)> {
    let assignment = format!(r#"(?i){NAME}{SEPARATOR}(?:\\?["'])?{value}"#);
    // A whole key first, then a key without its `END` line: two rules, because one with both as
    // alternatives reads to the end of the text for every `BEGIN` line without an `END` line.
    [
        (
            "private-key",
            Keep::Nothing,
            r"-----BEGIN [A-Z ]*PRIVATE KEY-----[\s\S]*?-----END [A-Z ]*PRIVATE KEY-----"
                .to_string(),
        ),
        (
            "private-key",
            Keep::Nothing,
            format!(r"-----BEGIN [A-Z ]*PRIVATE KEY-----{KEY_LINES}"),
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
            // The password runs to the last `@` before the path: it may hold a raw `@`. It holds
            // no character a URL never holds raw (`"`, `<`, `{`), so it ends with its URL in a
            // JSON string, before an address later in the text.
            r#"(?i)\b([a-z][a-z0-9+.\-]*://[^\s/?#@:]*:)([^\s/?#"<>`{}|\\^]+)@"#.into(),
        ),
        (
            "url-password",
            Keep::User,
            // A user name holding a raw `@` (`jane@corp.example:<password>@host`). Its password
            // holds no `,` or `;` either, so a user, host and port before a list of addresses
            // (`ops@mail.example:587,admin@example.org`) is not read as one.
            r#"(?i)\b([a-z][a-z0-9+.\-]*://[^\s/?#:"<>`{}|\\^]*@[^\s/?#:"<>`{}|\\^]*:)([^\s/?#"<>`{}|\\^,;]+)@"#.into(),
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

/// Whether `text` starts with a character of an assigned value in a key (`key_unit!`).
fn starts_a_key_value(text: &str) -> bool {
    static UNIT: OnceLock<Regex> = OnceLock::new();
    UNIT.get_or_init(|| Regex::new(concat!(r"(?i)\A", key_unit!())).expect("a valid pattern"))
        .is_match(text)
}

/// Whether `value` is a `[masked:<shape>]` token, followed by nothing or by what ends a value in
/// a key (`&`, `#`, `%26`, `%%26`, `%2526`; whatever `key_unit!` does not read), as a masked
/// key shown in a text has it. A token followed by anything else holds the rest of a credential
/// an earlier mask missed (`[masked:url-password]@ss` for a password with a raw `@`), and is
/// masked again.
fn already_masked(value: &str) -> bool {
    let Some((shape, rest)) = value
        .strip_prefix("[masked:")
        .and_then(|v| v.split_once(']'))
    else {
        return false;
    };
    let token = !shape.is_empty()
        && shape
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    token && !starts_a_key_value(rest)
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
            // A user name ending in `[masked:` is a masked value's own `:` read as the end of a
            // user name (`x://password%3a[masked:secret-assignment]&u@h`): the token is whole.
            let token_split_at_its_colon = matches!(keep, Keep::User)
                && c[1].ends_with("[masked:")
                && already_masked(&format!("[masked:{}", &c[2]));
            if already_masked(value) || token_split_at_its_colon {
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
/// `%26` and `%23` (also encoded twice, `%2526`, or after a raw `%`, `%%26`), so the rest of a
/// URL's query and its fragment are kept.
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

    /// An access key assigned by name, as a cloud provider's environment and config files write
    /// it, is masked in a text and in a key.
    #[test]
    fn an_access_key_name_is_masked() {
        let v = format!("wJa{}", "Lr9Xq".repeat(6));
        for kept in [
            "AWS_SECRET_ACCESS_KEY=",
            "aws_secret_access_key = ",
            "access-key: ",
        ] {
            let input = format!("{kept}{v}");
            assert_eq!(
                mask(&input),
                (format!("{kept}[masked:secret-assignment]"), 1),
                "{input}"
            );
            assert!(!mask_key(&input).0.contains(&v), "{input}");
        }
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

    /// A quote after the name, `=>`, `%3A` and a percent-encoded quote all separate a name from
    /// its value, as the opt-in `Credential` class reads them (R4-03, R4-14).
    #[test]
    fn a_quoted_or_encoded_separator_is_masked() {
        let t = secret();
        for (input, masked) in [
            (
                format!(r#"{{"password": "{t}"}}"#),
                r#"{"password": [masked:secret-assignment]"}"#,
            ),
            (
                format!("'api_key' => '{t}',"),
                "'api_key' => [masked:secret-assignment]',",
            ),
            (
                format!("next=%2Fa%3Faccess_token%3A{t}"),
                "next=%2Fa%3Faccess_token%3A[masked:secret-assignment]",
            ),
            (
                format!("q=%22client_secret%22%3A%22{t}%22"),
                "q=%22client_secret%22%3A[masked:secret-assignment]",
            ),
        ] {
            assert_eq!(mask(&input), (masked.to_string(), 1), "{input}");
            assert!(!mask_key(&input).0.contains(&t), "{input}");
        }
    }

    /// A raw `@` in a URL password, and a raw `%` that is no escape in a key's value, are part of
    /// the value (R4-07, R4-12).
    #[test]
    fn an_at_sign_in_a_password_and_a_raw_percent_in_a_key_are_masked() {
        let t = secret();
        let url = format!("postgres://app:p@ss-{t}@db.internal:5432/x");
        let masked = "postgres://app:[masked:url-password]@db.internal:5432/x";
        assert_eq!(mask(&url), (masked.to_string(), 1));
        assert_eq!(mask_key(&url), (masked.to_string(), 1));
        assert_eq!(
            mask_key(&format!("https://example.org/a?access_token=ab%{t}&page=2")),
            (
                "https://example.org/a?access_token=[masked:secret-assignment]&page=2".into(),
                1
            )
        );
    }

    /// A private key cut before its `END` line is masked through its last line, and the prose
    /// after a lone `BEGIN` line is kept.
    #[test]
    fn a_private_key_without_its_end_line_is_masked() {
        // Assembled at runtime so no key-shaped literal sits in the source.
        let begin = |kind: &str| format!("-----BEGIN {kind}PRIVATE {}-----", "KEY");
        let body = format!("MIIE{}", "Qk7x".repeat(15));
        let cut = format!("{}\n{body}\n{body}\nPz8r", begin("RSA "));
        assert_eq!(
            mask(&format!("Deploy key: {cut}")),
            ("Deploy key: [masked:private-key]".into(), 1)
        );
        let prose = format!(
            "A key file opens with {} and a blank line.\n\nKeep it.",
            begin("")
        );
        let (out, n) = mask(&prose);
        assert_eq!(n, 1, "{out}");
        assert!(out.ends_with(" and a blank line.\n\nKeep it."), "{out}");
    }

    /// A masked key shown in a text keeps the query after its token; a token followed by the
    /// rest of a credential is masked again.
    #[test]
    fn a_masked_key_in_a_text_is_kept_and_a_partly_masked_value_is_masked_again() {
        for kept in [
            "see https://example.org/doc?access_token=[masked:secret-assignment]&page=2#top now",
            "go to https://example.org/a?next=%3Faccess_token%3D[masked:secret-assignment]%26p%3D2",
        ] {
            assert_eq!(mask(kept), (kept.to_string(), 0), "{kept}");
            assert_eq!(mask_key(kept), (kept.to_string(), 0), "{kept}");
        }
        let t = secret();
        let (out, n) = mask(&format!("password=[masked:secret-assignment]{t}"));
        assert_eq!(
            (out.as_str(), n),
            ("password=[masked:secret-assignment]", 1)
        );
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

    /// In a key, `&` and `#` end a value also when encoded twice (`%2526`) or after a raw `%`
    /// (`%%26`); an encoded or raw `%` before any other escape stays in the value. The masked
    /// key is masked again to itself, as a key and in a text.
    #[test]
    fn a_percent_before_an_encoded_separator_ends_a_key_value_and_one_before_an_escape_does_not() {
        let t = secret();
        let base = "https://example.org/a?";
        let m = "[masked:secret-assignment]";
        let name = "access_token=";
        for (name, value, rest) in [
            (name, t.clone(), "%%26page=2"),
            (name, t.clone(), "%%%23top"),
            (name, t.clone(), "%2526page=2"),
            (name, t.clone(), "%2%26page=2"),
            (name, format!("ab%25%21{t}"), "&page=2"),
            (name, format!("ab%%41{t}"), "&page=2"),
            (name, format!("ab%2525zz{t}"), "#top"),
            (name, format!("ab%2%41{t}"), "&page=2"),
            (name, format!("ab%252%41%a%x{t}"), "&page=2"),
            (
                "next=%252Fapi%253Faccess_token%253D",
                t.clone(),
                "%2526page%253D2",
            ),
        ] {
            let key = format!("{base}{name}{value}{rest}");
            let masked = format!("{base}{name}{m}{rest}");
            assert_eq!(mask_key(&key), (masked.clone(), 1), "{key}");
            assert_eq!(mask_key(&masked), (masked.clone(), 0), "{masked}");
            let shown = format!("see {masked} now");
            assert_eq!(mask(&shown), (shown.clone(), 0), "{shown}");
        }
    }

    /// A user name holding a raw `@` and a JSON string's escaped quotes are read as their
    /// unescaped forms; a URL with a port and no password, and prose, are kept.
    #[test]
    fn a_user_name_with_an_at_sign_and_an_escaped_quote_are_masked_and_ports_are_kept() {
        let t = secret();
        for (input, masked) in [
            (
                format!("ftp://jane@corp.example:{t}@files.example/x"),
                "ftp://jane@corp.example:[masked:url-password]@files.example/x".to_string(),
            ),
            (
                format!(r#"{{\"password\": \"{t}\"}}"#),
                r#"{\"password\": [masked:secret-assignment]"}"#.to_string(),
            ),
        ] {
            assert_eq!(mask(&input), (masked.clone(), 1), "{input}");
            assert_eq!(mask(&masked), (masked.clone(), 0), "{masked}");
            assert!(!mask_key(&input).0.contains(&t), "{input}");
        }
        for kept in [
            "ssh://git@host.example:22/repo.git and https://jane@corp.example:8443/a?x=1",
            "smtp://ops@mail.example:587,admin@example.org",
            r#"{\"user\": \"jane@corp.example\", \"note\": \"the password policy\"}"#,
        ] {
            assert_eq!(mask(kept), (kept.to_string(), 0), "{kept}");
            assert_eq!(mask_key(kept), (kept.to_string(), 0), "{kept}");
        }
    }
}
