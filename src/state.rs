//! What a source has already seen: the `cortex.instance.SeenDocument` records of
//! `spec/domains/instance.yaml`, held in one state file per source, and the `UNMAPPED:` rule of
//! `RunSource`'s `ran` that writes and reads them. A document is new when its key was never
//! applied, changed when its text hash or the hash of what cortex stores from it beside the text
//! differs ([`SeenState::fields`]), and skipped while its last application is younger than the
//! policy's `refresh_after_days`, unless it is a record read from a file ([`Origin::FileRecord`]),
//! which is delivered again as soon as it changed, a structured source's change whose text changed
//! only in its links ([`SeenState::unlinked`]), or a structured record whose mapping changed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use cortex_model::instance as m;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::sources::{
    Document, Origin, Window, BATCH_FAILURE_LIMIT, DOCUMENT_FAILURE_LIMIT, OVERLAP_MS,
};

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
    /// The hash of what cortex stores from the document beside its text, as the run that applied
    /// it saw it ([`SeenState::fields`]): a Web page's title, description and publication date
    /// ([`page_fields`]), or how a structured source's record is mapped
    /// ([`crate::structured::Source::mapping_hash`]). Absent for another document, and for one
    /// applied before cortex remembered it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields_hash: Option<String>,
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
    /// With `dropped: Supersede`, the fingerprint of what the last run that ended the dropped
    /// values compared the store with: its listing's keys and hashes, its unsettled records and
    /// its held prefixes (`crate::run`). A later run that selects nothing and has the same
    /// fingerprint reads nothing of the store. Absent until such a run, so a state file written
    /// before reads as one whose listing was never ended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_listing: Option<String>,
    /// Records of a structured source the store refused on their last try (a part EKR rejected,
    /// a reference it found ambiguous, a `store_lookup` refusal), by key. Such a record is not
    /// seen, so every run wants it again; [`SeenState::select`] takes it after every other wanted
    /// document, the one refused longest ago first, so a record refused on every run does not
    /// hold back the others under `max_documents_per_run`. Applying or retiring the record drops
    /// its entry; absent while none is refused.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub refused: BTreeMap<String, Refused>,
    /// This run's, never written: by key, the hash of the text without its links of each change
    /// record of a structured source's link ([`crate::structured::Source::link`]). A change whose
    /// text differs from the one last applied only in its links is wanted at once and never held,
    /// whatever `refresh_after_days`, and [`SeenState::record`] remembers it.
    #[serde(skip)]
    pub unlinked: BTreeMap<String, String>,
    /// This run's, never written: by key, the hash of what cortex stores from each fetched
    /// document beside its text ([`Seen::fields_hash`]). A document whose fields differ from the
    /// ones last applied is a changed document; a structured source's record whose mapping
    /// changed is wanted at once, whatever `refresh_after_days` (findings 4 and 9 of the 0.7.2
    /// review).
    #[serde(skip)]
    pub fields: BTreeMap<String, String>,
}

/// The last refusal of a record ([`SeenState::refused`]): the hash of the text refused and the
/// start of the run that tried it, in milliseconds since the Unix epoch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Refused {
    pub hash: String,
    pub at: i64,
}

pub fn text_hash(text: &str) -> String {
    hex(&Sha256::digest(text.as_bytes()))
}

/// The hash a document is remembered by: of its whole text when the run took it before the cut.
pub fn doc_hash(doc: &Document) -> String {
    doc.hash.clone().unwrap_or_else(|| text_hash(&doc.text))
}

/// The fields cortex stores from a Web page beside its text, its `WebPage` node's title,
/// description and publication date, as one hash ([`Seen::fields_hash`]); `None` for a document
/// that is no Web page.
pub fn page_fields(doc: &Document) -> Option<String> {
    (doc.origin == Origin::Url).then(|| {
        text_hash(&serde_json::json!([doc.title, doc.description, doc.published]).to_string())
    })
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
                state.refused.retain(|k, _| clean(k));
                Ok(state)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self {
                format: FORMAT.into(),
                documents: BTreeMap::new(),
                last_success_started_at: None,
                held_since: None,
                pending_since: None,
                child_failures: BTreeMap::new(),
                ended_listing: None,
                refused: BTreeMap::new(),
                unlinked: BTreeMap::new(),
                fields: BTreeMap::new(),
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
    /// at once, for a record read from a file), at most `max` of them, in fetch order; a record
    /// the store refused with the text it has now ([`SeenState::refused`]) after all the others,
    /// the one refused longest ago first.
    pub fn select(
        &self,
        docs: Vec<Document>,
        now_ms: i64,
        refresh_after_days: i64,
        max: usize,
    ) -> Vec<Document> {
        let (mut retried, fresh): (Vec<_>, Vec<_>) = docs
            .into_iter()
            .filter(|d| self.wants(d, now_ms, refresh_after_days))
            .partition(|d| self.refused_at(d).is_some());
        retried.sort_by_key(|d| self.refused_at(d));
        fresh.into_iter().chain(retried).take(max).collect()
    }

    /// When the store last refused `doc` with the text it has now, if it did.
    fn refused_at(&self, doc: &Document) -> Option<i64> {
        self.refused
            .get(&doc.key)
            .filter(|r| r.hash == doc_hash(doc))
            .map(|r| r.at)
    }

    /// Remembers that the store refused `doc`, tried by the run that started at `at`: it stays
    /// unseen, and is selected after the other wanted documents ([`SeenState::select`]).
    pub fn refuse(&mut self, doc: &Document, at: i64) {
        self.refused.insert(
            doc.key.clone(),
            Refused {
                hash: doc_hash(doc),
                at,
            },
        );
    }

    /// Retires the records a whole listing no longer lists (`dropped: Supersede`, once their
    /// values were ended): every key not in `listed` whose identity starts with none of `kept`
    /// (the records of a child call the run did not read) is forgotten, its saved fingerprint
    /// and refusal with it. A retired record that is listed again is new, so its values, which
    /// the end retracted, are applied again (finding 3 of the 0.7.2 review). Answers how many
    /// were retired.
    pub fn retire_unlisted(&mut self, listed: &BTreeSet<&str>, kept: &[&str]) -> usize {
        let gone =
            |k: &String| !listed.contains(k.as_str()) && !kept.iter().any(|p| k.starts_with(p));
        let before = self.documents.len();
        self.documents.retain(|k, _| !gone(k));
        self.refused.retain(|k, _| !gone(k));
        before - self.documents.len()
    }

    /// Whether a run should extract `doc`: its key is new, or it changed (its text, or what cortex
    /// stores from it beside the text, [`SeenState::fields`]) past the refresh window, or changed
    /// at all for a record read from a file, in its links alone, or as a structured record whose
    /// mapping changed. A key seen with no hash (adopted by key, not yet fetched) is not
    /// extracted.
    pub fn wants(&self, doc: &Document, now_ms: i64, refresh_after_days: i64) -> bool {
        match self.documents.get(&doc.key) {
            None => true,
            Some(seen) => self.change(doc, seen).is_some_and(|at_once| {
                at_once
                    || now_ms.saturating_sub(seen.applied_at)
                        >= refresh_after_days.saturating_mul(DAY_MS)
            }),
        }
    }

    /// Whether `doc` changed but is held back: its key was applied less than `refresh_after_days`
    /// ago, with other text or fields, and it is not a change that is wanted at once (a record
    /// read from a file, a change whose links alone changed, a record whose mapping changed). A
    /// key seen with no hash is not held.
    pub fn holds(&self, doc: &Document, now_ms: i64, refresh_after_days: i64) -> bool {
        self.documents.get(&doc.key).is_some_and(|seen| {
            self.change(doc, seen) == Some(false)
                && now_ms.saturating_sub(seen.applied_at)
                    < refresh_after_days.saturating_mul(DAY_MS)
        })
    }

    /// How `doc` differs from `seen`, the document applied under its key: `None` when it does not
    /// (or `seen` has no hash), `Some(true)` for a change wanted at once, `Some(false)` for one
    /// that waits for the refresh window. A changed text waits unless `doc` is a record read from
    /// a file or its links alone changed; changed fields wait for a Web page, and are wanted at
    /// once for a structured record, whose fields are its mapping.
    fn change(&self, doc: &Document, seen: &Seen) -> Option<bool> {
        let hash = seen.hash.as_ref()?;
        let text = *hash != doc_hash(doc);
        let fields = self.refielded(doc, seen);
        if !text && !fields {
            return None;
        }
        let remapped = fields && doc.origin != Origin::Url;
        Some((text && (!waits(doc) || self.relinked(doc, seen))) || remapped)
    }

    /// Whether what cortex stores from `doc` beside its text differs from what `seen` was applied
    /// with ([`Seen::fields_hash`]). A structured record applied with none is: the mapping it was
    /// applied with is unknown. A Web page applied with none is not; [`SeenState::adopt_fields`]
    /// takes its fields as fetched.
    fn refielded(&self, doc: &Document, seen: &Seen) -> bool {
        match (self.fields.get(&doc.key), seen.fields_hash.as_ref()) {
            (Some(now), Some(applied)) => now != applied,
            (Some(_), None) => doc.origin != Origin::Url,
            (None, _) => false,
        }
    }

    /// Records, for every fetched Web page seen with no fields hash (applied by a cortex that did
    /// not remember it), the fields this run fetched ([`SeenState::fields`]), so a later change of
    /// them is a change and these are not extracted again. A structured record is left without
    /// one, so it is applied again ([`SeenState::refielded`]).
    pub fn adopt_fields(&mut self, docs: &[Document]) {
        for doc in docs.iter().filter(|d| d.origin == Origin::Url) {
            let Some(fields) = self.fields.get(&doc.key) else {
                continue;
            };
            if let Some(seen) = self
                .documents
                .get_mut(&doc.key)
                .filter(|s| s.fields_hash.is_none())
            {
                seen.fields_hash = Some(fields.clone());
            }
        }
    }

    /// Whether `doc` is seen as applied with the text and the fields it has now.
    pub fn applied_as_now(&self, doc: &Document) -> bool {
        self.documents.get(&doc.key).is_some_and(|seen| {
            seen.hash.as_deref() == Some(doc_hash(doc).as_str()) && !self.refielded(doc, seen)
        })
    }

    /// [`Wanted::of`] `doc`, with the fields this run fetched of it.
    pub fn wanted(&self, doc: &Document) -> Wanted {
        Wanted {
            fields: self.fields.get(&doc.key).cloned(),
            ..Wanted::of(doc)
        }
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
            seen.fields_hash = self.fields.get(&doc.key).cloned();
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
                fields_hash: seen.fields_hash.clone(),
                applied_at: seen.applied_at,
            })
            .collect()
    }

    /// Remembers `doc` as applied at `now_ms`: its hash, its fields ([`SeenState::fields`]), and
    /// for a change record whose text holds links, its hash without them
    /// ([`SeenState::unlinked`]).
    pub fn record(&mut self, doc: &Document, now_ms: i64) {
        let hash = doc_hash(doc);
        let unlinked_hash = self
            .unlinked
            .get(&doc.key)
            .filter(|unlinked| **unlinked != hash)
            .cloned();
        self.refused.remove(&doc.key);
        self.documents.insert(
            doc.key.clone(),
            Seen {
                hash: Some(hash),
                applied_at: now_ms,
                unlinked_hash,
                fields_hash: self.fields.get(&doc.key).cloned(),
            },
        );
    }

    /// Whether `wanted`, as this run fetched it, is seen with its text and fields: applied by this
    /// run, since a wanted document's key is new or seen with other text or fields.
    fn holds_text(&self, wanted: &Wanted) -> bool {
        self.documents.get(&wanted.key).is_some_and(|seen| {
            seen.hash.as_ref() == Some(&wanted.hash)
                && (wanted.fields.is_none() || seen.fields_hash == wanted.fields)
        })
    }

    /// The `pending_since` a windowed run that did not succeed leaves, after what it applied: the
    /// earliest record time of a document it wanted and did not apply, less [`OVERLAP_MS`], when
    /// its fetch read every record of the window and that time is readable for every such
    /// document; the window's start otherwise, and when it left no wanted document unapplied. It
    /// is never before the window's start nor after its end, and never after the `held_since` the
    /// run computed, so a run that fails before it records `held_since` leaves no held change
    /// behind its window.
    pub fn pending_after(&self, backlog: &Backlog, window: Window) -> i64 {
        let pending = self.unapplied_since(backlog, window);
        backlog.held_since.map_or(pending, |held| pending.min(held))
    }

    fn unapplied_since(&self, backlog: &Backlog, window: Window) -> i64 {
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
    /// The earliest applied time, less the overlap, of a document the run holds back by
    /// `refresh_after_days`, as `held_since` records it.
    pub held_since: Option<i64>,
}

/// A document a windowed run wanted: its key, its hash ([`doc_hash`]), its record time
/// ([`crate::evidence::document_time_ms`]), `None` when that cannot be read, and its fields
/// ([`SeenState::fields`]), `None` when it has none ([`SeenState::wanted`]).
#[derive(Debug, Clone, PartialEq)]
pub struct Wanted {
    pub key: String,
    pub hash: String,
    pub time_ms: Option<i64>,
    pub fields: Option<String>,
}

impl Wanted {
    pub fn of(doc: &Document) -> Self {
        Self {
            key: doc.key.clone(),
            hash: doc_hash(doc),
            time_ms: crate::evidence::document_time_ms(doc),
            fields: None,
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
/// request, per key: the hash of the text that last failed, the count, and the last reason. A
/// document of a batch of several that failed is kept with a count of `0` and one more failed
/// batch ([`Failures::failed_in_batch`]): a batch that failed once is asked whole once more, and
/// from its [`BATCH_FAILURE_LIMIT`]th failed batch, or its first failure alone, the document is
/// asked alone ([`Failures::alone`]). The count
/// is kept per key across text changes; an applied document drops its entry. From
/// [`DOCUMENT_FAILURE_LIMIT`] on, a run leaves the document out while its text is the one that
/// last failed ([`Failures::skips`]): a changed text is asked once more alone, and a failure
/// leaves it out again.
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
    /// Failed batches of several documents it was in; absent when none.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub batches: u32,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
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

    /// The entry that leaves `doc` out of a run: it failed alone on [`DOCUMENT_FAILURE_LIMIT`] or
    /// more consecutive runs, the last with the text it has now.
    pub fn skips(&self, doc: &Document) -> Option<&Failed> {
        self.documents
            .get(&doc.key)
            .filter(|f| f.runs >= DOCUMENT_FAILURE_LIMIT && f.hash == doc_hash(doc))
    }

    /// Counts one more failure of `doc` alone, for `reason`, with its text now, and answers the
    /// count: one more than before, whatever text failed before.
    pub fn failed(&mut self, doc: &Document, reason: &str) -> u32 {
        let hash = doc_hash(doc);
        let (runs, batches) = self
            .documents
            .get(&doc.key)
            .map_or((1, 0), |f| (f.runs.saturating_add(1), f.batches));
        self.documents.insert(
            doc.key.clone(),
            Failed {
                hash,
                runs,
                reason: reason.to_string(),
                batches,
            },
        );
        runs
    }

    /// Counts one more failed batch of several documents for `doc`, for `reason`: a count of `0`
    /// when it has none, a count it has kept. Answers whether `doc` is now asked alone.
    pub fn failed_in_batch(&mut self, doc: &Document, reason: &str) -> bool {
        let entry = self
            .documents
            .entry(doc.key.clone())
            .or_insert_with(|| Failed {
                hash: doc_hash(doc),
                runs: 0,
                reason: reason.to_string(),
                batches: 0,
            });
        entry.batches = entry.batches.saturating_add(1);
        if entry.runs == 0 {
            entry.reason = reason.to_string();
        }
        self.alone(doc)
    }

    /// Whether a run asks `doc` alone, with any text: it failed alone before, or was in
    /// [`BATCH_FAILURE_LIMIT`] failed batches of several. A document whose batch failed once is
    /// asked in a batch once more.
    pub fn alone(&self, doc: &Document) -> bool {
        self.documents
            .get(&doc.key)
            .is_some_and(|f| f.runs > 0 || f.batches >= BATCH_FAILURE_LIMIT)
    }

    /// How many consecutive runs `doc` failed alone: `0` when it did not.
    pub fn runs(&self, doc: &Document) -> u32 {
        self.documents.get(&doc.key).map_or(0, |f| f.runs)
    }

    /// Drops the entries of documents the source no longer lists, `listed` being every key it
    /// lists; answers whether one was dropped.
    pub fn listed_only(&mut self, listed: &BTreeSet<&str>) -> bool {
        let before = self.documents.len();
        self.documents.retain(|k, _| listed.contains(k.as_str()));
        self.documents.len() != before
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
                fields_hash: None,
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
            fields_hash: None,
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

    /// Finding 6 of the 0.7.2 review: a refused record is selected after the other wanted ones,
    /// the one refused longest ago first; its changed text, or its application, ends that.
    #[test]
    fn a_refused_record_is_selected_after_the_others_and_the_oldest_refusal_first() {
        let keys = |docs: Vec<Document>| docs.into_iter().map(|d| d.key).collect::<Vec<_>>();
        let docs = || vec![doc("a", "1"), doc("b", "2"), doc("c", "3"), doc("d", "4")];
        let mut state = SeenState::default();
        state.refuse(&doc("a", "1"), 20);
        state.refuse(&doc("b", "2"), 10);
        assert_eq!(keys(state.select(docs(), 0, 0, 10)), ["c", "d", "b", "a"]);
        assert_eq!(keys(state.select(docs(), 0, 0, 1)), ["c"]);
        // A refusal of another text is no refusal of this one.
        state.refuse(&doc("c", "old"), 5);
        assert_eq!(keys(state.select(docs(), 0, 0, 1)), ["c"]);
        state.record(&doc("c", "3"), 1);
        state.record(&doc("d", "4"), 1);
        assert_eq!(keys(state.select(docs(), 0, 0, 1)), ["b"]);
        assert!(!state.refused.contains_key("c"));
    }

    /// Finding 3 of the 0.7.2 review: a whole listing retires the records it no longer lists, but
    /// those under a kept prefix, so a retired record listed again is new.
    #[test]
    fn retiring_forgets_the_unlisted_records_but_the_kept_ones() {
        let mut state = SeenState::default();
        for key in ["s:A", "s:B", "s/c:P:x"] {
            state.record(&doc(key, "t"), 1);
        }
        state.refuse(&doc("s:B", "u"), 2);
        let listed = BTreeSet::from(["s:A"]);
        assert_eq!(state.retire_unlisted(&listed, &["s/c:P:"]), 1);
        assert_eq!(
            state.documents.keys().collect::<Vec<_>>(),
            ["s/c:P:x", "s:A"]
        );
        assert!(state.refused.is_empty());
        assert!(state.wants(&doc("s:B", "t"), 0, 0));
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
                    fields_hash: None,
                    applied_at: 5,
                },
                m::SeenDocumentData {
                    document_id: m::DocumentId("t/news https://example.org/b".into()),
                    source_id: source.clone(),
                    key: "https://example.org/b".into(),
                    content_hash: Some(text_hash("two")),
                    unlinked_hash: None,
                    fields_hash: None,
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
            held_since: None,
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
            held_since: None,
            wanted: vec![Wanted::of(&a)],
        };
        assert_eq!(state.pending_after(&unread, WINDOW), WINDOW.since_ms);
        let unreadable = Backlog {
            complete: true,
            held_since: None,
            wanted: vec![Wanted::of(&a), Wanted::of(&untimed)],
        };
        assert_eq!(state.pending_after(&unreadable, WINDOW), WINDOW.since_ms);
        assert_eq!(
            state.pending_after(&Backlog::default(), WINDOW),
            WINDOW.since_ms
        );
        let none_left = Backlog {
            complete: true,
            held_since: None,
            wanted: vec![],
        };
        assert_eq!(state.pending_after(&none_left, WINDOW), WINDOW.since_ms);
        // A record time after the window's end moves it no further than the end.
        let late = timed("l", "three", Some("2030-01-01"));
        let ahead = Backlog {
            complete: true,
            held_since: None,
            wanted: vec![Wanted::of(&late)],
        };
        assert_eq!(state.pending_after(&ahead, WINDOW), WINDOW.until_ms);
    }

    /// A change the run held back by `refresh_after_days` keeps the window no later than its
    /// `held_since`, so a run that fails before it records `held_since` leaves the change ahead.
    #[test]
    fn the_pending_window_is_never_after_a_change_the_run_held_back() {
        let a = timed("a", "one", Some("2001-09-11T01:46:40Z"));
        let held = WINDOW.since_ms + 3_600_000;
        let backlog = Backlog {
            complete: true,
            held_since: Some(held),
            wanted: vec![Wanted::of(&a)],
        };
        let state = SeenState::default();
        assert_eq!(state.pending_after(&backlog, WINDOW), held);
        let earlier = Backlog {
            held_since: Some(WINDOW.since_ms - 1),
            ..backlog.clone()
        };
        assert_eq!(state.pending_after(&earlier, WINDOW), WINDOW.since_ms - 1);
        let later = Backlog {
            held_since: Some(WINDOW.until_ms),
            ..backlog
        };
        assert_eq!(
            state.pending_after(&later, WINDOW),
            1_000_172_800_000 - OVERLAP_MS
        );
    }

    /// `story:poison-document-skipped`: a document is skipped from its third consecutive failure
    /// alone while its text is the one that last failed; a changed text is asked once more and
    /// keeps the count; an applied document drops it.
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
        assert!(
            failures.skips(&changed).is_none(),
            "a changed text is asked"
        );
        assert!(failures.alone(&changed), "alone");
        assert_eq!(
            failures.failed(&changed, "refused"),
            DOCUMENT_FAILURE_LIMIT + 1
        );
        assert!(failures.skips(&changed).is_some(), "and left out again");
        assert!(
            failures.skips(&x).is_none(),
            "the text that failed is now the changed one"
        );
        assert!(failures.applied(&changed.key));
        assert!(!failures.applied(&changed.key));
        assert!(failures.documents.is_empty());
    }

    /// A document of a failed batch of several is kept with no count and is asked in a batch once
    /// more; from its second failed batch it is asked alone. A failed batch keeps a count the
    /// document already has, whatever its text, and the next failure alone counts one.
    #[test]
    fn a_document_of_a_failed_batch_is_asked_alone_from_its_second_failed_batch() {
        assert_eq!(BATCH_FAILURE_LIMIT, 2);
        let x = doc("https://example.org/x", "poison");
        let mut failures = Failures::default();
        assert!(!failures.alone(&x));
        assert!(!failures.failed_in_batch(&x, "batch refused"), "once more");
        assert!(!failures.alone(&x));
        assert_eq!(failures.documents[&x.key].runs, 0);
        assert!(failures.failed_in_batch(&x, "batch refused"), "then alone");
        assert!(failures.alone(&x));
        assert_eq!(failures.documents[&x.key].runs, 0);
        assert_eq!(failures.failed(&x, "refused"), 1);
        assert_eq!(failures.failed(&x, "refused"), 2);
        assert!(failures.failed_in_batch(&x, "batch refused"));
        assert_eq!(failures.documents[&x.key].runs, 2);
        assert_eq!(failures.documents[&x.key].reason, "refused");
        let changed = doc("https://example.org/x", "poison, edited");
        failures.failed_in_batch(&changed, "batch refused");
        assert_eq!(failures.documents[&x.key].runs, 2);
        assert!(failures.alone(&changed) && failures.alone(&x));
        assert_eq!(failures.runs(&changed), 2);
        assert_eq!(failures.runs(&doc("https://example.org/y", "fine")), 0);

        // One failure alone is enough to be asked alone.
        let z = doc("https://example.org/z", "poison");
        failures.failed(&z, "refused");
        assert!(failures.alone(&z));
    }

    /// A failures file written before `batches` existed reads with none, and an entry with none
    /// is written without it.
    #[test]
    fn a_failures_entry_without_batches_reads_as_none_and_is_written_without_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("news.json");
        std::fs::write(
            &path,
            r#"{"format": "cortex.failures/1", "documents": {"https://example.org/x":
                {"hash": "h", "runs": 0, "reason": "batch refused"}}}"#,
        )
        .unwrap();
        let failures = Failures::load(&path).unwrap();
        assert_eq!(failures.documents["https://example.org/x"].batches, 0);
        failures.save(&path).unwrap();
        assert!(!std::fs::read_to_string(&path).unwrap().contains("batches"));
    }

    /// With the source's full listing, the entries of documents it no longer lists are dropped.
    #[test]
    fn entries_of_documents_no_longer_listed_are_dropped() {
        let mut failures = Failures::default();
        failures.failed(&doc("https://example.org/x", "poison"), "refused");
        failures.failed_in_batch(&doc("https://example.org/y", "fine"), "refused");
        let listed = BTreeSet::from(["https://example.org/x", "https://example.org/y"]);
        assert!(!failures.listed_only(&listed));
        assert!(failures.listed_only(&BTreeSet::from(["https://example.org/y"])));
        assert_eq!(
            failures.documents.keys().collect::<Vec<_>>(),
            ["https://example.org/y"]
        );
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
