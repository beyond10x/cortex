//! What a source has already seen: the `cortex.instance.SeenDocument` records of
//! `spec/domains/instance.yaml`, held in one state file per source, and the `UNMAPPED:` rule of
//! `RunSource`'s `ran` that writes and reads them. A document is new when its key was never
//! applied, changed when its text hash differs, and skipped while its last application is younger
//! than the policy's `refresh_after_days`.

use std::collections::BTreeMap;
use std::path::Path;

use cortex_model::instance as m;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::sources::Document;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Seen {
    pub hash: String,
    /// Milliseconds since the Unix epoch.
    pub applied_at: i64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SeenState {
    pub format: String,
    pub documents: BTreeMap<String, Seen>,
    /// When the source's last successful run started, in milliseconds since the Unix epoch: the
    /// `{since}` of its next run. Absent until a run succeeds, so a state file written before
    /// reads as one that never had a successful run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_success_started_at: Option<i64>,
    /// The earliest window start that covers every change the last run held back by
    /// `refresh_after_days`, in milliseconds since the Unix epoch: the earliest last application
    /// of those documents, less the overlap. Recomputed by every run; absent while nothing is held
    /// back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub held_since: Option<i64>,
    /// The `{since}` of the first run since the last successful one that did not succeed, in
    /// milliseconds since the Unix epoch: no later run's `{since}` is after it until a run
    /// succeeds. Absent after a successful run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_since: Option<i64>,
    /// Consecutive runs whose child call failed, per parent document key. A successful call
    /// removes the key; absent when no call is failing.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub child_failures: BTreeMap<String, u32>,
}

pub fn text_hash(text: &str) -> String {
    hex(&Sha256::digest(text.as_bytes()))
}

/// The hash a document is remembered by: of its whole text when the run took it before the cut.
pub fn doc_hash(doc: &Document) -> String {
    doc.hash.clone().unwrap_or_else(|| text_hash(&doc.text))
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

const DAY_MS: i64 = 86_400_000;

impl SeenState {
    pub fn load(path: &Path) -> Result<Self, String> {
        match std::fs::read(path) {
            Ok(bytes) => {
                serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self {
                format: "cortex.seen/1".into(),
                documents: BTreeMap::new(),
                last_success_started_at: None,
                held_since: None,
                pending_since: None,
                child_failures: BTreeMap::new(),
            }),
            Err(e) => Err(format!("cannot read {}: {e}", path.display())),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let text = serde_json::to_string_pretty(self).expect("plain JSON");
        crate::home::write_atomic(path, text.as_bytes())
    }

    /// The documents a run should extract: new keys, and changed text past the refresh window,
    /// at most `max` of them, in fetch order.
    pub fn select(
        &self,
        docs: Vec<Document>,
        now_ms: i64,
        refresh_after_days: i64,
        max: usize,
    ) -> Vec<Document> {
        docs.into_iter()
            .filter(|d| self.wants(d, now_ms, refresh_after_days))
            .take(max)
            .collect()
    }

    /// Whether a run should extract `doc`: its key is new, or its text changed past the refresh
    /// window.
    pub fn wants(&self, doc: &Document, now_ms: i64, refresh_after_days: i64) -> bool {
        match self.documents.get(&doc.key) {
            None => true,
            Some(seen) => {
                now_ms - seen.applied_at >= refresh_after_days * DAY_MS
                    && seen.hash != doc_hash(doc)
            }
        }
    }

    /// Whether `doc` changed but is held back: its key was applied less than `refresh_after_days`
    /// ago, with other text.
    pub fn holds(&self, doc: &Document, now_ms: i64, refresh_after_days: i64) -> bool {
        self.documents.get(&doc.key).is_some_and(|seen| {
            now_ms - seen.applied_at < refresh_after_days * DAY_MS && seen.hash != doc_hash(doc)
        })
    }

    /// The seen documents of `source`, as `cortex.instance.SeenDocument` declares them, in key
    /// order. A document's id is the source id and its key, separated by a space.
    pub fn rows(&self, source: &m::SourceId) -> Vec<m::SeenDocumentData> {
        self.documents
            .iter()
            .map(|(key, seen)| m::SeenDocumentData {
                document_id: m::DocumentId(format!("{} {key}", source.0)),
                source_id: source.clone(),
                key: key.clone(),
                content_hash: seen.hash.clone(),
                applied_at: seen.applied_at,
            })
            .collect()
    }

    pub fn record(&mut self, doc: &Document, now_ms: i64) {
        self.documents.insert(
            doc.key.clone(),
            Seen {
                hash: doc_hash(doc),
                applied_at: now_ms,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::Origin;

    fn doc(key: &str, text: &str) -> Document {
        Document {
            key: key.into(),
            origin: Origin::Url,
            title: None,
            description: None,
            published: None,
            text: text.into(),
            hash: None,
        }
    }

    #[test]
    fn an_unchanged_or_recent_document_is_not_selected_again() {
        let mut state = SeenState::default();
        state.record(&doc("a", "one"), 0);
        state.record(&doc("b", "two"), 0);
        let fetched = || vec![doc("a", "one"), doc("b", "changed"), doc("c", "new")];
        let within = state.select(fetched(), DAY_MS, 7, 10);
        assert_eq!(
            within.iter().map(|d| d.key.as_str()).collect::<Vec<_>>(),
            ["c"]
        );
        let after = state.select(fetched(), 8 * DAY_MS, 7, 10);
        assert_eq!(
            after.iter().map(|d| d.key.as_str()).collect::<Vec<_>>(),
            ["b", "c"]
        );
        assert_eq!(state.select(fetched(), 8 * DAY_MS, 7, 1).len(), 1);
    }

    #[test]
    fn the_state_file_holds_the_seen_documents_the_specification_declares() {
        let mut state = SeenState::default();
        state.record(&doc("https://example.org/b", "two"), 7);
        state.record(&doc("https://example.org/a", "one"), 5);
        let source = m::SourceId("t/news".into());
        let rows = state.rows(&source);
        assert_eq!(
            rows,
            [
                m::SeenDocumentData {
                    document_id: m::DocumentId("t/news https://example.org/a".into()),
                    source_id: source.clone(),
                    key: "https://example.org/a".into(),
                    content_hash: text_hash("one"),
                    applied_at: 5,
                },
                m::SeenDocumentData {
                    document_id: m::DocumentId("t/news https://example.org/b".into()),
                    source_id: source.clone(),
                    key: "https://example.org/b".into(),
                    content_hash: text_hash("two"),
                    applied_at: 7,
                },
            ]
        );
        assert!(rows.iter().all(|r| r.broken_invariant().is_none()));
    }
}
