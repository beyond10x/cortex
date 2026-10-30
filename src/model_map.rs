//! The spec file, read through the generated serde types (`generated/spec-types`), mapped onto
//! the generated model types (`generated/cortex-model`). Both sides are generated from `spec/`;
//! this module is the one place that connects them, and the compiler checks every field.

use cortex_model::ask as ma;
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
            backend: opt(&spec.model.backend).map(|b| match *b {
                s::CortexInstanceModelBackend::V0 => m::ModelBackend::Claude,
                s::CortexInstanceModelBackend::V1 => m::ModelBackend::Codex,
            }),
            budget_usd: opt(&spec.model.budget_usd).map(Decimal),
            daily_ceiling_usd: opt(&spec.model.daily_ceiling_usd).map(Decimal),
            budget_tokens: opt(&spec.model.budget_tokens)
                .map(|n| int(&n, "model.budget_tokens"))
                .transpose()?,
            daily_ceiling_tokens: opt(&spec.model.daily_ceiling_tokens)
                .map(|n| int(&n, "model.daily_ceiling_tokens"))
                .transpose()?,
            gateway: opt(&spec.model.gateway)
                .map(|g| -> Result<m::ModelGateway, MapError> {
                    Ok(m::ModelGateway {
                        base_url: g.base_url.clone(),
                        token_env: g.token_env.clone(),
                        max_context_tokens: opt(&g.max_context_tokens)
                            .map(|n| int(&n, "model.gateway.max_context_tokens"))
                            .transpose()?,
                        max_output_tokens: opt(&g.max_output_tokens)
                            .map(|n| int(&n, "model.gateway.max_output_tokens"))
                            .transpose()?,
                    })
                })
                .transpose()?,
            timeout_s: int(&spec.model.timeout_s, "model.timeout_s")?,
            instructions: opt(&spec.model.instructions),
            batch_chars: opt(&spec.model.batch_chars)
                .map(|n| int(&n, "model.batch_chars"))
                .transpose()?,
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
        store: opt(&spec.store).map(|store| match *store {
            s::CortexInstanceStoreSpec::V0(p) => m::StoreSpec::Postgres(m::PostgresStore {
                config: p.value.config.clone(),
                connection: opt(&p.value.connection).map(|c| connection_ref(&c)),
                schema_connection: opt(&p.value.schema_connection).map(|c| connection_ref(&c)),
            }),
            s::CortexInstanceStoreSpec::V1(q) => {
                m::StoreSpec::Sqlite(opt(&q.value).map(|v| m::SqliteStore { path: opt(&v.path) }))
            }
        }),
        redaction: match &spec.redaction {
            EssPresence::Absent => None,
            EssPresence::Present(r) => Some(redaction(r)?),
        },
        snapshots: match &spec.snapshots {
            EssPresence::Absent => None,
            EssPresence::Present(p) => Some(m::SnapshotPolicy {
                keep: int(&p.keep, "snapshots.keep")?,
            }),
        },
        gate: opt(&spec.gate).map(|g| m::RunGate {
            checks: g
                .checks
                .iter()
                .map(|c| m::GateCheck {
                    measure: c.measure.clone(),
                    min: opt(&c.min).map(Decimal),
                    max: opt(&c.max).map(Decimal),
                })
                .collect(),
        }),
        monitoring: match &spec.monitoring {
            EssPresence::Absent => None,
            EssPresence::Present(o) => Some(m::MonitoringSpec {
                metrics: opt(&o.metrics),
                hook: match &o.hook {
                    EssPresence::Absent => None,
                    EssPresence::Present(h) => Some(m::RunHook {
                        command: h.command.clone(),
                        timeout_s: match &h.timeout_s {
                            EssPresence::Absent => None,
                            EssPresence::Present(t) => Some(int(t, "monitoring.hook.timeout_s")?),
                        },
                    }),
                },
            }),
        },
        ask: match &spec.ask {
            EssPresence::Absent => None,
            EssPresence::Present(a) => Some(ask_spec(a)?),
        },
    })
}

fn ask_spec(a: &s::CortexAskAskSpec) -> Result<ma::AskSpec, MapError> {
    Ok(ma::AskSpec {
        listen: a.listen.clone(),
        identity_header: a.identity_header.clone(),
        limits: ma::AskLimits {
            max_steps: int(&a.limits.max_steps, "ask.limits.max_steps")?,
            max_result_bytes: int(&a.limits.max_result_bytes, "ask.limits.max_result_bytes")?,
            max_seconds: int(&a.limits.max_seconds, "ask.limits.max_seconds")?,
            per_identity_per_minute: int(
                &a.limits.per_identity_per_minute,
                "ask.limits.per_identity_per_minute",
            )?,
            max_tool_calls: match &a.limits.max_tool_calls {
                EssPresence::Absent => None,
                EssPresence::Present(n) => Some(int(n, "ask.limits.max_tool_calls")?),
            },
        },
        link_templates: match &a.link_templates {
            EssPresence::Absent => None,
            EssPresence::Present(l) => Some(ma::LinkTemplates {
                node: opt(&l.node),
                evidence: opt(&l.evidence),
            }),
        },
        guidance: opt(&a.guidance),
        retention_days: match &a.retention_days {
            EssPresence::Absent => None,
            EssPresence::Present(d) => Some(int(d, "ask.retention_days")?),
        },
        slow_after_ms: match &a.slow_after_ms {
            EssPresence::Absent => None,
            EssPresence::Present(t) => Some(int(t, "ask.slow_after_ms")?),
        },
        records: match &a.records {
            EssPresence::Absent => None,
            EssPresence::Present(r) => Some(ma::PostgresRecords {
                url: r.url.clone(),
                schema: r.schema.clone(),
                ca_file: opt(&r.ca_file),
                connection: connection_ref(&r.connection),
            }),
        },
        max_in_flight: match &a.max_in_flight {
            EssPresence::Absent => None,
            EssPresence::Present(n) => Some(int(n, "ask.max_in_flight")?),
        },
    })
}

/// An answer's event as the stream carries it: the generated wire form of `cortex.ask.AskEvent`,
/// `{"event": <name>, "value": <fields>}`.
pub fn ask_event(event: &ma::AskEvent) -> s::CortexAskAskEvent {
    let n = serde_json::Number::from;
    match event {
        ma::AskEvent::Delta(d) => s::CortexAskAskEvent::V0(s::CortexAskAskEventVariant0 {
            event: s::CortexAskAskEventVariant0Event::V0,
            value: Box::new(s::CortexAskAskDelta {
                text: d.text.clone(),
            }),
        }),
        ma::AskEvent::Discard(d) => s::CortexAskAskEvent::V1(s::CortexAskAskEventVariant1 {
            event: s::CortexAskAskEventVariant1Event::V0,
            value: Box::new(s::CortexAskAskDiscard { step: n(d.step) }),
        }),
        ma::AskEvent::Done(d) => s::CortexAskAskEvent::V2(s::CortexAskAskEventVariant2 {
            event: s::CortexAskAskEventVariant2Event::V0,
            value: Box::new(s::CortexAskAskDone {
                answer: d.answer.clone(),
                answer_id: d.answer_id.clone(),
                cached_tokens: n(d.cached_tokens),
                context_tokens: n(d.context_tokens),
                evidence: d.evidence.clone(),
                input_tokens: n(d.input_tokens),
                nodes: d.nodes.clone(),
                output_tokens: n(d.output_tokens),
                steps: n(d.steps),
                unrestored: n(d.unrestored),
            }),
        }),
        ma::AskEvent::Error(e) => s::CortexAskAskEvent::V3(s::CortexAskAskEventVariant3 {
            event: s::CortexAskAskEventVariant3Event::V0,
            value: Box::new(s::CortexAskAskError {
                message: e.message.clone(),
                reason: e.reason.clone(),
                retry_after: match e.retry_after {
                    None => EssPresence::Absent,
                    Some(r) => EssPresence::Present(n(r)),
                },
            }),
        }),
        ma::AskEvent::Status(st) => s::CortexAskAskEvent::V4(s::CortexAskAskEventVariant4 {
            event: s::CortexAskAskEventVariant4Event::V0,
            value: Box::new(s::CortexAskAskStatus {
                format: st.format.clone(),
                message: st.message.clone(),
                step: n(st.step),
            }),
        }),
        ma::AskEvent::Tool(t) => s::CortexAskAskEvent::V5(s::CortexAskAskEventVariant5 {
            event: s::CortexAskAskEventVariant5Event::V0,
            value: Box::new(s::CortexAskAskTool {
                arguments: t.arguments.clone(),
                failed: t.failed,
                name: t.name.clone(),
                result_bytes: n(t.result_bytes),
                step: n(t.step),
                truncated: t.truncated,
                withheld: t.withheld,
            }),
        }),
    }
}

fn connection_ref(c: &s::CortexInstanceConnectionRef) -> m::ConnectionRef {
    m::ConnectionRef {
        adapter: c.adapter.clone(),
        connection: opt(&c.connection),
        profile: opt(&c.profile),
    }
}

fn redaction_class(class: &s::CortexInstanceRedactionClass) -> m::RedactionClass {
    match class {
        s::CortexInstanceRedactionClass::V0 => m::RedactionClass::Credential,
        s::CortexInstanceRedactionClass::V1 => m::RedactionClass::Email,
        s::CortexInstanceRedactionClass::V2 => m::RedactionClass::IpAddress,
        s::CortexInstanceRedactionClass::V3 => m::RedactionClass::PaymentCard,
        s::CortexInstanceRedactionClass::V4 => m::RedactionClass::Phone,
        s::CortexInstanceRedactionClass::V5 => m::RedactionClass::RareName,
        s::CortexInstanceRedactionClass::V6 => m::RedactionClass::Url,
    }
}

fn redaction(r: &s::CortexInstanceRedactionPolicy) -> Result<m::RedactionPolicy, MapError> {
    Ok(m::RedactionPolicy {
        classes: r.classes.iter().map(|c| redaction_class(c)).collect(),
        rules: r
            .rules
            .iter()
            .map(|rule| m::RedactionRule {
                name: rule.name.clone(),
                pattern: rule.pattern.clone(),
                replacement: rule.replacement.clone(),
            })
            .collect(),
        known_names: opt(&r.known_names),
        rare_limit: match &r.rare_limit {
            EssPresence::Absent => None,
            EssPresence::Present(n) => Some(int(n, "redaction.rare_limit")?),
        },
        refuse_if_left: opt(&r.refuse_if_left)
            .map(|classes| classes.iter().map(|c| redaction_class(c)).collect()),
    })
}

/// Why a paging other than a `connectors` source's own may not take `stop_older_than_since`.
const STOP_ONLY_ON_SOURCE: &str = "only a connectors source's own paging takes it";

/// A paging as the model holds it. `no_stop` is why it may not take `stop_older_than_since: true`,
/// when it may not: only a `connectors` source's own paging with a `time` does (the invariants on
/// `cortex.instance.Paging`'s holders).
fn paging(
    p: &EssPresence<Box<s::CortexInstancePaging>>,
    at: &str,
    no_stop: Option<&str>,
) -> Result<Option<m::Paging>, MapError> {
    Ok(match p {
        EssPresence::Absent => None,
        EssPresence::Present(p) => Some(m::Paging {
            stop_older_than_since: match (opt(&p.stop_older_than_since), no_stop) {
                (Some(true), Some(why)) => {
                    return Err(MapError(format!(
                        "{at}.paging.stop_older_than_since: {why}"
                    )))
                }
                (stop, _) => stop,
            },
            style: match *p.style {
                s::CortexInstancePageStyle::V0 => m::PageStyle::Keyset,
                s::CortexInstancePageStyle::V1 => m::PageStyle::NextLink,
                s::CortexInstancePageStyle::V2 => m::PageStyle::PageNumber,
                s::CortexInstancePageStyle::V3 => m::PageStyle::Token,
            },
            param: p.param.clone(),
            next: opt(&p.next),
            max_pages: int(&p.max_pages, &format!("{at}.paging.max_pages"))?,
        }),
    })
}

fn file_records(r: &s::CortexInstanceFileRecords) -> m::FileRecords {
    m::FileRecords {
        format: match *r.format {
            s::CortexInstanceRecordFormat::V0 => m::RecordFormat::JsonLines,
            s::CortexInstanceRecordFormat::V1 => m::RecordFormat::MarkdownSections,
            s::CortexInstanceRecordFormat::V2 => m::RecordFormat::WholeFile,
        },
        id: r.id.clone(),
        time: opt(&r.time),
        author: opt(&r.author),
        text: r.text.clone(),
        fallback_text: opt(&r.fallback_text),
        thread: opt(&r.thread),
        lookup: opt(&r.lookup),
        filters: r
            .filters
            .iter()
            .map(|f| m::RecordFilter {
                field: f.field.clone(),
                values: f.values.clone(),
                include: f.include,
            })
            .collect(),
    }
}

fn record_mapping(mapping: &s::CortexInstanceRecordMapping) -> m::RecordMapping {
    m::RecordMapping {
        node_type: mapping.node_type.clone(),
        id: mapping.id.clone(),
        name: mapping.name.clone(),
        aliases: mapping.aliases.clone(),
        properties: mapping
            .properties
            .iter()
            .map(|p| m::PropertyMapping {
                property: p.property.clone(),
                path: p.path.clone(),
                value_kind: opt(&p.value_kind).map(|k| match *k {
                    s::CortexInstanceSchemaValueKind::V0 => m::SchemaValueKind::Boolean,
                    s::CortexInstanceSchemaValueKind::V1 => m::SchemaValueKind::Decimal,
                    s::CortexInstanceSchemaValueKind::V2 => m::SchemaValueKind::Integer,
                    s::CortexInstanceSchemaValueKind::V3 => m::SchemaValueKind::String,
                    s::CortexInstanceSchemaValueKind::V4 => m::SchemaValueKind::Timestamp,
                }),
            })
            .collect(),
        relations: mapping
            .relations
            .iter()
            .map(|r| m::RelationMapping {
                relation: r.relation.clone(),
                target_type: r.target_type.clone(),
                target_name: r.target_name.clone(),
            })
            .collect(),
        alias_templates: opt(&mapping.alias_templates),
        store_lookup: opt(&mapping.store_lookup).map(|l| match *l {
            s::CortexInstanceStoreLookup::V0 => m::StoreLookup::AnyType,
            s::CortexInstanceStoreLookup::V1 => m::StoreLookup::OwnType,
        }),
    }
}

fn structured_child(
    child: &s::CortexInstanceStructuredChild,
    at: &str,
) -> Result<m::StructuredChild, MapError> {
    Ok(m::StructuredChild {
        operation: child.operation.clone(),
        input: json(&child.input),
        records: child.records.clone(),
        paging: paging(&child.paging, at, Some(STOP_ONLY_ON_SOURCE))?,
        mapping: record_mapping(&child.mapping),
        time: opt(&child.time),
        parent: child.parent.clone(),
        follow: follow(&child.follow, &format!("{at}.follow"))?,
    })
}

/// A child's `follow` rule; `days` and `number` above 0, as the rule's invariants declare.
fn follow(
    rule: &EssPresence<Box<s::CortexInstanceFollowRule>>,
    at: &str,
) -> Result<Option<m::FollowRule>, MapError> {
    let EssPresence::Present(rule) = rule else {
        return Ok(None);
    };
    let positive = |n: &serde_json::Number, name: &str| {
        let n = int(n, at)?;
        if n > 0 {
            Ok(n)
        } else {
            Err(MapError(format!(
                "{at}: {name} {n}: use a whole number above 0"
            )))
        }
    };
    Ok(Some(match &**rule {
        s::CortexInstanceFollowRule::V0(r) => m::FollowRule::AtLeast(m::FollowAtLeast {
            field: r.value.field.clone(),
            number: positive(&r.value.number, "number")?,
        }),
        s::CortexInstanceFollowRule::V1(r) => m::FollowRule::Equals(m::FollowEquals {
            field: r.value.field.clone(),
            value: json(&r.value.value),
        }),
        s::CortexInstanceFollowRule::V2(r) => m::FollowRule::WithinDays(m::FollowWithinDays {
            field: r.value.field.clone(),
            days: positive(&r.value.days, "days")?,
        }),
    }))
}

fn compare_link(link: &s::CortexInstanceCompareLink, at: &str) -> Result<m::CompareLink, MapError> {
    Ok(m::CompareLink {
        operation: link.operation.clone(),
        input: json(&link.input),
        records: link.records.clone(),
        paging: paging(&link.paging, at, Some(STOP_ONLY_ON_SOURCE))?,
        tags: link.tags.clone(),
        changes: link.changes.clone(),
        order: link.order.clone(),
        change_id: link.change_id.clone(),
        relation: link.relation.clone(),
    })
}

fn structured(
    st: &s::CortexInstanceStructuredSource,
    at: &str,
) -> Result<m::StructuredSource, MapError> {
    Ok(m::StructuredSource {
        input: match &*st.input {
            s::CortexInstanceStructuredInput::V0(c) => {
                m::StructuredInput::Connectors(m::StructuredConnectors {
                    adapter: c.value.adapter.clone(),
                    connection: opt(&c.value.connection),
                    profile: opt(&c.value.profile),
                    operation: c.value.operation.clone(),
                    inputs: c.value.inputs.iter().map(json).collect(),
                    paging: paging(&c.value.paging, at, Some(STOP_ONLY_ON_SOURCE))?,
                    children: match &c.value.children {
                        EssPresence::Absent => None,
                        EssPresence::Present(children) => Some(
                            children
                                .iter()
                                .enumerate()
                                .map(|(n, child)| {
                                    structured_child(child, &format!("{at}.children[{n}]"))
                                })
                                .collect::<Result<_, _>>()?,
                        ),
                    },
                    links: match &c.value.links {
                        EssPresence::Absent => None,
                        EssPresence::Present(links) => Some(
                            links
                                .iter()
                                .enumerate()
                                .map(|(n, link)| compare_link(link, &format!("{at}.links[{n}]")))
                                .collect::<Result<_, _>>()?,
                        ),
                    },
                })
            }
            s::CortexInstanceStructuredInput::V1(f) => {
                m::StructuredInput::Files(m::StructuredFiles {
                    paths: f.value.paths.clone(),
                    glob: f.value.glob.clone(),
                })
            }
        },
        records: st.records.clone(),
        mapping: record_mapping(&st.mapping),
        dropped: opt(&st.dropped).map(|d| match *d {
            s::CortexInstanceDropPolicy::V0 => m::DropPolicy::Keep,
            s::CortexInstanceDropPolicy::V1 => m::DropPolicy::Supersede,
        }),
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
                connection: opt(&c.value.connection),
                profile: opt(&c.value.profile),
                operation: c.value.operation.clone(),
                inputs: c.value.inputs.iter().map(json).collect(),
                records: c.value.records.clone(),
                id: c.value.id.clone(),
                key: opt(&c.value.key),
                time: opt(&c.value.time),
                text: c.value.text.clone(),
                paging: paging(
                    &c.value.paging,
                    at,
                    matches!(c.value.time, EssPresence::Absent)
                        .then_some("it reads each record's time, and the source names no time"),
                )?,
                child: match &c.value.child {
                    EssPresence::Absent => None,
                    EssPresence::Present(child) => Some(m::ChildCall {
                        operation: child.operation.clone(),
                        input: json(&child.input),
                        records: child.records.clone(),
                        paging: paging(
                            &child.paging,
                            &format!("{at}.child"),
                            Some(STOP_ONLY_ON_SOURCE),
                        )?,
                    }),
                },
            })
        }
        s::CortexInstanceSourceSettings::V1(f) => m::SourceSettings::Files(m::FilesSource {
            paths: f.value.paths.clone(),
            glob: f.value.glob.clone(),
            records: opt(&f.value.records).map(|r| file_records(&r)),
        }),
        s::CortexInstanceSourceSettings::V2(st) => {
            m::SourceSettings::Structured(structured(&st.value, at)?)
        }
        s::CortexInstanceSourceSettings::V3(w) => m::SourceSettings::Web(m::WebSource {
            adapter: opt(&w.value.adapter),
            connection: opt(&w.value.connection),
            profile: opt(&w.value.profile),
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
        m::SourceSettings::Structured(_) => m::SourceKind::Structured,
    }
}
