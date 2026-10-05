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

/// The bytes retained as the document's evidence: a header naming it, then its text.
pub fn payload(doc: &Document) -> Vec<u8> {
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
    head.push_str(&doc.text);
    head.into_bytes()
}

fn key(k: &str) -> Yaml {
    Yaml::String(k.to_string())
}

pub fn issue(doc: Document, operator: &str, observed_at_ms: i64) -> Issued {
    let id = uuid::Uuid::now_v7().to_string();
    let bytes = payload(&doc);
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
