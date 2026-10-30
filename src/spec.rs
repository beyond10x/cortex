//! Reading an instance spec file (`cortex.instance/1`).

use std::path::{Path, PathBuf};

use cortex_model::instance as m;

use crate::connectors::Reference;
use crate::model_map;

pub const FORMAT: &str = "cortex.instance/1";

/// The largest `refresh_after_days` a source policy may give: 100 years.
pub const MAX_REFRESH_AFTER_DAYS: i64 = 36_500;

/// The largest `monitoring.hook.timeout_s`: one day.
pub const MAX_HOOK_TIMEOUT_S: i64 = 86_400;

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
///
/// A spec it accepts with a dollar limit behind a model gateway gets one warning line on stderr
/// ([`dollar_limits_warning`]). A number out of range is refused here ([`bounds_refusal`]), where
/// a spec is written, and nowhere a spec is only run.
pub fn load_given(path: &Path) -> Result<Loaded, String> {
    given(path, true)
}

/// Reads the spec file `cortex ask serve` runs over: what [`load_given`] refuses, but for the
/// numbers [`bounds_refusal`] holds. The spec is an existing instance's, which 0.6.1 served with
/// any of them, and the answer service reads none of them; a run saturates or caps them where it
/// reads them.
pub fn load_served(path: &Path) -> Result<Loaded, String> {
    given(path, false)
}

fn given(path: &Path, bounded: bool) -> Result<Loaded, String> {
    let loaded = load(path)?;
    if let Some(reason) = reserved_refusal(&loaded.model)
        .or_else(|| infinite_refusal(&loaded.model))
        .or_else(|| gateway_refusal(&loaded.model))
        .or_else(|| bounded.then(|| bounds_refusal(&loaded.model)).flatten())
    {
        return Err(format!("{}: {reason}", path.display()));
    }
    if let Some(warning) = dollar_limits_warning(&loaded.model) {
        eprintln!("cortex: {}: {warning}", path.display());
    }
    Ok(loaded)
}

/// Why a spec with a model gateway cannot be given: it names no `model.budget_tokens`. Behind a
/// gateway the dollar limits bound nothing, so the tokens must. Held for `create`, `update` and
/// `adopt` ([`load_given`]); an instance's frozen copy, read with [`load`], is not held to it, and
/// one created before the rule keeps counting dollars ([`crate::spend::counts_dollars`]).
pub fn gateway_refusal(spec: &m::InstanceSpec) -> Option<String> {
    (spec.model.gateway.is_some() && spec.model.budget_tokens.is_none()).then(|| {
        "model.budget_tokens is required with model.gateway: behind a gateway the cost claude \
         reports is not counted, and the instance is bounded in tokens only"
            .into()
    })
}

/// Why a spec's dollar limit cannot be given: `model.budget_usd` or `model.daily_ceiling_usd`
/// has more digits than a finite number holds, so it would read as infinite and bound nothing.
/// Held for `create`, `update` and `adopt` ([`load_given`]); an instance's frozen copy, read with
/// [`load`], is not held to it, so one created before the rule keeps running.
pub fn infinite_refusal(spec: &m::InstanceSpec) -> Option<String> {
    [
        ("model.budget_usd", &spec.model.budget_usd),
        ("model.daily_ceiling_usd", &spec.model.daily_ceiling_usd),
    ]
    .into_iter()
    .find_map(|(field, value)| {
        let value = value.as_ref()?;
        value
            .0
            .parse::<f64>()
            .is_ok_and(|usd| !usd.is_finite())
            .then(|| format!("{field} is too large to be read as a finite number of US dollars"))
    })
}

/// The warning for a spec that gives `model.budget_usd`, `model.daily_ceiling_usd` or a run gate
/// check on `cost_usd` behind a model gateway, where they bound nothing; `None` for any other
/// spec.
pub fn dollar_limits_warning(spec: &m::InstanceSpec) -> Option<String> {
    spec.model.gateway.as_ref()?;
    let gated = spec
        .gate
        .iter()
        .flat_map(|g| g.checks.iter())
        .any(|c| c.measure == "cost_usd");
    let given: Vec<&str> = [
        ("model.budget_usd", spec.model.budget_usd.is_some()),
        (
            "model.daily_ceiling_usd",
            spec.model.daily_ceiling_usd.is_some(),
        ),
        ("a gate check on cost_usd", gated),
    ]
    .into_iter()
    .filter_map(|(field, given)| given.then_some(field))
    .collect();
    (!given.is_empty()).then(|| {
        format!(
            "{} bound nothing behind model.gateway: the cost claude reports there is not \
             counted; the instance is bounded by model.budget_tokens and \
             model.daily_ceiling_tokens",
            given.join(" and ")
        )
    })
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
    if let Some(reason) = alias_template_refusal(spec) {
        return Err(reason);
    }
    if let Some(reason) = key_template_refusal(spec) {
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
    // UNMAPPED in `spec/domains/instance.yaml` (`cortex.instance.ModelSpec`): `budget_usd` is
    // required without a gateway; `budget_tokens` with one, which `load_given` holds
    // ([`gateway_refusal`]).
    match &spec.model.budget_usd {
        None if spec.model.gateway.is_none() => {
            return Err("model.budget_usd is required without model.gateway".into());
        }
        None => {}
        Some(budget_usd) => {
            let budget: f64 = budget_usd
                .0
                .parse()
                .map_err(|_| format!("model.budget_usd {:?} is not a number", budget_usd.0))?;
            if budget.is_nan() || budget <= 0.0 {
                return Err("model.budget_usd must be above 0".into());
            }
        }
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
    if spec.model.batch_chars.is_some_and(|n| n < 1_000) {
        return Err("model.batch_chars must be at least 1000".into());
    }
    if spec.model.timeout_s <= 0 {
        return Err("model.timeout_s must be above 0".into());
    }
    if spec.runs.as_ref().is_some_and(|r| r.keep_days < 1) {
        return Err("runs.keep_days must be at least 1".into());
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
        let scheme = |prefix: &str| {
            gateway
                .base_url
                .get(..prefix.len())
                .is_some_and(|s| s.eq_ignore_ascii_case(prefix))
        };
        if !(scheme("http://") || scheme("https://")) {
            return Err(format!(
                "model.gateway.base_url {:?} must be an http:// or https:// URL",
                gateway.base_url
            ));
        }
        if gateway.max_context_tokens.is_some_and(|n| n <= 0) {
            return Err("model.gateway.max_context_tokens must be above 0".into());
        }
        if let Some(output) = gateway.max_output_tokens {
            if output <= 0 {
                return Err("model.gateway.max_output_tokens must be above 0".into());
            }
            if gateway
                .max_context_tokens
                .is_some_and(|context| output >= context)
            {
                return Err(
                    "model.gateway.max_output_tokens must be below model.gateway.max_context_tokens"
                        .into(),
                );
            }
        }
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

/// Why a number of `spec` is out of range: a `refresh_after_days` above
/// [`MAX_REFRESH_AFTER_DAYS`], a `monitoring.hook.timeout_s` above [`MAX_HOOK_TIMEOUT_S`] or a
/// `serve.view_port` that is no port (0 lets the viewer take a free one). Held for `create`,
/// `update` and `adopt` ([`load_given`]); an instance's frozen copy, read with [`load`], and the
/// spec `cortex ask serve` is given, read with [`load_served`], are not held to it, so one
/// created before the rule keeps running: a run saturates the refresh window and caps the hook's
/// time ([`crate::status`]), the port is read only when an instance is made, and the answer
/// service reads none of the three.
pub fn bounds_refusal(spec: &m::InstanceSpec) -> Option<String> {
    if let Some(source) = spec
        .sources
        .iter()
        .find(|s| s.policy.refresh_after_days > MAX_REFRESH_AFTER_DAYS)
    {
        return Some(format!(
            "sources.{}.policy.refresh_after_days: {}: use 0 to {MAX_REFRESH_AFTER_DAYS} days",
            source.name, source.policy.refresh_after_days
        ));
    }
    let timeout = spec
        .monitoring
        .as_ref()
        .and_then(|o| o.hook.as_ref())
        .and_then(|h| h.timeout_s);
    if let Some(t) = timeout.filter(|t| *t > MAX_HOOK_TIMEOUT_S) {
        return Some(format!(
            "monitoring.hook.timeout_s: {t}: use 1 to {MAX_HOOK_TIMEOUT_S} seconds"
        ));
    }
    let port = spec.serve.view_port;
    port.filter(|p| u16::try_from(*p).is_err())
        .map(|port| format!("serve.view_port: {port}: use a port from 0 to {}", u16::MAX))
}

/// Why an alias template of a structured mapping (`alias_templates`, the source's own mapping or a
/// child's) is no alias template ([`crate::structured::template_refusal`]): the specification
/// cannot state the grammar of a String; this is its `UNMAPPED:` rule. Names the place and the
/// template.
fn alias_template_refusal(spec: &m::InstanceSpec) -> Option<String> {
    for source in &spec.sources {
        let m::SourceSettings::Structured(st) = &source.settings else {
            continue;
        };
        let at = format!("sources.{}.settings.value", source.name);
        let children = match &st.input {
            m::StructuredInput::Connectors(c) => c.children.iter().flatten().collect(),
            m::StructuredInput::Files(_) => Vec::new(),
        };
        let mappings = std::iter::once((format!("{at}.mapping"), &st.mapping)).chain(
            children.into_iter().enumerate().map(|(i, c)| {
                (
                    format!("{at}.input.value.children[{i}].mapping"),
                    &c.mapping,
                )
            }),
        );
        for (place, mapping) in mappings {
            for template in mapping.alias_templates.iter().flatten() {
                if let Some(why) = crate::structured::template_refusal(template) {
                    return Some(format!("{place}.alias_templates: {template:?}: {why}"));
                }
            }
        }
    }
    None
}

/// Why a `connectors` source's `key` is no template: it is rendered by the grammar of an alias
/// template ([`crate::structured::template_refusal`]), so an empty key, one without a `{field}`
/// placeholder, or one with an unclosed `{` or a stray `}` is refused here, naming the place and
/// the key. So is an `{input.…}` placeholder on a source without inputs, or one that some input
/// does not hold as a text, number or boolean, naming the placeholder and the input. This is the
/// `UNMAPPED:` rule of `cortex.instance.ConnectorsSource.key`.
fn key_template_refusal(spec: &m::InstanceSpec) -> Option<String> {
    spec.sources.iter().find_map(|source| {
        let m::SourceSettings::Connectors(c) = &source.settings else {
            return None;
        };
        let key = c.key.as_deref()?;
        let at = format!("sources.{}.settings.value.key: {key:?}", source.name);
        if let Some(why) = crate::structured::template_refusal(key) {
            return Some(format!("{at}: {why}"));
        }
        // An `{input.…}` placeholder is rendered from the spec's own inputs (a run fills in only
        // their window values), so one that some input does not hold as a scalar is known now.
        let fields = crate::structured::template_input_fields(key);
        let first = fields.first()?;
        if c.inputs.is_empty() {
            return Some(format!(
                "{at}: the placeholder {{{first}}} names an input, and the source has no inputs"
            ));
        }
        c.inputs.iter().enumerate().find_map(|(n, input)| {
            let rendered =
                crate::structured::input_projection(key, &crate::model_map::to_serde(input));
            fields
                .iter()
                .zip(rendered)
                .find(|(_, text)| text.is_none())
                .map(|(field, _)| {
                    format!(
                        "{at}: the placeholder {{{field}}} names no text, number or boolean in \
                         inputs[{n}]"
                    )
                })
        })
    })
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
    if let Some(r) = spec.ask.as_ref().and_then(|a| a.records.as_ref()) {
        named.push((
            "ask.records.connection".into(),
            r.connection.connection.is_some(),
            r.connection.profile.is_some(),
        ));
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
