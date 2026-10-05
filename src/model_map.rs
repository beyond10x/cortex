//! The spec file, read through the generated serde types (`generated/spec-types`), mapped onto
//! the generated model types (`generated/cortex-model`). Both sides are generated from `spec/`;
//! this module is the one place that connects them, and the compiler checks every field.

use cortex_model::instance as m;
use cortex_model::json as mj;
use cortex_model::primitives::Decimal;
use cortex_spec_types as s;
use s::EssPresence;

/// A spec value the model cannot hold, such as a negative or fractional count.
#[derive(Debug)]
pub struct MapError(pub String);

impl std::fmt::Display for MapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

fn int(n: &serde_json::Number, at: &str) -> Result<i64, MapError> {
    n.as_i64()
        .ok_or_else(|| MapError(format!("{at}: {n} is not a whole number")))
}

fn opt<T: Clone>(p: &EssPresence<T>) -> Option<T> {
    match p {
        EssPresence::Absent => None,
        EssPresence::Present(v) => Some(v.clone()),
    }
}

fn json(v: &serde_json::Value) -> mj::Value {
    match v {
        serde_json::Value::Null => mj::Value::Null,
        serde_json::Value::Bool(b) => mj::Value::Bool(*b),
        serde_json::Value::Number(n) => mj::Value::Number(n.to_string()),
        serde_json::Value::String(t) => mj::Value::Text(t.clone()),
        serde_json::Value::Array(a) => mj::Value::Array(a.iter().map(json).collect()),
        serde_json::Value::Object(o) => {
            mj::Value::Object(o.iter().map(|(k, v)| (k.clone(), json(v))).collect())
        }
    }
}

/// The model's JSON value as a `serde_json::Value`, for building an operation input.
pub fn to_serde(v: &mj::Value) -> serde_json::Value {
    match v {
        mj::Value::Null => serde_json::Value::Null,
        mj::Value::Bool(b) => serde_json::Value::Bool(*b),
        mj::Value::Number(n) => serde_json::from_str(n).unwrap_or(serde_json::Value::Null),
        mj::Value::Text(t) => serde_json::Value::String(t.clone()),
        mj::Value::Array(a) => serde_json::Value::Array(a.iter().map(to_serde).collect()),
        mj::Value::Object(o) => {
            serde_json::Value::Object(o.iter().map(|(k, v)| (k.clone(), to_serde(v))).collect())
        }
    }
}

pub fn instance_spec(spec: &s::CortexInstanceInstanceSpec) -> Result<m::InstanceSpec, MapError> {
    Ok(m::InstanceSpec {
        format: spec.format.clone(),
        name: m::InstanceName(spec.name.0.clone()),
        description: spec.description.clone(),
        ekr: m::EkrPin {
            version: spec.ekr.version.clone(),
            bin: opt(&spec.ekr.bin),
        },
        seed: m::SeedSpec {
            schema: opt(&spec.seed.schema),
            ekr_seed: opt(&spec.seed.ekr_seed),
            documents: spec.seed.documents.clone(),
        },
        model: m::ModelSpec {
            model: spec.model.model.clone(),
            budget_usd: Decimal(spec.model.budget_usd.clone()),
            timeout_s: int(&spec.model.timeout_s, "model.timeout_s")?,
            instructions: opt(&spec.model.instructions),
        },
        sources: spec
            .sources
            .iter()
            .map(|source| source_spec(source))
            .collect::<Result<_, _>>()?,
        serve: m::ServeSpec {
            view_port: match &spec.serve.view_port {
                EssPresence::Absent => None,
                EssPresence::Present(n) => Some(int(n, "serve.view_port")?),
            },
        },
    })
}

fn source_spec(source: &s::CortexInstanceSourceSpec) -> Result<m::SourceSpec, MapError> {
    let at = format!("sources.{}", source.name);
    Ok(m::SourceSpec {
        name: source.name.clone(),
        schedule: source.schedule.clone(),
        settings: settings(&source.settings, &at)?,
        policy: m::FetchPolicy {
            refresh_after_days: int(&source.policy.refresh_after_days, &at)?,
            change: match *source.policy.change {
                s::CortexInstanceChangeDetection::V0 => m::ChangeDetection::ContentHash,
            },
            max_documents_per_run: int(&source.policy.max_documents_per_run, &at)?,
            max_chars_per_document: int(&source.policy.max_chars_per_document, &at)?,
        },
    })
}

fn settings(
    settings: &s::CortexInstanceSourceSettings,
    at: &str,
) -> Result<m::SourceSettings, MapError> {
    Ok(match settings {
        s::CortexInstanceSourceSettings::V0(c) => {
            m::SourceSettings::Connectors(m::ConnectorsSource {
                adapter: c.value.adapter.clone(),
                connection: c.value.connection.clone(),
                operation: c.value.operation.clone(),
                inputs: c.value.inputs.iter().map(json).collect(),
                records: c.value.records.clone(),
                id: c.value.id.clone(),
                time: opt(&c.value.time),
                text: c.value.text.clone(),
            })
        }
        s::CortexInstanceSourceSettings::V1(f) => m::SourceSettings::Files(m::FilesSource {
            paths: f.value.paths.clone(),
            glob: f.value.glob.clone(),
        }),
        s::CortexInstanceSourceSettings::V2(w) => m::SourceSettings::Web(m::WebSource {
            adapter: opt(&w.value.adapter),
            connection: w.value.connection.clone(),
            input: web_input(&w.value.input, at)?,
        }),
    })
}

fn web_input(input: &s::CortexInstanceWebInput, at: &str) -> Result<m::WebInput, MapError> {
    Ok(match input {
        s::CortexInstanceWebInput::V0(search) => {
            let p = &search.value.policy;
            m::WebInput::Search(m::SearchInput {
                queries: search.value.queries.clone(),
                policy: m::SearchPolicy {
                    topic: match *p.topic {
                        s::CortexInstanceSearchTopic::V0 => m::SearchTopic::General,
                        s::CortexInstanceSearchTopic::V1 => m::SearchTopic::News,
                    },
                    time_range: opt(&p.time_range).map(|t| match *t {
                        s::CortexInstanceTimeRange::V0 => m::TimeRange::Day,
                        s::CortexInstanceTimeRange::V1 => m::TimeRange::Month,
                        s::CortexInstanceTimeRange::V2 => m::TimeRange::Week,
                        s::CortexInstanceTimeRange::V3 => m::TimeRange::Year,
                    }),
                    max_results: int(&p.max_results, at)?,
                    include_domains: p.include_domains.clone(),
                    exclude_domains: p.exclude_domains.clone(),
                    country: opt(&p.country),
                    language: opt(&p.language),
                },
            })
        }
        s::CortexInstanceWebInput::V1(sites) => m::WebInput::Sites(m::SitesInput {
            urls: sites.value.urls.clone(),
            mode: match *sites.value.mode {
                s::CortexInstanceWebMode::V0 => m::WebMode::Crawl,
                s::CortexInstanceWebMode::V1 => m::WebMode::Pages,
            },
            policy: match &sites.value.policy {
                EssPresence::Absent => None,
                EssPresence::Present(c) => Some(m::CrawlPolicy {
                    max_depth: int(&c.max_depth, at)?,
                    max_breadth: int(&c.max_breadth, at)?,
                    limit: int(&c.limit, at)?,
                    select_paths: c.select_paths.clone(),
                    exclude_paths: c.exclude_paths.clone(),
                    instructions: opt(&c.instructions),
                    allow_external: c.allow_external,
                }),
            },
        }),
    })
}

/// The kind a source's settings declare.
pub fn kind(settings: &m::SourceSettings) -> m::SourceKind {
    match settings {
        m::SourceSettings::Web(_) => m::SourceKind::Web,
        m::SourceSettings::Connectors(_) => m::SourceKind::Connectors,
        m::SourceSettings::Files(_) => m::SourceKind::Files,
    }
}
