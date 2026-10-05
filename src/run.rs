//! `RunSource`, the obligation `generated/cortex-model/PLAN.md` leaves to the implementation:
//! fetch, keep what is new or changed, extract in batches within the run's budget, apply.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

use cortex_model::instance as m;
use serde_json::json;

use crate::connectors::Connectors;
use crate::evidence;
use crate::extract::{self, Known, Model};
use crate::instance::{now_ms, Layout};
use crate::mask::mask;
use crate::redact::{self, Redactor};
use crate::sources::{self, FetchError, Window};
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
}

#[derive(Debug)]
pub struct Report {
    pub documents_new: i64,
    pub documents_applied: i64,
    /// `0` when no model was asked, the sum when every answer carried a cost, `None` as soon as
    /// one answer carried none: a missing cost is never written as `0`.
    pub cost_usd: Option<f64>,
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
    /// [`sources::CHILD_FAILURE_LIMIT`] or more consecutive runs.
    pub skipped: Vec<String>,
}

impl Default for Report {
    /// A run that asked no model: it cost `0`.
    fn default() -> Self {
        Self {
            documents_new: 0,
            documents_applied: 0,
            cost_usd: Some(0.0),
            facts_refused: 0,
            parts_rejected: 0,
            masked: 0,
            redacted: None,
            unrestored: 0,
            stopped: None,
            truncated: 0,
            skipped: Vec::new(),
        }
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

fn load_entities(layout: &Layout) -> BTreeMap<String, Vec<String>> {
    std::fs::read(layout.entities())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn run(layout: &Layout, tools: &Tools, source: &m::SourceData) -> Result<Report, Failure> {
    let spec = layout.load_spec().map_err(Failure::Fetch)?;
    let meta = layout.load_meta().map_err(Failure::Fetch)?;
    let source_spec = spec
        .sources
        .iter()
        .find(|s| s.name == source.name)
        .ok_or_else(|| Failure::Fetch(format!("the spec has no source {:?}", source.name)))?;
    // The window a `connectors` source asks about: from just before the start of its last
    // successful run, its held-back changes or a run that did not succeed, to the start of this
    // one.
    let start = now_ms();
    let seen_path = layout.seen(&source.name);
    let state = SeenState::load(&seen_path).map_err(Failure::Fetch)?;
    let window = Window::of_run(
        start,
        state.last_success_started_at,
        state.held_since,
        state.pending_since,
        source_spec.policy.refresh_after_days,
    );
    let ran = sources::fetch(
        &source_spec.settings,
        &tools.connectors,
        &meta.spec_dir,
        &window,
        &state.child_failures,
    )
    .map_err(|e| match e {
        FetchError::Missing(m) | FetchError::Failed(m) => Failure::Fetch(m),
    })
    .and_then(|fetched| {
        let structured_source = match &source_spec.settings {
            m::SourceSettings::Structured(st) => Some(st),
            _ => None,
        };
        process(
            layout,
            tools,
            &spec,
            &source.name,
            fetched,
            &source_spec.policy,
            Some(window),
            structured_source,
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
/// the same pipeline a source run uses, recorded as the pseudo-source `seed`.
pub fn seed(layout: &Layout, tools: &Tools) -> Result<Report, Failure> {
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
    let fetched = sources::fetch(
        &files,
        &tools.connectors,
        &layout.dir,
        &window,
        &BTreeMap::new(),
    )
    .map_err(|e| match e {
        FetchError::Missing(m) | FetchError::Failed(m) => Failure::Fetch(m),
    })?;
    let policy = m::FetchPolicy {
        refresh_after_days: 0,
        change: m::ChangeDetection::ContentHash,
        max_documents_per_run: i64::MAX,
        max_chars_per_document: 100_000,
    };
    process(layout, tools, &spec, "seed", fetched, &policy, None, None)
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
) -> Result<Report, Failure> {
    let started = now_ms();
    let applied_at = window.map_or(started, |w| w.until_ms);
    let mut report = Report {
        skipped: fetched.skipped.clone(),
        ..Report::default()
    };
    // The keys of documents cut at `max_chars_per_document`: counted as they reach the model.
    let mut cut: HashSet<String> = HashSet::new();
    let redactor = redact::compile(spec.redaction.as_ref()).map_err(Failure::Extract)?;
    report.redacted = redactor.as_ref().map(Redactor::counts);
    // Each document's text before the cut, so a value the cut splits is still found, with what
    // the irreversible rules replaced in it.
    let mut uncut: HashMap<String, Vec<Uncut>> = HashMap::new();
    // A record's matches of the irreversible rules, counted when the record is applied.
    let mut record_scrubbed: HashMap<String, BTreeMap<String, usize>> = HashMap::new();
    let structured = structured_source.map(|st| structured::Source::new(st, redactor.as_ref()));
    let docs: Vec<_> = fetched
        .documents
        .into_iter()
        .map(|mut d| {
            if let Some(s) = &structured {
                let (n, scrubbed) = s.prepare(&mut d);
                report.masked += n;
                d.hash = Some(text_hash(&d.text));
                record_scrubbed.insert(d.key.clone(), scrubbed);
                return d;
            }
            let (masked, n) = mask(&d.text);
            report.masked += n;
            d.text = masked;
            let Some(r) = &redactor else {
                d.hash = Some(text_hash(&d.text));
                if truncate(&mut d.text, policy.max_chars_per_document as usize) {
                    cut.insert(d.key.clone());
                }
                return d;
            };
            // Irreversible rules, like masking, run before anything is stored.
            let mut scrubbed: BTreeMap<String, usize> = BTreeMap::new();
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
            if truncate(&mut d.text, policy.max_chars_per_document as usize) {
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
            d
        })
        .collect();

    let seen_path = layout.seen(label);
    let mut seen = SeenState::load(&seen_path).map_err(Failure::Fetch)?;
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
    let selected = seen.select(
        docs,
        started,
        policy.refresh_after_days,
        policy.max_documents_per_run as usize,
    );
    report.documents_new = selected.len() as i64;
    let run_dir = layout.runs().join(format!("{started}-{}", label));
    if selected.is_empty() {
        unread(&mut report, &fetched.unread);
        let failures = fetched.child_failures.clone();
        finish(
            &mut seen, &seen_path, &report, wanted, held_since, window, failures,
        )?;
        log(layout, label, &report, started);
        return Ok(report);
    }

    let store = layout.store_handle(spec);
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
        };
        apply_records(
            layout,
            &records,
            selected,
            &mut seen,
            &seen_path,
            &mut report,
        )?;
        unread(&mut report, &fetched.unread);
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
    };
    let instructions = match &spec.model.instructions {
        Some(rel) => Some(
            std::fs::read_to_string(layout.dir.join(rel))
                .map_err(|e| Failure::Extract(format!("instructions {rel}: {e}")))?,
        ),
        None => None,
    };
    let budget: f64 = spec.model.budget_usd.0.parse().unwrap_or(0.0);
    // Dollars the costed answers spent. An answer with no cost spends none; such a run is bounded
    // by `policy.max_documents_per_run` and `model.timeout_s`.
    let mut spent = 0.0;
    let mut entities = load_entities(layout);

    let issued: Vec<_> = selected
        .into_iter()
        .map(|d| evidence::issue(d, &operator, started))
        .collect();
    for (n, batch) in batches(issued).into_iter().enumerate() {
        let remaining = budget - spent;
        if remaining <= 0.0 {
            report.stopped = Some(format!("budget of {budget} USD spent"));
            break;
        }
        let ontology = store.ontology().map_err(Failure::Apply)?;
        // The model is shown the batch with personal data replaced by placeholders; the mapping
        // stays in `pseudonyms`, in memory, until this batch's answer is restored.
        let (prompt, pseudonyms) = match &redactor {
            None => {
                let known = Known::from_ontology(&ontology, entities.clone());
                let prompt =
                    extract::prompt(&spec.description, instructions.as_deref(), &known, &batch);
                (prompt, None)
            }
            Some(r) => {
                let mut p = r.batch();
                let (shown, known) = pseudonymise(&mut p, &batch, &uncut, &entities);
                let known = Known::from_ontology(&ontology, known);
                let prompt =
                    extract::prompt(&spec.description, instructions.as_deref(), &known, &shown);
                if let Some(counts) = report.redacted.as_mut() {
                    for (name, n) in p.counts() {
                        *counts.entry(name.clone()).or_default() += n;
                    }
                }
                (prompt, Some(p))
            }
        };
        let dir = run_dir.join(format!("batch-{n}"));
        let answer = match model.ask(&dir.join("claude"), &schema, &prompt, remaining) {
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
        let applied = match store.apply(&path) {
            Ok(r) => r,
            Err(e) if report.documents_applied == 0 => return Err(Failure::Apply(e)),
            Err(e) => {
                report.stopped = Some(e);
                break;
            }
        };
        if !pseudonymised {
            let _ = std::fs::write(
                dir.join("report.json"),
                serde_json::to_string_pretty(&applied).unwrap_or_default(),
            );
        }
        report.parts_rejected += crate::ekr::applied(&applied).rejected;
        for issued in &batch {
            seen.record(&issued.doc, applied_at);
        }
        for (ty, name) in extract::entity_names(&restored) {
            let names = entities.entry(ty).or_default();
            if !names.contains(&name) && names.len() < KNOWN_PER_TYPE {
                names.push(name);
            }
        }
        report.documents_applied += batch.len() as i64;
        seen.save(&seen_path).map_err(Failure::Apply)?;
        let _ = crate::home::write_atomic(
            &layout.entities(),
            serde_json::to_string_pretty(&entities)
                .unwrap_or_default()
                .as_bytes(),
        );
    }
    unread(&mut report, &fetched.unread);
    let failures = fetched.child_failures.clone();
    finish(
        &mut seen, &seen_path, &report, wanted, held_since, window, failures,
    )?;
    log(layout, label, &report, started);
    Ok(report)
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
}

/// Applies the `selected` records of a structured source with no model call: each batch is one
/// extraction document mapped by `src/structured.rs`, written to the run's batch directory as
/// `extraction.yaml` and applied; the report keeps its cost of `0`. A batch that fails to apply
/// fails the run when nothing was applied yet and stops it otherwise, as on the model path. A
/// record with a part EKR rejected is counted in `parts_rejected` but neither applied nor marked
/// seen, so the run is not successful and the record is read and tried again.
fn apply_records(
    layout: &Layout,
    r: &Records<'_>,
    selected: Vec<sources::Document>,
    seen: &mut SeenState,
    seen_path: &std::path::Path,
    report: &mut Report,
) -> Result<(), Failure> {
    let mut entities = load_entities(layout);
    let issued: Vec<_> = selected
        .into_iter()
        .map(|d| evidence::issue(d, r.operator, r.started))
        .collect();
    let index = r.source.index(&issued);
    for (n, batch) in batches(issued).into_iter().enumerate() {
        let mapped = r.source.document(&batch, &index);
        let (doc, refused) = extract::merge(&mapped.document, &batch);
        report.facts_refused += refused;
        let dir = r.run_dir.join(format!("batch-{n}"));
        std::fs::create_dir_all(&dir).map_err(|e| Failure::Apply(e.to_string()))?;
        let path = dir.join("extraction.yaml");
        let text = serde_yaml_ng::to_string(&doc).expect("YAML");
        std::fs::write(&path, text).map_err(|e| Failure::Apply(e.to_string()))?;
        let applied = match r.store.apply(&path) {
            Ok(applied) => applied,
            Err(e) if report.documents_applied == 0 => return Err(Failure::Apply(e)),
            Err(e) => {
                report.stopped = Some(e);
                break;
            }
        };
        let _ = std::fs::write(
            dir.join("report.json"),
            serde_json::to_string_pretty(&applied).unwrap_or_default(),
        );
        report.parts_rejected += crate::ekr::applied(&applied).rejected;
        let rejected = mapped.rejected(&applied);
        for issued in &batch {
            if rejected.contains(&issued.id) {
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

/// Adds to why the run stopped what its fetch left unread.
fn unread(report: &mut Report, unread: &[String]) {
    if unread.is_empty() {
        return;
    }
    let why = unread.join("; ");
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
    // Present only when a document was cut: a run that cut nothing logs what it did before.
    if report.truncated > 0 {
        line["truncated"] = json!(report.truncated);
    }
    // Present only when a parent was skipped: a run that skipped none logs what it did before.
    if !report.skipped.is_empty() {
        line["skipped"] = json!(report.skipped);
    }
    // Present only when the policy pseudonymises: a spec file without one logs what it did before.
    if let Some(redacted) = &report.redacted {
        line["redacted"] = json!(redacted);
        line["unrestored"] = json!(report.unrestored);
    }
    layout.log_line(&line);
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
        for text in [Some(&d.key), d.title.as_ref(), d.description.as_ref()]
            .into_iter()
            .flatten()
        {
            p.reserve(text);
        }
        p.reserve(&full(d));
    }
    for name in entities.values().flatten() {
        p.reserve(name);
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
}
