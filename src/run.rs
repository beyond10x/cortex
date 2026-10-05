//! `RunSource`, the obligation `generated/cortex-model/PLAN.md` leaves to the implementation:
//! fetch, keep what is new or changed, extract in batches within the run's budget, apply.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use cortex_model::instance as m;
use serde_json::json;

use crate::connectors::Connectors;
use crate::evidence;
use crate::extract::{self, Known, Model};
use crate::instance::{now_ms, Layout};
use crate::mask::mask;
use crate::redact::{self, Redactor};
use crate::sources::{self, FetchError};
use crate::state::SeenState;

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

fn truncate(text: &mut String, max: usize) {
    if let Some((i, _)) = text.char_indices().nth(max) {
        text.truncate(i);
    }
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
    let fetched = sources::fetch(&source_spec.settings, &tools.connectors, &meta.spec_dir)
        .map_err(|e| match e {
            FetchError::Missing(m) | FetchError::Failed(m) => Failure::Fetch(m),
        })?;
    process(
        layout,
        tools,
        &spec,
        &source.name,
        fetched,
        &source_spec.policy,
    )
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
    let fetched = sources::fetch(&files, &tools.connectors, &layout.dir).map_err(|e| match e {
        FetchError::Missing(m) | FetchError::Failed(m) => Failure::Fetch(m),
    })?;
    let policy = m::FetchPolicy {
        refresh_after_days: 0,
        change: m::ChangeDetection::ContentHash,
        max_documents_per_run: i64::MAX,
        max_chars_per_document: 100_000,
    };
    process(layout, tools, &spec, "seed", fetched, &policy)
}

fn process(
    layout: &Layout,
    tools: &Tools,
    spec: &m::InstanceSpec,
    label: &str,
    fetched: Vec<sources::Document>,
    policy: &m::FetchPolicy,
) -> Result<Report, Failure> {
    let started = now_ms();
    let mut report = Report::default();
    let redactor = redact::compile(spec.redaction.as_ref()).map_err(Failure::Extract)?;
    report.redacted = redactor.as_ref().map(Redactor::counts);
    // Each document's text before the cut, so a value the cut splits is still found, with what
    // the irreversible rules replaced in it.
    let mut uncut: HashMap<String, Vec<Uncut>> = HashMap::new();
    let docs: Vec<_> = fetched
        .into_iter()
        .map(|mut d| {
            let (masked, n) = mask(&d.text);
            report.masked += n;
            d.text = masked;
            let Some(r) = &redactor else {
                truncate(&mut d.text, policy.max_chars_per_document as usize);
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
            truncate(&mut d.text, policy.max_chars_per_document as usize);
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
    let selected = seen.select(
        docs,
        started,
        policy.refresh_after_days,
        policy.max_documents_per_run as usize,
    );
    report.documents_new = selected.len() as i64;
    let run_dir = layout.runs().join(format!("{started}-{}", label));
    if selected.is_empty() {
        log(layout, label, &report, started);
        return Ok(report);
    }

    let store = layout.store_handle(spec);
    let host = std::fs::read_to_string(layout.host()).map_err(|e| Failure::Apply(e.to_string()))?;
    let operator = crate::ekr::operator(&host).map_err(Failure::Apply)?;
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

    for (n, batch) in batches.into_iter().enumerate() {
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
            seen.record(&issued.doc, started);
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
    log(layout, label, &report, started);
    Ok(report)
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
