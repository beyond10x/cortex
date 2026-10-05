//! Fetching one source's documents: web pages through the Connectors `tavily` provider, records of
//! any Connectors operation, or local files.

use std::path::Path;

use cortex_model::instance as m;
use serde_json::{json, Value};

use crate::connectors::{body, Connectors, InvokeError};
use crate::model_map::to_serde;

/// What one document cites itself as in EKR.
#[derive(Debug, Clone, PartialEq)]
pub enum Origin {
    Url,
    File,
    Record,
}

/// One fetched document, before filtering.
#[derive(Debug, Clone)]
pub struct Document {
    /// Stable across runs: the URL, the file path, or `<operation>:<id>`.
    pub key: String,
    pub origin: Origin,
    pub title: Option<String>,
    /// A short description, where the source gives one (a search result's snippet).
    pub description: Option<String>,
    pub published: Option<String>,
    pub text: String,
}

#[derive(Debug)]
pub enum FetchError {
    Missing(String),
    Failed(String),
}

impl From<InvokeError> for FetchError {
    fn from(e: InvokeError) -> Self {
        match e {
            InvokeError::Missing(m) => FetchError::Missing(m),
            InvokeError::Failed(m) | InvokeError::Lapsed(m) => FetchError::Failed(m),
        }
    }
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(m) | Self::Failed(m) => f.write_str(m),
        }
    }
}

/// The Connectors adapter a web source uses when its spec names none.
pub const DEFAULT_WEB_ADAPTER: &str = "tavily";

/// A crawl's page limit when the spec gives no crawl policy.
const DEFAULT_CRAWL_LIMIT: i64 = 20;

/// The adapter a web source invokes `datasource.websearch/v1alpha1` on.
pub fn web_adapter(web: &m::WebSource) -> &str {
    web.adapter.as_deref().unwrap_or(DEFAULT_WEB_ADAPTER)
}

pub fn fetch(
    settings: &m::SourceSettings,
    connectors: &Connectors,
    base: &Path,
) -> Result<Vec<Document>, FetchError> {
    match settings {
        m::SourceSettings::Web(web) => fetch_web(web, connectors),
        m::SourceSettings::Connectors(c) => fetch_records(c, connectors),
        m::SourceSettings::Files(f) => fetch_files(f, base),
        // The spec accepts the settings; running them is `story:structured-source`'s.
        m::SourceSettings::Structured(_) => Err(FetchError::Failed(
            "a structured source is not run yet (story:structured-source)".into(),
        )),
    }
}

fn s(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(Value::as_str)
        .filter(|t| !t.trim().is_empty())
        .map(str::to_string)
}

fn topic(t: m::SearchTopic) -> &'static str {
    match t {
        m::SearchTopic::General => "general",
        m::SearchTopic::News => "news",
    }
}

fn time_range(t: m::TimeRange) -> &'static str {
    match t {
        m::TimeRange::Day => "day",
        m::TimeRange::Week => "week",
        m::TimeRange::Month => "month",
        m::TimeRange::Year => "year",
    }
}

/// The `websearch.search` input for one query, with its policy; content is always asked in full.
pub fn search_input(query: &str, p: &m::SearchPolicy) -> Value {
    let mut input = json!({
        "query": query,
        "max_results": p.max_results,
        "content": "full",
        "topic": topic(p.topic),
    });
    if let Some(t) = p.time_range {
        input["time_range"] = json!(time_range(t));
    }
    if !p.include_domains.is_empty() {
        input["include_domains"] = json!(p.include_domains);
    }
    if !p.exclude_domains.is_empty() {
        input["exclude_domains"] = json!(p.exclude_domains);
    }
    if let Some(c) = &p.country {
        input["country"] = json!(c);
    }
    if let Some(l) = &p.language {
        input["language"] = json!(l);
    }
    input
}

/// The `websearch.crawl` input from one start URL, with its policy.
pub fn crawl_input(url: &str, p: Option<&m::CrawlPolicy>) -> Value {
    let mut input = json!({"url": url, "limit": DEFAULT_CRAWL_LIMIT});
    if let Some(p) = p {
        input["limit"] = json!(p.limit);
        input["max_depth"] = json!(p.max_depth);
        input["max_breadth"] = json!(p.max_breadth);
        input["allow_external"] = json!(p.allow_external);
        if !p.select_paths.is_empty() {
            input["select_paths"] = json!(p.select_paths);
        }
        if !p.exclude_paths.is_empty() {
            input["exclude_paths"] = json!(p.exclude_paths);
        }
        if let Some(i) = &p.instructions {
            input["instructions"] = json!(i);
        }
    }
    input
}

/// One website of a `datasource.websearch/v1alpha1` result: a search result (`url`, `title`,
/// `description`, `content`, `published`) or a page (`url`, `title`, `content`). A website with no
/// content keeps its description as its text; one with neither is dropped.
fn website(item: &Value) -> Option<Document> {
    let url = s(item, "url")?;
    let description = s(item, "description");
    let text = s(item, "content").or_else(|| description.clone())?;
    Some(Document {
        key: url,
        origin: Origin::Url,
        title: s(item, "title"),
        description,
        published: s(item, "published"),
        text,
    })
}

fn items<'a>(answer: &'a Value, key: &str) -> impl Iterator<Item = &'a Value> {
    body(answer)[key].as_array().into_iter().flatten()
}

fn fetch_web(web: &m::WebSource, connectors: &Connectors) -> Result<Vec<Document>, FetchError> {
    let adapter = web_adapter(web);
    let mut docs = Vec::new();
    match &web.input {
        m::WebInput::Search(search) => {
            for query in &search.queries {
                let answer = connectors.invoke(
                    adapter,
                    &web.connection,
                    "websearch.search",
                    &search_input(query, &search.policy),
                )?;
                docs.extend(items(&answer, "results").filter_map(website));
            }
        }
        m::WebInput::Sites(sites) => match sites.mode {
            m::WebMode::Pages => {
                let input = json!({"urls": sites.urls});
                let answer =
                    connectors.invoke(adapter, &web.connection, "websearch.fetch", &input)?;
                docs.extend(items(&answer, "pages").filter_map(website));
            }
            m::WebMode::Crawl => {
                for url in &sites.urls {
                    let input = crawl_input(url, sites.policy.as_ref());
                    let answer =
                        connectors.invoke(adapter, &web.connection, "websearch.crawl", &input)?;
                    docs.extend(items(&answer, "pages").filter_map(website));
                }
            }
        },
    }
    dedupe(&mut docs);
    Ok(docs)
}

fn dedupe(docs: &mut Vec<Document>) {
    let mut seen = std::collections::BTreeSet::new();
    docs.retain(|d| seen.insert(d.key.clone()));
}

/// The value at a dotted path (`fields.summary`).
pub fn at<'a>(v: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.').try_fold(v, |v, step| v.get(step))
}

/// Fills `{a.b}` placeholders from `record`; a missing or non-text value renders empty.
pub fn render(template: &str, record: &Value) -> String {
    let mut out = String::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let Some(close) = rest[open..].find('}') else {
            out.push_str(&rest[open..]);
            return out;
        };
        let path = &rest[open + 1..open + close];
        match at(record, path) {
            Some(Value::String(t)) => out.push_str(t),
            Some(Value::Number(n)) => out.push_str(&n.to_string()),
            Some(Value::Bool(b)) => out.push_str(&b.to_string()),
            _ => {}
        }
        rest = &rest[open + close + 1..];
    }
    out.push_str(rest);
    out
}

fn fetch_records(
    c: &m::ConnectorsSource,
    connectors: &Connectors,
) -> Result<Vec<Document>, FetchError> {
    let mut docs = Vec::new();
    for input in &c.inputs {
        let answer =
            connectors.invoke(&c.adapter, &c.connection, &c.operation, &to_serde(input))?;
        let records = at(body(&answer), &c.records)
            .and_then(Value::as_array)
            .ok_or_else(|| {
                FetchError::Failed(format!(
                    "{}.{}: no array at {:?}",
                    c.adapter, c.operation, c.records
                ))
            })?;
        for record in records {
            let Some(id) = at(record, &c.id).map(|v| match v {
                Value::String(t) => t.clone(),
                other => other.to_string(),
            }) else {
                continue;
            };
            let text = c
                .text
                .iter()
                .map(|t| render(t, record))
                .filter(|t| !t.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n\n");
            if text.trim().is_empty() {
                continue;
            }
            docs.push(Document {
                key: format!("{}:{}:{id}", c.adapter, c.operation),
                origin: Origin::Record,
                title: None,
                description: None,
                published: c
                    .time
                    .as_deref()
                    .and_then(|t| at(record, t))
                    .and_then(Value::as_str)
                    .map(str::to_string),
                text,
            });
        }
    }
    dedupe(&mut docs);
    Ok(docs)
}

fn fetch_files(f: &m::FilesSource, base: &Path) -> Result<Vec<Document>, FetchError> {
    let glob = globset::Glob::new(&f.glob)
        .map_err(|e| FetchError::Failed(format!("glob {:?}: {e}", f.glob)))?
        .compile_matcher();
    let mut docs = Vec::new();
    for root in &f.paths {
        let root = crate::ekr::expand(root);
        let root = if root.is_absolute() {
            root
        } else {
            base.join(root)
        };
        for entry in walkdir::WalkDir::new(&root).sort_by_file_name() {
            let entry =
                entry.map_err(|e| FetchError::Failed(format!("{}: {e}", root.display())))?;
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry.path().strip_prefix(&root).unwrap_or(entry.path());
            if !glob.is_match(rel) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(entry.path()) else {
                continue;
            };
            if text.trim().is_empty() {
                continue;
            }
            docs.push(Document {
                key: entry.path().display().to_string(),
                origin: Origin::File,
                title: entry.file_name().to_str().map(str::to_string),
                description: None,
                published: None,
                text,
            });
        }
    }
    Ok(docs)
}
