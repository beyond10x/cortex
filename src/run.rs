//! `RunSource`, the obligation `generated/cortex-model/PLAN.md` leaves to the implementation:
//! fetch, keep what is new or changed, extract in batches within the run's budget, apply.

use std::collections::BTreeMap;
use std::path::PathBuf;

use cortex_model::instance as m;
use serde_json::json;

use crate::connectors::Connectors;
use crate::evidence;
use crate::extract::{self, Known, Model};
use crate::instance::{now_ms, Layout};
use crate::mask::mask;
use crate::sources::{self, FetchError};
use crate::state::SeenState;

pub struct Tools {
    pub connectors: Connectors,
    pub claude: PathBuf,
}

/// Which declared failure outcome a run takes.
#[derive(Debug)]
pub enum Failure {
    Fetch(String),
    Extract(String),
    Apply(String),
}

#[derive(Debug, Default)]
pub struct Report {
    pub documents_new: i64,
    pub documents_applied: i64,
    pub cost_usd: f64,
    pub facts_refused: usize,
    /// Parts of applied documents EKR rejected (`rejected` in its report).
    pub parts_rejected: usize,
    pub masked: usize,
    /// Why a run stopped before applying every new document, when it did.
    pub stopped: Option<String>,
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
    let docs: Vec<_> = fetched
        .into_iter()
        .map(|mut d| {
            let (masked, n) = mask(&d.text);
            report.masked += n;
            d.text = masked;
            truncate(&mut d.text, policy.max_chars_per_document as usize);
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
        claude: tools.claude.clone(),
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
        let remaining = budget - report.cost_usd;
        if remaining <= 0.0 {
            report.stopped = Some(format!("budget of {budget} USD spent"));
            break;
        }
        let ontology = store.ontology().map_err(Failure::Apply)?;
        let known = Known::from_ontology(&ontology, entities.clone());
        let prompt = extract::prompt(&spec.description, instructions.as_deref(), &known, &batch);
        let dir = run_dir.join(format!("batch-{n}"));
        let answer = match model.ask(&dir.join("claude"), &schema, &prompt, remaining) {
            Ok(answer) => answer,
            Err(e) if report.documents_applied == 0 => return Err(Failure::Extract(e)),
            Err(e) => {
                report.stopped = Some(e);
                break;
            }
        };
        report.cost_usd += answer.cost_usd;
        let (doc, refused) = extract::merge(&answer.document, &batch);
        report.facts_refused += refused;
        let path = dir.join("extraction.yaml");
        let text = serde_yaml_ng::to_string(&doc).expect("YAML");
        let _ = std::fs::write(dir.join("prompt.txt"), &prompt);
        let _ = std::fs::write(
            dir.join("model.json"),
            serde_json::to_string_pretty(&answer.document).unwrap_or_default(),
        );
        std::fs::write(&path, text).map_err(|e| Failure::Apply(e.to_string()))?;
        let applied = match store.apply(&path) {
            Ok(r) => r,
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
        for issued in &batch {
            seen.record(&issued.doc, started);
        }
        for (ty, name) in extract::entity_names(&answer.document) {
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
    layout.log_line(&json!({
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
    }));
}
