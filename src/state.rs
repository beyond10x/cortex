//! What a source has already seen: the `cortex.instance.SeenDocument` records of
//! `spec/domains/instance.yaml`, held in one state file per source, and the `UNMAPPED:` rule of
//! `RunSource`'s `ran` that writes and reads them. A document is new when its key was never
//! applied, changed when its text hash differs, and skipped while its last application is younger
//! than the policy's `refresh_after_days`, unless it is a record read from a file
//! ([`Origin::FileRecord`]), which is delivered again as soon as it changed, or a structured
//! source's change whose text changed only in its links ([`SeenState::unlinked`]).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use cortex_model::instance as m;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::sources::{Document, Origin, Window, DOCUMENT_FAILURE_LIMIT, OVERLAP_MS};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Seen {
    /// The hash of the text applied ([`doc_hash`]). Absent for a document adopted by its key alone
    /// (`cortex adopt --seen`, an entry with `applied_at` and no hash) until a run fetches it and
    /// records it ([`SeenState::hash_unhashed`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// Milliseconds since the Unix epoch.
    pub applied_at: i64,
    /// For a structured source's change record applied with links, the hash of its text without
    /// them ([`crate::structured::LINKS`]); absent when the text held none, as its `hash` is that.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unlinked_hash: Option<String>,
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
    /// This run's, never written: by key, the hash of the text without its links of each change
    /// record of a structured source's link ([`crate::structured::Source::link`]). A change whose
    /// text differs from the one last applied only in its links is wanted at once and never held,
    /// whatever `refresh_after_days`, and [`SeenState::record`] remembers it.
    #[serde(skip)]
    pub unlinked: BTreeMap<String, String>,
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
                unlinked: BTreeMap::new(),
            }),
            Err(e) => Err(format!("cannot read {}: {e}", path.display())),
        }
    }

    /// A `cortex.seen/1` document an operator names with `cortex adopt --seen`, read as a state
    /// file is: a missing file or another `format` is an error, and a key that still holds a
    /// credential is dropped. An entry names its `applied_at`, with or without a hash (a document
    /// adopted by its key alone); one with no `applied_at` is an error naming its key.
    pub fn read_document(path: &Path) -> Result<Self, String> {
        if !path.is_file() {
            return Err(format!("{} is no file", path.display()));
        }
        let text =
            std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        // Each entry is checked on its own first, so a refusal names its key: one with no
        // `applied_at` (missing or null), and one serde would refuse for any other field.
        if let Ok(serde_json::Value::Object(document)) = serde_json::from_slice(&text) {
            if let Some(serde_json::Value::Object(entries)) = document.get("documents") {
                for (key, entry) in entries {
                    if entry
                        .get("applied_at")
                        .is_none_or(serde_json::Value::is_null)
                    {
                        return Err(format!(
                            "{}: document {key:?} has no applied_at: name it, with or without a \
                             hash",
                            path.display()
                        ));
                    }
                    if let Err(e) = Seen::deserialize(entry) {
                        return Err(format!("{}: document {key:?}: {e}", path.display()));
                    }
                }
            }
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
    /// window, or changed at all for a record read from a file or in its links alone. A key seen
    /// with no hash (adopted by key, not yet fetched) is not extracted.
    pub fn wants(&self, doc: &Document, now_ms: i64, refresh_after_days: i64) -> bool {
        match self.documents.get(&doc.key) {
            None => true,
            Some(seen) => seen.hash.as_ref().is_some_and(|hash| {
                (!waits(doc)
                    || now_ms - seen.applied_at >= refresh_after_days * DAY_MS
                    || self.relinked(doc, seen))
                    && *hash != doc_hash(doc)
            }),
        }
    }

    /// Whether `doc` changed but is held back: its key was applied less than `refresh_after_days`
    /// ago, with other text, and it is not a record read from a file nor a change whose links
    /// alone changed. A key seen with no hash is not held.
    pub fn holds(&self, doc: &Document, now_ms: i64, refresh_after_days: i64) -> bool {
        waits(doc)
            && self.documents.get(&doc.key).is_some_and(|seen| {
                seen.hash.as_ref().is_some_and(|hash| {
                    now_ms - seen.applied_at < refresh_after_days * DAY_MS
                        && *hash != doc_hash(doc)
                        && !self.relinked(doc, seen)
                })
            })
    }

    /// Whether `doc`, a change record of a link, has the text `seen` was applied with but for its
    /// links: its hash without them ([`SeenState::unlinked`]) is the one remembered, or, for a
    /// text applied with none, its `hash`.
    fn relinked(&self, doc: &Document, seen: &Seen) -> bool {
        self.unlinked.get(&doc.key).is_some_and(|unlinked| {
            seen.unlinked_hash.as_ref().or(seen.hash.as_ref()) == Some(unlinked)
        })
    }

    /// Records, for every fetched document whose key is seen with no hash (adopted by its key
    /// alone), the hash of the document as this run fetched it, keeping its `applied_at`, and
    /// answers how many it recorded. Such a document is then seen as applied with that text, so it
    /// is not extracted; a key that is not fetched keeps no hash.
    pub fn hash_unhashed(&mut self, docs: &[Document]) -> usize {
        let mut hashed = 0;
        for doc in docs {
            let Some(seen) = self
                .documents
                .get_mut(&doc.key)
                .filter(|seen| seen.hash.is_none())
            else {
                continue;
            };
            let hash = doc_hash(doc);
            seen.unlinked_hash = self
                .unlinked
                .get(&doc.key)
                .filter(|unlinked| **unlinked != hash)
                .cloned();
            seen.hash = Some(hash);
            hashed += 1;
        }
        hashed
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
                unlinked_hash: seen.unlinked_hash.clone(),
                applied_at: seen.applied_at,
            })
            .collect()
    }

    /// Remembers `doc` as applied at `now_ms`: its hash, and for a change record whose text holds
    /// links, its hash without them ([`SeenState::unlinked`]).
    pub fn record(&mut self, doc: &Document, now_ms: i64) {
        let hash = doc_hash(doc);
        let unlinked_hash = self
            .unlinked
            .get(&doc.key)
            .filter(|unlinked| **unlinked != hash)
            .cloned();
        self.documents.insert(
            doc.key.clone(),
            Seen {
                hash: Some(hash),
                applied_at: now_ms,
                unlinked_hash,
            },
        );
    }

    /// Whether `wanted`, as this run fetched it, is seen with its text: applied by this run, since
    /// a wanted document's key is new or seen with other text.
    fn holds_text(&self, wanted: &Wanted) -> bool {
        self.documents
            .get(&wanted.key)
            .and_then(|seen| seen.hash.as_ref())
            == Some(&wanted.hash)
    }

    /// The `pending_since` a windowed run that did not succeed leaves, after what it applied: the
    /// earliest record time of a document it wanted and did not apply, less [`OVERLAP_MS`], when
    /// its fetch read every record of the window and that time is readable for every such
    /// document; the window's start otherwise, and when it left no wanted document unapplied. It
    /// is never before the window's start nor after its end.
    pub fn pending_after(&self, backlog: &Backlog, window: Window) -> i64 {
        if !backlog.complete {
            return window.since_ms;
        }
        let mut earliest: Option<i64> = None;
        for wanted in backlog.wanted.iter().filter(|w| !self.holds_text(w)) {
            let Some(at) = wanted.time_ms else {
                return window.since_ms;
            };
            earliest = Some(earliest.map_or(at, |e| e.min(at)));
        }
        earliest.map_or(window.since_ms, |at| {
            at.saturating_sub(OVERLAP_MS)
                .clamp(window.since_ms, window.until_ms.max(window.since_ms))
        })
    }
}

/// What a windowed run wanted of its fetch, for the window it leaves ([`SeenState::pending_after`]).
/// Kept with the run's report, so a run that failed, or was undone, leaves its window by the
/// same rule.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Backlog {
    /// Whether the fetch read every record of the window: no page or child call left unread.
    pub complete: bool,
    /// Every document the run wanted, as it fetched it; a document skipped after failing on
    /// [`DOCUMENT_FAILURE_LIMIT`] consecutive runs is not one of them.
    pub wanted: Vec<Wanted>,
}

/// A document a windowed run wanted: its key, its hash ([`doc_hash`]) and its record time
/// ([`crate::evidence::document_time_ms`]), `None` when that cannot be read.
#[derive(Debug, Clone, PartialEq)]
pub struct Wanted {
    pub key: String,
    pub hash: String,
    pub time_ms: Option<i64>,
}

impl Wanted {
    pub fn of(doc: &Document) -> Self {
        Self {
            key: doc.key.clone(),
            hash: doc_hash(doc),
            time_ms: crate::evidence::document_time_ms(doc),
        }
    }
}

/// The format of a failures file.
pub const FAILURES_FORMAT: &str = "cortex.failures/1";

/// `<instance>/failures/<source>.json`: the source's [`Failures`], outside `state/`, so neither a
/// snapshot nor a rewind of `state/` takes it back.
pub fn failures_path(instance_dir: &Path, source: &str) -> PathBuf {
    instance_dir.join("failures").join(format!("{source}.json"))
}

/// Consecutive runs whose extraction of a document failed with the document alone in its
/// request, per key: the hash of the text that failed, the count, and the last reason. A document
/// of a batch of several that failed is marked with a count of `0` ([`Failures::failed_in_batch`]),
/// so a later run splits a failing batch that holds it ([`Failures::marked`]). A changed text
/// starts the count again; an applied document drops its entry. From [`DOCUMENT_FAILURE_LIMIT`]
/// on, a run leaves the document out ([`Failures::skips`]).
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Failures {
    pub format: String,
    #[serde(default)]
    pub documents: BTreeMap<String, Failed>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Failed {
    pub hash: String,
    pub runs: u32,
    pub reason: String,
}

impl Failures {
    /// The failures file at `path`; a missing one holds none. A key that still holds a
    /// credential is dropped, as [`SeenState::load`] drops it.
    pub fn load(path: &Path) -> Result<Self, String> {
        match std::fs::read(path) {
            Ok(bytes) => {
                let mut failures: Self = serde_json::from_slice(&bytes)
                    .map_err(|e| format!("{}: {e}", path.display()))?;
                failures
                    .documents
                    .retain(|k, _| crate::mask::mask_key(k).1 == 0);
                Ok(failures)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self {
                format: FAILURES_FORMAT.into(),
                documents: BTreeMap::new(),
            }),
            Err(e) => Err(format!("cannot read {}: {e}", path.display())),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let text = serde_json::to_string_pretty(self).expect("plain JSON");
        crate::home::write_atomic(path, text.as_bytes())
    }

    /// The entry that leaves `doc` out of a run: its text failed alone on
    /// [`DOCUMENT_FAILURE_LIMIT`] or more consecutive runs.
    pub fn skips(&self, doc: &Document) -> Option<&Failed> {
        self.documents
            .get(&doc.key)
            .filter(|f| f.runs >= DOCUMENT_FAILURE_LIMIT && f.hash == doc_hash(doc))
    }

    /// Counts one more failure of `doc` alone, for `reason`, and answers the count: one more than
    /// before for the same text, `1` for a text that changed.
    pub fn failed(&mut self, doc: &Document, reason: &str) -> u32 {
        let hash = doc_hash(doc);
        let runs = match self.documents.get(&doc.key) {
            Some(f) if f.hash == hash => f.runs.saturating_add(1),
            _ => 1,
        };
        self.documents.insert(
            doc.key.clone(),
            Failed {
                hash,
                runs,
                reason: reason.to_string(),
            },
        );
        runs
    }

    /// Marks `doc`, of a batch of several documents that failed, for `reason`: a count of `0`,
    /// unless its text already has one.
    pub fn failed_in_batch(&mut self, doc: &Document, reason: &str) {
        let hash = doc_hash(doc);
        if self.documents.get(&doc.key).is_some_and(|f| f.hash == hash) {
            return;
        }
        self.documents.insert(
            doc.key.clone(),
            Failed {
                hash,
                runs: 0,
                reason: reason.to_string(),
            },
        );
    }

    /// Whether `doc`'s text failed before, alone or in a batch: a failing batch that holds it is
    /// split.
    pub fn marked(&self, doc: &Document) -> bool {
        self.documents
            .get(&doc.key)
            .is_some_and(|f| f.hash == doc_hash(doc))
    }

    /// Drops the count of an applied document; answers whether it had one.
    pub fn applied(&mut self, key: &str) -> bool {
        self.documents.remove(key).is_some()
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

    /// An entry of a `--seen` document may name a key and its `applied_at` with no hash (adopted
    /// by key); one with no `applied_at` is refused, naming its key.
    #[test]
    fn a_seen_document_entry_needs_applied_at_and_may_lack_a_hash() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("chat.json");
        std::fs::write(
            &path,
            r#"{"format": "cortex.seen/1", "documents": {"archive/2026-09.jsonl#1.0": {"applied_at": 1}}}"#,
        )
        .unwrap();
        let read = SeenState::read_document(&path).unwrap();
        assert_eq!(
            read.documents["archive/2026-09.jsonl#1.0"],
            Seen {
                hash: None,
                applied_at: 1,
                unlinked_hash: None,
            }
        );
        // Saved as the adopted state: still no hash.
        read.save(&path).unwrap();
        assert!(!std::fs::read_to_string(&path).unwrap().contains("hash"));

        for entry in [r#"{}"#, r#"{"hash": "ab"}"#] {
            std::fs::write(
                &path,
                format!(r#"{{"format": "cortex.seen/1", "documents": {{"a#7": {entry}}}}}"#),
            )
            .unwrap();
            let refused = SeenState::read_document(&path).unwrap_err();
            assert!(refused.contains("\"a#7\""), "{refused}");
            assert!(refused.contains("applied_at"), "{refused}");
        }
    }

    /// Every entry a `--seen` document cannot hold is refused naming its key, not by a parse
    /// error that names none: a null `applied_at`, a field not of its type, an entry that is no
    /// object. A null `hash` reads as no hash.
    #[test]
    fn a_seen_document_entry_of_the_wrong_shape_is_refused_naming_its_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("chat.json");
        let write = |entry: &str| {
            std::fs::write(
                &path,
                format!(r#"{{"format": "cortex.seen/1", "documents": {{"ok#1": {{"applied_at": 1}}, "a#7": {entry}}}}}"#),
            )
            .unwrap();
        };
        for entry in [
            r#"{"applied_at": null}"#,
            r#"{"hash": null, "applied_at": null}"#,
            r#"{"hash": "ab", "applied_at": null}"#,
            r#"{"applied_at": "1"}"#,
            r#"{"applied_at": 1.5}"#,
            r#"{"applied_at": true}"#,
            r#"{"hash": 5, "applied_at": 1}"#,
            r#"{"hash": ["ab"], "applied_at": 1}"#,
            r#"{"applied_at": 1, "unlinked_hash": 5}"#,
            r#"5"#,
            r#"null"#,
            r#""ab""#,
        ] {
            write(entry);
            let refused = SeenState::read_document(&path).unwrap_err();
            assert!(refused.contains("\"a#7\""), "{entry}: {refused}");
        }
        write(r#"{"hash": null, "applied_at": 2}"#);
        let read = SeenState::read_document(&path).unwrap();
        assert_eq!(read.documents["a#7"].hash, None);
        assert_eq!(read.documents["a#7"].applied_at, 2);
    }

    /// A document seen with no hash is neither wanted nor held; once a run fetched it, it carries
    /// the hash of what the run fetched under its key, keeps its `applied_at`, and is extracted
    /// again only when that hash changes. A key that is not fetched keeps no hash.
    #[test]
    fn a_document_seen_without_a_hash_is_hashed_when_fetched_and_not_extracted() {
        let record = |key: &str, text: &str| Document {
            origin: Origin::FileRecord,
            ..doc(key, text)
        };
        let adopted = Seen {
            hash: None,
            applied_at: 1,
            unlinked_hash: None,
        };
        let mut state = SeenState::default();
        state.documents.insert("a".into(), adopted.clone());
        state.documents.insert("gone".into(), adopted.clone());
        state.documents.insert("c".into(), adopted.clone());
        let fetched = vec![record("a", "one"), doc("c", "three"), record("b", "two")];
        assert!(!state.wants(&fetched[0], DAY_MS, 7));
        assert!(!state.holds(&fetched[1], DAY_MS, 7));

        assert_eq!(state.hash_unhashed(&fetched), 2);
        assert_eq!(state.documents["a"].hash, Some(text_hash("one")));
        assert_eq!(state.documents["a"].applied_at, 1);
        assert_eq!(state.documents["c"].hash, Some(text_hash("three")));
        assert_eq!(state.documents["gone"], adopted);
        assert_eq!(state.hash_unhashed(&fetched), 0, "hashed once");

        let keys = |docs: Vec<Document>| docs.into_iter().map(|d| d.key).collect::<Vec<_>>();
        assert_eq!(keys(state.select(fetched, DAY_MS, 7, 10)), ["b"]);
        let edited = vec![record("a", "one, edited"), doc("c", "three, edited")];
        assert_eq!(keys(state.select(edited.clone(), DAY_MS, 7, 10)), ["a"]);
        assert_eq!(keys(state.select(edited, 8 * DAY_MS, 7, 10)), ["a", "c"]);

        let rows = state.rows(&m::SourceId("t/chat".into()));
        let gone = rows.iter().find(|r| r.key == "gone").unwrap();
        assert_eq!(gone.content_hash, None);
        assert!(gone.broken_invariant().is_none());
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

    /// A change record of a link whose text differs from the one last applied only in its links
    /// is selected inside the refresh window and never held, whether it gains, changes or loses
    /// them; one whose text changed besides waits, as does any other document. The hash without
    /// the links is remembered only for a text that held some, and this run's map is never
    /// written.
    #[test]
    fn a_change_whose_links_alone_changed_is_selected_inside_the_refresh_window() {
        let keys = |docs: Vec<Document>| docs.into_iter().map(|d| d.key).collect::<Vec<_>>();
        let mut state = SeenState {
            unlinked: BTreeMap::from([
                ("a".to_string(), text_hash("one")),
                ("b".to_string(), text_hash("two")),
            ]),
            ..SeenState::default()
        };
        state.record(&doc("a", "one"), 0);
        state.record(&doc("b", "two"), 0);
        state.record(&doc("c", "three"), 0);
        assert_eq!(state.documents["a"].unlinked_hash, None);

        // `a` gains a link; `b` gains one and changed besides; `c` is no change of a link.
        state.unlinked = BTreeMap::from([
            ("a".to_string(), text_hash("one")),
            ("b".to_string(), text_hash("two, edited")),
        ]);
        let fetched = || {
            vec![
                doc("a", "one+link"),
                doc("b", "two, edited+link"),
                doc("c", "three, edited"),
            ]
        };
        assert_eq!(keys(state.select(fetched(), DAY_MS, 7, 10)), ["a"]);
        let held: Vec<_> = fetched()
            .into_iter()
            .filter(|d| state.holds(d, DAY_MS, 7))
            .map(|d| d.key)
            .collect();
        assert_eq!(held, ["b", "c"]);

        // Applied with its link, `a` remembers its text without it: losing the link is a change
        // of its links alone, an edit besides is not.
        state.record(&doc("a", "one+link"), DAY_MS);
        assert_eq!(state.documents["a"].unlinked_hash, Some(text_hash("one")));
        assert_eq!(
            keys(state.select(vec![doc("a", "one")], 2 * DAY_MS, 7, 10)),
            ["a"]
        );
        state.unlinked.insert("a".into(), text_hash("one, edited"));
        assert!(state
            .select(vec![doc("a", "one, edited+link")], 2 * DAY_MS, 7, 10)
            .is_empty());

        // Written and read back: the hash without links stays, this run's map does not.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("projects.json");
        state.save(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("\"unlinked\""), "{text}");
        let loaded = SeenState::load(&path).unwrap();
        assert_eq!(loaded.documents, state.documents);
        assert!(loaded.unlinked.is_empty());
        let rows = loaded.rows(&m::SourceId("t/projects".into()));
        assert_eq!(rows[0].unlinked_hash, Some(text_hash("one")));
        assert_eq!(rows[1].unlinked_hash, None);
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
                    content_hash: Some(text_hash("one")),
                    unlinked_hash: None,
                    applied_at: 5,
                },
                m::SeenDocumentData {
                    document_id: m::DocumentId("t/news https://example.org/b".into()),
                    source_id: source.clone(),
                    key: "https://example.org/b".into(),
                    content_hash: Some(text_hash("two")),
                    unlinked_hash: None,
                    applied_at: 7,
                },
            ]
        );
        assert!(rows.iter().all(|r| r.broken_invariant().is_none()));
    }

    fn timed(key: &str, text: &str, published: Option<&str>) -> Document {
        Document {
            published: published.map(str::to_string),
            ..doc(key, text)
        }
    }

    const WINDOW: Window = Window {
        since_ms: 1_000_000_000_000,
        until_ms: 1_000_900_000_000,
    };

    /// `story:window-advances-past-applied`: the window moves to the earliest wanted document left
    /// unapplied, less the overlap; applied documents, however early, do not hold it.
    #[test]
    fn the_pending_window_is_the_earliest_unapplied_record_time_less_the_overlap() {
        // 2001-09-09T01:46:40Z is 1_000_000_000_000 ms; the two records come a day and two later.
        let (a, b, c) = (
            timed("a", "one", Some("2001-09-10T01:46:40Z")),
            timed("b", "two", Some("2001-09-11T01:46:40Z")),
            timed("c", "three", Some("2001-09-09T01:50:00Z")),
        );
        let backlog = Backlog {
            complete: true,
            wanted: [&a, &b, &c].into_iter().map(Wanted::of).collect(),
        };
        let mut state = SeenState::default();
        state.record(&c, 1);
        assert_eq!(
            state.pending_after(&backlog, WINDOW),
            1_000_086_400_000 - OVERLAP_MS,
            "a, the earliest left unapplied"
        );
        state.record(&a, 1);
        assert_eq!(
            state.pending_after(&backlog, WINDOW),
            1_000_172_800_000 - OVERLAP_MS
        );
        // c applied earlier with other text is wanted again, and holds the window at its time
        // less the overlap, which is before the window's start: the start, then.
        state.record(&timed("c", "older", None), 1);
        assert_eq!(state.pending_after(&backlog, WINDOW), WINDOW.since_ms);
    }

    /// The window keeps its start when records were left unread, when a document left unapplied
    /// has no readable time, and when no wanted document is left unapplied.
    #[test]
    fn the_pending_window_keeps_its_start_when_it_cannot_tell() {
        let a = timed("a", "one", Some("2001-09-10T01:46:40Z"));
        let untimed = timed("u", "two", Some("last week"));
        let state = SeenState::default();
        let unread = Backlog {
            complete: false,
            wanted: vec![Wanted::of(&a)],
        };
        assert_eq!(state.pending_after(&unread, WINDOW), WINDOW.since_ms);
        let unreadable = Backlog {
            complete: true,
            wanted: vec![Wanted::of(&a), Wanted::of(&untimed)],
        };
        assert_eq!(state.pending_after(&unreadable, WINDOW), WINDOW.since_ms);
        assert_eq!(
            state.pending_after(&Backlog::default(), WINDOW),
            WINDOW.since_ms
        );
        let none_left = Backlog {
            complete: true,
            wanted: vec![],
        };
        assert_eq!(state.pending_after(&none_left, WINDOW), WINDOW.since_ms);
        // A record time after the window's end moves it no further than the end.
        let late = timed("l", "three", Some("2030-01-01"));
        let ahead = Backlog {
            complete: true,
            wanted: vec![Wanted::of(&late)],
        };
        assert_eq!(state.pending_after(&ahead, WINDOW), WINDOW.until_ms);
    }

    /// `story:poison-document-skipped`: a document is skipped from its third consecutive failure
    /// with the same text; a changed text starts again at one; an applied document drops it.
    #[test]
    fn a_document_is_skipped_from_its_limit_th_failure_with_the_same_text() {
        let x = doc("https://example.org/x", "poison");
        let mut failures = Failures::default();
        for n in 1..DOCUMENT_FAILURE_LIMIT {
            assert_eq!(failures.failed(&x, "refused"), n);
            assert!(failures.skips(&x).is_none());
        }
        assert_eq!(failures.failed(&x, "refused again"), DOCUMENT_FAILURE_LIMIT);
        let failed = failures.skips(&x).expect("skipped at the limit");
        assert_eq!(
            (failed.runs, failed.reason.as_str()),
            (DOCUMENT_FAILURE_LIMIT, "refused again")
        );
        let changed = doc("https://example.org/x", "poison, edited");
        assert!(failures.skips(&changed).is_none());
        assert!(!failures.marked(&changed));
        assert_eq!(failures.failed(&changed, "refused"), 1);
        assert!(failures.applied(&changed.key));
        assert!(!failures.applied(&changed.key));
        assert!(failures.documents.is_empty());
    }

    /// A document of a failed batch of several is marked with no count; a mark keeps a count its
    /// text already has, and the next failure alone counts one.
    #[test]
    fn a_document_of_a_failed_batch_is_marked_and_keeps_its_count() {
        let x = doc("https://example.org/x", "poison");
        let mut failures = Failures::default();
        assert!(!failures.marked(&x));
        failures.failed_in_batch(&x, "batch refused");
        assert!(failures.marked(&x));
        assert_eq!(failures.documents[&x.key].runs, 0);
        assert_eq!(failures.failed(&x, "refused"), 1);
        assert_eq!(failures.failed(&x, "refused"), 2);
        failures.failed_in_batch(&x, "batch refused");
        assert_eq!(failures.documents[&x.key].runs, 2);
        let changed = doc("https://example.org/x", "poison, edited");
        failures.failed_in_batch(&changed, "batch refused");
        assert_eq!(failures.documents[&x.key].runs, 0);
        assert!(failures.marked(&changed) && !failures.marked(&x));
    }

    /// The failures file lives beside `state/`, not in it; a missing one holds none, and a key
    /// that still holds a credential is dropped on load.
    #[test]
    fn the_failures_file_lives_outside_state_and_drops_credential_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = failures_path(dir.path(), "news");
        assert_eq!(path, dir.path().join("failures/news.json"));
        let empty = Failures::load(&path).unwrap();
        assert_eq!(empty.format, FAILURES_FORMAT);
        assert!(empty.documents.is_empty());

        let token = format!("Zr4{}", "p0".repeat(8));
        let mut failures = Failures::load(&path).unwrap();
        failures.failed(&doc("https://example.org/x", "poison"), "refused");
        failures.failed(
            &doc(
                &format!("https://example.org/y?access_token={token}"),
                "poison",
            ),
            "refused",
        );
        failures.save(&path).unwrap();
        let loaded = Failures::load(&path).unwrap();
        assert_eq!(
            loaded.documents.keys().collect::<Vec<_>>(),
            ["https://example.org/x"]
        );
    }
}
