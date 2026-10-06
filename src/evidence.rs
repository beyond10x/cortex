//! Evidence items: minted by cortex, never by the model. A fact may cite only ids issued here.

use serde_yaml_ng::value::{Tag, TaggedValue};
use serde_yaml_ng::{Mapping, Value as Yaml};
use sha2::{Digest, Sha256};

use crate::sources::{Document, Origin};

/// How EKR hashes a payload: `sha256("ekr.payload.v1" || bytes)` (`ekr hash`).
pub fn content_hash(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b"ekr.payload.v1");
    h.update(bytes);
    crate::state::hex(&h.finalize())
}

/// One evidence item and the document it stands for.
pub struct Issued {
    pub id: String,
    pub doc: Document,
    pub item: Yaml,
}

/// The identity EKR records for a document. EKR 0.0.30 admits only `!HumanStatement` evidence
/// (`extraction-evidence-kind-unsupported`), so a URL, a file or a record is named here.
pub fn identity(doc: &Document) -> String {
    match doc.origin {
        Origin::Url => doc.key.clone(),
        Origin::File | Origin::FileRecord => format!("file:{}", doc.key),
        Origin::Record => format!("record:{}", doc.key),
    }
}

/// The most bytes EKR 0.0.30 takes in one evidence payload. It reads a payload as a YAML sequence
/// of one element per byte and refuses a transaction document with a longer sequence
/// (`transaction document limit: sequence_elements (at most 16384)`), so it rejects every fact
/// citing a longer payload.
pub const PAYLOAD_MAX_BYTES: usize = 16_384;

/// The header of a document's evidence, which names it.
fn head(doc: &Document) -> String {
    let mut head = format!("Source: {}\n", identity(doc));
    if let Some(t) = &doc.title {
        head.push_str(&format!("Title: {t}\n"));
    }
    if let Some(p) = &doc.published {
        head.push_str(&format!("Published: {p}\n"));
    }
    if let Some(d) = &doc.description {
        head.push_str(&format!("Description: {d}\n"));
    }
    head.push('\n');
    head
}

/// The bytes retained as the document's evidence: a header naming it, then its text, cut at a
/// character boundary to at most [`PAYLOAD_MAX_BYTES`]. The cut keeps whole characters, not
/// grapheme clusters: a letter may lose a combining mark that follows it. The model is shown the
/// text cut at the same byte ([`fit`]), so what a fact cites is what the evidence holds.
pub fn payload(doc: &Document) -> Vec<u8> {
    payload_led(doc, "")
}

/// [`payload`] with `lead` written between the header and the text, so the cut reaches it last.
fn payload_led(doc: &Document, lead: &str) -> Vec<u8> {
    let mut bytes = head(doc);
    bytes.push_str(lead);
    bytes.push_str(&doc.text);
    bytes.truncate(bytes.floor_char_boundary(PAYLOAD_MAX_BYTES));
    bytes.into_bytes()
}

/// How many bytes `doc`'s header and `lead` take in its payload: over [`PAYLOAD_MAX_BYTES`], the
/// cut would reach into `lead`.
pub fn lead_bytes(doc: &Document, lead: &str) -> usize {
    head(doc).len() + lead.len()
}

/// Cuts `doc`'s text, at a character boundary, to what its payload holds within
/// [`PAYLOAD_MAX_BYTES`], so the model is shown no text its evidence does not hold. Answers whether
/// it cut anything.
pub fn fit(doc: &mut Document) -> bool {
    let room = PAYLOAD_MAX_BYTES.saturating_sub(head(doc).len());
    if doc.text.len() <= room {
        return false;
    }
    doc.text.truncate(doc.text.floor_char_boundary(room));
    true
}

fn key(k: &str) -> Yaml {
    Yaml::String(k.to_string())
}

/// The days from 1970-01-01 to the civil date `y-m-d` (Howard Hinnant's `days_from_civil`).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `digits` as a number when it is one or more ASCII digits and nothing else.
fn number(digits: &str) -> Option<i64> {
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// The milliseconds a fraction of a second (the digits after the point) holds, cut to whole ones.
fn fraction_ms(digits: &str) -> Option<i64> {
    number(digits)?;
    let ms: String = digits.chars().chain("000".chars()).take(3).collect();
    ms.parse().ok()
}

/// A `YYYY-MM-DD` date as days since the epoch, when it names a day of the calendar.
fn date(text: &str) -> Option<i64> {
    let b = text.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let (y, m, d) = (
        number(&text[..4])?,
        number(&text[5..7])?,
        number(&text[8..])?,
    );
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let last = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    (1..=last).contains(&d).then(|| days_from_civil(y, m, d))
}

/// An RFC 3339 instant (`2026-03-04T01:00:00.5+01:00`; `T` or a space, `Z` or an offset) in
/// milliseconds since the epoch.
fn rfc3339_ms(text: &str) -> Option<i64> {
    let b = text.as_bytes();
    if b.len() < 20 || !matches!(b[10], b'T' | b't' | b' ') || b[13] != b':' || b[16] != b':' {
        return None;
    }
    let days = date(text.get(..10)?)?;
    let (h, mi, s) = (
        number(&text[11..13])?,
        number(&text[14..16])?,
        number(&text[17..19])?,
    );
    if h > 23 || mi > 59 || s > 60 {
        return None;
    }
    let rest = &text[19..];
    let zone_at = rest.find(['Z', 'z', '+', '-'])?;
    let (fraction, zone) = rest.split_at(zone_at);
    let ms = match fraction.strip_prefix('.') {
        Some(digits) => fraction_ms(digits)?,
        None if fraction.is_empty() => 0,
        None => return None,
    };
    let offset_s = match zone {
        "Z" | "z" => 0,
        _ => {
            let sign = if zone.starts_with('-') { -1 } else { 1 };
            let z = &zone[1..];
            if z.len() != 5 || z.as_bytes()[2] != b':' {
                return None;
            }
            let (oh, om) = (number(&z[..2])?, number(&z[3..])?);
            if oh > 23 || om > 59 {
                return None;
            }
            sign * (oh * 3600 + om * 60)
        }
    };
    let secs = days * 86_400 + h * 3600 + mi * 60 + s - offset_s;
    Some(secs * 1000 + ms)
}

/// Epoch seconds with an optional fraction (a chat export's `ts`, `1767312000.250`) in milliseconds.
/// The seconds take 9 to 11 digits (1973 to 5138), so a year or a count (`2026`) is not read as an
/// instant of 1970.
fn epoch_ms(text: &str) -> Option<i64> {
    let (secs, fraction) = text.split_once('.').unwrap_or((text, ""));
    if !(9..=11).contains(&secs.len()) {
        return None;
    }
    let ms = if text.contains('.') {
        fraction_ms(fraction)?
    } else {
        0
    };
    number(secs)?.checked_mul(1000)?.checked_add(ms)
}

/// When a document says it was written, in milliseconds since the epoch: its `published` text read
/// as RFC 3339, as a date alone (00:00 UTC) or as epoch seconds with an optional fraction.
/// `None` when it has no such text, the text is none of these, or it names an instant before 1970.
pub fn document_time_ms(doc: &Document) -> Option<i64> {
    let text = doc.published.as_deref()?.trim();
    if !text.is_ascii() {
        return None;
    }
    date(text)
        .map(|days| days * 86_400_000)
        .or_else(|| rfc3339_ms(text))
        .or_else(|| epoch_ms(text))
        .filter(|&ms| ms >= 0)
}

/// The `observed_at` of `doc`'s evidence: the document's own time ([`document_time_ms`]), or
/// `run_start_ms` when it has none or names an instant after the run started.
pub fn observed_at(doc: &Document, run_start_ms: i64) -> i64 {
    document_time_ms(doc)
        .filter(|&t| t <= run_start_ms)
        .unwrap_or(run_start_ms)
}

/// Issues `doc`'s evidence, observed when the document says it was written ([`observed_at`]), or
/// at `run_start_ms`.
pub fn issue(doc: Document, operator: &str, run_start_ms: i64) -> Issued {
    issue_led(doc, "", operator, run_start_ms)
}

/// [`issue`], with `lead` written first in the payload after the header (a structured record's
/// mapped values), so the cut at [`PAYLOAD_MAX_BYTES`] takes the document's text before it.
pub fn issue_led(doc: Document, lead: &str, operator: &str, run_start_ms: i64) -> Issued {
    let observed_at_ms = observed_at(&doc, run_start_ms);
    let id = uuid::Uuid::now_v7().to_string();
    let bytes = payload_led(&doc, lead);
    let mut source = Mapping::new();
    source.insert(key("identity"), Yaml::String(identity(&doc)));
    let mut evidence = Mapping::new();
    evidence.insert(key("id"), Yaml::String(id.clone()));
    evidence.insert(
        key("source"),
        Yaml::Tagged(Box::new(TaggedValue {
            tag: Tag::new("HumanStatement"),
            value: Yaml::Mapping(source),
        })),
    );
    evidence.insert(key("content_hash"), Yaml::String(content_hash(&bytes)));
    evidence.insert(key("extracted_by"), Yaml::String(operator.to_string()));
    evidence.insert(key("observed_at"), Yaml::Number(observed_at_ms.into()));
    evidence.insert(key("confidence"), Yaml::Number(8000.into()));
    let mut item = Mapping::new();
    item.insert(key("evidence"), Yaml::Mapping(evidence));
    item.insert(
        key("payload"),
        Yaml::Sequence(
            bytes
                .iter()
                .map(|b| Yaml::Number((*b as u64).into()))
                .collect(),
        ),
    );
    Issued {
        id,
        doc,
        item: Yaml::Mapping(item),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(description: &str, text: &str) -> Document {
        Document {
            key: "https://example.org/a".into(),
            origin: Origin::Url,
            title: Some("A".into()),
            description: Some(description.into()),
            published: None,
            text: text.into(),
            hash: None,
        }
    }

    #[test]
    fn a_payload_holds_at_most_the_bound_and_ends_on_a_character_boundary() {
        for text in ["x".repeat(20_000), "ü".repeat(10_000), "示".repeat(8_000)] {
            let mut d = doc("d", &text);
            assert!(fit(&mut d));
            let bytes = payload(&d);
            assert!(bytes.len() <= PAYLOAD_MAX_BYTES && bytes.len() > PAYLOAD_MAX_BYTES - 3);
            let kept = String::from_utf8(bytes).expect("a character boundary");
            assert!(kept.ends_with(&d.text) && text.starts_with(&d.text));
        }
        let mut short = doc("d", "Example Labs.");
        assert!(!fit(&mut short));
        assert_eq!(short.text, "Example Labs.");
        // A header over the bound alone: the payload is still cut to it, the text kept empty.
        let mut long_head = doc(&"é".repeat(9_000), "text");
        assert!(fit(&mut long_head));
        assert_eq!(long_head.text, "");
        let bytes = payload(&long_head);
        assert!(bytes.len() <= PAYLOAD_MAX_BYTES);
        String::from_utf8(bytes).expect("a character boundary");
    }

    fn published(text: &str) -> Document {
        Document {
            published: Some(text.into()),
            ..doc("d", "t")
        }
    }

    #[test]
    fn a_document_time_is_rfc3339_a_date_or_epoch_seconds() {
        let at = |text: &str| document_time_ms(&published(text));
        let jan_2 = 1_767_312_000_000;
        for (text, ms) in [
            ("2026-01-02", jan_2),
            (" 2026-01-02 ", jan_2),
            ("2026-01-02T00:00:00Z", jan_2),
            ("2026-01-02t00:00:00z", jan_2),
            ("2026-01-02 00:00:00Z", jan_2),
            ("2026-01-02T01:30:00+01:30", jan_2),
            ("2026-01-01T22:00:00-02:00", jan_2),
            ("2026-01-02T00:00:00.25Z", jan_2 + 250),
            ("2026-01-02T00:00:00.123456789Z", jan_2 + 123),
            ("1767312000", jan_2),
            ("1767312000.250", jan_2 + 250),
            ("1767312000.000100", jan_2),
            ("2024-02-29", 1_709_164_800_000),
        ] {
            assert_eq!(at(text), Some(ms), "{text}");
        }
        for text in [
            "",
            "next tuesday",
            "2026-1-2",
            "2026-13-01",
            "2026-02-29",
            "2026-04-31",
            "2026-01-02T24:00:00Z",
            "2026-01-02T00:00:00",
            "2026-01-02T00:00:00+0100",
            "2026-01-02T00:00:00.Z",
            "2026-01-02T00:00:00 Z",
            "Fri, 02 Jan 2026 00:00:00 GMT",
            "1767312000.",
            ".5",
            "-1767312000",
            "１７６７３１２０００",
            "99999999999999999999",
            "0",
            "12345678",
            "123456789012",
            "1969-12-31",
            "1969-12-31T23:59:59Z",
            "1970-01-01T01:00:00+02:00",
        ] {
            assert_eq!(at(text), None, "{text}");
        }
        assert_eq!(document_time_ms(&doc("d", "t")), None);
        assert_eq!(
            crate::sources::rfc3339(at("2026-03-04T01:00:00+01:00").unwrap()),
            "2026-03-04T00:00:00Z"
        );
    }

    #[test]
    fn a_year_or_a_count_is_no_epoch_and_nothing_dates_before_1970() {
        let start = 1_791_000_000_000;
        let at = |text: &str| document_time_ms(&published(text));
        assert_eq!(at("2026"), None);
        assert_eq!(observed_at(&published("2026"), start), start);
        assert_eq!(at("1700000000.000100"), Some(1_700_000_000_000));
        assert_eq!(
            observed_at(&published("1700000000.000100"), start),
            1_700_000_000_000
        );
        assert_eq!(at("1969-12-31"), None);
        assert_eq!(observed_at(&published("1969-12-31"), start), start);
    }

    #[test]
    fn evidence_is_observed_when_its_document_was_written_and_never_after_the_run_started() {
        let start = 1_791_000_000_000;
        assert_eq!(
            observed_at(&published("2026-01-02"), start),
            1_767_312_000_000
        );
        assert_eq!(observed_at(&published("1791000000"), start), start);
        assert_eq!(observed_at(&published("1791000000.001"), start), start);
        assert_eq!(observed_at(&published("2999-01-01"), start), start);
        assert_eq!(observed_at(&published("soon"), start), start);
        assert_eq!(observed_at(&doc("d", "t"), start), start);
        let issued = issue(published("2026-01-02"), "op", start);
        assert_eq!(
            issued.item["evidence"]["observed_at"],
            Yaml::Number(1_767_312_000_000_i64.into())
        );
    }

    #[test]
    fn the_hash_is_ekrs() {
        // `ekr hash` on these 68 bytes printed this content_hash (EKR 0.0.30, 2026-10-05).
        let bytes = b"Example Labs develops the Widget engine. Its website is example.org.";
        assert_eq!(
            super::content_hash(bytes),
            "879fb030db2857eeb8a4421c6ed5a446b63615826e305d81ab0ea12ddd88719d"
        );
    }
}
