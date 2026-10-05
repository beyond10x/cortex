//! Pseudonymising personal data in what the model is shown, by the classes, known names and rules
//! an instance's `redaction` policy names, restoring it in what the model answers, and refusing a
//! batch in which a class the policy's `refuse_if_left` names is still detected.
//!
//! Within one batch, each distinct value a class, the known names or a rule finds is replaced by a
//! placeholder, `[Email-1]`, `[Phone-2]`, `[Card-1]`, `[IpAddress-1]`, `[Url-1]`, `[Name-1]` or
//! `[<rule name>-1]`, the same value by the same placeholder in every text of the batch. Before the
//! answer is applied, every placeholder in it is replaced by the value it stands for; one that has
//! no value (mangled or invented by the model) stays as it is and is counted as unrestored. The
//! mapping lives in memory for one batch only. The stored evidence is the original text.
//!
//! `known_names` are names replaced wherever they appear as whole words, however often, in any case
//! and either Unicode normal form. `RareName` replaces a capitalised word (one capital and at least
//! one lower-case letter) seen at most `rare_limit` times (default 1) in the batch's document texts,
//! unless it is a stop word or a placeholder's label. Only what the model would see counts: words
//! inside a span another step hides, and the known entity names, are not counted. Both are counted
//! apart: `known_names` and `RareName`.
//!
//! `Credential` and a rule with a non-empty `replacement` are irreversible: `scrub` replaces their
//! matches by `[masked:<shape>]` or the rule's text before the document is stored, a batch replaces
//! any left before the model is shown it, and nothing is ever restored to them. A credential is
//! also scrubbed from a document key (`scrub_key`). Credential shapes are masked on every run
//! regardless (`crate::mask`); the class adds shapes that module does not cover.
//!
//! Detection is by pattern, and runs once over the original text: a rule never sees another
//! class's placeholder. [`Batch::left`] runs the detectors of the `refuse_if_left` classes over
//! what the model would be shown; for `Phone` it adds a number of 6 to 15 digits after a word such
//! as "call" or "mobile", which the `Phone` class itself does not replace.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::ops::Range;
use std::sync::OnceLock;

use cortex_model::instance as m;
use regex::{Regex, RegexBuilder};
use serde_json::Value;
use unicode_normalization::UnicodeNormalization;

/// The built-in classes this module replaces by a placeholder, in the order they claim text: a
/// link before the address or digits inside it, an email address before the digits inside it are
/// read as a phone number, a card before its digit groups are.
const BUILT_IN: [m::RedactionClass; 5] = [
    m::RedactionClass::Url,
    m::RedactionClass::Email,
    m::RedactionClass::PaymentCard,
    m::RedactionClass::IpAddress,
    m::RedactionClass::Phone,
];

/// What the replacements of a known name are counted under.
const KNOWN_NAMES: &str = "known_names";

/// Capitalised words that open sentences and are never a name.
const STOP_WORDS: &[&str] = &[
    "A", "All", "An", "And", "Any", "Are", "As", "At", "Be", "But", "By", "Each", "Every", "For",
    "From", "He", "Her", "Here", "His", "How", "I", "If", "In", "Is", "It", "Its", "My", "No",
    "Not", "Of", "On", "Or", "Our", "She", "So", "Some", "That", "The", "Their", "Then", "There",
    "These", "They", "This", "Those", "To", "We", "What", "When", "Where", "Who", "Why", "With",
    "Yes", "You", "Your",
];

pub fn class_name(class: m::RedactionClass) -> &'static str {
    match class {
        m::RedactionClass::Email => "Email",
        m::RedactionClass::Phone => "Phone",
        m::RedactionClass::IpAddress => "IpAddress",
        m::RedactionClass::PaymentCard => "PaymentCard",
        m::RedactionClass::Url => "Url",
        m::RedactionClass::Credential => "Credential",
        m::RedactionClass::RareName => "RareName",
    }
}

/// The name a class's placeholders carry.
fn label(class: m::RedactionClass) -> &'static str {
    match class {
        m::RedactionClass::PaymentCard => "Card",
        m::RedactionClass::RareName => "Name",
        other => class_name(other),
    }
}

/// Decides a candidate match: the range to replace, or `None` to reject it. Gets the whole
/// text, so it can look at what surrounds the candidate.
type Refine = fn(&str, Range<usize>) -> Option<Range<usize>>;

struct Step {
    /// What it is counted under: the class, `known_names` or the rule name.
    name: String,
    /// What its placeholders are called.
    label: String,
    regex: Regex,
    /// The capture group a match replaces; `0` for the whole match.
    group: usize,
    /// Built-ins only. Their candidates are bounded in length, so a rejected one is retried one
    /// character on; a rule's match is taken as it is.
    refine: Option<Refine>,
    /// An irreversible step's text: its matches become this and get no placeholder.
    replacement: Option<String>,
    /// `RareName`: a candidate is taken only when the batch saw it at most `rare_limit` times.
    rare: bool,
}

impl Step {
    fn new(name: &str, label: &str, regex: Regex) -> Self {
        Step {
            name: name.to_string(),
            label: label.to_string(),
            regex,
            group: 0,
            refine: None,
            replacement: None,
            rare: false,
        }
    }
}

/// What a policy pseudonymises and refuses, compiled once per run.
pub struct Redactor {
    /// Irreversible steps first, so they claim text before anything that would be restored.
    steps: Vec<Step>,
    /// Anything in an answer that looks like one of this redactor's placeholders, mangled or not;
    /// `None` when every step is irreversible.
    placeholder: Option<Regex>,
    /// Each class `refuse_if_left` names, with the detectors that find what is left of it.
    checks: Vec<(&'static str, Vec<Step>)>,
    /// The most times a batch may see a capitalised word for it to be a rare name.
    rare_limit: usize,
    /// Whether a batch counts its capitalised words: `RareName` is replaced or checked.
    words: bool,
    /// Capitalised words that are never a rare name: stop words, placeholder labels and the words
    /// of an irreversible rule's replacement.
    not_names: HashSet<String>,
    /// The `Credential` shapes as a document key is scrubbed with: an assigned value ends at `&`
    /// and `#`, as `crate::mask` ends it in a key.
    key_steps: Vec<Step>,
}

fn regex(pattern: &str) -> Regex {
    Regex::new(pattern).expect("a valid built-in pattern")
}

/// A link: a scheme, `www.`, or a dotted host followed by a path (`host.tld/path`); then anything
/// up to whitespace, a quote or a bracket. A `[masked:<shape>]` token masking put inside it is part
/// of it, so the rest of the link after a masked password or token is part of it too.
fn url() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        let host = r"[\p{L}\p{N}][\p{L}\p{N}\-]*(?:\.[\p{L}\p{N}\-]+)*\.(?:\p{L}{2,}|xn--[\p{L}\p{N}\-]+)(?::[0-9]{1,5})?/";
        regex(&format!(
            r#"(?i)(?:\b(?:https?|ftps?|sftp|ssh|wss?|file)://|\bwww\.|{host})(?:[^\s<>"'`{{}}|\\^\[\]]|\[masked:[a-z0-9\-]+\])*"#
        ))
    })
}

fn email() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        // Unbounded: bounded repetitions of these Unicode classes exceed the regex size limit,
        // and an email match is never retried (it has no `refine`).
        let local = r#"(?:\b[\p{L}\p{N}_][\p{L}\p{N}\p{M}._%+'\-]*|"[^"\r\n]+")"#;
        let label = r"[\p{L}\p{N}\p{M}][\p{L}\p{N}\p{M}\-]*";
        let domain = format!(r"{label}(?:\.{label})*\.(?:\p{{L}}{{2,}}|xn--[\p{{L}}\p{{N}}\-]+)\b");
        // An address a provider cut before its top-level domain, at the very end of a text.
        let cut = r"[\p{L}\p{N}\p{M}\-.]*[\p{Zs}…]*\z";
        // `%40` is `@` percent-encoded in a URL; the local part's `%` admits `%2B` and the like.
        regex(&format!("{local}(?:@|%40)(?:{domain}|{cut})"))
    })
}

fn payment_card() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| regex(r"[0-9]{1,19}(?:[\p{Zs}\-][0-9]{1,19}){0,7}"))
}

fn ip_address() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        regex(concat!(
            r"(?i)[0-9a-f.]{0,45}:[0-9a-f.:]{0,45}:[0-9a-f.:]{0,45}",
            r"|\b[0-9]{1,3}(?:\.[0-9]{1,3}){3}\b",
        ))
    })
}

fn phone() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        let sep = r"[\p{Zs}.\-]";
        regex(&format!(
            concat!(
                "(?:",
                // International: `+` or `00`, then the country code.
                r"(?:\+|\b00)[1-9][0-9]{{0,2}}(?:{sep}?\(?[0-9]{{1,4}}\)?){{2,5}}",
                // A parenthesised area code.
                r"|\([0-9]{{2,5}}\){sep}?[0-9]{{2,4}}(?:{sep}?[0-9]{{2,4}}){{1,3}}",
                // North American grouping.
                r"|\b[0-9]{{3}}{sep}[0-9]{{3}}{sep}[0-9]{{4}}",
                // A national number with its trunk prefix `0`.
                r"|\b0[0-9]{{2,4}}(?:{sep}|/)?[0-9]{{3,8}}(?:{sep}[0-9]{{2,6}}){{0,3}}",
                ")",
                // An extension belongs to the number.
                r"(?:\p{{Zs}}?(?:[xX]|[eE][xX][tT]\.?|[eE]xtension)\p{{Zs}}?[0-9]{{1,6}})?",
            ),
            sep = sep
        ))
    })
}

/// A number after a word that says it is one to call: what the gate finds of a phone number the
/// `Phone` class does not replace. Group 1 is the number.
fn phone_in_context() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        regex(concat!(
            r"(?i)\b(?:call|calls|called|phone|tel|telephone|mobile|cell|cellphone|sms|whatsapp",
            r"|fax|dial|ring|handy|telefon|mobil|rufnummer)\b[^\d\n]{0,30}?",
            r"(\+?\(?[0-9][0-9\p{Zs}.\-/()]{4,}[0-9])",
        ))
    })
}

/// `0900-1700` or `0800–1200`: two clock times joined by a dash.
fn time_range() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        regex(r"^(?:[01][0-9]|2[0-4]):?[0-5][0-9]\p{Zs}*[\-–]\p{Zs}*(?:[01][0-9]|2[0-4]):?[0-5][0-9]$")
    })
}

/// `2026-10-05` or `05.10.2026`.
fn date() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        regex(r"^(?:[0-9]{4}[\-/.][0-9]{1,2}[\-/.][0-9]{1,2}|[0-9]{1,2}[\-/.][0-9]{1,2}[\-/.][0-9]{2,4})$")
    })
}

/// A word that starts with a capital letter; `O'Neil` and `Jean-Luc` are one word, `Jane's` is
/// `Jane`.
fn capitalised_word() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| regex(r"\p{Lu}[\p{L}\p{M}]*(?:(?:['’]\p{Lu}|-\p{L})[\p{L}\p{M}]*)*"))
}

/// A digest written as one, `sha256:<hex>` or `sha384-<base64>`: not a credential.
fn digest() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        regex(r"(?i)^(?:sha(?:1|224|256|384|512)|sha3-[0-9]+|md5|blake2[bs]?)[:\-=][0-9a-z+/=_\-]{16,}$")
    })
}

/// An environment variable's name, `DEPLOY_TOKEN`: a placeholder for a value, not a value.
fn constant() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| regex(r"^[A-Z][A-Z0-9]*(?:_[A-Z0-9]+)+$"))
}

/// One credential shape: name, pattern, the group that is the secret, and what decides a match.
type Shape = (&'static str, Regex, usize, Refine);

/// An assigned value in text: up to whitespace or a quote, `;#&,<>` included.
const TEXT_VALUE: &str = r#"([^\s"']{6,})"#;
/// An assigned value in a document key: also up to `&` or `#`, raw or percent-encoded, as
/// `crate::mask` ends it, so the rest of a URL's query is kept.
const KEY_VALUE: &str = r#"((?:[^\s"'&#%]|%(?:[013-9a-f][0-9a-f]|2[0-24-57-9a-f])){6,})"#;

/// The credential shapes of the `Credential` class, for text or, with `key`, for a document key.
fn credentials(key: bool) -> &'static [Shape] {
    static TEXT: OnceLock<Vec<Shape>> = OnceLock::new();
    static KEY: OnceLock<Vec<Shape>> = OnceLock::new();
    if key {
        KEY.get_or_init(|| credential_shapes(KEY_VALUE))
    } else {
        TEXT.get_or_init(|| credential_shapes(TEXT_VALUE))
    }
}

fn credential_shapes(value: &str) -> Vec<Shape> {
    {
        // `auth` alone is not a name: `auth: oauth2-pkce` is a setting, not a secret.
        let assignment = format!(
            concat!(
                r#"(?i)((?:\b|%[0-9a-f]{{2}})(?:[a-z0-9]+[_\-])*"#,
                r"(?:password|passwd|passphrase|pwd|secret|token|api[_\-]?key|access[_\-]?key",
                r"|private[_\-]?key|credentials?|authorization))",
                r#"(["']?\s*(?:=>|[:=]|%3d)\s*)["']?{value}"#,
            ),
            value = value
        );
        let assignment = assignment.as_str();
        let shapes: [(&'static str, &str, usize, Refine); 23] = [
            (
                "private-key",
                r"-----BEGIN [A-Z0-9 ]*PRIVATE KEY(?: BLOCK)?-----[\s\S]*?-----END [A-Z0-9 ]*PRIVATE KEY(?: BLOCK)?-----",
                0,
                as_written,
            ),
            (
                "aws-access-key",
                r"\b(?:AKIA|ASIA|AGPA|AIDA|AROA|ANPA|ANVA|AIPA)[0-9A-Z]{16}\b",
                0,
                not_placeholder,
            ),
            (
                "google-api-key",
                r"\bAIza[0-9A-Za-z_\-]{35}",
                0,
                not_placeholder,
            ),
            (
                "google-oauth-token",
                r"\bya29\.[0-9A-Za-z_\-]{20,}",
                0,
                not_placeholder,
            ),
            (
                "azure-account-key",
                r"(?i)\bAccountKey=([A-Za-z0-9+/]{40,}={0,2})",
                1,
                not_placeholder,
            ),
            (
                "digitalocean-token",
                r"\bdo[por]_v1_[a-f0-9]{64}\b",
                0,
                not_placeholder,
            ),
            (
                "gitlab-token",
                r"\bgl(?:pat|dt|rt|ptt|cbt|oas|soat|ft|imt|agent)-[A-Za-z0-9_\-]{20,}",
                0,
                not_placeholder,
            ),
            (
                "github-token",
                r"\b(?:gh[pousr]_[A-Za-z0-9]{36,}|github_pat_[A-Za-z0-9_]{40,})",
                0,
                not_placeholder,
            ),
            (
                "bitbucket-token",
                r"\bATBB[A-Za-z0-9_\-=]{32,}",
                0,
                not_placeholder,
            ),
            ("npm-token", r"\bnpm_[A-Za-z0-9]{36}\b", 0, not_placeholder),
            (
                "pypi-token",
                r"\bpypi-AgEIcHlwaS5vcmc[A-Za-z0-9_\-]{50,}",
                0,
                not_placeholder,
            ),
            (
                "slack-token",
                r"\bxox[abposr]-[A-Za-z0-9\-]{10,}|\bxapp-[0-9]-[A-Za-z0-9\-]{20,}",
                0,
                not_placeholder,
            ),
            (
                "slack-webhook",
                r"https://hooks\.slack\.com/(?:services|workflows|triggers)/[A-Za-z0-9/_\-]{20,}",
                0,
                not_placeholder,
            ),
            (
                "discord-webhook",
                r"https://(?:ptb\.|canary\.)?discord(?:app)?\.com/api/webhooks/[0-9]+/[A-Za-z0-9_\-]{20,}",
                0,
                not_placeholder,
            ),
            (
                "telegram-bot-token",
                r"\b[0-9]{8,10}:AA[A-Za-z0-9_\-]{33}\b",
                0,
                not_placeholder,
            ),
            (
                "stripe-key",
                r"\b(?:sk|rk)_(?:live|test)_[0-9A-Za-z]{20,}",
                0,
                not_placeholder,
            ),
            (
                "sendgrid-key",
                r"\bSG\.[A-Za-z0-9_\-]{22}\.[A-Za-z0-9_\-]{43}\b",
                0,
                not_placeholder,
            ),
            (
                "model-provider-key",
                r"\b(?:sk-[A-Za-z0-9_\-]{20,}|hf_[A-Za-z0-9]{34,}|gsk_[A-Za-z0-9]{40,}|r8_[A-Za-z0-9]{37,}|xai-[A-Za-z0-9]{40,}|pplx-[A-Za-z0-9]{40,}|tvly-[A-Za-z0-9_\-]{16,})",
                0,
                not_placeholder,
            ),
            (
                "jwt",
                r"\beyJ[A-Za-z0-9_\-]{8,}\.eyJ[A-Za-z0-9_\-]{8,}\.[A-Za-z0-9_\-]{8,}",
                0,
                not_placeholder,
            ),
            (
                "basic-auth",
                r"(?i)\bauthorization\s*[:=]\s*basic\s+([A-Za-z0-9+/]{12,}={0,2})",
                1,
                not_placeholder,
            ),
            (
                "bearer",
                r"(?i)\bbearer\s+([A-Za-z0-9\-._~+/]{16,}=*)",
                1,
                mixed_secret,
            ),
            (
                "url-password",
                r"(?i)\b[a-z][a-z0-9+.\-]*://[^\s/?#@:]*:([^\s/?#@]+)@",
                1,
                not_placeholder,
            ),
            ("secret-assignment", assignment, 3, assigned_secret),
        ];
        shapes
            .into_iter()
            .map(|(name, pattern, group, refine)| (name, regex(pattern), group, refine))
            .collect()
    }
}

fn before(text: &str, at: usize) -> Option<char> {
    text[..at].chars().next_back()
}

fn after(text: &str, at: usize) -> Option<char> {
    text[at..].chars().next()
}

fn alphanumeric(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_alphanumeric())
}

fn digit(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_ascii_digit())
}

fn digits(text: &str) -> impl Iterator<Item = u32> + '_ {
    text.chars().filter_map(|c| c.to_digit(10))
}

fn luhn(text: &str) -> bool {
    let ds: Vec<u32> = digits(text).collect();
    if !(13..=19).contains(&ds.len()) {
        return false;
    }
    let sum: u32 = ds
        .iter()
        .rev()
        .enumerate()
        .map(|(i, &d)| match (i % 2, d * 2) {
            (0, _) => d,
            (_, x) if x > 9 => x - 9,
            (_, x) => x,
        })
        .sum();
    sum.is_multiple_of(10)
}

/// Whether what starts at `at` may follow a phone or card number: nothing alphanumeric, or a word
/// glued to it (`0958Fax`, `1111Exp`), which stays. More digits may not, nor letters that make
/// the whole a hexadecimal identifier (`0123456789ab`).
fn may_follow_number(text: &str, at: usize) -> bool {
    let tail: Vec<char> = text[at..]
        .chars()
        .take_while(|c| c.is_alphanumeric())
        .collect();
    tail.is_empty()
        || (tail.iter().all(|c| c.is_alphabetic()) && tail.iter().any(|c| !c.is_ascii_hexdigit()))
}

/// The longest run of whole digit groups, from the candidate's start, that is a card number.
fn card(text: &str, r: Range<usize>) -> Option<Range<usize>> {
    if alphanumeric(before(text, r.start)) {
        return None;
    }
    let s = &text[r.clone()];
    let mut ends: Vec<usize> = s
        .char_indices()
        .filter(|&(i, c)| c.is_ascii_digit() && !digit(after(s, i + 1)))
        .map(|(i, _)| i + 1)
        .collect();
    ends.reverse();
    ends.into_iter()
        .map(|end| r.start..r.start + end)
        .find(|g| luhn(&text[g.clone()]) && may_follow_number(text, g.end))
}

fn ip(text: &str, r: Range<usize>) -> Option<Range<usize>> {
    let (mut a, mut b) = (r.start, r.end);
    if !text[a..b].contains(':') {
        let ok = !alphanumeric(before(text, a))
            && before(text, a) != Some('.')
            && !alphanumeric(after(text, b))
            && !(after(text, b) == Some('.') && digit(after(text, b + 1)));
        return (ok && text[a..b].parse::<Ipv4Addr>().is_ok()).then_some(a..b);
    }
    while b > a && text[a..b].ends_with('.') {
        b -= 1;
    }
    if text[a..b].ends_with(':') && !text[a..b].ends_with("::") {
        b -= 1;
    }
    if text[a..b].starts_with(':') && !text[a..b].starts_with("::") {
        a += 1;
    }
    let t = &text[a..b];
    let groups = t.split(':').filter(|g| !g.is_empty()).count();
    let ok = !alphanumeric(before(text, a))
        && !alphanumeric(after(text, b))
        && groups >= 2
        && t.chars().any(|c| c.is_ascii_digit())
        && t.parse::<Ipv6Addr>().is_ok();
    ok.then_some(a..b)
}

fn phone_number(text: &str, r: Range<usize>) -> Option<Range<usize>> {
    let s = &text[r.clone()];
    let b = before(text, r.start);
    if !may_follow_number(text, r.end) {
        return None;
    }
    // A label may touch a number that opens with `+` or `(`; nothing may touch one that opens
    // with a digit.
    let touched = match s.chars().next() {
        Some('+' | '(') => digit(b),
        _ => alphanumeric(b) || b == Some('+'),
    };
    let main = &s[..s.find(|c: char| c.is_ascii_alphabetic()).unwrap_or(s.len())];
    let main = main.trim_end();
    let ok = !touched && (7..=15).contains(&digits(main).count()) && !time_range().is_match(main);
    ok.then_some(r)
}

/// A number the gate reads as a phone number for the word before it: 6 to 15 digits, standing
/// alone, and not a time range or a date.
fn phone_by_context(text: &str, r: Range<usize>) -> Option<Range<usize>> {
    let s = &text[r.clone()];
    let ok = !alphanumeric(before(text, r.start))
        && may_follow_number(text, r.end)
        && (6..=15).contains(&digits(s).count())
        && !time_range().is_match(s)
        && !date().is_match(s);
    ok.then_some(r)
}

/// A link without the punctuation that closes the sentence around it.
fn link(text: &str, r: Range<usize>) -> Option<Range<usize>> {
    let s = &text[r.clone()];
    let mut end = s.len();
    while let Some(last) = s[..end].chars().next_back() {
        let kept = &s[..end];
        let unbalanced = last == ')' && kept.matches('(').count() < kept.matches(')').count();
        if !(matches!(last, '.' | ',' | ';' | ':' | '!' | '?' | '*') || unbalanced) {
            break;
        }
        end -= last.len_utf8();
    }
    // A scheme or `www.` with nothing after it is no link.
    let opening = match s.find("://") {
        Some(i) => i + 3,
        None if s.len() >= 4 && s[..4].eq_ignore_ascii_case("www.") => 4,
        None => 0,
    };
    // A host is not read inside another word, address or path (`jane@host.tld/x`, `a/b.tld/c`).
    let b = before(text, r.start);
    let ok =
        end > opening && !alphanumeric(b) && !matches!(b, Some('@' | '.' | '/' | '-' | '_' | '%'));
    ok.then_some(r.start..r.start + end)
}

/// A capitalised word standing alone, with a lower-case letter in it: `API` is not one.
fn capitalised(text: &str, r: Range<usize>) -> Option<Range<usize>> {
    let b = before(text, r.start);
    let ok = !alphanumeric(b)
        && b != Some('_')
        && !alphanumeric(after(text, r.end))
        && text[r.clone()].chars().any(char::is_lowercase);
    ok.then_some(r)
}

/// Whether a credential-shaped value is a placeholder written in its place: bracketed, templated,
/// masked, an environment variable's name, a run of one character or a digest.
fn placeholder_value(v: &str) -> bool {
    let lower = v.to_lowercase();
    let mut chars = v.chars();
    let first = chars.next();
    matches!(first, Some('[' | '<' | '{' | '$' | '%'))
        || chars.all(|c| Some(c) == first)
        || [
            "xxxx",
            "****",
            "....",
            "redacted",
            "changeme",
            "change_me",
            "example",
            "placeholder",
            "your_",
            "your-",
            "dummy",
        ]
        .iter()
        .any(|p| lower.contains(p))
        || constant().is_match(v)
        || digest().is_match(v)
}

fn as_written(_: &str, r: Range<usize>) -> Option<Range<usize>> {
    Some(r)
}

fn not_placeholder(text: &str, r: Range<usize>) -> Option<Range<usize>> {
    (!placeholder_value(&text[r.clone()])).then_some(r)
}

/// A bearer token: letters and digits both, not a placeholder.
fn mixed_secret(text: &str, r: Range<usize>) -> Option<Range<usize>> {
    let v = &text[r.clone()];
    let mixed = v.chars().any(char::is_alphabetic) && v.chars().any(|c| c.is_ascii_digit());
    (mixed && !placeholder_value(v)).then_some(r)
}

/// An assigned secret: letters and digits both, or at least 20 characters; not a placeholder.
fn assigned_secret(text: &str, r: Range<usize>) -> Option<Range<usize>> {
    let v = &text[r.clone()];
    let mixed = v.chars().any(char::is_alphabetic) && v.chars().any(|c| c.is_ascii_digit());
    ((mixed || v.chars().count() >= 20) && !placeholder_value(v)).then_some(r)
}

fn built_in(class: m::RedactionClass) -> Step {
    let (regex, refine): (&Regex, Option<Refine>) = match class {
        m::RedactionClass::Url => (url(), Some(link)),
        m::RedactionClass::Email => (email(), None),
        m::RedactionClass::PaymentCard => (payment_card(), Some(card)),
        m::RedactionClass::IpAddress => (ip_address(), Some(ip)),
        m::RedactionClass::Phone => (phone(), Some(phone_number)),
        other => unreachable!("{} is not built in", class_name(other)),
    };
    Step {
        refine,
        ..Step::new(class_name(class), label(class), regex.clone())
    }
}

/// The `Credential` class's steps: irreversible, each match replaced by `[masked:<shape>]`. With
/// `key`, the steps for a document key.
fn credential_steps(key: bool) -> Vec<Step> {
    credentials(key)
        .iter()
        .map(|(shape, regex, group, refine)| Step {
            group: *group,
            refine: Some(*refine),
            replacement: Some(format!("[masked:{shape}]")),
            ..Step::new("Credential", "Credential", regex.clone())
        })
        .collect()
}

fn rare_step() -> Step {
    Step {
        refine: Some(capitalised),
        rare: true,
        ..Step::new(
            "RareName",
            label(m::RedactionClass::RareName),
            capitalised_word().clone(),
        )
    }
}

/// What the gate runs for `class` over what the model would be shown.
fn detectors(class: m::RedactionClass) -> Vec<Step> {
    match class {
        m::RedactionClass::Credential => credential_steps(false),
        m::RedactionClass::RareName => vec![rare_step()],
        m::RedactionClass::Phone => vec![
            built_in(class),
            Step {
                group: 1,
                refine: Some(phone_by_context),
                ..Step::new("Phone", "Phone", phone_in_context().clone())
            },
        ],
        other => vec![built_in(other)],
    }
}

/// A pattern for one word of a known name that matches it composed (NFC) or decomposed (NFD),
/// character by character, so a text in either form, or a mix, matches.
fn either_form(word: &str) -> String {
    word.nfc()
        .map(|c| {
            let composed = c.to_string();
            let decomposed: String = composed.nfd().collect();
            if decomposed == composed {
                regex::escape(&composed)
            } else {
                format!(
                    "(?:{}|{})",
                    regex::escape(&composed),
                    regex::escape(&decomposed)
                )
            }
        })
        .collect()
}

/// The step that replaces the policy's `known_names` as whole words, longest first, in any case
/// and in either Unicode normal form (NFC or NFD); `None` when it names none. Inner whitespace
/// matches any run of whitespace.
fn known_names(names: Option<&[String]>) -> Result<Option<Step>, String> {
    let mut names: Vec<String> = names
        .unwrap_or_default()
        .iter()
        .map(|n| n.trim().nfc().collect::<String>())
        .filter(|n| !n.is_empty())
        .collect();
    if names.is_empty() {
        return Ok(None);
    }
    names.sort_by(|a, b| b.len().cmp(&a.len()).then(a.cmp(b)));
    names.dedup();
    let word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
    let pattern = names
        .iter()
        .map(|n| {
            let body = n
                .split_whitespace()
                .map(either_form)
                .collect::<Vec<_>>()
                .join(r"\s+");
            let open = if word(n.chars().next()) { r"\b" } else { "" };
            let close = if word(n.chars().next_back()) {
                r"\b"
            } else {
                ""
            };
            format!("{open}{body}{close}")
        })
        .collect::<Vec<_>>()
        .join("|");
    let regex = RegexBuilder::new(&pattern)
        .case_insensitive(true)
        .size_limit(256 << 20)
        .build()
        .map_err(|e| format!("redaction known_names: {e}"))?;
    Ok(Some(Step::new(
        KNOWN_NAMES,
        label(m::RedactionClass::RareName),
        regex,
    )))
}

/// The redactor for `policy`, or `None` when it names nothing this module acts on. A rule whose
/// pattern does not compile is refused, naming the rule. A rule with an empty `replacement` is
/// reversible (`[<rule>-N]`); one with a `replacement` is irreversible.
pub fn compile(policy: Option<&m::RedactionPolicy>) -> Result<Option<Redactor>, String> {
    let Some(policy) = policy else {
        return Ok(None);
    };
    let has = |class: &m::RedactionClass| policy.classes.contains(class);
    let mut irreversible = Vec::new();
    let mut reversible = Vec::new();
    for rule in &policy.rules {
        let regex = Regex::new(&rule.pattern)
            .map_err(|e| format!("redaction rule {:?}: invalid pattern: {e}", rule.name))?;
        let step = Step {
            replacement: Some(rule.replacement.clone()).filter(|r| !r.is_empty()),
            ..Step::new(&rule.name, &rule.name, regex)
        };
        match step.replacement {
            Some(_) => irreversible.push(step),
            None => reversible.push(step),
        }
    }
    if has(&m::RedactionClass::Credential) {
        irreversible.extend(credential_steps(false));
    }
    let mut steps = irreversible;
    steps.extend(BUILT_IN.into_iter().filter(has).map(built_in));
    steps.extend(known_names(policy.known_names.as_deref())?);
    steps.extend(reversible);
    if has(&m::RedactionClass::RareName) {
        steps.push(rare_step());
    }
    let mut refuse: Vec<m::RedactionClass> = Vec::new();
    for class in policy.refuse_if_left.iter().flatten() {
        if !refuse.contains(class) {
            refuse.push(*class);
        }
    }
    if steps.is_empty() && refuse.is_empty() {
        return Ok(None);
    }
    let mut labels: Vec<&str> = steps
        .iter()
        .filter(|s| s.replacement.is_none())
        .map(|s| s.label.as_str())
        .collect();
    labels.sort_by(|a, b| b.len().cmp(&a.len()).then(a.cmp(b)));
    labels.dedup();
    let placeholder = if labels.is_empty() {
        None
    } else {
        let labels = labels
            .iter()
            .map(|l| regex::escape(l))
            .collect::<Vec<_>>()
            .join("|");
        let placeholder = Regex::new(&format!(
            r"(?i)\[\s*(?:{labels})\s*[-_ ]?\s*\d+\s*\]|\b(?:{labels})-\d+\b"
        ))
        .map_err(|e| format!("redaction rule names: {e}"))?;
        Some(placeholder)
    };
    let mut not_names: HashSet<String> = STOP_WORDS.iter().map(|w| w.to_string()).collect();
    not_names.extend(
        [
            m::RedactionClass::Email,
            m::RedactionClass::Phone,
            m::RedactionClass::IpAddress,
            m::RedactionClass::PaymentCard,
            m::RedactionClass::Url,
            m::RedactionClass::RareName,
        ]
        .into_iter()
        .map(|c| label(c).to_string()),
    );
    not_names.extend(steps.iter().map(|s| s.label.clone()));
    // An irreversible rule's replacement is shown to the model as written; it names nobody.
    for replacement in steps.iter().filter_map(|s| s.replacement.as_deref()) {
        not_names.extend(
            capitalised_word()
                .find_iter(replacement)
                .map(|w| w.as_str().to_string()),
        );
    }
    Ok(Some(Redactor {
        key_steps: if has(&m::RedactionClass::Credential) {
            credential_steps(true)
        } else {
            Vec::new()
        },
        words: has(&m::RedactionClass::RareName) || refuse.contains(&m::RedactionClass::RareName),
        checks: refuse
            .into_iter()
            .map(|class| (class_name(class), detectors(class)))
            .collect(),
        rare_limit: policy.rare_limit.map_or(1, |n| n.max(0) as usize),
        steps,
        placeholder,
        not_names,
    }))
}

fn next_char(text: &str, at: usize) -> usize {
    at + text[at..].chars().next().map_or(1, char::len_utf8)
}

impl Redactor {
    /// Every name this redactor counts under, each at `0`.
    pub fn counts(&self) -> BTreeMap<String, usize> {
        self.steps.iter().map(|s| (s.name.clone(), 0)).collect()
    }

    /// Whether the policy names classes to refuse when they are left (`refuse_if_left`).
    pub fn refuses(&self) -> bool {
        !self.checks.is_empty()
    }

    /// Whether `word` is a rare name for a batch that counted `words`.
    /// A word the batch's document texts do not hold (a known entity name not mentioned) was seen
    /// `0` times, and is rare.
    fn rare(&self, word: &str, words: Option<&HashMap<String, usize>>) -> bool {
        !self.not_names.contains(word)
            && words.is_some_and(|w| w.get(word).copied().unwrap_or(0) <= self.rare_limit)
    }

    /// `step`'s matches in `text` that overlap nothing in `taken`, in text order. A rare-name
    /// step finds nothing without the batch's `words`.
    fn scan(
        &self,
        step: &Step,
        text: &str,
        taken: &[Range<usize>],
        words: Option<&HashMap<String, usize>>,
    ) -> Vec<Range<usize>> {
        let mut found: Vec<Range<usize>> = Vec::new();
        if step.rare && words.is_none() {
            return found;
        }
        let mut pos = 0;
        while pos <= text.len() {
            let (whole, candidate) = if step.group == 0 {
                let Some(m) = step.regex.find_at(text, pos) else {
                    break;
                };
                (m.range(), Some(m.range()))
            } else {
                let Some(c) = step.regex.captures_at(text, pos) else {
                    break;
                };
                let whole = c.get(0).expect("a match").range();
                (whole, c.get(step.group).map(|g| g.range()))
            };
            let retry = next_char(text, whole.start);
            let range = match (candidate, step.refine) {
                (None, _) => None,
                (Some(c), _) if c.is_empty() => None,
                (Some(c), Some(refine)) => refine(text, c),
                (Some(c), None) => Some(c),
            };
            let Some(range) = range
                .filter(|r| !r.is_empty())
                .filter(|r| !step.rare || self.rare(&text[r.clone()], words))
            else {
                pos = retry;
                continue;
            };
            if let Some(t) = taken
                .iter()
                .chain(found.iter())
                .find(|t| t.start < range.end && range.start < t.end)
            {
                pos = if step.refine.is_some() {
                    retry
                } else {
                    t.end.max(retry)
                };
                continue;
            }
            pos = range.end.max(retry);
            found.push(range);
        }
        found
    }

    /// The matches in `text` of the steps `take` selects, by step, in text order and never
    /// overlapping: the first step to claim a stretch of text keeps it.
    fn find(
        &self,
        text: &str,
        take: impl Fn(&Step) -> bool,
        words: Option<&HashMap<String, usize>>,
    ) -> Vec<(usize, Range<usize>)> {
        let mut taken: Vec<Range<usize>> = Vec::new();
        let mut hits = Vec::new();
        for (i, step) in self.steps.iter().enumerate() {
            if !take(step) {
                continue;
            }
            for range in self.scan(step, text, &taken, words) {
                taken.push(range.clone());
                hits.push((i, range));
            }
        }
        hits.sort_by_key(|(_, r)| r.start);
        hits
    }

    /// `text` with every match of the irreversible steps `take` selects replaced by the step's
    /// `replacement`, and each replacement's step name and offset in the result. A match that
    /// already is the replacement is left alone and not counted.
    fn replace_irreversible(
        &self,
        text: &str,
        take: impl Fn(&Step) -> bool,
    ) -> (String, Vec<(String, usize)>) {
        let mut out = String::with_capacity(text.len());
        let mut hits = Vec::new();
        let mut last = 0;
        for (step, range) in self.find(text, |s| s.replacement.is_some() && take(s), None) {
            let step = &self.steps[step];
            let replacement = step.replacement.as_deref().expect("an irreversible step");
            if &text[range.clone()] == replacement {
                continue;
            }
            out.push_str(&text[last..range.start]);
            hits.push((step.name.clone(), out.len()));
            out.push_str(replacement);
            last = range.end;
        }
        out.push_str(&text[last..]);
        (out, hits)
    }

    /// `text` with every match of an irreversible step (a `Credential` shape or a rule with a
    /// `replacement`) replaced, and each replacement's step name and offset in the result. Run
    /// before a document is stored.
    pub fn scrub(&self, text: &str) -> (String, Vec<(String, usize)>) {
        self.replace_irreversible(text, |_| true)
    }

    /// A document key with every `Credential` shape in it masked, and how many were: a credential
    /// is never stored, in the evidence identity either.
    pub fn scrub_key(&self, key: &str) -> (String, usize) {
        let mut taken: Vec<Range<usize>> = Vec::new();
        let mut hits: Vec<(Range<usize>, &str)> = Vec::new();
        for step in &self.key_steps {
            let replacement = step.replacement.as_deref().expect("an irreversible step");
            for range in self.scan(step, key, &taken, None) {
                taken.push(range.clone());
                if key[range.clone()] != *replacement {
                    hits.push((range, replacement));
                }
            }
        }
        hits.sort_by_key(|(r, _)| r.start);
        let mut out = String::with_capacity(key.len());
        let mut last = 0;
        for (range, replacement) in &hits {
            out.push_str(&key[last..range.start]);
            out.push_str(replacement);
            last = range.end;
        }
        out.push_str(&key[last..]);
        (out, hits.len())
    }

    /// A batch: the placeholders of one model call.
    pub fn batch(&self) -> Batch<'_> {
        Batch {
            redactor: self,
            forward: HashMap::new(),
            back: HashMap::new(),
            next: HashMap::new(),
            reserved: HashSet::new(),
            words: HashMap::new(),
            counts: self.counts(),
        }
    }

    /// `text` through a batch of its own, each replacement added to `counts`.
    pub fn redact(&self, text: &str, counts: &mut BTreeMap<String, usize>) -> String {
        let mut batch = self.batch();
        batch.reserve(text);
        let out = batch.replace(text);
        for (name, n) in batch.counts {
            *counts.entry(name).or_default() += n;
        }
        out
    }
}

/// The placeholders of one batch and the values they stand for. Never written anywhere.
pub struct Batch<'a> {
    redactor: &'a Redactor,
    forward: HashMap<(usize, String), String>,
    back: HashMap<String, String>,
    next: HashMap<String, usize>,
    /// Placeholder-shaped text the originals already hold: never handed out, never restored.
    reserved: HashSet<String>,
    /// How often the batch's originals hold each capitalised word, when `RareName` is replaced or
    /// checked.
    words: HashMap<String, usize>,
    counts: BTreeMap<String, usize>,
}

impl Batch<'_> {
    /// Notes the placeholder-shaped text in an original, so no placeholder of this batch
    /// collides with it, and counts its capitalised words. Call it for every text of the batch
    /// before the first `replace`.
    pub fn reserve(&mut self, text: &str) {
        if self.redactor.words {
            // A word inside what another step hides (an address, a link, a known name, a
            // credential) is not seen.
            let hidden: Vec<Range<usize>> = self
                .redactor
                .find(text, |s| !s.rare, None)
                .into_iter()
                .map(|(_, r)| r)
                .collect();
            for found in capitalised_word().find_iter(text) {
                let r = found.range();
                if capitalised(text, r.clone()).is_some()
                    && !hidden.iter().any(|h| h.start < r.end && r.start < h.end)
                {
                    *self.words.entry(found.as_str().to_string()).or_default() += 1;
                }
            }
        }
        self.reserve_name(text);
    }

    /// Notes the placeholder-shaped text in a known entity name, as `reserve` does for a
    /// document's text, without counting its words: rarity counts only the batch's documents.
    pub fn reserve_name(&mut self, text: &str) {
        let Some(placeholder) = &self.redactor.placeholder else {
            return;
        };
        for found in placeholder.find_iter(text) {
            self.reserved.insert(found.as_str().to_string());
        }
    }

    /// Adds `n` replacements made outside this batch (by `Redactor::scrub`) to its counts.
    pub fn add(&mut self, name: &str, n: usize) {
        *self.counts.entry(name.to_string()).or_default() += n;
    }

    fn placeholder(&mut self, step: usize, value: &str) -> String {
        if let Some(p) = self.forward.get(&(step, value.to_string())) {
            return p.clone();
        }
        let label = &self.redactor.steps[step].label;
        let n = self.next.entry(label.clone()).or_insert(0);
        let p = loop {
            *n += 1;
            let p = format!("[{label}-{n}]");
            if !self.reserved.contains(&p) && !self.back.contains_key(&p) {
                break p;
            }
        };
        self.forward.insert((step, value.to_string()), p.clone());
        self.back.insert(p.clone(), value.to_string());
        p
    }

    /// `text` with every value found replaced by its placeholder, each replacement counted.
    pub fn replace(&mut self, text: &str) -> String {
        self.replace_prefix(text, text.len())
    }

    /// The first `keep` bytes of `full` with every value found in `full` replaced by its
    /// placeholder, a value the cut at `keep` splits included. Only what is kept is counted, and
    /// a split value is restored to the part kept. An irreversible step's match is replaced by
    /// its `replacement` instead, unless it already is that text.
    pub fn replace_prefix(&mut self, full: &str, keep: usize) -> String {
        let mut out = String::with_capacity(keep);
        let mut last = 0;
        let hits = self.redactor.find(full, |_| true, Some(&self.words));
        for (step, range) in hits {
            if range.start >= keep {
                break;
            }
            let end = range.end.min(keep);
            let replacement = self.redactor.steps[step].replacement.as_deref();
            if replacement == Some(&full[range.clone()]) {
                continue;
            }
            out.push_str(&full[last..range.start]);
            let p = match replacement {
                Some(r) => r.to_string(),
                None => self.placeholder(step, &full[range.start..end]),
            };
            out.push_str(&p);
            *self
                .counts
                .entry(self.redactor.steps[step].name.clone())
                .or_default() += 1;
            last = end;
        }
        out.push_str(&full[last..keep]);
        out
    }

    /// Every placeholder of this batch in the strings of `value` replaced by its value. Returns
    /// how many placeholder-shaped strings had no value; those stay as they are.
    pub fn restore(&self, value: &mut Value) -> usize {
        match value {
            Value::String(s) => {
                let Some(placeholder) = &self.redactor.placeholder else {
                    return 0;
                };
                let mut unrestored = 0;
                let restored = placeholder.replace_all(s, |c: &regex::Captures| {
                    let found = &c[0];
                    match self.back.get(found) {
                        Some(original) => original.clone(),
                        None => {
                            if !self.reserved.contains(found) {
                                unrestored += 1;
                            }
                            found.to_string()
                        }
                    }
                });
                if let std::borrow::Cow::Owned(r) = restored {
                    *s = r;
                }
                unrestored
            }
            Value::Array(items) => items.iter_mut().map(|v| self.restore(v)).sum(),
            Value::Object(map) => map.values_mut().map(|v| self.restore(v)).sum(),
            _ => 0,
        }
    }

    /// The replacements made so far, by class or rule.
    pub fn counts(&self) -> &BTreeMap<String, usize> {
        &self.counts
    }

    /// The classes `refuse_if_left` names that are still detected in `text`, a text as the model
    /// would be shown it. A capitalised word counts as a rare name only when the batch's
    /// originals hold it at most `rare_limit` times.
    pub fn left(&self, text: &str) -> BTreeSet<&'static str> {
        self.redactor
            .checks
            .iter()
            .filter(|(_, steps)| {
                steps.iter().any(|step| {
                    !self
                        .redactor
                        .scan(step, text, &[], Some(&self.words))
                        .is_empty()
                })
            })
            .map(|(class, _)| *class)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeSet;

    const ALL: [m::RedactionClass; 4] = [
        m::RedactionClass::Email,
        m::RedactionClass::Phone,
        m::RedactionClass::IpAddress,
        m::RedactionClass::PaymentCard,
    ];

    fn policy(classes: &[m::RedactionClass], rules: &[(&str, &str, &str)]) -> m::RedactionPolicy {
        m::RedactionPolicy {
            classes: classes.to_vec(),
            rules: rules
                .iter()
                .map(|(name, pattern, replacement)| m::RedactionRule {
                    name: name.to_string(),
                    pattern: pattern.to_string(),
                    replacement: replacement.to_string(),
                })
                .collect(),
            known_names: None,
            rare_limit: None,
            refuse_if_left: None,
        }
    }

    fn redactor(classes: &[m::RedactionClass], rules: &[(&str, &str, &str)]) -> Redactor {
        compile(Some(&policy(classes, rules)))
            .expect("a valid policy")
            .expect("a policy that redacts")
    }

    /// `text` through one batch of a redactor for `classes` and `rules`, and the counts it took.
    fn run(
        classes: &[m::RedactionClass],
        rules: &[(&str, &str, &str)],
        text: &str,
    ) -> (String, BTreeMap<String, usize>) {
        let redactor = redactor(classes, rules);
        let mut counts = redactor.counts();
        let out = redactor.redact(text, &mut counts);
        (out, counts)
    }

    // ---- Email ---------------------------------------------------------------------------------

    #[test]
    fn an_email_address_is_replaced_by_a_numbered_placeholder_and_counted() {
        let (out, counts) = run(
            &[m::RedactionClass::Email],
            &[],
            "Write to jane.doe+news@mail.example.com or ops@example.org, then jane.doe+news@mail.example.com.",
        );
        assert_eq!(
            out, "Write to [Email-1] or [Email-2], then [Email-1].",
            "{out}"
        );
        assert_eq!(counts["Email"], 3);
    }

    /// An apostrophe is valid in a local part, and the part before it is a person's name.
    #[test]
    fn email_with_an_apostrophe_leaves_no_part_of_the_local_part() {
        let (out, counts) = run(&ALL, &[], "Write to mary.o'neil@example.com today.");
        assert!(!out.contains("mary"), "{out}");
        assert_eq!(counts["Email"], 1, "{out}");
    }

    /// An internationalised local part (RFC 6531).
    #[test]
    fn email_with_a_non_ascii_local_part_is_replaced() {
        let (out, counts) = run(&ALL, &[], "Write to jörg.müller@example.de today.");
        assert!(!out.contains("müller"), "{out}");
        assert_eq!(counts["Email"], 1, "{out}");
    }

    /// An internationalised domain name.
    #[test]
    fn email_with_an_idn_domain_is_replaced() {
        let (out, counts) = run(&ALL, &[], "Write to jane.doe@bücher.example today.");
        assert!(!out.contains("jane.doe"), "{out}");
        assert_eq!(counts["Email"], 1, "{out}");
    }

    /// A quoted local part (RFC 5322).
    #[test]
    fn email_with_a_quoted_local_part_is_replaced() {
        let (out, _) = run(&ALL, &[], "Write to \"jane doe\"@example.com today.");
        assert!(!out.contains("jane doe"), "{out}");
    }

    /// A provider truncates `content` before cortex sees it; an address cut before its
    /// top-level domain still names the person.
    #[test]
    fn email_cut_by_a_provider_truncation_is_replaced() {
        let (out, _) = run(&ALL, &[], "Write to jane.doe@exam");
        assert!(!out.contains("jane.doe"), "{out}");
    }

    // ---- Phone ---------------------------------------------------------------------------------

    #[test]
    fn a_phone_number_is_replaced_and_dates_versions_and_counts_are_kept() {
        let text = "Call +44 20 7946 0958, (030) 1234 5678, 202-555-0143 or 0171 2345678. \
                    Released 2026-10-05 as 0.0.30 with 12 500 users.";
        let (out, counts) = run(&[m::RedactionClass::Phone], &[], text);
        for planted in [
            "+44 20 7946 0958",
            "(030) 1234 5678",
            "202-555-0143",
            "0171 2345678",
        ] {
            assert!(!out.contains(planted), "{planted} survived: {out}");
        }
        assert_eq!(counts["Phone"], 4, "{out}");
        assert!(
            out.contains("Released 2026-10-05 as 0.0.30 with 12 500 users."),
            "{out}"
        );
    }

    /// Web pages write phone numbers with non-breaking spaces (`&nbsp;`) so they do not wrap.
    #[test]
    fn phone_with_non_breaking_spaces_is_replaced() {
        let (out, counts) = run(&ALL, &[], "Call +44\u{a0}20\u{a0}7946\u{a0}0958 today.");
        assert!(!out.contains("7946"), "{out}");
        assert_eq!(counts["Phone"], 1, "{out}");
    }

    #[test]
    fn a_phone_number_with_other_unicode_spaces_is_replaced() {
        let (out, counts) = run(&ALL, &[], "Call 030\u{202f}1234\u{2009}5678 today.");
        assert!(!out.contains("1234"), "{out}");
        assert_eq!(counts["Phone"], 1, "{out}");
    }

    /// The `00` international prefix is the written form across most of Europe.
    #[test]
    fn phone_with_00_international_prefix_is_replaced() {
        let (out, counts) = run(&ALL, &[], "Call 0044 20 7946 0958 today.");
        assert!(!out.contains("7946"), "{out}");
        assert_eq!(counts["Phone"], 1, "{out}");
    }

    /// Scraped text often loses the space between a label and the number it labels.
    #[test]
    fn phone_directly_after_its_label_is_replaced() {
        let (out, _) = run(&ALL, &[], "Phone+44 20 7946 0958");
        assert!(!out.contains("7946"), "{out}");
    }

    /// A North American number with an `x` extension.
    #[test]
    fn phone_with_an_x_extension_is_replaced() {
        let (out, _) = run(&ALL, &[], "Reach me at 202-555-0143x12.");
        assert!(!out.contains("555-0143"), "{out}");
    }

    #[test]
    fn a_phone_number_is_replaced_with_its_extension() {
        let (out, counts) = run(
            &ALL,
            &[],
            "Reach me at 202-555-0143x12 or 202-555-0199 ext. 7.",
        );
        assert_eq!(out, "Reach me at [Phone-1] or [Phone-2].", "{out}");
        assert_eq!(counts["Phone"], 2);
    }

    /// Opening hours are a fact about a business, not a phone number.
    #[test]
    fn opening_hours_are_not_read_as_a_phone_number() {
        let (out, counts) = run(&ALL, &[], "Open Monday to Friday 0900-1700.");
        assert_eq!(out, "Open Monday to Friday 0900-1700.", "{out}");
        assert_eq!(counts["Phone"], 0);
    }

    /// Time ranges in both written forms are kept; a number of the same shape that is not a
    /// time range is replaced.
    #[test]
    fn time_ranges_are_kept_and_numbers_of_the_same_shape_are_replaced() {
        let kept = "Open 0900-1700, 09:00-17:00, 0800–1200 and 08:30–12:45.";
        let (out, counts) = run(&ALL, &[], kept);
        assert_eq!(out, kept);
        assert_eq!(counts["Phone"], 0);
        let (out, counts) = run(&ALL, &[], "Call 0900-17005, 0171-2345678 or 0800 1234567.");
        assert_eq!(out, "Call [Phone-1], [Phone-2] or [Phone-3].", "{out}");
        assert_eq!(counts["Phone"], 3);
    }

    #[test]
    fn digits_inside_an_identifier_are_not_a_phone_number() {
        let text = "Evidence 01926a3e-7c4f-7123-8abc-0123456789ab and SKU A0171234567.";
        let (out, counts) = run(&ALL, &[], text);
        assert_eq!(out, text);
        assert_eq!(counts["Phone"], 0);
    }

    // ---- IpAddress -----------------------------------------------------------------------------

    #[test]
    fn an_ip_address_is_replaced_and_a_version_is_kept() {
        let text = "Hosts 192.168.10.24 and 2001:db8:85a3::8a2e:370:7334 and fe80::1 \
                    run ekr 0.0.30; std::fs and 999.1.1.1 are not addresses.";
        let (out, counts) = run(&[m::RedactionClass::IpAddress], &[], text);
        assert!(!out.contains("192.168.10.24"), "{out}");
        assert!(!out.contains("2001:db8"), "{out}");
        assert!(!out.contains("fe80::1"), "{out}");
        assert_eq!(counts["IpAddress"], 3, "{out}");
        assert!(out.contains("ekr 0.0.30"), "{out}");
        assert!(out.contains("std::fs and 999.1.1.1"), "{out}");
    }

    /// `host:10.0.0.1` is a common log and configuration shape.
    #[test]
    fn ipv4_directly_after_a_colon_label_is_replaced() {
        let (out, counts) = run(&ALL, &[], "Connected host:192.168.10.24 at noon.");
        assert!(!out.contains("192.168.10.24"), "{out}");
        assert_eq!(counts["IpAddress"], 1, "{out}");
    }

    #[test]
    fn ipv6_directly_after_a_colon_label_is_replaced() {
        let (out, counts) = run(
            &ALL,
            &[],
            "Connected server:2001:db8:85a3::8a2e:370:7334 at noon.",
        );
        assert!(!out.contains("8a2e:370:7334"), "{out}");
        assert_eq!(counts["IpAddress"], 1, "{out}");
    }

    /// An IPv4-mapped IPv6 address is one address; none of its IPv4 octets may survive.
    #[test]
    fn ipv4_mapped_ipv6_leaves_no_octets() {
        let (out, counts) = run(&ALL, &[], "Client ::ffff:192.168.10.24 logged in.");
        assert!(!out.contains("168.10.24"), "{out}");
        assert_eq!(counts["IpAddress"], 1, "{out}");
    }

    #[test]
    fn times_and_an_address_with_its_port_are_told_apart() {
        let (out, counts) = run(&ALL, &[], "At 12:30:45 host 192.168.10.24:8080 answered.");
        assert_eq!(
            out, "At 12:30:45 host [IpAddress-1]:8080 answered.",
            "{out}"
        );
        assert_eq!(counts["IpAddress"], 1);
    }

    // ---- PaymentCard ---------------------------------------------------------------------------

    #[test]
    fn a_payment_card_number_is_replaced_and_a_number_failing_luhn_is_kept() {
        let text = "Card 4111 1111 1111 1111, card 5555-5555-5555-4444, \
                    amex 378282246310005; order 1234 5678 9012 3456 ships.";
        let (out, counts) = run(&[m::RedactionClass::PaymentCard], &[], text);
        assert!(!out.contains("4111 1111"), "{out}");
        assert!(!out.contains("5555-5555"), "{out}");
        assert!(!out.contains("378282246310005"), "{out}");
        assert_eq!(counts["PaymentCard"], 3, "{out}");
        assert!(out.contains("order 1234 5678 9012 3456 ships"), "{out}");
    }

    /// A 19-digit card is printed 4-4-4-4-3; this number passes Luhn over all 19 digits.
    #[test]
    fn nineteen_digit_card_in_five_groups_is_replaced() {
        let (out, counts) = run(&ALL, &[], "Card 6250 9410 1652 8599 122 paid.");
        assert!(!out.contains("6250 9410"), "{out}");
        assert_eq!(counts["PaymentCard"], 1, "{out}");
    }

    /// Non-breaking spaces between card digit groups, as an HTML receipt prints them.
    #[test]
    fn card_with_non_breaking_spaces_is_replaced() {
        let (out, counts) = run(&ALL, &[], "Card 4111\u{a0}1111\u{a0}1111\u{a0}1111 paid.");
        assert!(!out.contains("4111"), "{out}");
        assert_eq!(counts["PaymentCard"], 1, "{out}");
    }

    #[test]
    fn a_card_followed_by_more_digits_is_replaced_and_the_digits_kept() {
        let (out, counts) = run(
            &[m::RedactionClass::PaymentCard],
            &[],
            "Card 4111 1111 1111 1111 2026 paid.",
        );
        assert_eq!(out, "Card [Card-1] 2026 paid.", "{out}");
        assert_eq!(counts["PaymentCard"], 1);
    }

    // ---- Rules ---------------------------------------------------------------------------------

    #[test]
    fn a_custom_rule_is_replaced_by_a_placeholder_named_after_it_and_counted() {
        let (out, counts) = run(
            &[],
            &[("employee-id", r"\bEMP-\d{6}\b", "")],
            "Filed by EMP-004211 and EMP-918273; EMP-12 is no id.",
        );
        assert_eq!(
            out, "Filed by [employee-id-1] and [employee-id-2]; EMP-12 is no id.",
            "{out}"
        );
        assert_eq!(counts["employee-id"], 2);
    }

    /// A rule must not see a built-in's placeholder, nor count it.
    #[test]
    fn a_rule_does_not_match_a_built_in_replacement() {
        let (out, counts) = run(
            &[m::RedactionClass::Email],
            &[("account", r"\b[A-Z]{8}\b", "")],
            "Mail jane.doe@example.com today.",
        );
        assert_eq!(out, "Mail [Email-1] today.", "{out}");
        assert_eq!(counts["account"], 0, "{out}");
    }

    #[test]
    fn a_rule_does_not_match_inside_a_placeholder() {
        let (out, counts) = run(
            &[m::RedactionClass::Email],
            &[("digits", r"\d+", ""), ("label", r"Email", "")],
            "Mail jane.doe@example.com and 42.",
        );
        assert_eq!(out, "Mail [Email-1] and [digits-1].", "{out}");
        assert_eq!(counts["digits"], 1);
        assert_eq!(counts["label"], 0);
    }

    #[test]
    fn a_class_the_policy_does_not_name_is_left_alone() {
        let text = "Mail ops@example.org from 192.168.10.24.";
        let (out, counts) = run(&[m::RedactionClass::Email], &[], text);
        assert_eq!(out, "Mail [Email-1] from 192.168.10.24.");
        assert_eq!(counts.keys().collect::<Vec<_>>(), ["Email"]);
    }

    #[test]
    fn every_named_class_and_rule_is_counted_from_zero() {
        let (_, counts) = run(
            &[m::RedactionClass::Email, m::RedactionClass::Phone],
            &[("ticket", r"T-\d+", "[t]")],
            "nothing personal",
        );
        assert_eq!(
            counts,
            BTreeMap::from([
                ("Email".to_string(), 0),
                ("Phone".to_string(), 0),
                ("ticket".to_string(), 0),
            ])
        );
    }

    #[test]
    fn no_policy_and_an_empty_policy_redact_nothing() {
        assert!(compile(None).unwrap().is_none());
        assert!(compile(Some(&policy(&[], &[]))).unwrap().is_none());
    }

    #[test]
    fn an_invalid_rule_pattern_is_refused_naming_the_rule() {
        let err = compile(Some(&policy(&[], &[("broken", "(unclosed", "[x]")])))
            .err()
            .expect("an invalid pattern is refused");
        assert!(err.contains("broken"), "{err}");
    }

    /// A rule with a `replacement` is irreversible: its matches become that text, are counted
    /// under the rule's name, and nothing in an answer is ever restored to them.
    #[test]
    fn a_rule_with_a_replacement_is_replaced_by_it_and_never_restored() {
        let r = redactor(
            &[m::RedactionClass::Email],
            &[("api-key", r"\bAK-[0-9A-F]{8}\b", "[api key]")],
        );
        let mut batch = r.batch();
        let out = batch.replace("Key AK-0123ABCD for jane.doe@example.com, AK-0123ABCD again.");
        assert_eq!(
            out, "Key [api key] for [Email-1], [api key] again.",
            "{out}"
        );
        assert_eq!(batch.counts()["api-key"], 2);
        let mut answer = json!(["[api key]", "[api-key-1]", "[Email-1]"]);
        assert_eq!(batch.restore(&mut answer), 0, "{answer}");
        assert_eq!(
            answer,
            json!(["[api key]", "[api-key-1]", "jane.doe@example.com"])
        );
    }

    /// `scrub` applies only the irreversible rules: it runs before the text is stored.
    #[test]
    fn scrub_replaces_only_what_a_rule_with_a_replacement_matches() {
        let r = redactor(
            &[m::RedactionClass::Email],
            &[
                ("api-key", r"\bAK-[0-9A-F]{8}\b", "[api key]"),
                ("ticket", r"\bT-\d+\b", ""),
            ],
        );
        let (out, hits) = r.scrub("Key AK-0123ABCD, T-42, jane.doe@example.com.");
        assert_eq!(out, "Key [api key], T-42, jane.doe@example.com.");
        assert_eq!(hits, [("api-key".to_string(), 4)]);
    }

    /// A replacement its own pattern matches is not replaced or counted a second time.
    #[test]
    fn a_replacement_its_rule_matches_is_counted_once() {
        let r = redactor(&[], &[("caps", r"\b[A-Z]{8}\b", "REDACTED")]);
        let (out, hits) = r.scrub("Account ABCDEFGH.");
        assert_eq!(out, "Account REDACTED.");
        assert_eq!(hits.len(), 1);
        let mut batch = r.batch();
        assert_eq!(batch.replace(&out), "Account REDACTED.");
        assert_eq!(batch.counts()["caps"], 0);
    }

    // ---- Glued, encoded and look-alike values (adversary pass 2) -------------------------------

    /// Scraped table text glues a number to the next cell's label.
    #[test]
    fn phone_glued_to_the_label_after_it_is_replaced() {
        let (out, _) = run(&ALL, &[], "Tel +44 20 7946 0958Fax +44 20 7946 0959");
        assert!(!out.contains("7946 0958"), "{out}");
        assert_eq!(out, "Tel [Phone-1]Fax [Phone-2]");
    }

    #[test]
    fn card_glued_to_the_label_after_it_is_replaced() {
        let (out, _) = run(&ALL, &[], "Card 4111 1111 1111 1111Exp 12/27");
        assert!(!out.contains("4111 1111"), "{out}");
        assert_eq!(out, "Card [Card-1]Exp 12/27");
    }

    /// URLs carry `@` percent-encoded; the placeholder stands for the encoded form, and restores
    /// to it.
    #[test]
    fn url_encoded_email_is_replaced_and_restored_encoded() {
        let r = redactor(&ALL, &[]);
        let mut batch = r.batch();
        let out = batch.replace("https://example.org/unsubscribe?email=jane.doe%40example.com");
        assert!(!out.contains("jane.doe"), "{out}");
        assert_eq!(out, "https://example.org/unsubscribe?email=[Email-1]");
        let out = batch.replace("https://example.org/u?to=jane.doe%2Bnews%40example.com&x=1");
        assert_eq!(out, "https://example.org/u?to=[Email-2]&x=1");
        let mut answer = json!(["[Email-1]", "[Email-2]"]);
        assert_eq!(batch.restore(&mut answer), 0);
        assert_eq!(
            answer,
            json!(["jane.doe%40example.com", "jane.doe%2Bnews%40example.com"])
        );
    }

    /// Zero-padded octets, as some logs print them.
    #[test]
    fn zero_padded_ipv4_is_replaced() {
        let (out, _) = run(&ALL, &[], "Gateway 010.000.000.001 answered.");
        assert!(!out.contains("010.000"), "{out}");
    }

    /// The operator's decision: a four-part version reads as an IPv4 address and is hidden from
    /// the model, and is restored, so the store keeps it.
    #[test]
    fn four_part_version_is_hidden_from_the_model_and_restored() {
        let r = redactor(&ALL, &[]);
        let mut batch = r.batch();
        let out = batch.replace("Released version 4.18.2.1 today.");
        assert_eq!(out, "Released version [IpAddress-1] today.");
        let mut answer = json!("[IpAddress-1]");
        assert_eq!(batch.restore(&mut answer), 0);
        assert_eq!(answer, json!("4.18.2.1"));
    }

    /// A millisecond timestamp (13 digits) that happens to pass Luhn reads as a card.
    #[test]
    fn millisecond_timestamp_is_hidden_from_the_model_and_restored() {
        let r = redactor(&ALL, &[]);
        let mut batch = r.batch();
        let out = batch.replace("observed_at 1728125400007 by the crawler");
        assert_eq!(out, "observed_at [Card-1] by the crawler");
        let mut answer = json!("[Card-1]");
        assert_eq!(batch.restore(&mut answer), 0);
        assert_eq!(answer, json!("1728125400007"));
    }

    /// One document of a batch holds the literal `[Email-1]`; another holds a real address. The
    /// address must not become `[Email-1]`, and an answer echoing the literal must not get it.
    #[test]
    fn a_placeholder_literal_in_one_document_never_captures_another_documents_value() {
        let r = redactor(&ALL, &[]);
        let mut batch = r.batch();
        let a = "Mail jane.doe@example.com for the licence.";
        let b = "Reply to [Email-1] now, and to [Email-2] too.";
        batch.reserve(a);
        batch.reserve(b);
        let shown_a = batch.replace(a);
        let shown_b = batch.replace(b);
        assert_eq!(shown_b, b);
        assert_eq!(shown_a, "Mail [Email-3] for the licence.");
        let mut answer = json!({"a": "[Email-3]", "b": "[Email-1]", "c": "[Email-2]"});
        assert_eq!(batch.restore(&mut answer), 0, "{answer}");
        assert_eq!(
            answer,
            json!({"a": "jane.doe@example.com", "b": "[Email-1]", "c": "[Email-2]"})
        );
    }

    // ---- One batch: stable placeholders, truncation, restoring ---------------------------------

    #[test]
    fn one_value_has_one_placeholder_in_every_text_of_a_batch() {
        let r = redactor(&ALL, &[]);
        let mut batch = r.batch();
        let a = batch.replace("Mail jane.doe@example.com or call +44 20 7946 0958.");
        let b = batch.replace("Title by jane.doe@example.com and ops@example.org");
        assert_eq!(a, "Mail [Email-1] or call [Phone-1].");
        assert_eq!(b, "Title by [Email-1] and [Email-2]");
        assert_eq!(batch.counts()["Email"], 3);
        assert_eq!(batch.counts()["Phone"], 1);
    }

    #[test]
    fn a_placeholder_already_in_the_text_is_not_reused() {
        let r = redactor(&ALL, &[]);
        let mut batch = r.batch();
        let text = "The form shows [Email-1]; write to jane.doe@example.com.";
        batch.reserve(text);
        let out = batch.replace(text);
        assert_eq!(out, "The form shows [Email-1]; write to [Email-2].");
        let mut answer = json!({"value": "[Email-2]", "other": "[Email-1]"});
        assert_eq!(
            batch.restore(&mut answer),
            0,
            "an original's placeholder-shaped text is not unrestored: {answer}"
        );
        assert_eq!(
            answer,
            json!({"value": "jane.doe@example.com", "other": "[Email-1]"})
        );
    }

    /// A value the cut of `max_chars_per_document` splits is replaced in what is kept, and only
    /// what is kept is counted.
    #[test]
    fn a_value_cut_by_truncation_is_replaced_and_only_the_kept_text_is_counted() {
        let r = redactor(&ALL, &[]);
        let mut batch = r.batch();
        let full = "Card 4111 1111 1111 1111 paid by jane.doe@example.com.";
        let keep = "Card 4111 1111 1111 11".len();
        let out = batch.replace_prefix(full, keep);
        assert_eq!(out, "Card [Card-1]", "{out}");
        assert_eq!(batch.counts()["PaymentCard"], 1);
        assert_eq!(batch.counts()["Email"], 0);
        let mut answer = json!("[Card-1]");
        assert_eq!(batch.restore(&mut answer), 0);
        assert_eq!(
            answer,
            json!("4111 1111 1111 11"),
            "restored to the text kept"
        );
    }

    #[test]
    fn every_placeholder_in_an_answer_is_restored_and_a_mangled_or_unknown_one_counted() {
        let r = redactor(&ALL, &[("ticket", r"\bT-\d+\b", "")]);
        let mut batch = r.batch();
        let _ = batch.replace("jane.doe@example.com, +44 20 7946 0958, 4111 1111 1111 1111, T-42");
        let mut answer = json!({
            "entities": [{"node_type": "Person", "aliases": ["[Email-1]"]}],
            "facts": [
                {"value": {"value": "call [Phone-1] or pay with [Card-1]"}},
                {"quote": "see [ticket-1]"},
                {"value": {"value": "[email 1]"}},
                {"value": {"value": "[Phone-9]"}},
                {"value": {"value": "Card-1"}},
                {"value": {"value": "[masked:api-key]"}},
            ],
        });
        let unrestored = batch.restore(&mut answer);
        assert_eq!(
            answer,
            json!({
                "entities": [{"node_type": "Person", "aliases": ["jane.doe@example.com"]}],
                "facts": [
                    {"value": {"value": "call +44 20 7946 0958 or pay with 4111 1111 1111 1111"}},
                    {"quote": "see T-42"},
                    {"value": {"value": "[email 1]"}},
                    {"value": {"value": "[Phone-9]"}},
                    {"value": {"value": "Card-1"}},
                    {"value": {"value": "[masked:api-key]"}},
                ],
            })
        );
        assert_eq!(unrestored, 3, "{answer}");
    }

    // ---- Url, Credential, RareName, known names, refuse_if_left (story:redaction-names-and-gate)

    /// A policy with `known_names`, `rare_limit` and `refuse_if_left` set.
    fn full_policy(
        classes: &[m::RedactionClass],
        known_names: &[&str],
        rare_limit: Option<i64>,
        refuse_if_left: &[m::RedactionClass],
    ) -> m::RedactionPolicy {
        m::RedactionPolicy {
            known_names: (!known_names.is_empty())
                .then(|| known_names.iter().map(|n| n.to_string()).collect()),
            rare_limit,
            refuse_if_left: (!refuse_if_left.is_empty()).then(|| refuse_if_left.to_vec()),
            ..policy(classes, &[])
        }
    }

    fn compiled(policy: &m::RedactionPolicy) -> Redactor {
        compile(Some(policy))
            .expect("a valid policy")
            .expect("a policy that redacts")
    }

    #[test]
    fn a_url_is_replaced_by_a_placeholder_and_restored() {
        let r = redactor(&[m::RedactionClass::Url, m::RedactionClass::Email], &[]);
        let mut batch = r.batch();
        let text = "See https://intranet.example.org/people/jane-doe?tab=1, or \
                    http://example.com/a/b. Mail jane.doe@example.com.";
        batch.reserve(text);
        let out = batch.replace(text);
        for planted in [
            "intranet.example.org",
            "jane-doe",
            "example.com/a/b",
            "jane.doe@",
        ] {
            assert!(!out.contains(planted), "{planted} survived: {out}");
        }
        assert_eq!(
            out, "See [Url-1], or [Url-2]. Mail [Email-1].",
            "the sentence's own punctuation is kept"
        );
        assert_eq!(batch.counts()["Url"], 2);
        let mut answer = json!(["[Url-1]", "[Url-2]"]);
        assert_eq!(batch.restore(&mut answer), 0);
        assert_eq!(
            answer,
            json!([
                "https://intranet.example.org/people/jane-doe?tab=1",
                "http://example.com/a/b"
            ])
        );
    }

    /// Every shape the class covers is masked irreversibly, before storage (`scrub`) and in what
    /// the model is shown; placeholders and digests in an assignment are kept.
    #[test]
    fn a_credential_is_masked_irreversibly_and_never_restored() {
        // Assembled at run time: no credential-shaped literal sits in the source.
        let planted = [
            format!("Authorization: Bearer {}", "aB3dE5gH7jK9mN1pQ3sT5v"),
            format!("AIza{}", "Sy0123456789abcdefghijklmnopqrstuvw"),
            format!("hf_{}", "aBcDeFgHiJkLmNoPqRsTuVwXyZ01234567"),
            format!("sk_live_{}", "0123456789abcdefABCDEF"),
            format!("xapp-1-{}", "A0123456789-0123456789-abcdef"),
            format!(
                "https://hooks.slack.com/services/T0{}/B0{}/{}",
                "1234567", "7654321", "aBcD1234eFgH5678iJkL9012"
            ),
            format!("npm_{}", "aBcDeFgHiJkLmNoPqRsTuVwXyZ0123456789"),
            format!(
                "-----BEGIN PGP {p} KEY BLOCK-----\n{}\n-----END PGP {p} KEY BLOCK-----",
                "lQOYBF0aBcD",
                p = "PRIVATE"
            ),
            format!("session_token = {}", "q8Zr2Lx7Vw4Np1Ks"),
            format!("https://deploy:{}@git.example.org/repo.git", "Tr0ub4dor3x"),
        ];
        let digest = format!(
            "client_secret = sha256:{}",
            "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
        );
        let kept = [
            "api_key = <your-api-key>",
            "token: ${DEPLOY_TOKEN}",
            digest.as_str(),
            "password = [masked:secret-assignment]",
            "The token expired yesterday.",
        ];
        let r = redactor(&[m::RedactionClass::Credential], &[]);
        for secret in &planted {
            let text = format!("Before {secret} after.");
            let (stored, hits) = r.scrub(&text);
            assert_eq!(hits.len(), 1, "{secret} -> {stored}");
            assert!(
                hits.iter().all(|(name, _)| name == "Credential"),
                "{hits:?}"
            );
            assert!(stored.contains("[masked:"), "{stored}");
            let mut batch = r.batch();
            let shown = batch.replace(&text);
            assert_eq!(shown, stored, "the model is shown what is stored");
            let mut answer = json!(shown.clone());
            assert_eq!(batch.restore(&mut answer), 0);
            assert_eq!(answer, json!(stored), "nothing is restored to a credential");
        }
        // The value itself is gone, whatever the shape keeps of its name.
        let (stored, _) = r.scrub(&planted[0]);
        assert!(!stored.contains("aB3dE5gH7jK9mN1pQ3sT5v"), "{stored}");
        assert!(stored.starts_with("Authorization: Bearer "), "{stored}");
        let (stored, _) = r.scrub(&planted[9]);
        assert!(!stored.contains("Tr0ub4dor3x"), "{stored}");
        assert!(stored.contains("deploy:"), "{stored}");
        for text in kept {
            assert_eq!(r.scrub(text), (text.to_string(), Vec::new()), "{text}");
        }
    }

    #[test]
    fn a_rare_name_is_replaced_and_a_frequent_capitalised_word_kept() {
        let r = compiled(&full_policy(
            &[m::RedactionClass::RareName],
            &[],
            Some(1),
            &[],
        ));
        let mut batch = r.batch();
        let a = "Widget release notes: Rowan Pell fixed the Widget parser.";
        let b = "The Widget engine is fast. API and SKU stay.";
        batch.reserve(a);
        batch.reserve(b);
        let shown_a = batch.replace(a);
        let shown_b = batch.replace(b);
        assert!(
            !shown_a.contains("Rowan") && !shown_a.contains("Pell"),
            "{shown_a}"
        );
        assert_eq!(
            shown_a, "Widget release notes: [Name-1] [Name-2] fixed the Widget parser.",
            "Widget is seen three times in the batch and kept"
        );
        assert_eq!(
            shown_b, "The Widget engine is fast. API and SKU stay.",
            "a stop word and all-capital words are not names"
        );
        assert_eq!(batch.counts()["RareName"], 2);
        let mut answer = json!("[Name-1] [Name-2]");
        assert_eq!(batch.restore(&mut answer), 0);
        assert_eq!(answer, json!("Rowan Pell"));
    }

    #[test]
    fn a_rare_limit_above_one_replaces_a_name_seen_that_often() {
        let r = compiled(&full_policy(
            &[m::RedactionClass::RareName],
            &[],
            Some(2),
            &[],
        ));
        let mut batch = r.batch();
        let text = "Rowan wrote. Then Rowan left. Widget, Widget, Widget.";
        batch.reserve(text);
        let out = batch.replace(text);
        assert!(!out.contains("Rowan"), "{out}");
        assert!(out.contains("Widget, Widget, Widget."), "{out}");
    }

    #[test]
    fn a_known_name_is_replaced_however_often_it_is_seen_and_restored() {
        let r = compiled(&full_policy(&[], &["Jane Doe", "Rowan"], None, &[]));
        let mut batch = r.batch();
        let text = "Jane Doe met Rowan. Jane  Doe and Rowan agreed; Rowanberry is a fruit.";
        batch.reserve(text);
        let out = batch.replace(text);
        assert!(!out.contains("Jane") && !out.contains("Doe"), "{out}");
        assert_eq!(
            out,
            "[Name-1] met [Name-2]. [Name-3] and [Name-2] agreed; Rowanberry is a fruit."
        );
        assert_eq!(batch.counts()["known_names"], 4);
        let mut answer = json!(["[Name-1]", "[Name-2]"]);
        assert_eq!(batch.restore(&mut answer), 0);
        assert_eq!(answer, json!(["Jane Doe", "Rowan"]));
    }

    /// A phone number with person context that the `Phone` class does not replace is still
    /// detected by the gate; one it replaced is not.
    #[test]
    fn a_phone_number_left_after_masking_is_reported_by_the_gate() {
        let r = compiled(&full_policy(
            &[m::RedactionClass::Phone],
            &[],
            None,
            &[m::RedactionClass::Phone],
        ));
        let mut batch = r.batch();
        let left = "Jane's mobile is 7946 0958, call her.";
        let masked = "Call +44 20 7946 0958 today.";
        batch.reserve(left);
        batch.reserve(masked);
        let shown_left = batch.replace(left);
        let shown_masked = batch.replace(masked);
        assert_eq!(shown_left, left, "the Phone class does not replace it");
        assert_eq!(batch.left(&shown_left), BTreeSet::from(["Phone"]));
        assert_eq!(shown_masked, "Call [Phone-1] today.");
        assert!(batch.left(&shown_masked).is_empty(), "{shown_masked}");
        assert!(
            batch
                .left("Released 2026-10-05, call us 0900-1700 about version 4.18.")
                .is_empty(),
            "a date, opening hours and a version are not a phone number"
        );
    }

    /// The gate checks only the classes `refuse_if_left` names, on what the model would see.
    #[test]
    fn the_gate_checks_only_the_classes_it_names() {
        let r = compiled(&full_policy(
            &[m::RedactionClass::Email],
            &[],
            None,
            &[m::RedactionClass::Url, m::RedactionClass::RareName],
        ));
        let mut batch = r.batch();
        let text = "Mail jane.doe@example.com, see https://example.org/x. Rowan said so.";
        batch.reserve(text);
        let shown = batch.replace(text);
        assert_eq!(batch.left(&shown), BTreeSet::from(["RareName", "Url"]));
        let shown_names = batch.replace("write to jane.doe@example.com.");
        assert!(
            batch.left(&shown_names).is_empty(),
            "a placeholder's label is no rare name: {shown_names}"
        );
    }

    /// A policy that only refuses still compiles to a redactor.
    #[test]
    fn a_policy_that_only_refuses_compiles_and_replaces_nothing() {
        let r = compiled(&full_policy(&[], &[], None, &[m::RedactionClass::Phone]));
        assert!(r.refuses());
        let mut batch = r.batch();
        let text = "Call +44 20 7946 0958 today.";
        assert_eq!(batch.replace(text), text);
        assert_eq!(batch.left(text), BTreeSet::from(["Phone"]));
    }

    // ---- Adversary pass 1 (story:redaction-names-and-gate) --------------------------------------

    fn names_policy(
        classes: &[m::RedactionClass],
        known_names: &[&str],
        refuse_if_left: &[m::RedactionClass],
    ) -> m::RedactionPolicy {
        full_policy(classes, known_names, None, refuse_if_left)
    }

    /// What the model is shown of one text, as `run.rs` builds it: credentials masked, irreversible
    /// steps scrubbed, then the batch's placeholders; and what the gate finds left in it.
    fn shown(r: &Redactor, text: &str) -> (String, BTreeSet<&'static str>) {
        let (masked, _) = crate::mask::mask(text);
        let (scrubbed, _) = r.scrub(&masked);
        let mut batch = r.batch();
        batch.reserve(&scrubbed);
        let out = batch.replace(&scrubbed);
        let left = batch.left(&out);
        (out, left)
    }

    /// A clone URL with a password: masking on every run puts `[masked:url-password]` inside it,
    /// and the whole URL, host and path included, is still one link.
    #[test]
    fn url_with_userinfo_password_leaks_nothing_of_its_host_and_path() {
        let r = compiled(&names_policy(
            &[m::RedactionClass::Url],
            &[],
            &[m::RedactionClass::Url],
        ));
        let text = format!(
            "Clone https://jane.doe:{}@git.example.org/people/jane-doe/notes.git today.",
            ["Tr0ub", "4dor3x"].concat()
        );
        let (out, left) = shown(&r, &text);
        for planted in ["git.example.org", "jane-doe", "jane.doe"] {
            assert!(
                !out.contains(planted),
                "{planted} reached the model (gate found {left:?}): {out}"
            );
        }
    }

    /// With `Credential` on, a token in a URL's query is masked; the rest of the query is still
    /// part of the link.
    #[test]
    fn url_query_after_a_masked_credential_does_not_reach_the_model() {
        let r = compiled(&names_policy(
            &[m::RedactionClass::Url, m::RedactionClass::Credential],
            &[],
            &[m::RedactionClass::Url],
        ));
        let text = format!(
            "Reset at https://intranet.example.org/reset?token={}&user=jane.doe today.",
            ["q8Zr2Lx7", "Vw4Np1Ks"].concat()
        );
        let (out, left) = shown(&r, &text);
        assert!(
            !out.contains("jane.doe"),
            "the link's tail reached the model (gate found {left:?}): {out}"
        );
    }

    /// A link written without a scheme, as chat and tickets write them.
    #[test]
    fn a_link_without_a_scheme_does_not_reach_the_model() {
        let r = compiled(&names_policy(
            &[m::RedactionClass::Url],
            &[],
            &[m::RedactionClass::Url],
        ));
        let (out, left) = shown(
            &r,
            "Her profile is at intranet.example.org/people/jane-doe for reference.",
        );
        assert!(
            !out.contains("jane-doe"),
            "a link reached the model (gate found {left:?}): {out}"
        );
    }

    /// A name seen once in prose and once inside an address the `Email` class hides is seen once:
    /// it stays rare.
    #[test]
    fn a_name_inside_a_masked_email_does_not_make_the_visible_name_frequent() {
        let r = compiled(&names_policy(
            &[m::RedactionClass::Email, m::RedactionClass::RareName],
            &[],
            &[m::RedactionClass::RareName],
        ));
        let (out, left) = shown(&r, "Thanks, Rowan Pell <Rowan.Pell@example.org>");
        assert!(
            !out.contains("Rowan") && !out.contains("Pell"),
            "a person's name reached the model (gate found {left:?}): {out}"
        );
    }

    /// A known name in capitals or in lower case, as subjects and chat write it.
    #[test]
    fn a_known_name_in_another_case_does_not_reach_the_model() {
        let r = compiled(&names_policy(
            &[m::RedactionClass::RareName],
            &["Jane Doe"],
            &[],
        ));
        let (out, _) = shown(&r, "Subject: JANE DOE escalation. jane doe replied.");
        assert!(
            !out.to_lowercase().contains("jane"),
            "a known name reached the model: {out}"
        );
    }

    /// A known name in decomposed Unicode (NFD), as text copied from some systems carries it.
    #[test]
    fn a_known_name_in_decomposed_unicode_does_not_reach_the_model() {
        let r = compiled(&names_policy(&[], &["Zoë Faber"], &[]));
        let (out, _) = shown(&r, "Zoe\u{308} Faber wrote the release notes.");
        assert!(
            !out.contains("Faber"),
            "a known name reached the model: {out}"
        );
    }

    /// An assignment to `pwd` whose value has `;`, `#` and `&` in it is masked whole before
    /// storage.
    #[test]
    fn a_password_with_punctuation_is_kept_out_of_the_store() {
        let r = compiled(&names_policy(&[m::RedactionClass::Credential], &[], &[]));
        let secret = ["Xk9;mP2", "#vL7&qR4w"].concat();
        let text = format!("Database login: pwd: {secret}");
        let (masked, _) = crate::mask::mask(&text);
        let (stored, _) = r.scrub(&masked);
        assert!(
            !stored.contains("mP2") && !stored.contains("qR4w"),
            "the password is stored: {stored}"
        );
    }

    /// A credential false positive is irreversible: a plain `auth:` setting is not a secret.
    #[test]
    fn a_non_secret_auth_setting_is_kept_in_the_store() {
        let r = compiled(&names_policy(&[m::RedactionClass::Credential], &[], &[]));
        let (stored, _) =
            r.scrub("The gateway is configured with auth: oauth2-pkce for all clients.");
        assert!(
            stored.contains("oauth2-pkce"),
            "a non-secret fact was masked irreversibly: {stored}"
        );
    }
}
