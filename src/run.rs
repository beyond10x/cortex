//! `RunSource`, the obligation `generated/cortex-model/PLAN.md` leaves to the implementation:
//! fetch, keep what is new or changed, extract in batches within the run's budget, apply.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::PathBuf;

use cortex_model::instance as m;
use serde_json::json;

use crate::connectors::Connectors;
use crate::evidence;
use crate::extract::{self, Known, Model};
use crate::gate;
use crate::instance::{now_ms, Layout};
use crate::mask::{mask, mask_key};
use crate::redact::{self, Redactor};
use crate::schedule::Systemd;
use crate::snapshot::{self, BeforeApply, RestoreError};
use crate::sources::{self, FetchError, Window};
use crate::spend;
use crate::state::{text_hash, SeenState};
use crate::structured;

pub struct Tools {
    pub connectors: Connectors,
    pub claude: PathBuf,
    pub codex: PathBuf,
}

/// Which declared failure outcome a run takes.
#[derive(Debug)]
pub enum Failure {
    Fetch(String),
    Extract(String),
    Apply(String),
    /// EKR could not say whether a store write of the run took effect (a stage verb,
    /// `apply-extraction` or a transaction; `crate::ekr::outcome_unknown`): a fault of the store,
    /// not of the source, which the next run settles.
    StoreOutcomeUnknown(String),
}

#[derive(Debug)]
pub struct Report {
    pub documents_new: i64,
    /// Fetched documents whose key was seen with no hash (adopted by key alone): their hash was
    /// recorded and they were not extracted ([`SeenState::hash_unhashed`]).
    pub hashed: usize,
    pub documents_applied: i64,
    /// `0` when no model was asked, the sum when every answer carried a cost, `None` as soon as
    /// one answer carried none: a missing cost is never written as `0`.
    pub cost_usd: Option<f64>,
    /// Whether `cost_usd` counted against the instance's dollar limits: false behind a model
    /// gateway, where it is what `claude` reported and bounded nothing (`spend::counts_dollars`).
    pub cost_counted: bool,
    pub facts_refused: usize,
    /// Parts of applied documents EKR rejected (`rejected` in its report).
    pub parts_rejected: usize,
    pub masked: usize,
    /// Values replaced by a placeholder in the prompts sent, per class or rule of the instance's
    /// `redaction` policy, every one it names counted from `0`; `None` when the policy names
    /// nothing to pseudonymise.
    pub redacted: Option<BTreeMap<String, usize>>,
    /// Placeholder-shaped strings in the model's answers that had no value to restore.
    pub unrestored: usize,
    /// Why a run stopped before applying every new document, when it did.
    pub stopped: Option<String>,
    /// Documents the model was shown cut at the policy's `max_chars_per_document`.
    pub truncated: usize,
    /// Parents left out without holding the window: their child call failed on
    /// [`sources::CHILD_FAILURE_LIMIT`] or more consecutive runs; and structured records refused
    /// because their mapped values alone are over EKR's evidence bound.
    pub skipped: Vec<String>,
    /// Applied documents a part EKR rejected belongs to, each with EKR's refusal.
    pub rejected: Vec<Rejected>,
    /// Structured records left out because a mapped value does not convert to its property's
    /// kind (`structured::convert`); neither applied nor marked seen, so the next run reads them
    /// again.
    pub unconverted: Vec<Unconverted>,
    /// Assertions of a structured source with `dropped: Supersede` this run superseded by a
    /// changed value, and retracted as no longer listed.
    pub superseded: usize,
    pub retracted: usize,
    /// Whether the run stopped before a batch because the day's spend or tokens reached the
    /// instance's daily ceiling (said in `stopped`); not when it stopped under the ceiling for
    /// another reason, such as an answer whose tokens could not be counted.
    pub daily_ceiling_reached: bool,
    /// Every model call the run made, a call asked again and a failed one included.
    pub model_calls: i64,
    /// The tokens those calls reported, summed with the rule of [`add_cost`]: `0` when no model
    /// was asked, `None` as soon as one answer did not carry the count.
    pub usage: extract::Usage,
    /// The input tokens (cache reads and writes included) apportioned by bytes to the fixed part
    /// of each request and to its documents ([`extract::split_tokens`]); `None` as the input is.
    pub fixed_input_tokens: Option<i64>,
    pub documents_input_tokens: Option<i64>,
}

/// An applied document a part EKR rejected belongs to: its key and EKR's refusals, joined.
#[derive(Debug)]
pub struct Rejected {
    pub document: String,
    pub refusal: String,
}

/// A structured record left out of its batch: its key, and the mapped property whose value does
/// not convert to the kind the property is written in.
#[derive(Debug)]
pub struct Unconverted {
    pub document: String,
    pub property: String,
    pub value_kind: m::SchemaValueKind,
}

impl Unconverted {
    pub fn json(&self) -> serde_json::Value {
        json!({
            "document": self.document,
            "property": self.property,
            "value_kind": structured::kind_name(self.value_kind),
        })
    }
}

impl Rejected {
    fn new(document: &str, refusals: &[String]) -> Self {
        let mut unique: Vec<&str> = Vec::new();
        for r in refusals {
            if !unique.contains(&r.as_str()) {
                unique.push(r);
            }
        }
        Self {
            document: document.to_string(),
            refusal: unique.join("; "),
        }
    }

    pub fn json(&self) -> serde_json::Value {
        json!({"document": self.document, "refusal": self.refusal})
    }
}

impl Default for Report {
    /// A run that asked no model: it cost `0`.
    fn default() -> Self {
        Self {
            documents_new: 0,
            hashed: 0,
            documents_applied: 0,
            cost_usd: Some(0.0),
            cost_counted: true,
            facts_refused: 0,
            parts_rejected: 0,
            masked: 0,
            redacted: None,
            unrestored: 0,
            stopped: None,
            truncated: 0,
            skipped: Vec::new(),
            rejected: Vec::new(),
            unconverted: Vec::new(),
            superseded: 0,
            retracted: 0,
            daily_ceiling_reached: false,
            model_calls: 0,
            usage: extract::Usage::ZERO,
            fixed_input_tokens: Some(0),
            documents_input_tokens: Some(0),
        }
    }
}

impl Report {
    /// Counts the model calls of one batch and their tokens, its request's fixed part `fixed`
    /// bytes long and its documents `documents` bytes.
    pub fn add_calls(&mut self, calls: i64, usage: extract::Usage, fixed: usize, documents: usize) {
        self.model_calls += calls;
        self.usage = self.usage.plus(usage);
        let split = usage
            .all_input()
            .map(|tokens| extract::split_tokens(tokens, fixed, documents));
        let add = |total: Option<i64>, part: Option<i64>| total?.checked_add(part?);
        self.fixed_input_tokens = add(self.fixed_input_tokens, split.map(|s| s.0));
        self.documents_input_tokens = add(self.documents_input_tokens, split.map(|s| s.1));
    }

    /// The model calls and tokens as a run's result and its `cortex.log` line state them.
    pub fn calls_json(&self) -> serde_json::Value {
        json!({
            "model_calls": self.model_calls,
            "input_tokens": self.usage.input_tokens,
            "cache_read_input_tokens": self.usage.cache_read_input_tokens,
            "cache_creation_input_tokens": self.usage.cache_creation_input_tokens,
            "output_tokens": self.usage.output_tokens,
            "fixed_input_tokens": self.fixed_input_tokens,
            "documents_input_tokens": self.documents_input_tokens,
        })
    }
}

/// A run's cost after one more answer: the sum while every answer is costed, `None` from the
/// first answer that is not.
pub fn add_cost(total: Option<f64>, answer: Option<f64>) -> Option<f64> {
    Some(total? + answer?)
}

/// Characters of document text per model call.
const BATCH_CHARS: usize = 60_000;
/// Known entity names per node type given to the model.
const KNOWN_PER_TYPE: usize = 50;

/// Cuts `text` to `max` characters; answers whether it cut anything.
fn truncate(text: &mut String, max: usize) -> bool {
    let Some((i, _)) = text.char_indices().nth(max) else {
        return false;
    };
    text.truncate(i);
    true
}

/// Cuts `d`'s text to `max` characters, then to what its evidence payload holds within EKR's bound
/// ([`evidence::fit`]); answers whether it cut anything.
fn cut_text(d: &mut sources::Document, max: usize) -> bool {
    let chars = truncate(&mut d.text, max);
    evidence::fit(d) || chars
}

fn load_entities(layout: &Layout) -> BTreeMap<String, Vec<String>> {
    std::fs::read(layout.entities())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// What a fetch-only run counted (`cortex.instance.SourceFetched`): the window its fetch asked
/// about, and the documents it fetched, masked and scrubbed as a run's are, against the source's
/// seen state.
#[derive(Debug, Default)]
pub struct Counts {
    pub since_ms: i64,
    pub until_ms: i64,
    /// `new` + `changed` + `held` + `unchanged`.
    pub fetched: usize,
    /// Keys the seen state does not carry.
    pub new: usize,
    /// Seen keys whose text a run would extract now ([`SeenState::wants`]).
    pub changed: usize,
    /// Seen keys whose change `refresh_after_days` holds back ([`SeenState::holds`]).
    pub held: usize,
    pub unchanged: usize,
    /// Why the fetch left records unread, one line each, masked as a run's `stopped` is.
    pub unread: Vec<String>,
}

/// What a source's run reads before it fetches: the instance's spec, the source's place in it,
/// its seen state and the window it asks about.
struct Planned {
    spec: m::InstanceSpec,
    spec_dir: PathBuf,
    source: usize,
    state: SeenState,
    seen_path: PathBuf,
    window: Window,
}

impl Planned {
    fn of(layout: &Layout, source: &m::SourceData) -> Result<Self, Failure> {
        let spec = layout.load_spec().map_err(Failure::Fetch)?;
        let meta = layout.load_meta().map_err(Failure::Fetch)?;
        let index = spec
            .sources
            .iter()
            .position(|s| s.name == source.name)
            .ok_or_else(|| Failure::Fetch(format!("the spec has no source {:?}", source.name)))?;
        // The window a `connectors` source asks about: from just before the start of its last
        // successful run, its held-back changes or a run that did not succeed, to the start of
        // this one.
        let start = now_ms();
        let seen_path = layout.seen(&source.name);
        let state = SeenState::load(&seen_path).map_err(Failure::Fetch)?;
        let window = Window::of_run(
            start,
            state.last_success_started_at,
            state.held_since,
            state.pending_since,
            spec.sources[index].policy.refresh_after_days,
        );
        Ok(Self {
            spec,
            spec_dir: meta.spec_dir,
            source: index,
            state,
            seen_path,
            window,
        })
    }

    fn source(&self) -> &m::SourceSpec {
        &self.spec.sources[self.source]
    }

    fn structured(&self) -> Option<&m::StructuredSource> {
        match &self.source().settings {
            m::SourceSettings::Structured(st) => Some(st),
            _ => None,
        }
    }

    /// Fetches the source over the window. A structured source's links order and match records
    /// by the identities the run gives them, which the redaction policy takes part in.
    fn fetch(&self, tools: &Tools) -> Result<sources::Fetched, Failure> {
        let source = self.source();
        let redactor = redact::compile(self.spec.redaction.as_ref()).map_err(Failure::Extract)?;
        sources::fetch(
            &source.settings,
            &tools.connectors,
            &self.spec_dir,
            &self.window,
            &self.state.child_failures,
            source.policy.max_chars_per_document.max(0) as usize,
            redactor.as_ref(),
        )
        .map_err(|e| match e {
            FetchError::Missing(m) | FetchError::Failed(m) => Failure::Fetch(m),
        })
    }
}

/// `cortex run --fetch-only`: fetches the source as [`run`] does, masks and scrubs what it
/// fetched and counts it against the seen state, then stops. It asks no model and writes nothing:
/// not the store, the seen state (a failed fetch records no `pending_since`), `runs/`, a
/// snapshot, an EKR stage or `cortex.log`.
pub fn count(layout: &Layout, tools: &Tools, source: &m::SourceData) -> Result<Counts, Failure> {
    let planned = Planned::of(layout, source)?;
    let fetched = planned.fetch(tools)?;
    let mut counts = Counts {
        since_ms: planned.window.since_ms,
        until_ms: planned.window.until_ms,
        ..Counts::default()
    };
    process_in(
        layout,
        tools,
        &planned.spec,
        &source.name,
        fetched,
        &planned.source().policy,
        Some(planned.window),
        planned.structured(),
        false,
        &mut Undo::new(BeforeApply::none()),
        Some(&mut counts),
    )?;
    Ok(counts)
}

pub fn run(layout: &Layout, tools: &Tools, source: &m::SourceData) -> Result<Report, Failure> {
    let planned = Planned::of(layout, source)?;
    let (window, seen_path) = (planned.window, planned.seen_path.clone());
    let ran = planned.fetch(tools).and_then(|fetched| {
        process(
            layout,
            tools,
            &planned.spec,
            &source.name,
            fetched,
            &planned.source().policy,
            Some(window),
            planned.structured(),
            true,
        )
    });
    if ran.is_err() {
        // A failed run reads nothing it can be trusted to have read: the next run's `{since}` is
        // no later than this one's. The failure is the run's answer; a state that cannot be
        // written leaves the window as the last run left it.
        if let Ok(mut state) = SeenState::load(&seen_path) {
            state.pending_since = Some(
                state
                    .pending_since
                    .map_or(window.since_ms, |p| p.min(window.since_ms)),
            );
            let _ = state.save(&seen_path);
        }
    }
    ran
}

/// The seed documents of a new instance: every file under the spec's `seed.documents`, through
/// the same pipeline a source run uses, recorded as the pseudo-source `seed`. The seed of a new
/// store takes no snapshot and is held to no gate.
pub fn seed(layout: &Layout, tools: &Tools) -> Result<Report, Failure> {
    seed_documents(layout, tools, false)
}

/// The seed documents of a live instance (`cortex run <name>/seed`): those no seed extraction
/// applied yet, as [`seed`] extracts them, but undoable as a source run is: the store and `state/`
/// are kept as a snapshot named `seed` before the first apply, and the run is held to the spec's
/// `gate`.
pub fn rerun_seed(layout: &Layout, tools: &Tools) -> Result<Report, Failure> {
    seed_documents(layout, tools, true)
}

fn seed_documents(layout: &Layout, tools: &Tools, undoable: bool) -> Result<Report, Failure> {
    let spec = layout.load_spec().map_err(Failure::Fetch)?;
    if spec.seed.documents.is_empty() {
        return Ok(Report::default());
    }
    let files = m::SourceSettings::Files(m::FilesSource {
        paths: spec.seed.documents.clone(),
        glob: "**/*".into(),
        records: None,
    });
    let start = now_ms();
    let window = Window::of_run(start, None, None, None, 0);
    let policy = m::FetchPolicy {
        refresh_after_days: 0,
        change: m::ChangeDetection::ContentHash,
        max_documents_per_run: i64::MAX,
        max_chars_per_document: 100_000,
    };
    let fetched = sources::fetch(
        &files,
        &tools.connectors,
        &layout.dir,
        &window,
        &BTreeMap::new(),
        policy.max_chars_per_document as usize,
        None,
    )
    .map_err(|e| match e {
        FetchError::Missing(m) | FetchError::Failed(m) => Failure::Fetch(m),
    })?;
    process(
        layout,
        tools,
        &spec,
        crate::instance::SEED,
        fetched,
        &policy,
        None,
        None,
        undoable,
    )
}

/// `window`, when given, is the window the source's fetch asked about; its end is recorded in the
/// source's state as the start of its last successful run once every document the run wanted is
/// applied and every record was read: a run that stopped early, left records unread (a
/// `max_pages`, an empty page that named a next one, a failed child call), or left documents
/// beyond `max_documents_per_run` keeps its window's start as `pending_since`, so what it did not
/// apply is asked for again. `held_since` is recomputed from the documents this run holds back by
/// `refresh_after_days`. A windowed run records its window's end as each applied document's
/// `applied_at`, so a later change to it is inside a window that starts there.
///
/// With a `structured_source`, each document is a record's JSON: its key becomes the record's
/// identity, every string in it is masked and scrubbed instead of the text as a whole, it is never
/// cut, and the documents are applied as `src/structured.rs` maps them, with no model call (see
/// [`apply_records`]).
///
/// `undoable`: a run into a live store (a source run, or `cortex run <name>/seed`) keeps a
/// snapshot before its first apply and is held to the spec's `gate`; the seed of a new store does
/// neither.
#[allow(clippy::too_many_arguments)]
fn process(
    layout: &Layout,
    tools: &Tools,
    spec: &m::InstanceSpec,
    label: &str,
    fetched: sources::Fetched,
    policy: &m::FetchPolicy,
    window: Option<Window>,
    structured_source: Option<&m::StructuredSource>,
    undoable: bool,
) -> Result<Report, Failure> {
    // A staged run that was killed left its stage and the `state/` from before it.
    Staged::recover(layout, spec)?;
    let mut before = Undo::new(BeforeApply::none());
    let ran = process_in(
        layout,
        tools,
        spec,
        label,
        fetched,
        policy,
        window,
        structured_source,
        undoable,
        &mut before,
        None,
    );
    settle(layout, spec, label, ran, before)
}

/// [`process`], with the run's snapshot and stage in `before`, which [`settle`] settles once the
/// run ended, whichever way it did.
///
/// With `counts`, a fetch-only run ([`count`]): it fills `counts` once the documents are masked,
/// scrubbed and compared with the seen state, and returns before anything is selected or written.
#[allow(clippy::too_many_arguments)]
fn process_in(
    layout: &Layout,
    tools: &Tools,
    spec: &m::InstanceSpec,
    label: &str,
    fetched: sources::Fetched,
    policy: &m::FetchPolicy,
    window: Option<Window>,
    structured_source: Option<&m::StructuredSource>,
    undoable: bool,
    before: &mut Undo,
    counts: Option<&mut Counts>,
) -> Result<Report, Failure> {
    let started = now_ms();
    let applied_at = window.map_or(started, |w| w.until_ms);
    let redactor = redact::compile(spec.redaction.as_ref()).map_err(Failure::Extract)?;
    // A message the fetch built can name a document key; it is masked as the key is.
    let mut report = Report {
        skipped: fetched
            .skipped
            .iter()
            .map(|s| masked_key(redactor.as_ref(), s))
            .collect(),
        cost_counted: spend::counts_dollars(&spec.model),
        ..Report::default()
    };
    // The keys of documents cut at `max_chars_per_document`: counted as they reach the model.
    let mut cut: HashSet<String> = HashSet::new();
    report.redacted = redactor.as_ref().map(Redactor::counts);
    // Each document's text before the cut, so a value the cut splits is still found, with what
    // the irreversible rules replaced in it.
    let mut uncut: HashMap<String, Vec<Uncut>> = HashMap::new();
    // Masked keys, so two documents whose keys differ only in a credential are one.
    let mut keys: HashSet<String> = HashSet::new();
    // A record's matches of the irreversible rules, counted when the record is applied.
    let mut record_scrubbed: HashMap<String, BTreeMap<String, usize>> = HashMap::new();
    let mut structured = structured_source.map(|st| structured::Source::new(st, redactor.as_ref()));
    // A child call that failed for one parent: the run names the parent, by its identity, and the
    // operation in `skipped`, and keeps the values of that parent's records of the operation.
    let mut held: Vec<structured::Held> = Vec::new();
    if let Some(s) = &structured {
        for f in &fetched.failed_children {
            let (parent, prefix) = s.failed_child(&f.parent, &f.operation);
            held.extend(prefix.map(|prefix| structured::Held {
                prefix,
                relation: None,
            }));
            let op = &f.operation;
            report.skipped.push(masked_key(
                redactor.as_ref(),
                &format!(
                    "{op}: child call failed for {parent}: {}; its {op} records were not read, \
                     and none of its earlier ones is ended",
                    f.reason
                ),
            ));
        }
        // A link not found in full for one parent: the run names the parent and the link, and
        // keeps the links of that relation its changes hold in the store; every other value of
        // those changes ends as it would without the link.
        for f in &fetched.failed_links {
            let (parent, prefix) = s.failed_child(&f.parent, &f.changes);
            let (op, changes, relation) = (&f.operation, &f.changes, &f.relation);
            held.extend(prefix.map(|prefix| structured::Held {
                prefix,
                relation: Some(relation.clone()),
            }));
            report.skipped.push(masked_key(
                redactor.as_ref(),
                &format!(
                    "{op}: {relation} links of {parent} not found in full: {}; its {changes} \
                     records not linked before that get no {relation} link in this run, and none \
                     of their earlier {relation} links is ended",
                    f.reason
                ),
            ));
        }
    }
    let fetched_links = &fetched.links;
    // Each change record's hash without its links, by its key ([`structured::Source::link`]).
    let mut unlinked: BTreeMap<String, String> = BTreeMap::new();
    let docs: Vec<_> = fetched
        .documents
        .into_iter()
        .filter_map(|mut d| {
            // A record's key is its identity, which never holds a raw id the mask or a rule
            // would change; its strings are masked and scrubbed by `prepare`.
            if let Some(s) = &structured {
                let (n, scrubbed) = s.prepare(&mut d);
                report.masked += n;
                // A change's links are part of the text the run hashes, so a change applied
                // before its tag appeared is applied again with its link; its hash without them
                // tells a change of its links alone, which no refresh window holds back.
                let links = fetched_links.get(&d.key).map(Vec::as_slice);
                if let Some(text) = s.link(&mut d, links.unwrap_or_default()) {
                    unlinked.insert(d.key.clone(), text_hash(&text));
                }
                d.hash = Some(text_hash(&d.text));
                record_scrubbed.insert(d.key.clone(), scrubbed);
                return Some(d);
            }
            // Credentials are masked, irreversibly, in every field the model, the evidence or the
            // store sees: the key (a URL's query may carry a token), title, description and text.
            // Only a key that holds a credential changes, and it changes the same way every run.
            // A document whose masked key an earlier one already has is left out, as the fetch
            // leaves out a repeated key, and its masks are not counted. The `Credential` class
            // masks the shapes it adds in the key too, so they are never stored either.
            let (key, n) = mask_key(&d.key);
            let (key, credentials) = match &redactor {
                Some(r) => r.scrub_key(&key),
                None => (key, 0),
            };
            if !keys.insert(key.clone()) {
                return None;
            }
            d.key = key;
            report.masked += n;
            for field in [d.title.as_mut(), d.description.as_mut(), Some(&mut d.text)]
                .into_iter()
                .flatten()
            {
                let (masked, n) = mask(field);
                report.masked += n;
                *field = masked;
            }
            let Some(r) = &redactor else {
                d.hash = Some(text_hash(&d.text));
                if cut_text(&mut d, policy.max_chars_per_document as usize) {
                    cut.insert(d.key.clone());
                }
                return Some(d);
            };
            // Irreversible steps, like masking, run before anything is stored.
            let mut scrubbed: BTreeMap<String, usize> = BTreeMap::new();
            if credentials > 0 {
                scrubbed.insert(
                    redact::class_name(m::RedactionClass::Credential).into(),
                    credentials,
                );
            }
            for field in [&mut d.title, &mut d.description].into_iter().flatten() {
                let (text, hits) = r.scrub(field);
                *field = text;
                for (name, _) in hits {
                    *scrubbed.entry(name).or_default() += 1;
                }
            }
            let (text, hits) = r.scrub(&d.text);
            d.text = text.clone();
            d.hash = Some(text_hash(&d.text));
            if cut_text(&mut d, policy.max_chars_per_document as usize) {
                cut.insert(d.key.clone());
            }
            for (name, at) in hits {
                if at < d.text.len() {
                    *scrubbed.entry(name).or_default() += 1;
                }
            }
            uncut
                .entry(d.key.clone())
                .or_default()
                .push(Uncut { text, scrubbed });
            Some(d)
        })
        .collect();

    // With `dropped: Supersede`, every record the run read, so the values the store holds of this
    // source can be compared with what it lists now ([`end_dropped`]).
    let listed: Option<Vec<sources::Document>> = structured_source
        .filter(|st| st.dropped == Some(m::DropPolicy::Supersede))
        .map(|_| docs.clone());
    // Every parent record the run read, so a child names its parent by all its aliases whether or
    // not the parent is applied in this run.
    let parents = structured
        .as_ref()
        .map(|s| s.parents(&docs))
        .unwrap_or_default();
    let seen_path = layout.seen(label);
    let mut seen = SeenState::load(&seen_path).map_err(Failure::Fetch)?;
    seen.unlinked = unlinked;
    // A document adopted by its key alone is seen with the text fetched now, and not extracted;
    // its hash is written with the rest of the run's state, so a run that is undone puts it back.
    report.hashed = seen.hash_unhashed(&docs);
    let wanted = docs
        .iter()
        .filter(|d| seen.wants(d, started, policy.refresh_after_days))
        .count();
    // Each held change was made after its document was last applied: the earliest such instant,
    // less the overlap for a provider's late clock, covers them all.
    let held_since = docs
        .iter()
        .filter(|d| seen.holds(d, started, policy.refresh_after_days))
        .filter_map(|d| seen.documents.get(&d.key))
        .map(|s| s.applied_at - sources::OVERLAP_MS)
        .min();
    // A fetch-only run stops here, with what this run would select counted.
    if let Some(counts) = counts {
        let refresh = policy.refresh_after_days;
        for d in &docs {
            let count = if !seen.documents.contains_key(&d.key) {
                &mut counts.new
            } else if seen.wants(d, started, refresh) {
                &mut counts.changed
            } else if seen.holds(d, started, refresh) {
                &mut counts.held
            } else {
                &mut counts.unchanged
            };
            *count += 1;
        }
        counts.fetched = docs.len();
        counts.unread = fetched
            .unread
            .iter()
            .map(|u| masked_key(redactor.as_ref(), u))
            .collect();
        return Ok(report);
    }
    let selected = seen.select(
        docs,
        started,
        policy.refresh_after_days,
        policy.max_documents_per_run as usize,
    );
    report.documents_new = selected.len() as i64;
    let run_dir = layout.runs().join(format!("{started}-{}", label));
    // A run with nothing new still compares the store with what the source lists, when it ends
    // what the source no longer lists.
    if selected.is_empty() && listed.is_none() {
        unread(&mut report, &fetched.unread, redactor.as_ref());
        let failures = fetched.child_failures.clone();
        finish(
            &mut seen, &seen_path, &report, wanted, held_since, window, failures,
        )?;
        log(layout, label, &report, started);
        return Ok(report);
    }

    let mut store = layout.store_handle(spec);
    // Each mapped property is written in the kind its mapping names, else the one the store
    // declares; a mapping whose kind differs from the store's is refused before anything is
    // written.
    if let (Some(s), Some(st)) = (structured.as_mut(), structured_source) {
        let ontology = store.ontology().map_err(Failure::Apply)?;
        s.kinds = structured::kinds(st, &ontology)
            .map_err(|e| Failure::Apply(format!("source {label}: {e}")))?;
    }
    // A run into a live store copies the store and `state/` once, before its first
    // `apply-extraction`, and keeps the copy, named by the run's start and label, once that apply
    // commits; the seed of a new store takes none.
    *before = Undo::new(if undoable {
        BeforeApply::new(layout, &store, label, started, snapshot::keep(spec))
    } else {
        BeforeApply::none()
    });
    // On a `postgres` store, whose snapshot is the operator's database backup, a run held to a
    // gate runs in an EKR stage instead.
    if undoable && gated(spec) {
        before.staged = Staged::begin(layout, &mut store)?;
    }
    let host = std::fs::read_to_string(layout.host()).map_err(|e| Failure::Apply(e.to_string()))?;
    let operator = crate::ekr::operator(&host).map_err(Failure::Apply)?;
    if let Some(source) = &structured {
        let records = Records {
            store: &store,
            operator: &operator,
            source,
            started,
            applied_at,
            run_dir: &run_dir,
            scrubbed: &record_scrubbed,
            parents: &parents,
            held: &held,
        };
        apply_records(
            layout,
            &records,
            selected,
            &mut seen,
            &seen_path,
            &mut report,
            before,
        )?;
        // A fetch that left records unread lists only part of the source: nothing is ended.
        if let Some(listed) = listed.as_deref().filter(|_| fetched.unread.is_empty()) {
            end_dropped(&records, listed, &seen, &mut report, before)?;
        }
        unread(&mut report, &fetched.unread, redactor.as_ref());
        if undoable {
            held_to_gate(layout, spec, &store, label, &report, before, started)?;
        }
        let failures = fetched.child_failures.clone();
        finish(
            &mut seen, &seen_path, &report, wanted, held_since, window, failures,
        )?;
        log(layout, label, &report, started);
        return Ok(report);
    }
    let schema = crate::ekr::Binary(store.bin.clone())
        .model_schema()
        .map_err(Failure::Extract)?;
    let model = Model {
        backend: spec.model.backend.unwrap_or(m::ModelBackend::Claude),
        claude: tools.claude.clone(),
        codex: tools.codex.clone(),
        model: spec.model.model.clone(),
        timeout_s: spec.model.timeout_s,
        gateway: spec.model.gateway.clone(),
    };
    let instructions = match &spec.model.instructions {
        Some(rel) => Some(
            std::fs::read_to_string(layout.dir.join(rel))
                .map_err(|e| Failure::Extract(format!("instructions {rel}: {e}")))?,
        ),
        None => None,
    };
    // `None` behind a model gateway: the run is bounded in tokens only, and the cost its answers
    // report is recorded as not counted.
    let budget = spend::dollar_budget(&spec.model);
    // Dollars the costed answers spent. An answer with no cost spends none; such a run is bounded
    // by `policy.max_documents_per_run` and `model.timeout_s`.
    let mut spent = 0.0;
    // The instance's daily ceiling, across its sources and commands; a run holds the home's lock.
    let mut daily = spend::Daily::new(layout, &spec.model, None);
    // The run's budget in tokens, and the tokens its answers reported: an answer whose tokens
    // could not be counted stops the run under the budget before its next batch.
    let mut tokens = spend::TokenBudget::new(&spec.model);
    let mut entities = load_entities(layout);

    let issued: Vec<_> = selected
        .into_iter()
        .map(|d| evidence::issue(d, &operator, started))
        .collect();
    let batches = batches(issued);
    // `refuse_if_left`: every batch is checked as the model would be shown it before the first
    // model call, so a refusal fails the run with nothing sent and nothing stored.
    if let Some(r) = redactor.as_ref().filter(|r| r.refuses()) {
        for batch in &batches {
            let mut p = r.batch();
            let (shown, known) = pseudonymise(&mut p, batch, &uncut, &entities);
            if let Some(why) = refused(&p, &shown, &known) {
                return Err(Failure::Extract(why));
            }
        }
    }
    for (n, batch) in batches.into_iter().enumerate() {
        // Behind a gateway no dollar amount bounds the call, and none is passed to `claude`.
        let remaining = match budget {
            Some(budget) if budget - spent <= 0.0 => {
                report.stopped = Some(format!("budget of {budget} USD spent"));
                break;
            }
            Some(budget) => budget - spent,
            None => f64::INFINITY,
        };
        if let Err(why) = tokens.check() {
            report.stopped = Some(why);
            break;
        }
        let remaining = match daily.check(remaining) {
            Ok(usd) => usd,
            Err(refusal) => {
                // Only a ceiling the day reached; not an uncounted answer or an unknown spend.
                report.daily_ceiling_reached = refusal.reached;
                report.stopped = Some(refusal.reason);
                break;
            }
        };
        let ontology = store.ontology().map_err(Failure::Apply)?;
        // The model is shown the batch with personal data replaced by placeholders; the mapping
        // stays in `pseudonyms`, in memory, until this batch's answer is restored.
        let ((prompt, fixed), pseudonyms) = match &redactor {
            None => {
                let known = Known::from_ontology(&ontology, entities.clone());
                let prompt = extract::prompt_split(
                    &spec.description,
                    instructions.as_deref(),
                    &known,
                    &batch,
                );
                (prompt, None)
            }
            Some(r) => {
                let mut p = r.batch();
                let (shown, known) = pseudonymise(&mut p, &batch, &uncut, &entities);
                // Checked again: the known entity names grow with each batch applied.
                if let Some(why) = refused(&p, &shown, &known) {
                    if report.documents_applied == 0 {
                        return Err(Failure::Extract(why));
                    }
                    report.stopped = Some(why);
                    break;
                }
                let known = Known::from_ontology(&ontology, known);
                let prompt = extract::prompt_split(
                    &spec.description,
                    instructions.as_deref(),
                    &known,
                    &shown,
                );
                if let Some(counts) = report.redacted.as_mut() {
                    for (name, n) in p.counts() {
                        *counts.entry(name.clone()).or_default() += n;
                    }
                }
                (prompt, Some(p))
            }
        };
        let dir = run_dir.join(format!("batch-{n}"));
        let asked = model.ask(&dir.join("claude"), &schema, &prompt, remaining);
        let (calls, usage) = match &asked {
            Ok(answer) => (answer.calls, answer.usage),
            Err(refused) => (refused.calls, refused.usage),
        };
        // Every costed answer and every answer's tokens, a failed one included, add to the spend
        // of the day and the tokens to the run's.
        daily.add_answer(
            match &asked {
                Ok(answer) => answer.cost_usd,
                Err(refused) => refused.cost_usd,
            },
            &usage,
        );
        tokens.add(&usage);
        // Every model call counts, a failed one included. The fixed part is the system prompt,
        // the answer's schema (both sent as flags) and the prompt up to the documents.
        let fixed_bytes = extract::SYSTEM_PROMPT.len() + schema.to_string().len() + fixed;
        report.add_calls(calls, usage, fixed_bytes, prompt.len() - fixed);
        let answer = match asked.map_err(|refused| refused.message) {
            Ok(answer) => answer,
            Err(e) if report.documents_applied == 0 => return Err(Failure::Extract(e)),
            Err(e) => {
                report.stopped = Some(e);
                break;
            }
        };
        spent += answer.cost_usd.unwrap_or(0.0);
        report.truncated += batch.iter().filter(|i| cut.contains(&i.doc.key)).count();
        report.cost_usd = add_cost(report.cost_usd, answer.cost_usd);
        let mut restored = answer.document.clone();
        // With a policy, the batch directory keeps only the restored `extraction.yaml`: the
        // prompt and the raw answer carry placeholders, and beside it they would give the mapping.
        let pseudonymised = pseudonyms.is_some();
        if let Some(p) = &pseudonyms {
            report.unrestored += p.restore(&mut restored);
        }
        drop(pseudonyms);
        let (doc, refused) = extract::merge(&restored, &batch);
        report.facts_refused += refused;
        let path = dir.join("extraction.yaml");
        let text = serde_yaml_ng::to_string(&doc).expect("YAML");
        if !pseudonymised {
            let _ = std::fs::write(dir.join("prompt.txt"), &prompt);
            let _ = std::fs::write(
                dir.join("model.json"),
                serde_json::to_string_pretty(&answer.document).unwrap_or_default(),
            );
        }
        std::fs::write(&path, text).map_err(|e| Failure::Apply(e.to_string()))?;
        before.take()?;
        let applied = match store.apply(&path) {
            Ok(r) => r,
            Err(e) if crate::ekr::outcome_unknown(&e) => return Err(unknown_write(e, before)),
            Err(e) if report.documents_applied == 0 => return Err(Failure::Apply(e)),
            Err(e) => {
                report.stopped = Some(e);
                break;
            }
        };
        before.committed();
        if !pseudonymised {
            let _ = std::fs::write(
                dir.join("report.json"),
                serde_json::to_string_pretty(&applied).unwrap_or_default(),
            );
        }
        report.parts_rejected += crate::ekr::applied(&applied).rejected;
        // A document a rejected fact cites is seen all the same: EKR would reject the fact again
        // on the same text, and a retry would cost a model call and a slot of the run on every
        // run. The report names it with EKR's refusal.
        let rejected = extract::cited_by_rejected(&doc, &applied);
        for issued in &batch {
            if let Some(refusals) = rejected.get(&issued.id) {
                report
                    .rejected
                    .push(Rejected::new(&issued.doc.key, refusals));
            }
            seen.record(&issued.doc, applied_at);
        }
        report.documents_applied += batch.len() as i64;
        for (ty, name) in extract::entity_names(&restored) {
            let names = entities.entry(ty).or_default();
            if !names.contains(&name) && names.len() < KNOWN_PER_TYPE {
                names.push(name);
            }
        }
        seen.save(&seen_path).map_err(Failure::Apply)?;
        let _ = crate::home::write_atomic(
            &layout.entities(),
            serde_json::to_string_pretty(&entities)
                .unwrap_or_default()
                .as_bytes(),
        );
    }
    unread(&mut report, &fetched.unread, redactor.as_ref());
    if undoable {
        held_to_gate(layout, spec, &store, label, &report, before, started)?;
    }
    let failures = fetched.child_failures.clone();
    finish(
        &mut seen, &seen_path, &report, wanted, held_since, window, failures,
    )?;
    log(layout, label, &report, started);
    Ok(report)
}

/// The run's snapshot ([`BeforeApply`]), the name it was kept under, and whether an apply
/// committed: what the run gate needs to undo the run. On a `postgres` store, the run's stage
/// ([`Staged`]) instead.
struct Undo {
    before: BeforeApply,
    /// The snapshot's name, once the first `apply-extraction` committed and it was kept.
    kept: Option<String>,
    applied: bool,
    /// The stage the run runs in, until the gate publishes or abandons it, or [`settle`] does.
    staged: Option<Staged>,
}

impl Undo {
    fn new(before: BeforeApply) -> Self {
        Self {
            before,
            kept: None,
            applied: false,
            staged: None,
        }
    }

    /// Copies the store and `state/` before the run's first `apply-extraction`.
    fn take(&mut self) -> Result<(), Failure> {
        self.before.take().map_err(Failure::Apply)
    }

    /// After an `apply-extraction` committed: keeps the run's snapshot the first time. The run's
    /// answer stays what the store did; a snapshot that cannot be kept is said on stderr.
    fn committed(&mut self) {
        self.applied = true;
        match self.before.publish() {
            Ok(Some(name)) => self.kept = Some(name),
            Ok(None) => {}
            Err(e) => eprintln!("cortex: the snapshot taken before this run was not kept: {e}"),
        }
    }
}

/// Settles the stage of a run that ended without its gate settling it: one that failed part-way,
/// or one that committed nothing. A run that committed something keeps what it applied, as on
/// SQLite: its stage is published, and a publish that is refused is said in the run's reason. A
/// run that committed nothing has its stage abandoned, and `state/` stays as the run left it, as a
/// SQLite run's does. A run that failed because EKR could not say whether one of its writes took
/// effect ([`unknown_write`]) leaves its stage in `stage.json`, neither published nor abandoned:
/// the next run abandons it and puts `state/` back ([`Staged::recover`]).
fn settle(
    layout: &Layout,
    spec: &m::InstanceSpec,
    label: &str,
    ran: Result<Report, Failure>,
    mut before: Undo,
) -> Result<Report, Failure> {
    let Some(staged) = before.staged.take() else {
        return match ran {
            // A gated run outside a stage whose write was unknown: an earlier batch committed
            // and the run never reached its gate, so it is undone as a run its gate refuses is.
            Err(Failure::StoreOutcomeUnknown(why)) if gated(spec) && before.kept.is_some() => {
                Err(undo_unknown(layout, spec, label, why, &before))
            }
            ran => ran,
        };
    };
    if matches!(ran, Err(Failure::StoreOutcomeUnknown(_))) {
        return ran;
    }
    if !before.applied {
        if let Err(e) = staged.discard() {
            eprintln!("cortex: {e}");
        }
        return ran;
    }
    // A run that failed part-way keeps its own outcome when the publish fails too, an unknown
    // publish outcome included: the run's failure is the source's, and `stage.json` is kept.
    match (ran, staged.publish()) {
        (ran, Ok(())) => ran,
        (Ok(_), Err(e)) => Err(e),
        (Err(failed), Err(e)) => Err(failed.and(e.reason())),
    }
}

/// The failure of a store write of the run whose outcome EKR could not say
/// ([`crate::ekr::outcome_unknown`]): `apply-extraction`, or the commit of a transaction. The run
/// fails as `store-outcome-unknown` before it marks the batch seen. In a stage, the stage is left
/// to the next run ([`settle`]), which abandons it and puts `state/` back, so the run's documents
/// are fetched again; outside one, the next run extracts the batch again, so nothing is lost, and
/// a batch that did land is written a second time: a second set of assertions and evidence, as
/// any re-extraction writes. A gated run that kept its snapshot is undone from it instead
/// ([], from []).
fn unknown_write(error: String, before: &Undo) -> Failure {
    let next = match &before.staged {
        Some(staged) => format!(
            "its stage {} is kept in stage.json, and the next run abandons it and fetches the \
             run's documents again",
            staged.record.stage
        ),
        None => "the batch was not marked seen, so the next run extracts it again".to_string(),
    };
    Failure::StoreOutcomeUnknown(format!("{error}; {next}"))
}

/// The failure of a gated run outside a stage that ended on an unknown store write after an earlier
/// batch committed: the run's snapshot is restored and `state/` put back with [`undo`], the code
/// that undoes a run its gate refused, so no batch stays in the store without being held to the
/// gate. The run still fails as `store-outcome-unknown`; its reason and its `cortex.log` line say
/// what was done to the snapshot.
fn undo_unknown(
    layout: &Layout,
    spec: &m::InstanceSpec,
    label: &str,
    why: String,
    before: &Undo,
) -> Failure {
    let home = layout
        .dir
        .parent()
        .and_then(std::path::Path::parent)
        .unwrap_or(&layout.dir);
    let (restored, undone) = undo(
        layout,
        &spec.name.0,
        before.kept.as_deref(),
        &Systemd::from_env(home),
    );
    let reason = format!("{why}; {undone}");
    layout.log_line(&json!({
        "at": now_ms(),
        "source": label,
        "outcome": "store-outcome-unknown",
        "restored": restored,
        "reason": reason,
    }));
    Failure::StoreOutcomeUnknown(reason)
}

/// The failure a stage verb's error is: [`Failure::StoreOutcomeUnknown`] when EKR could not say
/// whether it took effect ([`crate::ekr::outcome_unknown`]), [`Failure::Apply`] otherwise.
fn stage_failure(error: String) -> Failure {
    if crate::ekr::outcome_unknown(&error) {
        Failure::StoreOutcomeUnknown(error)
    } else {
        Failure::Apply(error)
    }
}

impl Failure {
    /// The same failure, its reason followed by `more`.
    fn and(self, more: &str) -> Self {
        match self {
            Failure::Fetch(m) => Failure::Fetch(format!("{m}; {more}")),
            Failure::Extract(m) => Failure::Extract(format!("{m}; {more}")),
            Failure::Apply(m) => Failure::Apply(format!("{m}; {more}")),
            Failure::StoreOutcomeUnknown(m) => Failure::StoreOutcomeUnknown(format!("{m}; {more}")),
        }
    }

    /// The failure's reason, whatever its outcome.
    fn reason(&self) -> &str {
        match self {
            Failure::Fetch(m)
            | Failure::Extract(m)
            | Failure::Apply(m)
            | Failure::StoreOutcomeUnknown(m) => m,
        }
    }
}

/// Whether the spec holds a run to a gate.
fn gated(spec: &m::InstanceSpec) -> bool {
    spec.gate.as_ref().is_some_and(|g| !g.checks.is_empty())
}

/// A run on a `postgres` store held to a gate runs in an EKR stage (`ekr stage`, EKR 0.0.33):
/// every `ekr` of the run joins it (`EKR_STAGE`, which crosses a Connectors launch), so its reads
/// see its own commits and its commits reach the store only when the stage is published, after
/// the run passed its gate. Until then [`snapshot::stage_record`] holds the stage, the head the run
/// began from and `state/` as it was before the run.
struct Staged {
    /// The store in no stage, for the stage verbs.
    store: crate::ekr::Store,
    layout: Layout,
    record: snapshot::StageRecord,
}

impl Staged {
    /// On a `postgres` store: begins a stage at the store's head, records it and that head (the
    /// stage's base) with a copy of `state/`, and puts `store` in the stage. A SQLite store runs
    /// in none. A begin whose outcome is unknown fails the run as `store-outcome-unknown`: a stage
    /// EKR may hold as begun is recorded nowhere, and the next run abandons it ([`Self::recover`]).
    fn begin(layout: &Layout, store: &mut crate::ekr::Store) -> Result<Option<Self>, Failure> {
        if store.backend != crate::ekr::Backend::Postgres {
            return Ok(None);
        }
        let (stage, head) = store.begin_stage().map_err(stage_failure)?;
        let record = match snapshot::StageRecord::write(layout, &stage, head) {
            Ok(record) => record,
            Err(e) => {
                let _ = store.abandon_stage(&stage);
                return Err(Failure::Apply(e));
            }
        };
        let plain = store.clone();
        store.stage = Some(stage);
        Ok(Some(Self {
            store: plain,
            layout: Layout::new(layout.dir.clone()),
            record,
        }))
    }

    /// Settles what a run that did not finish left. The stage `stage.json` names: one the run
    /// began to publish is published again at the same head, which completes a publish a kill
    /// interrupted (or answers the one that completed), and keeps `state/` as it is; any other, or
    /// one whose publish is refused because the head moved, is abandoned and `state/` put back to
    /// the copy from before that run, so its documents are fetched again. Then every stage of a
    /// `postgres` store still `Begun` (`ekr stage list`), such as one begun by a run killed before
    /// it wrote `stage.json`, is abandoned: nothing records it, and it holds a copy of the store.
    /// A stage that cannot be settled keeps `stage.json` for the run after; when EKR could not say
    /// whether the settling verb took effect, the run fails as `store-outcome-unknown`.
    fn recover(layout: &Layout, spec: &m::InstanceSpec) -> Result<(), Failure> {
        let store = layout.store_handle(spec);
        if let Some(record) = snapshot::StageRecord::read(layout).map_err(Failure::Apply)? {
            let not_settled = |e: String| {
                stage_failure(format!(
                    "stage {} of a run that did not finish is not settled: {e}",
                    record.stage
                ))
            };
            let published = record.publishing
                && match store.publish_stage(&record.stage, record.head) {
                    Ok(()) => true,
                    Err(e) if e.contains("stage-head-moved") => false,
                    Err(e) => return Err(not_settled(e)),
                };
            if !published {
                match store.abandon_stage(&record.stage) {
                    Ok(()) => record.rewind_state(layout).map_err(Failure::Apply)?,
                    Err(e) if e.contains("stage-already-published") => {}
                    Err(e) => return Err(not_settled(e)),
                }
            }
            snapshot::StageRecord::remove(layout).map_err(Failure::Apply)?;
        }
        if store.backend == crate::ekr::Backend::Postgres {
            match store.begun_stages() {
                Ok(begun) => {
                    for stage in begun {
                        if let Err(e) = store.abandon_stage(&stage) {
                            eprintln!("cortex: stage {stage}, left begun, was not abandoned: {e}");
                        }
                    }
                }
                Err(e) => eprintln!("cortex: the store's stages were not listed: {e}"),
            }
        }
        Ok(())
    }

    /// Publishes the stage at the head the run began from, recorded in `stage.json` first so a
    /// run killed while publishing is completed by the next. A refusal (the store's head moved
    /// since, `stage-head-moved`) publishes nothing: the stage is abandoned and `state/` is put
    /// back, and the run fails as `apply-refused`. A publish whose outcome is unknown may have
    /// landed: `stage.json` stays with `publishing` set, `state/` is not put back and the stage is
    /// not abandoned, so the next run publishes it at the same head ([`Self::recover`]); the run
    /// fails as `store-outcome-unknown`.
    fn publish(mut self) -> Result<(), Failure> {
        let published = match self.record.set_publishing(&self.layout, true) {
            Ok(()) => self
                .store
                .publish_stage(&self.record.stage, self.record.head),
            Err(e) => Err(format!("the stage's publish could not be recorded: {e}")),
        };
        match published {
            Ok(()) => {
                if let Err(e) = snapshot::StageRecord::remove(&self.layout) {
                    eprintln!("cortex: {e}");
                }
                Ok(())
            }
            Err(e) if crate::ekr::outcome_unknown(&e) => {
                Err(Failure::StoreOutcomeUnknown(format!(
                    "the run's publish has an unknown outcome: {e}; its stage {} is kept in \
                     stage.json, and the next run publishes it at head {} or settles it",
                    self.record.stage, self.record.head
                )))
            }
            Err(e) => {
                let state = self.rewind();
                let stage = self.record.stage.clone();
                let abandoned = match self.store.abandon_stage(&stage) {
                    Ok(()) => {
                        if let Err(e) = snapshot::StageRecord::remove(&self.layout) {
                            eprintln!("cortex: {e}");
                        }
                        format!("its stage {stage} was abandoned")
                    }
                    Err(a) => {
                        // The next run abandons it, and publishes nothing of it.
                        if let Err(e) = self.record.set_publishing(&self.layout, false) {
                            eprintln!("cortex: {e}");
                        }
                        format!(
                            "its stage {stage} was not abandoned ({a}); the next run abandons it"
                        )
                    }
                };
                Err(Failure::Apply(format!(
                    "the run was not published, so nothing of it is in the store: {e}; \
                     {abandoned}; {state}"
                )))
            }
        }
    }

    /// Abandons the stage of a run that failed its gate: nothing of the run reaches the store, and
    /// `state/` is put back. An abandon that fails leaves `stage.json` for the next run, which
    /// abandons the stage. Answers what was done, for the failure's reason.
    fn abandon(self) -> String {
        let (stage, head) = (&self.record.stage, self.record.head);
        let state = self.rewind();
        match self.store.abandon_stage(stage) {
            Ok(()) => {
                if let Err(e) = snapshot::StageRecord::remove(&self.layout) {
                    eprintln!("cortex: {e}");
                }
                format!(
                    "the run was undone: its stage {stage} was abandoned, so the store's head is \
                     still {head}, and {state}"
                )
            }
            Err(e) => format!(
                "the run was undone: its stage {stage} was not published, so the store's head is \
                 still {head}, but it was not abandoned either ({e}), and the next run abandons \
                 it; {state}"
            ),
        }
    }

    /// Abandons the stage of a run that committed nothing in it, leaving `state/` as the run left
    /// it. An abandon that fails leaves `stage.json`, its copy of `state/` taken now, so the next
    /// run abandons the stage and keeps `state/` as it is.
    fn discard(self) -> Result<(), String> {
        let (stage, head) = (&self.record.stage, self.record.head);
        match self.store.abandon_stage(stage) {
            Ok(()) => snapshot::StageRecord::remove(&self.layout),
            Err(e) => {
                snapshot::StageRecord::write(&self.layout, stage, head)?;
                Err(format!(
                    "stage {stage}, in which nothing was committed, was not abandoned: {e}; the \
                     next run abandons it"
                ))
            }
        }
    }

    /// Puts `state/` back to the copy from before the run; answers what was done.
    fn rewind(&self) -> String {
        match self.record.rewind_state(&self.layout) {
            Ok(()) => {
                "state/ is back where it was before the run, so its documents are fetched again"
                    .into()
            }
            Err(e) => format!("state/ was not put back: {e}"),
        }
    }
}

/// Holds a source run that applied something to the spec's `gate`. A failed check fails the run
/// as `apply-refused`, so the scheduler counts it as a failed run of the source: the run is undone
/// ([`undo`]), and `cortex.log` records each failed check with its measure and value, the snapshot
/// restored, and the failure's reason.
fn held_to_gate(
    layout: &Layout,
    spec: &m::InstanceSpec,
    store: &crate::ekr::Store,
    label: &str,
    report: &Report,
    before: &mut Undo,
    started: i64,
) -> Result<(), Failure> {
    let Some(checks) = spec.gate.as_ref().filter(|g| !g.checks.is_empty()) else {
        return Ok(());
    };
    if !before.applied {
        return Ok(());
    }
    let failed = gate::evaluate(checks, report, || gate::quality(store));
    if failed.is_empty() {
        return match before.staged.take() {
            Some(staged) => staged.publish(),
            None => Ok(()),
        };
    }
    // `<home>/instances/<name>`: the units live with the home, not the instance.
    let home = layout
        .dir
        .parent()
        .and_then(std::path::Path::parent)
        .unwrap_or(&layout.dir);
    let (restored, undone) = match before.staged.take() {
        Some(staged) => (None, staged.abandon()),
        None => undo(
            layout,
            &spec.name.0,
            before.kept.as_deref(),
            &Systemd::from_env(home),
        ),
    };
    let why = format!("{}; {undone}", gate::reason(&failed));
    let mut line = log_line(label, report, started);
    line["gate"] = json!({
        "failed": failed.iter().map(gate::Failed::json).collect::<Vec<_>>(),
        "restored": restored,
        "reason": why,
    });
    layout.log_line(&line);
    Err(Failure::Apply(why))
}

/// Undoes a run that failed its gate from `kept`, the snapshot taken before it: the store and
/// `state/` are restored (`snapshot::restore_held`; the command holds the home's lock). Answers
/// the snapshot restored, and what was done, for the failure's reason. A restore that is refused or
/// fails still puts `state/` back, so the run's documents are fetched again, and the reason says
/// the store still holds the run. Without a kept snapshot (`snapshots.keep` 0) nothing is undone.
/// A run on a `postgres` store is undone by abandoning its stage ([`Staged::abandon`]).
fn undo(
    layout: &Layout,
    instance: &str,
    kept: Option<&str>,
    systemd: &Systemd,
) -> (Option<String>, String) {
    let Some(name) = kept else {
        return (
            None,
            "the run was not undone: no snapshot was kept before it (snapshots.keep 0, or a \
             snapshot that could not be kept)"
                .into(),
        );
    };
    let refused = match snapshot::restore_held(layout, instance, name, systemd) {
        Ok(done) => {
            if let Some(e) = done.viewer_failed {
                eprintln!("cortex: the viewer did not start again after the undo: {e}");
            }
            return (
                Some(name.to_string()),
                format!("the run was undone: the store and state/ are back at snapshot {name}"),
            );
        }
        Err(RestoreError::Refused(snapshot::Refusal::Unsupported)) => {
            "the store's backend has no snapshot to restore".to_string()
        }
        Err(RestoreError::Refused(snapshot::Refusal::Busy(reason))) => reason,
        Err(RestoreError::Refused(snapshot::Refusal::NoSuchSnapshot)) => {
            format!("snapshot {name} is gone")
        }
        Err(RestoreError::Failed(e)) => e,
    };
    let state = match snapshot::rewind_state(layout, name) {
        Ok(true) => {
            format!("state/ is back at snapshot {name}, so its documents are fetched again")
        }
        Ok(false) => format!("snapshot {name} holds no copy of state/ to put back"),
        Err(e) => format!("state/ was not put back either: {e}"),
    };
    (
        None,
        format!("the store still holds the run, its restore was refused: {refused}; {state}"),
    )
}

/// `issued` in batches of at most [`BATCH_CHARS`] characters of text, a longer document alone.
fn batches(issued: Vec<evidence::Issued>) -> Vec<Vec<evidence::Issued>> {
    let mut batches: Vec<Vec<evidence::Issued>> = vec![Vec::new()];
    let mut chars = 0;
    for item in issued {
        let len = item.doc.text.len();
        if chars + len > BATCH_CHARS && !batches.last().expect("one batch").is_empty() {
            batches.push(Vec::new());
            chars = 0;
        }
        chars += len;
        batches.last_mut().expect("one batch").push(item);
    }
    batches
}

/// What a structured run applies its records with.
struct Records<'a> {
    store: &'a crate::ekr::Store,
    operator: &'a str,
    source: &'a structured::Source<'a>,
    started: i64,
    applied_at: i64,
    run_dir: &'a std::path::Path,
    /// Each record's matches of the irreversible rules, by document key.
    scrubbed: &'a HashMap<String, BTreeMap<String, usize>>,
    /// Every parent record the run read ([`structured::Source::parents`]).
    parents: &'a structured::Parents,
    /// What the run keeps of the records a failed child call left unread, and of the changes of a
    /// link not found in full ([`structured::Held`]).
    held: &'a [structured::Held],
}

/// Applies the `selected` records of a structured source with no model call: each batch is one
/// extraction document mapped by `src/structured.rs`, written to the run's batch directory as
/// `extraction.yaml` and applied; the report keeps its cost of `0`. A batch that fails to apply
/// fails the run when nothing was applied yet and stops it otherwise, as on the model path. A
/// record with a part EKR rejected is counted in `parts_rejected`, named in `rejected` with EKR's
/// refusal, and neither applied nor marked seen, so the run is not successful and the record is
/// read and tried again. Each record's evidence leads with its mapped values
/// ([`structured::Source::lead`]) before the cut at [`evidence::PAYLOAD_MAX_BYTES`]; a record whose
/// mapped values alone are over that bound is not applied, but named in `skipped` and recorded as
/// seen, as its next run would refuse it again. A record with a mapped value that does not convert
/// to its property's kind ([`structured::Source::unconverted`]) is left out of its batch, named in
/// `unconverted`, and marked seen like an applied one, so it is skipped until its text changes.
fn apply_records(
    layout: &Layout,
    r: &Records<'_>,
    selected: Vec<sources::Document>,
    seen: &mut SeenState,
    seen_path: &std::path::Path,
    report: &mut Report,
    before: &mut Undo,
) -> Result<(), Failure> {
    let mut entities = load_entities(layout);
    let mut issued = Vec::new();
    for d in selected {
        let unconverted = r.source.unconverted(&d);
        if !unconverted.is_empty() {
            report
                .unconverted
                .extend(
                    unconverted
                        .into_iter()
                        .map(|(property, value_kind)| Unconverted {
                            document: d.key.clone(),
                            property,
                            value_kind,
                        }),
                );
            seen.record(&d, r.applied_at);
            continue;
        }
        let lead = r.source.lead(&d);
        let bytes = evidence::lead_bytes(&d, &lead);
        if bytes > evidence::PAYLOAD_MAX_BYTES {
            report.skipped.push(format!(
                "{}: its mapped values take {bytes} bytes of evidence, over the {} bytes EKR \
                 takes in one payload; the record was not applied",
                d.key,
                evidence::PAYLOAD_MAX_BYTES
            ));
            seen.record(&d, r.applied_at);
            continue;
        }
        issued.push(evidence::issue_led(d, &lead, r.operator, r.started));
    }
    if issued.is_empty() {
        return Ok(());
    }
    let index = r.source.index(&issued, r.parents);
    for (n, batch) in batches(issued).into_iter().enumerate() {
        let mapped = r.source.document(&batch, &index);
        let (doc, refused) = extract::merge(&mapped.document, &batch);
        report.facts_refused += refused;
        let dir = r.run_dir.join(format!("batch-{n}"));
        std::fs::create_dir_all(&dir).map_err(|e| Failure::Apply(e.to_string()))?;
        let path = dir.join("extraction.yaml");
        let text = serde_yaml_ng::to_string(&doc).expect("YAML");
        std::fs::write(&path, text).map_err(|e| Failure::Apply(e.to_string()))?;
        before.take()?;
        let applied = match r.store.apply(&path) {
            Ok(applied) => applied,
            Err(e) if crate::ekr::outcome_unknown(&e) => return Err(unknown_write(e, before)),
            Err(e) if report.documents_applied == 0 => return Err(Failure::Apply(e)),
            Err(e) => {
                report.stopped = Some(e);
                break;
            }
        };
        before.committed();
        let _ = std::fs::write(
            dir.join("report.json"),
            serde_json::to_string_pretty(&applied).unwrap_or_default(),
        );
        report.parts_rejected += crate::ekr::applied(&applied).rejected;
        let rejected = mapped.rejected(&applied);
        for issued in &batch {
            if let Some(refusals) = rejected.get(&issued.id) {
                report
                    .rejected
                    .push(Rejected::new(&issued.doc.key, refusals));
                continue;
            }
            report.documents_applied += 1;
            seen.record(&issued.doc, r.applied_at);
            if let (Some(counts), Some(scrubbed)) =
                (report.redacted.as_mut(), r.scrubbed.get(&issued.doc.key))
            {
                for (name, n) in scrubbed {
                    *counts.entry(name.clone()).or_default() += n;
                }
            }
        }
        for (ty, name) in extract::entity_names(&mapped.document) {
            let names = entities.entry(ty).or_default();
            if !names.contains(&name) && names.len() < KNOWN_PER_TYPE {
                names.push(name);
            }
        }
        seen.save(seen_path).map_err(Failure::Apply)?;
        let _ = crate::home::write_atomic(
            &layout.entities(),
            serde_json::to_string_pretty(&entities)
                .unwrap_or_default()
                .as_bytes(),
        );
    }
    Ok(())
}

/// Operations of one transaction that ends values: well inside EKR's 10,000 per transaction.
const ENDS_PER_TRANSACTION: usize = 1_000;

/// `dropped: Supersede`: after a structured run applied its records, ends each value the store
/// holds of this source that the source no longer lists ([`structured::Source::ended`]): read from
/// the store (`ekr snapshot`, `ekr ontology`), not from `state/`, and compared with `listed`, every
/// record the run read. A record whose current text `seen` does not hold (held back, rejected,
/// or beyond `max_documents_per_run`), or whose mapped values are over the evidence bound, keeps
/// its values; a run that lists no record ends nothing. The operations are committed as
/// transactions of their own (`runs/<run>/ends-<n>.yaml`), after the run's snapshot is taken; one
/// that fails fails the run when nothing was applied yet and stops it otherwise.
fn end_dropped(
    r: &Records<'_>,
    listed: &[sources::Document],
    seen: &SeenState,
    report: &mut Report,
    before: &mut Undo,
) -> Result<(), Failure> {
    // A source that lists no record at all ends nothing: an empty list is read as a list that
    // failed, so a source cannot drop its last record.
    if listed.is_empty() {
        return Ok(());
    }
    // A record whose mapped values are over the evidence bound is recorded as seen but never
    // applied ([`apply_records`]): on every run it stays too large, it keeps its values.
    let unsettled: BTreeSet<String> = listed
        .iter()
        .filter(|d| {
            seen.documents
                .get(&d.key)
                .is_none_or(|s| s.hash.as_deref() != Some(&crate::state::doc_hash(d)))
                || evidence::lead_bytes(d, &r.source.lead(d)) > evidence::PAYLOAD_MAX_BYTES
        })
        .map(|d| d.key.clone())
        .collect();
    let snapshot = r.store.snapshot().map_err(Failure::Apply)?;
    let ontology = r.store.ontology().map_err(Failure::Apply)?;
    let ops = r.source.ended(
        &snapshot,
        &ontology,
        &r.source.listed(listed),
        &unsettled,
        r.held,
    );
    if ops.is_empty() {
        return Ok(());
    }
    std::fs::create_dir_all(r.run_dir).map_err(|e| Failure::Apply(e.to_string()))?;
    for (n, chunk) in ops.chunks(ENDS_PER_TRANSACTION).enumerate() {
        before.take()?;
        let path = r.run_dir.join(format!("ends-{n}.yaml"));
        match r.store.transact(&path, r.operator, chunk) {
            Ok(_) => before.committed(),
            Err(e) if crate::ekr::outcome_unknown(&e) => return Err(unknown_write(e, before)),
            Err(e) if !before.applied => return Err(Failure::Apply(e)),
            Err(e) => {
                report.stopped = Some(match report.stopped.take() {
                    Some(stopped) => format!("{stopped}; {e}"),
                    None => e,
                });
                return Ok(());
            }
        }
        for op in chunk {
            match op {
                crate::ekr::Operation::Supersede { .. } => report.superseded += 1,
                crate::ekr::Operation::Retract { .. } => report.retracted += 1,
                crate::ekr::Operation::DeleteEdge(_)
                | crate::ekr::Operation::DefineNodeType { .. }
                | crate::ekr::Operation::DefineEdgeType { .. }
                | crate::ekr::Operation::ModifyProperty { .. } => {}
            }
        }
    }
    Ok(())
}

/// `key`, or a message that can name one, masked as a document key is: credential shapes, and the
/// `Credential` class's when the policy names it.
fn masked_key(redactor: Option<&Redactor>, key: &str) -> String {
    let masked = mask_key(key).0;
    match redactor {
        Some(r) => r.scrub_key(&masked).0,
        None => masked,
    }
}

/// Why the run refuses a batch: the classes `refuse_if_left` names that are still detected in
/// what the model would be shown of it (`shown`, the known entity names `known`). Names the
/// classes, never a value.
fn refused(
    p: &redact::Batch<'_>,
    shown: &[evidence::Issued],
    known: &BTreeMap<String, Vec<String>>,
) -> Option<String> {
    let mut left = BTreeSet::new();
    for issued in shown {
        let d = &issued.doc;
        for text in [Some(&d.key), d.title.as_ref(), d.description.as_ref()]
            .into_iter()
            .flatten()
            .chain([&d.text])
        {
            left.extend(p.left(text));
        }
    }
    for name in known.values().flatten() {
        left.extend(p.left(name));
    }
    (!left.is_empty()).then(|| {
        format!(
            "redaction refused a batch: {} still detected after masking in what the model would \
             be shown (refuse_if_left); nothing of it was sent or stored",
            left.into_iter().collect::<Vec<_>>().join(", ")
        )
    })
}

/// Adds to why the run stopped what its fetch left unread.
fn unread(report: &mut Report, unread: &[String], redactor: Option<&Redactor>) {
    if unread.is_empty() {
        return;
    }
    // A message the fetch built can name a document key; it is masked as the key is.
    let why = unread
        .iter()
        .map(|u| masked_key(redactor, u))
        .collect::<Vec<_>>()
        .join("; ");
    report.stopped = Some(match report.stopped.take() {
        Some(stopped) => format!("{stopped}; {why}"),
        None => why,
    });
}

/// Records what a windowed run leaves for the next: its window's end as the start of the last
/// successful run when it applied every one of the `wanted` documents and read every record, its
/// window's start as `pending_since` when it did not; `held_since` as this run computed it; and
/// the child call failure counts.
fn finish(
    seen: &mut SeenState,
    path: &std::path::Path,
    report: &Report,
    wanted: usize,
    held_since: Option<i64>,
    window: Option<Window>,
    child_failures: BTreeMap<String, u32>,
) -> Result<(), Failure> {
    let Some(window) = window else {
        return Ok(());
    };
    seen.held_since = held_since;
    seen.child_failures = child_failures;
    if report.stopped.is_none() && report.documents_applied == wanted as i64 {
        seen.last_success_started_at = Some(window.until_ms);
        seen.pending_since = None;
    } else {
        seen.pending_since = Some(
            seen.pending_since
                .map_or(window.since_ms, |p| p.min(window.since_ms)),
        );
    }
    seen.save(path).map_err(Failure::Apply)
}

fn log(layout: &Layout, label: &str, report: &Report, started: i64) {
    layout.log_line(&log_line(label, report, started));
}

/// The `cortex.log` line of a run.
fn log_line(label: &str, report: &Report, started: i64) -> serde_json::Value {
    let mut line = json!({
        "at": started,
        "source": label,
        "documents_new": report.documents_new,
        "documents_applied": report.documents_applied,
        "facts_refused": report.facts_refused,
        "parts_rejected": report.parts_rejected,
        "masked": report.masked,
        "cost_usd": report.cost_usd,
        "seconds": (now_ms() - started) as f64 / 1000.0,
        "stopped": report.stopped,
    });
    if let (Some(line), serde_json::Value::Object(calls)) =
        (line.as_object_mut(), report.calls_json())
    {
        line.extend(calls);
    }
    // Present only behind a model gateway, where `cost_usd` is as reported and counted against no
    // dollar limit: other runs log what they did before.
    if !report.cost_counted {
        line["cost_counted"] = json!(false);
    }
    // Present only when a document was cut: a run that cut nothing logs what it did before.
    if report.truncated > 0 {
        line["truncated"] = json!(report.truncated);
    }
    // Present only when a document adopted by key was hashed: other runs log what they did before.
    if report.hashed > 0 {
        line["hashed"] = json!(report.hashed);
    }
    // Present only when a parent was skipped: a run that skipped none logs what it did before.
    if !report.skipped.is_empty() {
        line["skipped"] = json!(report.skipped);
    }
    // Present only when EKR rejected a part: a run that had none logs what it did before.
    if !report.rejected.is_empty() {
        line["rejected"] = json!(report
            .rejected
            .iter()
            .map(Rejected::json)
            .collect::<Vec<_>>());
    }
    // Present only when a record's value did not convert: a run that had none logs as before.
    if !report.unconverted.is_empty() {
        line["unconverted"] = json!(report
            .unconverted
            .iter()
            .map(Unconverted::json)
            .collect::<Vec<_>>());
    }
    // Present only when the policy pseudonymises: a spec file without one logs what it did before.
    if let Some(redacted) = &report.redacted {
        line["redacted"] = json!(redacted);
        line["unrestored"] = json!(report.unrestored);
    }
    // Present only when the run ended a value: a run that ended none logs what it did before.
    if report.superseded + report.retracted > 0 {
        line["superseded"] = json!(report.superseded);
        line["retracted"] = json!(report.retracted);
    }
    line
}

/// A document's masked and scrubbed text before the cut, and what the irreversible rules
/// replaced in its kept text, title and description.
struct Uncut {
    text: String,
    scrubbed: BTreeMap<String, usize>,
}

/// What the model is shown of `batch`, and the known entity names, with every value the
/// redaction policy finds replaced by its placeholder in `p`: the document key (the `Source:`
/// line), title, description and text. The text is the kept part of the text before the cut
/// (`uncut`), so a value the cut splits is replaced too. What the irreversible rules replaced
/// before storage is counted in `p` here, as the document is sent.
fn pseudonymise(
    p: &mut redact::Batch<'_>,
    batch: &[evidence::Issued],
    uncut: &HashMap<String, Vec<Uncut>>,
    entities: &BTreeMap<String, Vec<String>>,
) -> (Vec<evidence::Issued>, BTreeMap<String, Vec<String>>) {
    let find = |doc: &sources::Document| -> Option<&Uncut> {
        uncut
            .get(&doc.key)
            .and_then(|texts| texts.iter().find(|t| t.text.starts_with(&doc.text)))
    };
    let full = |doc: &sources::Document| -> String {
        find(doc).map_or_else(|| doc.text.clone(), |u| u.text.clone())
    };
    for issued in batch {
        if let Some(u) = find(&issued.doc) {
            for (name, n) in &u.scrubbed {
                p.add(name, *n);
            }
        }
    }
    for issued in batch {
        let d = &issued.doc;
        // A record read from a file repeats text that is not its own: its thread context (earlier
        // records), its key (its id) and its title (the file's name, on every record of it).
        // Only its own text counts toward how often a name occurs, so a repetition never makes a
        // name look frequent; the rest is reserved without counting, and a name in it gets the
        // batch's placeholder like any other.
        let file_record = d.origin == sources::Origin::FileRecord;
        for text in [Some(&d.key), d.title.as_ref(), d.description.as_ref()]
            .into_iter()
            .flatten()
        {
            if file_record {
                p.reserve_name(text);
            } else {
                p.reserve(text);
            }
        }
        let text = full(d);
        if file_record {
            let (own, context) = sources::split_context(&text);
            p.reserve(own);
            p.reserve_name(context);
        } else {
            p.reserve(&text);
        }
    }
    for name in entities.values().flatten() {
        p.reserve_name(name);
    }
    let shown = batch
        .iter()
        .map(|issued| {
            let d = &issued.doc;
            evidence::Issued {
                id: issued.id.clone(),
                item: serde_yaml_ng::Value::Null,
                doc: sources::Document {
                    key: p.replace(&d.key),
                    origin: d.origin.clone(),
                    title: d.title.as_deref().map(|t| p.replace(t)),
                    description: d.description.as_deref().map(|t| p.replace(t)),
                    published: d.published.clone(),
                    text: p.replace_prefix(&full(d), d.text.len()),
                    hash: d.hash.clone(),
                },
            }
        })
        .collect();
    let known = entities
        .iter()
        .map(|(ty, names)| (ty.clone(), names.iter().map(|n| p.replace(n)).collect()))
        .collect();
    (shown, known)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_costs_zero_without_a_model_call_the_sum_when_all_are_costed_and_none_otherwise() {
        assert_eq!(Report::default().cost_usd, Some(0.0));
        let costed = [Some(0.25), Some(0.5)]
            .into_iter()
            .fold(Report::default().cost_usd, add_cost);
        assert_eq!(costed, Some(0.75));
        let one_uncosted = [Some(0.25), None, Some(0.5)]
            .into_iter()
            .fold(Report::default().cost_usd, add_cost);
        assert_eq!(one_uncosted, None);
    }

    #[test]
    fn an_undo_whose_store_restore_is_refused_still_rewinds_state() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path().to_path_buf());
        std::fs::write(
            layout.spec(),
            "format: cortex.instance/1\nname: t\ndescription: d\nekr: {version: \"0.0.30\"}\n\
             seed: {documents: []}\nmodel: {model: m, budget_usd: \"1\", timeout_s: 60}\n\
             sources: []\nserve: {}\n",
        )
        .unwrap();
        let db = rusqlite::Connection::open(layout.store()).unwrap();
        db.execute_batch("CREATE TABLE t (v TEXT); INSERT INTO t VALUES ('before');")
            .unwrap();
        std::fs::create_dir_all(layout.seen("news").parent().unwrap()).unwrap();
        std::fs::write(layout.seen("news"), "seen before").unwrap();
        let store = crate::ekr::Store {
            bin: PathBuf::from("ekr-unused"),
            host: layout.host(),
            backend: crate::ekr::Backend::Sqlite,
            store: layout.store(),
            launch: None,
            stage: None,
        };
        let mut before = BeforeApply::new(&layout, &store, "news", 1000, 3);
        before.take().unwrap();
        let name = before.publish().unwrap().expect("a snapshot");
        db.execute_batch("INSERT INTO t VALUES ('the run');")
            .unwrap();
        std::fs::write(layout.seen("news"), "seen by the run").unwrap();

        // Another connection holds the write lock past the restore's retries.
        db.execute_batch("BEGIN IMMEDIATE").unwrap();
        let systemd = Systemd {
            systemctl: PathBuf::from("systemctl-unused"),
            unit_dir: tmp.path().join("no-units"),
            home_root: tmp.path().to_path_buf(),
            taken_over: Default::default(),
        };
        let (restored, why) = undo(&layout, "t", Some(&name), &systemd);
        assert_eq!(restored, None, "{why}");
        assert!(why.contains("the store still holds the run"), "{why}");
        assert_eq!(
            std::fs::read_to_string(layout.seen("news")).unwrap(),
            "seen before",
            "state/ is put back so the run's documents are fetched again: {why}"
        );
        db.execute_batch("ROLLBACK").unwrap();
        let rows: i64 = db
            .query_row("SELECT count(*) FROM t", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 2, "the store still holds the run");
    }
}
