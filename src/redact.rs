//! Pseudonymising personal data in what the model is shown, by the classes and rules an
//! instance's `redaction` policy names, and restoring it in what the model answers.
//!
//! Within one batch, each distinct value a class or rule finds is replaced by a placeholder,
//! `[Email-1]`, `[Phone-2]`, `[Card-1]`, `[IpAddress-1]` or `[<rule name>-1]`, the same value by
//! the same placeholder in every text of the batch. Before the answer is applied, every
//! placeholder in it is replaced by the value it stands for; one that has no value (mangled or
//! invented by the model) stays as it is and is counted as unrestored. The mapping lives in
//! memory for one batch only. The stored evidence is the original text.
//!
//! A rule with a non-empty `replacement` is irreversible, like a credential: `scrub` replaces its
//! matches by that text before the document is stored, a batch replaces any left (in a document
//! key) before the model is shown it, and nothing is ever restored to them.
//!
//! Detection is by pattern, and runs once over the original text: a rule never sees another
//! class's placeholder. Names of people are not detected. `Url`, `Credential` and `RareName` are
//! `story:redaction-names-and-gate`'s and are not acted on here; credential shapes are masked on
//! every run regardless (`crate::mask`), irreversibly.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::ops::Range;
use std::sync::OnceLock;

use cortex_model::instance as m;
use regex::Regex;
use serde_json::Value;

/// The built-in classes this module acts on, in the order they claim text: an email address
/// before the digits inside it are read as a phone number, a card before its digit groups are.
const BUILT_IN: [m::RedactionClass; 4] = [
    m::RedactionClass::Email,
    m::RedactionClass::PaymentCard,
    m::RedactionClass::IpAddress,
    m::RedactionClass::Phone,
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
        other => class_name(other),
    }
}

/// Decides a candidate match: the range to replace, or `None` to reject it. Gets the whole
/// text, so it can look at what surrounds the candidate.
type Refine = fn(&str, Range<usize>) -> Option<Range<usize>>;

struct Step {
    /// What it is counted under: the class or the rule name.
    name: String,
    /// What its placeholders are called.
    label: String,
    regex: Regex,
    /// Built-ins only. Their candidates are bounded in length, so a rejected one is retried one
    /// character on; a rule's match is taken as it is.
    refine: Option<Refine>,
    /// An irreversible rule's text: its matches become this and get no placeholder.
    replacement: Option<String>,
}

/// What a policy pseudonymises, compiled once per run.
pub struct Redactor {
    /// Irreversible rules first, so they claim text before anything that would be restored.
    steps: Vec<Step>,
    /// Anything in an answer that looks like one of this redactor's placeholders, mangled or not;
    /// `None` when every step is irreversible.
    placeholder: Option<Regex>,
}

fn regex(pattern: &str) -> Regex {
    Regex::new(pattern).expect("a valid built-in pattern")
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

/// `0900-1700` or `0800–1200`: two clock times joined by a dash.
fn time_range() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        regex(r"^(?:[01][0-9]|2[0-4]):?[0-5][0-9]\p{Zs}*[\-–]\p{Zs}*(?:[01][0-9]|2[0-4]):?[0-5][0-9]$")
    })
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

fn built_in(class: m::RedactionClass) -> Step {
    let (regex, refine): (&Regex, Option<Refine>) = match class {
        m::RedactionClass::Email => (email(), None),
        m::RedactionClass::PaymentCard => (payment_card(), Some(card)),
        m::RedactionClass::IpAddress => (ip_address(), Some(ip)),
        m::RedactionClass::Phone => (phone(), Some(phone_number)),
        other => unreachable!("{} is not built in", class_name(other)),
    };
    Step {
        name: class_name(class).to_string(),
        label: label(class).to_string(),
        regex: regex.clone(),
        refine,
        replacement: None,
    }
}

/// The redactor for `policy`, or `None` when it names nothing this module acts on. A rule whose
/// pattern does not compile is refused, naming the rule. A rule with an empty `replacement` is
/// reversible (`[<rule>-N]`); one with a `replacement` is irreversible.
pub fn compile(policy: Option<&m::RedactionPolicy>) -> Result<Option<Redactor>, String> {
    let Some(policy) = policy else {
        return Ok(None);
    };
    let mut irreversible = Vec::new();
    let mut reversible = Vec::new();
    for rule in &policy.rules {
        let regex = Regex::new(&rule.pattern)
            .map_err(|e| format!("redaction rule {:?}: invalid pattern: {e}", rule.name))?;
        let step = Step {
            name: rule.name.clone(),
            label: rule.name.clone(),
            regex,
            refine: None,
            replacement: Some(rule.replacement.clone()).filter(|r| !r.is_empty()),
        };
        match step.replacement {
            Some(_) => irreversible.push(step),
            None => reversible.push(step),
        }
    }
    let steps: Vec<Step> = irreversible
        .into_iter()
        .chain(
            BUILT_IN
                .into_iter()
                .filter(|class| policy.classes.contains(class))
                .map(built_in),
        )
        .chain(reversible)
        .collect();
    if steps.is_empty() {
        return Ok(None);
    }
    let mut labels: Vec<&str> = steps
        .iter()
        .filter(|s| s.replacement.is_none())
        .map(|s| s.label.as_str())
        .collect();
    labels.sort_by_key(|l| std::cmp::Reverse(l.len()));
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
    Ok(Some(Redactor { steps, placeholder }))
}

fn next_char(text: &str, at: usize) -> usize {
    at + text[at..].chars().next().map_or(1, char::len_utf8)
}

impl Redactor {
    /// Every name this redactor counts under, each at `0`.
    pub fn counts(&self) -> BTreeMap<String, usize> {
        self.steps.iter().map(|s| (s.name.clone(), 0)).collect()
    }

    /// The matches in `text`, by step, in text order and never overlapping: the first step to
    /// claim a stretch of text keeps it. Only the irreversible steps' when `irreversible_only`.
    fn find(&self, text: &str, irreversible_only: bool) -> Vec<(usize, Range<usize>)> {
        let mut taken: Vec<Range<usize>> = Vec::new();
        let mut hits = Vec::new();
        for (i, step) in self.steps.iter().enumerate() {
            if irreversible_only && step.replacement.is_none() {
                continue;
            }
            let mut pos = 0;
            while pos <= text.len() {
                let Some(found) = step.regex.find_at(text, pos) else {
                    break;
                };
                let retry = next_char(text, found.start());
                let range = match step.refine {
                    _ if found.is_empty() => None,
                    Some(refine) => refine(text, found.range()),
                    None => Some(found.range()),
                };
                let Some(range) = range.filter(|r| !r.is_empty()) else {
                    pos = retry;
                    continue;
                };
                if let Some(t) = taken
                    .iter()
                    .find(|t| t.start < range.end && range.start < t.end)
                {
                    pos = if step.refine.is_some() {
                        retry
                    } else {
                        t.end.max(retry)
                    };
                    continue;
                }
                pos = range.end;
                taken.push(range.clone());
                hits.push((i, range));
            }
        }
        hits.sort_by_key(|(_, r)| r.start);
        hits
    }

    /// `text` with every match of an irreversible rule replaced by the rule's `replacement`, and
    /// each replacement's rule name and offset in the result. Run before a document is stored.
    /// A match that already is the replacement is left alone and not counted.
    pub fn scrub(&self, text: &str) -> (String, Vec<(String, usize)>) {
        let mut out = String::with_capacity(text.len());
        let mut hits = Vec::new();
        let mut last = 0;
        for (step, range) in self.find(text, true) {
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

    /// A batch: the placeholders of one model call.
    pub fn batch(&self) -> Batch<'_> {
        Batch {
            redactor: self,
            forward: HashMap::new(),
            back: HashMap::new(),
            next: HashMap::new(),
            reserved: HashSet::new(),
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
    counts: BTreeMap<String, usize>,
}

impl Batch<'_> {
    /// Notes the placeholder-shaped text in an original, so no placeholder of this batch
    /// collides with it. Call it for every text before the first `replace`.
    pub fn reserve(&mut self, text: &str) {
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
    /// a split value is restored to the part kept. An irreversible rule's match is replaced by
    /// its `replacement` instead, unless it already is that text.
    pub fn replace_prefix(&mut self, full: &str, keep: usize) -> String {
        let mut out = String::with_capacity(keep);
        let mut last = 0;
        for (step, range) in self.redactor.find(full, false) {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
}
