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

pub fn issue(doc: Document, operator: &str, observed_at_ms: i64) -> Issued {
    issue_led(doc, "", operator, observed_at_ms)
}

/// [`issue`], with `lead` written first in the payload after the header (a structured record's
/// mapped values), so the cut at [`PAYLOAD_MAX_BYTES`] takes the document's text before it.
pub fn issue_led(doc: Document, lead: &str, operator: &str, observed_at_ms: i64) -> Issued {
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
