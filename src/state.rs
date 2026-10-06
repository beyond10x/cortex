//! What a source has already seen: the `cortex.instance.SeenDocument` records of
//! `spec/domains/instance.yaml`, held in one state file per source, and the `UNMAPPED:` rule of
//! `RunSource`'s `ran` that writes and reads them. A document is new when its key was never
//! applied, changed when its text hash differs, and skipped while its last application is younger
//! than the policy's `refresh_after_days`, unless it is a record read from a file
//! ([`Origin::FileRecord`]), which is delivered again as soon as it changed.

use std::collections::BTreeMap;
use std::path::Path;

use cortex_model::instance as m;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::sources::{Document, Origin};

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

/// The format of a state file.
pub const FORMAT: &str = "cortex.seen/1";

/// Whether a change to `doc` waits for the refresh window: every document but a record read from
/// a file, whose edit is delivered on the next run.
fn waits(doc: &Document) -> bool {
    doc.origin != Origin::FileRecord
}

impl SeenState {
    pub fn load(path: &Path) -> Result<Self, String> {
        match std::fs::read(path) {
            Ok(bytes) => {
                let mut state: Self = serde_json::from_slice(&bytes)
                    .map_err(|e| format!("{}: {e}", path.display()))?;
                // Every key is masked before it is recorded, so a key that still holds a
                // credential was written before that and is never matched again: dropped, so the
                // next save leaves the credential out.
                let clean = |k: &String| crate::mask::mask_key(k).1 == 0;
                state.documents.retain(|k, _| clean(k));
                state.child_failures.retain(|k, _| clean(k));
                Ok(state)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self {
                format: FORMAT.into(),
                documents: BTreeMap::new(),
                last_success_started_at: None,
                held_since: None,
                pending_since: None,
                child_failures: BTreeMap::new(),
            }),
            Err(e) => Err(format!("cannot read {}: {e}", path.display())),
        }
    }

    /// A `cortex.seen/1` document an operator names with `cortex adopt --seen`, read as a state
    /// file is: a missing file or another `format` is an error, and a key that still holds a
    /// credential is dropped.
    pub fn read_document(path: &Path) -> Result<Self, String> {
        if !path.is_file() {
            return Err(format!("{} is no file", path.display()));
        }
        let state = Self::load(path)?;
        if state.format != FORMAT {
            return Err(format!(
                "{}: format is {:?}, not {FORMAT:?}",
                path.display(),
                state.format
            ));
        }
        Ok(state)
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let text = serde_json::to_string_pretty(self).expect("plain JSON");
        crate::home::write_atomic(path, text.as_bytes())
    }

    /// The documents a run should extract: new keys, and changed text past the refresh window (or
    /// at once, for a record read from a file), at most `max` of them, in fetch order.
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
    /// window, or changed at all for a record read from a file.
    pub fn wants(&self, doc: &Document, now_ms: i64, refresh_after_days: i64) -> bool {
        match self.documents.get(&doc.key) {
            None => true,
            Some(seen) => {
                (!waits(doc) || now_ms - seen.applied_at >= refresh_after_days * DAY_MS)
                    && seen.hash != doc_hash(doc)
            }
        }
    }

    /// Whether `doc` changed but is held back: its key was applied less than `refresh_after_days`
    /// ago, with other text, and it is not a record read from a file.
    pub fn holds(&self, doc: &Document, now_ms: i64, refresh_after_days: i64) -> bool {
        waits(doc)
            && self.documents.get(&doc.key).is_some_and(|seen| {
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

    /// A state file written before keys were masked can hold a credential in a document key or a
    /// child-failure key. Such an entry is never matched again, since every key is now masked, so
    /// loading drops it and the next save leaves it out.
    #[test]
    fn loading_drops_entries_whose_keys_hold_a_credential() {
        let token = format!("Zr4{}", "p0".repeat(8));
        let raw = format!("https://example.org/a?access_token={token}");
        let masked = "https://example.org/a?access_token=[masked:secret-assignment]";
        let mut state = SeenState::default();
        state.record(&doc(&raw, "one"), 1);
        state.record(&doc(masked, "one"), 2);
        state.record(&doc("https://example.org/b", "two"), 3);
        state.child_failures.insert(format!("t:op:{raw}"), 2);
        state.child_failures.insert(format!("t:op:{masked}"), 1);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("news.json");
        state.save(&path).unwrap();

        let loaded = SeenState::load(&path).unwrap();
        assert_eq!(
            loaded.documents.keys().collect::<Vec<_>>(),
            [
                "https://example.org/a?access_token=[masked:secret-assignment]",
                "https://example.org/b"
            ]
        );
        assert_eq!(
            loaded.child_failures,
            BTreeMap::from([(format!("t:op:{masked}"), 1)])
        );
        loaded.save(&path).unwrap();
        assert!(!std::fs::read_to_string(&path).unwrap().contains(&token));
    }

    /// `cortex adopt --seen` reads a `cortex.seen/1` document as a state file, and refuses a
    /// missing file or another format rather than starting from an empty state.
    #[test]
    fn a_seen_document_is_read_as_a_state_file_and_another_format_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.json");
        let mut state = SeenState::load(&path).unwrap();
        state.record(&doc("https://example.org/a", "one"), 4);
        state.save(&path).unwrap();
        let read = SeenState::read_document(&path).unwrap();
        assert_eq!(read.documents, state.documents);

        let missing = SeenState::read_document(&dir.path().join("none.json")).unwrap_err();
        assert!(missing.contains("none.json"), "{missing}");
        std::fs::write(&path, r#"{"format": "other/1", "documents": {}}"#).unwrap();
        let other = SeenState::read_document(&path).unwrap_err();
        assert!(other.contains("other/1"), "{other}");
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

    /// A record read from a file is delivered again as soon as its text changes; the refresh
    /// window holds back only the other kinds.
    #[test]
    fn a_changed_file_record_is_selected_inside_the_refresh_window_and_never_held() {
        let record = |key: &str, text: &str| Document {
            origin: Origin::FileRecord,
            ..doc(key, text)
        };
        let mut state = SeenState::default();
        state.record(&record("a", "one"), 0);
        state.record(&record("b", "two"), 0);
        state.record(&doc("c", "three"), 0);
        let fetched = || {
            vec![
                record("a", "one"),
                record("b", "changed"),
                doc("c", "changed"),
            ]
        };
        let within = state.select(fetched(), DAY_MS, 7, 10);
        assert_eq!(
            within.iter().map(|d| d.key.as_str()).collect::<Vec<_>>(),
            ["b"]
        );
        let held: Vec<_> = fetched()
            .into_iter()
            .filter(|d| state.holds(d, DAY_MS, 7))
            .map(|d| d.key)
            .collect();
        assert_eq!(held, ["c"]);
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
