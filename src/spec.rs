//! Reading an instance spec file (`cortex.instance/1`).

use std::path::{Path, PathBuf};

use cortex_model::instance as m;

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
    let budget: f64 = spec.model.budget_usd.0.parse().map_err(|_| {
        format!(
            "model.budget_usd {:?} is not a number",
            spec.model.budget_usd.0
        )
    })?;
    if budget.is_nan() || budget <= 0.0 {
        return Err("model.budget_usd must be above 0".into());
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
    Ok(())
}

/// The connection ids the spec's sources name, with their adapter.
pub fn connections(spec: &m::InstanceSpec) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for source in &spec.sources {
        match &source.settings {
            m::SourceSettings::Web(w) => out.push((
                crate::sources::web_adapter(w).to_string(),
                w.connection.clone(),
            )),
            m::SourceSettings::Connectors(c) => out.push((c.adapter.clone(), c.connection.clone())),
            m::SourceSettings::Structured(m::StructuredSource {
                input: m::StructuredInput::Connectors(c),
                ..
            }) => out.push((c.adapter.clone(), c.connection.clone())),
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
