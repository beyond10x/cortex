//! Reading an instance spec file (`cortex.instance/1`).

use std::path::{Path, PathBuf};

use cortex_model::instance as m;

use crate::connectors::Reference;
use crate::model_map;

pub const FORMAT: &str = "cortex.instance/1";

/// The `store` forms a spec may write, for the refusal of any other.
const STORE_FORMS: &str = "use `backend: sqlite` with an optional `value: {path}`, \
                           or `backend: postgres` with `value: {config}`";

/// The schema's `Decimal` pattern (`website/static/schemas/instance-spec.schema.json`).
const DECIMAL: &str = r"^-?(0|[1-9][0-9]*)(\.[0-9]+)?$";

fn is_decimal(text: &str) -> bool {
    static PATTERN: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    PATTERN
        .get_or_init(|| regex::Regex::new(DECIMAL).expect("a valid pattern"))
        .is_match(text)
}

/// A spec file, read and checked.
pub struct Loaded {
    /// The directory relative paths in the spec are read against.
    pub dir: PathBuf,
    /// The file's bytes, as copied into the instance directory.
    pub text: String,
    pub model: m::InstanceSpec,
}

/// Reads `path`: YAML through the generated serde types, then the checks the types cannot state.
pub fn load(path: &Path) -> Result<Loaded, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let model = parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    // A bare file name's parent is the empty path, which names no directory to canonicalise.
    let dir = path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    Ok(Loaded { dir, text, model })
}

/// Reads a spec file an operator gives to `create`, `update` or `adopt`: [`load`], then the
/// refusal of a source named after a state file cortex keeps for itself
/// ([`crate::instance::RESERVED_STATE_NAMES`]). An instance's frozen copy is read with [`load`]
/// alone, so an instance created before the rule keeps loading and can be updated to a new name.
pub fn load_given(path: &Path) -> Result<Loaded, String> {
    let loaded = load(path)?;
    if let Some(reason) = reserved_refusal(&loaded.model) {
        return Err(format!("{}: {reason}", path.display()));
    }
    Ok(loaded)
}

/// Why a source name of `spec` cannot be used: it is one of the state file names cortex keeps.
pub fn reserved_refusal(spec: &m::InstanceSpec) -> Option<String> {
    spec.sources
        .iter()
        .find(|s| crate::instance::RESERVED_STATE_NAMES.contains(&s.name.as_str()))
        .map(|s| {
            format!(
                "source name {:?} is reserved: cortex keeps its own state/{}.json; \
                 rename the source (reserved: {})",
                s.name,
                s.name,
                crate::instance::RESERVED_STATE_NAMES.join(", ")
            )
        })
}

/// Parses spec text. YAML is read into a JSON value first: the generated types decode from JSON.
pub fn parse(text: &str) -> Result<m::InstanceSpec, String> {
    let value: serde_json::Value =
        serde_yaml_ng::from_str(text).map_err(|e| format!("not YAML: {e}"))?;
    let spec: cortex_spec_types::CortexInstanceInstanceSpec = serde_json::from_value(value.clone())
        .map_err(|e| match value.get("store") {
            // A union refusal names no field; a refused `store` names itself and its backends.
            Some(store)
                if serde_json::from_value::<cortex_spec_types::CortexInstanceStoreSpec>(
                    store.clone(),
                )
                .is_err() =>
            {
                format!("not a {FORMAT} spec: store: {STORE_FORMS}")
            }
            _ => format!("not a {FORMAT} spec: {e}"),
        })?;
    let model = model_map::instance_spec(&spec).map_err(|e| e.to_string())?;
    check(&model)?;
    Ok(model)
}

fn is_name(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 63
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !text.starts_with('-')
}

fn check(spec: &m::InstanceSpec) -> Result<(), String> {
    if spec.format != FORMAT {
        return Err(format!("format is {:?}, not {FORMAT:?}", spec.format));
    }
    if !is_name(&spec.name.0) {
        return Err(format!(
            "name {:?}: use 1–63 lowercase letters, digits or '-'",
            spec.name.0
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    for source in &spec.sources {
        if !is_name(&source.name) {
            return Err(format!(
                "source name {:?}: use 1–63 lowercase letters, digits or '-'",
                source.name
            ));
        }
        if !seen.insert(source.name.as_str()) {
            return Err(format!("source name {:?} appears twice", source.name));
        }
        let p = &source.policy;
        for (field, value) in [
            ("refresh_after_days", p.refresh_after_days),
            ("max_documents_per_run", p.max_documents_per_run),
            ("max_chars_per_document", p.max_chars_per_document),
        ] {
            if value < 0 || (field != "refresh_after_days" && value == 0) {
                return Err(format!("sources.{}.policy.{field}: {value}", source.name));
            }
        }
    }
    if let Some(reason) = reference_refusal(spec) {
        return Err(reason);
    }
    // A `connectors` source cites a record as `record:<adapter>:<operation>:<id>`, as a structured
    // source does, so a structured source that ends what it no longer lists would claim the
    // values the model extracted from that operation's records.
    for source in &spec.sources {
        let m::SourceSettings::Structured(st) = &source.settings else {
            continue;
        };
        let m::StructuredInput::Connectors(c) = &st.input else {
            continue;
        };
        if st.dropped != Some(m::DropPolicy::Supersede) {
            continue;
        }
        let shared = spec.sources.iter().find(|other| {
            matches!(&other.settings, m::SourceSettings::Connectors(o)
                if o.adapter == c.adapter && o.operation == c.operation)
        });
        if let Some(other) = shared {
            return Err(format!(
                "sources.{}: dropped: Supersede with {}:{}, which the connectors source {:?} \
                 also reads: its records are cited alike, so this source would end the values \
                 extracted from them; use dropped: Keep, or read another operation",
                source.name, c.adapter, c.operation, other.name
            ));
        }
    }
    // A child record's identity names its child by the operation (`structured::child_key`), so the
    // records of two children of one operation would be one record set, mapped by the first.
    for source in &spec.sources {
        let m::SourceSettings::Structured(m::StructuredSource {
            input: m::StructuredInput::Connectors(c),
            ..
        }) = &source.settings
        else {
            continue;
        };
        let mut operations = std::collections::BTreeSet::new();
        for child in c.children.iter().flatten() {
            if !operations.insert(child.operation.as_str()) {
                return Err(format!(
                    "sources.{}: children: operation {:?} appears twice; give each child \
                     operation one entry",
                    source.name, child.operation
                ));
            }
        }
        // A link orders the records of one child and links those of another: both are children
        // of the source.
        for (n, link) in c.links.iter().flatten().enumerate() {
            for (field, operation) in [("tags", &link.tags), ("changes", &link.changes)] {
                if !operations.contains(operation.as_str()) {
                    return Err(format!(
                        "sources.{}: links[{n}].{field}: {operation:?} names no child operation \
                         of the source; name one of its children",
                        source.name
                    ));
                }
            }
        }
    }
    let budget: f64 = spec.model.budget_usd.0.parse().map_err(|_| {
        format!(
            "model.budget_usd {:?} is not a number",
            spec.model.budget_usd.0
        )
    })?;
    if budget.is_nan() || budget <= 0.0 {
        return Err("model.budget_usd must be above 0".into());
    }
    if let Some(ceiling) = &spec.model.daily_ceiling_usd {
        let usd: f64 = ceiling
            .0
            .parse()
            .map_err(|_| format!("model.daily_ceiling_usd {:?} is not a number", ceiling.0))?;
        if usd.is_nan() || usd <= 0.0 {
            return Err("model.daily_ceiling_usd must be above 0".into());
        }
    }
    for (field, limit) in [
        ("budget_tokens", spec.model.budget_tokens),
        ("daily_ceiling_tokens", spec.model.daily_ceiling_tokens),
    ] {
        if limit.is_some_and(|n| n <= 0) {
            return Err(format!("model.{field} must be above 0"));
        }
    }
    if spec.model.timeout_s <= 0 {
        return Err("model.timeout_s must be above 0".into());
    }
    for (i, check) in spec.gate.iter().flat_map(|g| g.checks.iter()).enumerate() {
        for (bound, value) in [("min", &check.min), ("max", &check.max)] {
            if let Some(value) = value {
                if !is_decimal(&value.0) {
                    return Err(format!(
                        "gate.checks[{i}].{bound} {:?} is not a decimal ({DECIMAL})",
                        value.0
                    ));
                }
            }
        }
    }
    // Refused before any unit is written: the viewer's unit copies the same variables by value.
    if let Some(gateway) = &spec.model.gateway {
        if let Some(refusal) = crate::schedule::token_env_refusal(&gateway.token_env) {
            return Err(refusal);
        }
    }
    if let Some(hook) = spec.monitoring.as_ref().and_then(|o| o.hook.as_ref()) {
        if hook.command.first().is_none_or(|c| c.is_empty()) {
            return Err("monitoring.hook.command must name a program".into());
        }
        if hook.timeout_s.is_some_and(|t| t <= 0) {
            return Err("monitoring.hook.timeout_s must be above 0".into());
        }
    }
    Ok(())
}

/// Why a connection the spec names cannot be used: it names both an id (`connection`) and a
/// profile (`profile`), or neither. The specification states only "at least one"; "not both" is
/// its `UNMAPPED:` rule, held here. The reason names the place and the fields, never a value.
fn reference_refusal(spec: &m::InstanceSpec) -> Option<String> {
    let mut named: Vec<(String, bool, bool)> = Vec::new();
    for source in &spec.sources {
        let at = format!("sources.{}.settings.value", source.name);
        match &source.settings {
            m::SourceSettings::Web(w) => {
                named.push((at, w.connection.is_some(), w.profile.is_some()))
            }
            m::SourceSettings::Connectors(c) => {
                named.push((at, c.connection.is_some(), c.profile.is_some()))
            }
            m::SourceSettings::Structured(m::StructuredSource {
                input: m::StructuredInput::Connectors(c),
                ..
            }) => named.push((
                format!("{at}.input.value"),
                c.connection.is_some(),
                c.profile.is_some(),
            )),
            _ => {}
        }
    }
    if let Some(m::StoreSpec::Postgres(p)) = &spec.store {
        for (field, reference) in [
            ("connection", &p.connection),
            ("schema_connection", &p.schema_connection),
        ] {
            if let Some(r) = reference {
                named.push((
                    format!("store.value.{field}"),
                    r.connection.is_some(),
                    r.profile.is_some(),
                ));
            }
        }
    }
    named
        .into_iter()
        .find_map(|(at, id, profile)| match (id, profile) {
            (true, true) => Some(format!(
                "{at}: names both connection and profile; name the connection by id (connection) \
             or by profile (profile), not both"
            )),
            (false, false) => Some(format!(
                "{at}: names no connection; name it by id (connection) or by profile (profile)"
            )),
            _ => None,
        })
}

/// The connections the spec's sources name, by id or by profile, with their adapter.
pub fn connections(spec: &m::InstanceSpec) -> Vec<(String, Reference)> {
    let mut out = Vec::new();
    for source in &spec.sources {
        match &source.settings {
            m::SourceSettings::Web(w) => out.push((
                crate::sources::web_adapter(w).to_string(),
                Reference::of(w.connection.as_ref(), w.profile.as_ref()),
            )),
            m::SourceSettings::Connectors(c) => out.push((
                c.adapter.clone(),
                Reference::of(c.connection.as_ref(), c.profile.as_ref()),
            )),
            m::SourceSettings::Structured(m::StructuredSource {
                input: m::StructuredInput::Connectors(c),
                ..
            }) => out.push((
                c.adapter.clone(),
                Reference::of(c.connection.as_ref(), c.profile.as_ref()),
            )),
            m::SourceSettings::Files(_)
            | m::SourceSettings::Structured(m::StructuredSource {
                input: m::StructuredInput::Files(_),
                ..
            }) => {}
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Every seed file the spec names, relative to the spec's directory.
pub fn seed_files(spec: &m::InstanceSpec) -> Vec<String> {
    let mut out: Vec<String> = spec.seed.schema.iter().cloned().collect();
    out.extend(spec.seed.ekr_seed.iter().cloned());
    out.extend(spec.seed.documents.iter().cloned());
    out.extend(spec.model.instructions.iter().cloned());
    out
}
