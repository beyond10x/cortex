//! Fetching one source's documents: web pages through the Connectors `tavily` provider, records of
//! any Connectors operation (as text, or as records a `structured` source maps), or local files,
//! whole or as records (JSON lines or markdown sections).

use std::collections::{BTreeMap, BTreeSet};
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
    /// A record of a `files` source read as records: cited as `file:<path>#<id>`. Its changed
    /// text is delivered again whatever the refresh window (`SeenState::wants`).
    FileRecord,
    Record,
}

/// One fetched document, before filtering.
#[derive(Debug, Clone)]
pub struct Document {
    /// Stable across runs: the URL, the file path, `<file path>#<id>`, or `<operation>:<id>`.
    pub key: String,
    pub origin: Origin,
    pub title: Option<String>,
    /// A short description, where the source gives one (a search result's snippet).
    pub description: Option<String>,
    pub published: Option<String>,
    pub text: String,
    /// The hash of the whole text, taken by the run before `max_chars_per_document` cuts `text`;
    /// `None` before that, when the hash is that of `text`.
    pub hash: Option<String>,
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

/// How far a run's `{since}` reaches back before the start of the last successful run: a record a
/// provider shows late, stamped up to this long before that start, is still asked for. The
/// seen-state hash keeps the overlap from applying a document twice.
pub const OVERLAP_MS: i64 = 5 * 60 * 1000;

/// The instants a run asks a `connectors` source about, in milliseconds since the Unix epoch:
/// `{since}` and `{until}` in its `inputs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    pub since_ms: i64,
    pub until_ms: i64,
}

impl Window {
    /// The window of a run that starts at `start_ms`. It reaches back [`OVERLAP_MS`] before the start
    /// of the source's last successful run, or, before the first, `refresh_after_days` before
    /// `start_ms`; and never later than `held_since_ms`, which covers every change the last run
    /// held back, or `pending_since_ms`, the `{since}` of a run since the last successful one that
    /// did not succeed.
    pub fn of_run(
        start_ms: i64,
        last_success_ms: Option<i64>,
        held_since_ms: Option<i64>,
        pending_since_ms: Option<i64>,
        refresh_after_days: i64,
    ) -> Self {
        let since = last_success_ms.map_or(start_ms - refresh_after_days * 86_400_000, |last| {
            last - OVERLAP_MS
        });
        Self {
            since_ms: [held_since_ms, pending_since_ms]
                .into_iter()
                .flatten()
                .fold(since, i64::min),
            until_ms: start_ms,
        }
    }
}

/// An instant as RFC 3339 in UTC, to the second (`2026-10-05T12:34:56Z`). Both ends of a window
/// are cut the same way, so one run's `{until}` and the next run's `{since}` stay equal.
pub fn rfc3339(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let (days, day) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Howard Hinnant's `civil_from_days`.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        day / 3600,
        day % 3600 / 60,
        day % 60
    )
}

/// `input` with `{since}` and `{until}` in every string value replaced by the window's instants.
fn with_window(input: &Value, window: &Window) -> Value {
    match input {
        Value::String(t) => Value::String(
            t.replace("{since}", &rfc3339(window.since_ms))
                .replace("{until}", &rfc3339(window.until_ms)),
        ),
        Value::Array(items) => Value::Array(items.iter().map(|v| with_window(v, window)).collect()),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(k, v)| (k.clone(), with_window(v, window)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// `template` with every string value's `{a.b}` placeholders filled from `record`.
fn render_json(template: &Value, record: &Value) -> Value {
    match template {
        Value::String(t) => Value::String(render(t, record)),
        Value::Array(items) => Value::Array(items.iter().map(|v| render_json(v, record)).collect()),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(k, v)| (k.clone(), render_json(v, record)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Sets the value at a dotted path, creating the objects on the way. Like [`at`], the path may
/// start at the root (`$.page`).
fn set_at(v: &mut Value, path: &str, value: Value) {
    let path = path.strip_prefix("$.").unwrap_or(path);
    let mut v = v;
    for step in path.split('.') {
        if !v.is_object() {
            *v = json!({});
        }
        v = v
            .as_object_mut()
            .expect("an object")
            .entry(step)
            .or_insert(Value::Null);
    }
    *v = value;
}

/// The page to ask for after `answer`, or `None` when the walk ends. With `next`, the value at
/// that path of the answer, whatever the style; a missing, null or empty one ends the walk.
/// Without it, a `PageNumber` walk asks for the page after the current one (the input's `param`,
/// `1` when absent); a `Token` or `Keyset` walk has nothing to advance by and ends.
fn next_page(p: &m::Paging, input: &Value, answer: &Value) -> Option<Value> {
    let next = match &p.next {
        Some(path) => match at(answer, path)? {
            Value::String(t) if !t.trim().is_empty() => Value::String(t.clone()),
            Value::Number(n) => Value::Number(n.clone()),
            _ => return None,
        },
        None => match p.style {
            m::PageStyle::PageNumber => {
                let current = match at(input, &p.param) {
                    Some(Value::Number(n)) => n.as_i64()?,
                    Some(Value::String(t)) => t.trim().parse().ok()?,
                    _ => 1,
                };
                json!(current + 1)
            }
            m::PageStyle::Token | m::PageStyle::Keyset => return None,
        },
    };
    Some(next)
}

/// Every record of an operation for `input`, page after page as `paging` says. The walk ends at
/// an empty page, at a missing `next` (see [`next_page`]), at a page or token it has already asked
/// for, or after `max_pages` pages; without `paging` one page is read. A walk that leaves pages
/// unread adds why to `unread`, once per operation: it reached `max_pages` with a next page still
/// to read, or an empty page still named a next one.
fn walk(
    invoke: &mut dyn FnMut(&Value) -> Result<Value, FetchError>,
    what: &str,
    input: &Value,
    records: &str,
    paging: Option<&m::Paging>,
    unread: &mut Vec<String>,
) -> Result<Vec<Value>, FetchError> {
    let mut input = input.clone();
    let mut found = Vec::new();
    // Every page or token asked for: a provider whose tokens cycle would be asked forever.
    let mut asked: Vec<Value> = paging
        .and_then(|p| at(&input, &p.param))
        .into_iter()
        .cloned()
        .collect();
    let pages = paging.map_or(1, |p| p.max_pages.max(1));
    for n in 1..=pages {
        let answer = invoke(&input)?;
        let answer = body(&answer);
        let page = at(answer, records)
            .and_then(Value::as_array)
            .ok_or_else(|| FetchError::Failed(format!("{what}: no array at {records:?}")))?;
        if page.is_empty() {
            if let Some(p) = paging.filter(|p| p.next.is_some()) {
                if next_page(p, &input, answer).is_some() {
                    note(unread, format!("{what}: an empty page named a next page"));
                }
            }
            break;
        }
        found.extend(page.iter().cloned());
        let Some(p) = paging else { break };
        let Some(next) = next_page(p, &input, answer) else {
            break;
        };
        if asked.contains(&next) {
            break;
        }
        if n == pages {
            note(
                unread,
                format!("{what}: max_pages {pages} reached with pages left"),
            );
            break;
        }
        asked.push(next.clone());
        set_at(&mut input, &p.param, next);
    }
    Ok(found)
}

/// Adds `why` to `unread` unless it is there already.
fn note(unread: &mut Vec<String>, why: String) {
    if !unread.contains(&why) {
        unread.push(why);
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

/// What one fetch read.
#[derive(Debug, Default)]
pub struct Fetched {
    pub documents: Vec<Document>,
    /// Why the fetch left records unread, one line each: a paged operation that reached its
    /// `max_pages` with pages left or met an empty page that named a next one, or a child call
    /// that failed for a parent, which is then left out. Empty when it read everything.
    pub unread: Vec<String>,
    /// Parents left out because their child call failed on [`CHILD_FAILURE_LIMIT`] or more
    /// consecutive runs, one line each. They do not hold the window.
    pub skipped: Vec<String>,
    /// Consecutive failed child calls per parent key, after this fetch.
    pub child_failures: BTreeMap<String, u32>,
}

/// Consecutive runs a parent's child call may fail and still hold the window; from this many on,
/// the parent is skipped.
pub const CHILD_FAILURE_LIMIT: u32 = 3;

/// The documents of one source. `window` fills `{since}` and `{until}` in a `connectors`
/// source's inputs, and `child_failures` counts the consecutive runs each parent's child call
/// failed in; the other kinds use neither. `max_context` caps the characters of thread context a
/// record of a `files` source carries (the policy's `max_chars_per_document`).
pub fn fetch(
    settings: &m::SourceSettings,
    connectors: &Connectors,
    base: &Path,
    window: &Window,
    child_failures: &BTreeMap<String, u32>,
    max_context: usize,
) -> Result<Fetched, FetchError> {
    let documents = |documents| Fetched {
        documents,
        unread: Vec::new(),
        skipped: Vec::new(),
        child_failures: child_failures.clone(),
    };
    match settings {
        m::SourceSettings::Web(web) => fetch_web(web, connectors).map(documents),
        m::SourceSettings::Connectors(c) => fetch_records(c, connectors, window, child_failures),
        m::SourceSettings::Files(f) => {
            fetch_files(f, base, max_context).map(|(docs, skipped)| Fetched {
                skipped,
                ..documents(docs)
            })
        }
        m::SourceSettings::Structured(st) => match &st.input {
            m::StructuredInput::Connectors(c) => {
                fetch_structured(st, c, connectors, window).map(|mut fetched| {
                    fetched.child_failures = child_failures.clone();
                    fetched
                })
            }
            // The spec accepts the settings; reading files is that story's.
            m::StructuredInput::Files(_) => Err(FetchError::Failed(
                "a structured source reading files is not run yet \
                 (story:structured-from-files-and-drops)"
                    .into(),
            )),
        },
    }
}

/// The records of a `structured` source's Connectors operation, one document per record, read as
/// a `connectors` source reads them: each input with the run's window filled in, walked page by
/// page. A record with no scalar at the mapping's `id` or `name` is left out, and so is a second
/// record with an id already read. The document's text is the record's JSON, which
/// `src/structured.rs` maps. Its key here, `<adapter>:<operation>:<raw id>`, only tells records
/// apart: the run replaces it with the record's identity (`structured::Source::prepare`) before
/// anything is stored, so a raw id that masking or a redaction rule would change is never stored.
fn fetch_structured(
    st: &m::StructuredSource,
    c: &m::StructuredConnectors,
    connectors: &Connectors,
    window: &Window,
) -> Result<Fetched, FetchError> {
    let mut docs = Vec::new();
    let mut unread = Vec::new();
    let mut keys = std::collections::BTreeSet::new();
    let mut invoke =
        |input: &Value| Ok(connectors.invoke(&c.adapter, &c.connection, &c.operation, input)?);
    for input in &c.inputs {
        let records = walk(
            &mut invoke,
            &format!("{}.{}", c.adapter, c.operation),
            &with_window(&to_serde(input), window),
            &st.records,
            c.paging.as_ref(),
            &mut unread,
        )?;
        for record in &records {
            let Some(id) = crate::structured::text_at(record, &st.mapping.id) else {
                continue;
            };
            if crate::structured::text_at(record, &st.mapping.name).is_none() {
                continue;
            }
            let key = format!("{}:{}:{id}", c.adapter, c.operation);
            if !keys.insert(key.clone()) {
                continue;
            }
            docs.push(Document {
                key,
                origin: Origin::Record,
                title: None,
                description: None,
                published: None,
                text: record.to_string(),
                hash: None,
            });
        }
    }
    Ok(Fetched {
        documents: docs,
        unread,
        ..Fetched::default()
    })
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
        hash: None,
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

/// The value at a dotted path (`fields.summary`), which may start at the root `$`
/// (`$.fields.summary`); `$` alone is the value itself.
pub fn at<'a>(v: &'a Value, path: &str) -> Option<&'a Value> {
    if path == "$" {
        return Some(v);
    }
    let path = path.strip_prefix("$.").unwrap_or(path);
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

/// One invocation of `operation` on the source's adapter and connection.
fn invoker<'a>(
    connectors: &'a Connectors,
    c: &'a m::ConnectorsSource,
    operation: &'a str,
) -> impl FnMut(&Value) -> Result<Value, FetchError> + 'a {
    move |input| Ok(connectors.invoke(&c.adapter, &c.connection, operation, input)?)
}

/// The records of a `connectors` source, one document per record. Each input, with the run's
/// window filled in, is walked page by page; with a `child`, the child operation is invoked once
/// per record, its input filled from the record, its pages walked the same way, and each of its
/// records appended to the parent's text as one JSON paragraph. A parent whose child call fails
/// is left out of this fetch and named in `unread`; from its [`CHILD_FAILURE_LIMIT`]th consecutive
/// failed run on, in `skipped` instead. A successful call clears the parent's count.
fn fetch_records(
    c: &m::ConnectorsSource,
    connectors: &Connectors,
    window: &Window,
    child_failures: &BTreeMap<String, u32>,
) -> Result<Fetched, FetchError> {
    let mut docs = Vec::new();
    let mut unread = Vec::new();
    let mut skipped = Vec::new();
    let mut failures = child_failures.clone();
    let mut keys = std::collections::BTreeSet::new();
    for input in &c.inputs {
        let records = walk(
            &mut invoker(connectors, c, &c.operation),
            &format!("{}.{}", c.adapter, c.operation),
            &with_window(&to_serde(input), window),
            &c.records,
            c.paging.as_ref(),
            &mut unread,
        )?;
        for record in &records {
            let Some(id) = at(record, &c.id).map(|v| match v {
                Value::String(t) => t.clone(),
                other => other.to_string(),
            }) else {
                continue;
            };
            let mut text = c
                .text
                .iter()
                .map(|t| render(t, record))
                .filter(|t| !t.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n\n");
            if text.trim().is_empty() {
                continue;
            }
            let key = format!("{}:{}:{id}", c.adapter, c.operation);
            if !keys.insert(key.clone()) {
                continue;
            }
            if let Some(child) = &c.child {
                let children = match walk(
                    &mut invoker(connectors, c, &child.operation),
                    &format!("{}.{}", c.adapter, child.operation),
                    &render_json(&to_serde(&child.input), record),
                    &child.records,
                    child.paging.as_ref(),
                    &mut unread,
                ) {
                    Ok(children) => {
                        failures.remove(&crate::mask::mask_key(&key).0);
                        children
                    }
                    Err(e) => {
                        let runs = failures.entry(crate::mask::mask_key(&key).0).or_insert(0);
                        *runs += 1;
                        let op = &child.operation;
                        if *runs >= CHILD_FAILURE_LIMIT {
                            let why = format!(
                                "{op}: child call failed for {key} on {runs} runs; skipped"
                            );
                            note(&mut skipped, why);
                        } else {
                            note(
                                &mut unread,
                                format!("{op}: child call failed for {key}: {e}"),
                            );
                        }
                        continue;
                    }
                };
                for child in &children {
                    text.push_str("\n\n");
                    text.push_str(&child.to_string());
                }
            }
            docs.push(Document {
                key,
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
                hash: None,
            });
        }
    }
    Ok(Fetched {
        documents: docs,
        unread,
        skipped,
        child_failures: failures,
    })
}

/// The files of a `files` source: one document per file, or with `records`, one per record of
/// each file ([`file_records`]), and what was left out as `skipped`. A record's thread context
/// holds at most `max_context` characters.
fn fetch_files(
    f: &m::FilesSource,
    base: &Path,
    max_context: usize,
) -> Result<(Vec<Document>, Vec<String>), FetchError> {
    let glob = globset::Glob::new(&f.glob)
        .map_err(|e| FetchError::Failed(format!("glob {:?}: {e}", f.glob)))?
        .compile_matcher();
    let mut docs = Vec::new();
    let mut read = RecordsRead::default();
    let mut skipped = Vec::new();
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
            let Ok(bytes) = std::fs::read(entry.path()) else {
                continue;
            };
            let path = entry.path().display().to_string();
            let title = entry.file_name().to_str();
            match &f.records {
                Some(r) => docs.extend(file_records(
                    r,
                    &path,
                    title,
                    &bytes,
                    max_context,
                    &mut read,
                    &mut skipped,
                )),
                None => docs.extend(whole_file(path, title, bytes)),
            }
        }
    }
    Ok((docs, skipped))
}

/// A file as one document, keyed by its path; none when it is not UTF-8 text or is blank.
fn whole_file(path: String, title: Option<&str>, bytes: Vec<u8>) -> Option<Document> {
    let text = String::from_utf8(bytes).ok()?;
    if text.trim().is_empty() {
        return None;
    }
    Some(Document {
        key: path,
        origin: Origin::File,
        title: title.map(str::to_string),
        description: None,
        published: None,
        text,
        hash: None,
    })
}

/// What reading records carries from one file of a source to the next.
#[derive(Debug, Default)]
struct RecordsRead {
    /// The keys read so far: a key read twice counts once.
    keys: BTreeSet<String>,
    /// The texts of the records read so far, per thread, oldest first.
    threads: BTreeMap<String, Vec<String>>,
}

/// Where a record's thread context starts in its text: the first paragraph that opens with
/// `[context` (a `[context] ` record or the `[context cut]` mark).
const CONTEXT: &str = "\n\n[context";

/// A record's own text and its thread context, the latter empty when there is none. Redaction
/// counts how often a name occurs over a record's own text only, so a message repeated as context
/// does not make a name look frequent. A record whose own text holds `\n\n[context` is split
/// there; the words after it are then not counted, which only makes a name rarer.
pub fn split_context(text: &str) -> (&str, &str) {
    match text.find(CONTEXT) {
        Some(at) => text.split_at(at),
        None => (text, ""),
    }
}

/// The documents of one file read as `r` says, `path` being the file's path and `content` its
/// bytes. A leading byte-order mark is ignored. A record a filter leaves out, without an id, with
/// empty text or with a key already read is skipped; a JSON line that is not UTF-8 or not JSON, a
/// markdown file that is not UTF-8 or has no section, is named in `skipped`.
///
/// A document's key is `<path>#<id>`; in a markdown file, the second section whose key is taken
/// gets `<path>#<id>-2`, the third `-3`, counted in file order. Its text is `<author>: <text>`
/// when the record has an author. When `r` names a `thread` field and the record has it, the text
/// is followed by the earlier records of that thread, nearest first, as `[context]` paragraphs
/// ([`context`]); a record without the field carries no context and starts the thread named by
/// its own id. `WholeFile` is the file as one document, as without `records`.
fn file_records(
    r: &m::FileRecords,
    path: &str,
    title: Option<&str>,
    content: &[u8],
    max_context: usize,
    read: &mut RecordsRead,
    skipped: &mut Vec<String>,
) -> Vec<Document> {
    if content.iter().all(u8::is_ascii_whitespace) {
        return Vec::new();
    }
    let unmarked = content.strip_prefix(b"\xef\xbb\xbf").unwrap_or(content);
    let records = match r.format {
        m::RecordFormat::WholeFile => {
            return whole_file(path.into(), title, content.to_vec())
                .into_iter()
                .collect();
        }
        m::RecordFormat::JsonLines => json_lines(path, unmarked, skipped),
        m::RecordFormat::MarkdownSections => {
            let Ok(text) = std::str::from_utf8(unmarked) else {
                note(skipped, format!("{path}: not UTF-8 text; skipped"));
                return Vec::new();
            };
            let sections = markdown_sections(text);
            if sections.is_empty() {
                note(skipped, format!("{path}: no ## section; skipped"));
            }
            sections
        }
    };
    let mut docs = Vec::new();
    let mut in_file = BTreeSet::new();
    for record in &records {
        let Some(id) = scalar(record, &r.id) else {
            continue;
        };
        let mut key = format!("{path}#{id}");
        if r.format == m::RecordFormat::MarkdownSections {
            let first = key.clone();
            let mut n = 1;
            while in_file.contains(&key) {
                n += 1;
                key = format!("{first}-{n}");
            }
            in_file.insert(key.clone());
        }
        if !r.filters.iter().all(|f| passes(f, record)) {
            continue;
        }
        let mut text = r
            .text
            .iter()
            .map(|t| render(t, record))
            .filter(|t| !t.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
        if text.trim().is_empty() {
            continue;
        }
        if !read.keys.insert(key.clone()) {
            continue;
        }
        if let Some(author) = r.author.as_deref().and_then(|a| scalar(record, a)) {
            text = format!("{author}: {text}");
        }
        let mut full = text.clone();
        if let Some(field) = &r.thread {
            match scalar(record, field) {
                Some(thread) => {
                    let earlier = read.threads.entry(thread).or_default();
                    full.push_str(&context(earlier, max_context));
                    earlier.push(text);
                }
                None => {
                    read.threads.insert(id, vec![text]);
                }
            }
        }
        docs.push(Document {
            key,
            origin: Origin::FileRecord,
            title: title.map(str::to_string),
            description: None,
            published: r.time.as_deref().and_then(|t| scalar(record, t)),
            text: full,
            hash: None,
        });
    }
    docs
}

/// The `[context]` paragraphs of a record whose thread holds `earlier` (oldest first): nearest
/// first, at most `max` characters of their text, the last one cut there, then a `[context cut]`
/// paragraph when anything was left out. Each starts with `\n\n`.
fn context(earlier: &[String], max: usize) -> String {
    let mut out = String::new();
    let mut left = max;
    for text in earlier.iter().rev() {
        if left == 0 {
            out.push_str("\n\n[context cut]");
            break;
        }
        out.push_str("\n\n[context] ");
        let n = text.chars().count();
        if n <= left {
            out.push_str(text);
            left -= n;
        } else {
            out.extend(text.chars().take(left));
            out.push_str("\n\n[context cut]");
            break;
        }
    }
    out
}

/// Whether `record` passes `f`: its field equals one of the values when `f` includes them, and
/// equals none of them when it excludes them. An absent field equals no value.
fn passes(f: &m::RecordFilter, record: &Value) -> bool {
    let hit = scalar(record, &f.field).is_some_and(|v| f.values.contains(&v));
    hit == f.include
}

/// The text, number or boolean at `path` of `record`, as text; `None` for anything else or blank
/// text.
fn scalar(record: &Value, path: &str) -> Option<String> {
    match at(record, path)? {
        Value::String(t) if !t.trim().is_empty() => Some(t.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Every non-blank line of a JSON-lines file as a record, line by line. A line that is not UTF-8
/// or not JSON is left out and named in `skipped`; the other lines are still read.
fn json_lines(path: &str, content: &[u8], skipped: &mut Vec<String>) -> Vec<Value> {
    let mut records = Vec::new();
    for (n, line) in content.split(|b| *b == b'\n').enumerate() {
        let Ok(line) = std::str::from_utf8(line) else {
            note(
                skipped,
                format!("{path}: line {} is not UTF-8; skipped", n + 1),
            );
            continue;
        };
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str(line) {
            Ok(record) => records.push(record),
            Err(_) => note(
                skipped,
                format!("{path}: line {} is not JSON; skipped", n + 1),
            ),
        }
    }
    records
}

/// The opening code fence `line` is, as CommonMark has it: at most three spaces, then three or
/// more backticks or tildes (a backtick fence's info string holds no backtick). Its character and
/// length.
fn fence_opens(line: &str) -> Option<(char, usize)> {
    let t = line.trim_start_matches(' ');
    if line.len() - t.len() > 3 {
        return None;
    }
    let c = t.chars().next().filter(|c| matches!(c, '`' | '~'))?;
    let n = t.chars().take_while(|&x| x == c).count();
    (n >= 3 && !(c == '`' && t[n..].contains('`'))).then_some((c, n))
}

/// Whether `line` closes a fence opened with `n` of `c`: at most three spaces, at least `n` of
/// `c`, then only spaces.
fn fence_closes(line: &str, c: char, n: usize) -> bool {
    let t = line.trim_start_matches(' ');
    let m = t.chars().take_while(|&x| x == c).count();
    line.len() - t.len() <= 3 && m >= n && t[m..].trim().is_empty()
}

/// Whether `line` underlines a setext level-two heading: at most three spaces, then only `-`.
fn setext_underline(line: &str) -> bool {
    let t = line.trim_start_matches(' ');
    let t2 = t.trim_end();
    line.len() - t.len() <= 3 && !t2.is_empty() && t2.chars().all(|c| c == '-')
}

/// Whether `line` can be a one-line paragraph a setext underline turns into a heading: not blank,
/// not indented code, not a heading, quote, list item, fence or thematic break.
fn paragraph_line(line: &str) -> bool {
    let t = line.trim_start();
    let digits = t.chars().take_while(char::is_ascii_digit).count();
    let ordered = digits > 0 && (t[digits..].starts_with(". ") || t[digits..].starts_with(") "));
    line.len() - t.len() < 4
        && !t.is_empty()
        && !t.starts_with('#')
        && !t.starts_with('>')
        && !["- ", "* ", "+ "].iter().any(|p| t.starts_with(p))
        && !ordered
        && fence_opens(line).is_none()
        && !t.chars().all(|c| matches!(c, '-' | '*' | '_' | ' '))
}

/// Every level-two section of a markdown file as a record `{"heading", "body"}`: the heading's
/// text, and the lines up to the next level-two heading, trimmed. A level-two heading is a
/// `## ` line, or a one-line paragraph underlined by `-` (setext). Neither counts inside a fenced
/// code block, which closes only on its own character, at least as long as it opened (CommonMark).
/// A leading `---` front matter block, and text before the first heading, are not sections.
fn markdown_sections(content: &str) -> Vec<Value> {
    let lines: Vec<&str> = content.lines().collect();
    let mut start = 0;
    if lines.first().map(|l| l.trim_end()) == Some("---") {
        if let Some(end) = lines[1..]
            .iter()
            .position(|l| matches!(l.trim_end(), "---" | "..."))
        {
            start = end + 2;
        }
    }
    let mut preamble: Vec<&str> = Vec::new();
    let mut sections: Vec<(String, Vec<&str>)> = Vec::new();
    let mut fence: Option<(char, usize)> = None;
    for &line in &lines[start..] {
        if let Some((c, n)) = fence {
            if fence_closes(line, c, n) {
                fence = None;
            }
        } else if let Some(open) = fence_opens(line) {
            fence = Some(open);
        } else if let Some(heading) = line.strip_prefix("## ") {
            sections.push((heading.trim().to_string(), Vec::new()));
            continue;
        } else if setext_underline(line) {
            let body = match sections.last_mut() {
                Some((_, body)) => body,
                None => &mut preamble,
            };
            let n = body.len();
            let heading = (n >= 1
                && (n == 1 || body[n - 2].trim().is_empty())
                && paragraph_line(body[n - 1]))
            .then(|| body.pop())
            .flatten();
            if let Some(heading) = heading {
                sections.push((heading.trim().to_string(), Vec::new()));
                continue;
            }
        }
        match sections.last_mut() {
            Some((_, body)) => body.push(line),
            None => preamble.push(line),
        }
    }
    sections
        .into_iter()
        .map(|(heading, body)| json!({"heading": heading, "body": body.join("\n").trim()}))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paging(style: m::PageStyle, next: Option<&str>, max_pages: i64) -> m::Paging {
        m::Paging {
            style,
            param: "page".into(),
            next: next.map(str::to_string),
            max_pages,
        }
    }

    /// Walks a stand-in operation that answers `pages(input)`, and answers the records and the
    /// inputs it was asked with.
    fn walked(
        p: Option<&m::Paging>,
        input: Value,
        pages: impl Fn(&Value) -> Value,
    ) -> (Vec<Value>, Vec<Value>) {
        let mut asked = Vec::new();
        let mut capped = Vec::new();
        let records = walk(
            &mut |input: &Value| {
                asked.push(input.clone());
                Ok(pages(input))
            },
            "t.op",
            &input,
            "items",
            p,
            &mut capped,
        )
        .unwrap();
        (records, asked)
    }

    #[test]
    fn a_path_may_start_at_the_root_and_the_root_alone_is_the_value() {
        let v = json!({"a": {"b": 1}, "$x": 2});
        assert_eq!(at(&v, "a.b"), Some(&json!(1)));
        assert_eq!(at(&v, "$.a.b"), Some(&json!(1)));
        assert_eq!(at(&v, "$"), Some(&v));
        assert_eq!(at(&v, "$x"), Some(&json!(2)));
        assert_eq!(at(&v, "a.c"), None);
    }

    #[test]
    fn a_page_number_walk_counts_up_from_the_input_and_stops_at_an_empty_page() {
        let p = paging(m::PageStyle::PageNumber, None, 10);
        let (records, asked) = walked(Some(&p), json!({"q": "x"}), |input| {
            match input.get("page").and_then(Value::as_i64) {
                None | Some(2) => json!({"items": [1]}),
                _ => json!({"items": []}),
            }
        });
        assert_eq!(records, [json!(1), json!(1)]);
        assert_eq!(
            asked,
            [
                json!({"q": "x"}),
                json!({"q": "x", "page": 2}),
                json!({"q": "x", "page": 3})
            ]
        );
    }

    #[test]
    fn a_walk_reads_at_most_max_pages() {
        let p = paging(m::PageStyle::PageNumber, None, 2);
        let (records, asked) = walked(Some(&p), json!({}), |_| json!({"items": [1]}));
        assert_eq!((records.len(), asked.len()), (2, 2));
    }

    #[test]
    fn a_token_walk_follows_next_and_stops_when_it_is_missing_or_repeated() {
        let p = paging(m::PageStyle::Token, Some("meta.next"), 10);
        let (records, asked) = walked(Some(&p), json!({}), |input| match input.get("page") {
            None => json!({"items": ["a"], "meta": {"next": "t2"}}),
            Some(_) => json!({"items": ["b"], "meta": {"next": null}}),
        });
        assert_eq!(records, [json!("a"), json!("b")]);
        assert_eq!(asked.len(), 2);

        let (_, asked) = walked(
            Some(&p),
            json!({}),
            |_| json!({"items": ["a"], "meta": {"next": "same"}}),
        );
        assert_eq!(asked, [json!({}), json!({"page": "same"})]);
    }

    #[test]
    fn a_keyset_or_token_walk_without_next_reads_one_page_as_an_unpaged_one_does() {
        let p = paging(m::PageStyle::Keyset, None, 10);
        let (_, asked) = walked(Some(&p), json!({}), |_| json!({"items": [1]}));
        assert_eq!(asked.len(), 1);
        let (_, asked) = walked(None, json!({}), |_| json!({"items": [1]}));
        assert_eq!(asked.len(), 1);
    }

    #[test]
    fn the_window_fills_since_and_until_in_every_string_of_an_input() {
        let window = Window::of_run(86_400_000, None, None, None, 1);
        assert_eq!(
            with_window(
                &json!({"jql": "updated >= \"{since}\"", "range": ["{since}", "{until}"], "n": 1}),
                &window
            ),
            json!({"jql": "updated >= \"1970-01-01T00:00:00Z\"",
                "range": ["1970-01-01T00:00:00Z", "1970-01-02T00:00:00Z"], "n": 1})
        );
        let after_a_run = Window::of_run(1_000_000, Some(900_000), None, None, 7);
        assert_eq!(after_a_run.since_ms, 900_000 - OVERLAP_MS);
        let held = Window::of_run(1_000_000, Some(900_000), Some(100), None, 7);
        assert_eq!((held.since_ms, held.until_ms), (100, 1_000_000));
        let pending = Window::of_run(1_000_000, None, Some(500), Some(200), 0);
        assert_eq!(pending.since_ms, 200);
    }
}

#[cfg(test)]
mod file_records_tests {
    use super::*;

    fn chat(filters: Vec<m::RecordFilter>) -> m::FileRecords {
        m::FileRecords {
            format: m::RecordFormat::JsonLines,
            id: "ts".into(),
            time: Some("time".into()),
            author: Some("user".into()),
            text: vec!["{text}".into()],
            thread: Some("thread".into()),
            filters,
        }
    }

    fn read(r: &m::FileRecords, content: &str) -> (Vec<Document>, Vec<String>) {
        let mut skipped = Vec::new();
        let docs = file_records(
            r,
            "/x/chat.jsonl",
            Some("chat.jsonl"),
            content.as_bytes(),
            5000,
            &mut RecordsRead::default(),
            &mut skipped,
        );
        (docs, skipped)
    }

    fn keys(docs: &[Document]) -> Vec<&str> {
        docs.iter().map(|d| d.key.as_str()).collect()
    }

    const THREAD: &str = concat!(
        r#"{"ts":"1","user":"a","text":"one"}"#,
        "\n",
        r#"{"ts":"2","user":"b","text":"two","thread":"1"}"#,
        "\n",
        r#"{"ts":"3","user":"c","text":"three","thread":"1"}"#,
        "\n",
    );

    /// The record replied to comes first, then the older ones; the context holds at most
    /// `max_context` characters of their text, and a cut says so.
    #[test]
    fn context_is_nearest_first_and_capped_with_the_cut_marked() {
        let (docs, _) = read(&chat(Vec::new()), THREAD);
        assert_eq!(
            docs[2].text,
            "c: three\n\n[context] b: two\n\n[context] a: one"
        );
        let mut skipped = Vec::new();
        let capped = file_records(
            &chat(Vec::new()),
            "/x/chat.jsonl",
            None,
            THREAD.as_bytes(),
            8,
            &mut RecordsRead::default(),
            &mut skipped,
        );
        assert_eq!(capped[1].text, "b: two\n\n[context] a: one");
        assert_eq!(
            capped[2].text,
            "c: three\n\n[context] b: two\n\n[context] a:\n\n[context cut]"
        );
        let none = file_records(
            &chat(Vec::new()),
            "/x/chat.jsonl",
            None,
            THREAD.as_bytes(),
            0,
            &mut RecordsRead::default(),
            &mut skipped,
        );
        assert_eq!(none[2].text, "c: three\n\n[context cut]");
        assert_eq!(
            split_context(&capped[2].text),
            ("c: three", &capped[2].text[8..])
        );
        assert_eq!(split_context(&none[2].text).0, "c: three");
        assert_eq!(split_context("no context").1, "");
    }

    /// A record without the thread field starts its own thread: it carries nothing, and later
    /// replies to its id see it, not an earlier file's thread of the same id.
    #[test]
    fn a_record_without_the_thread_field_starts_its_own_thread() {
        let mut read = RecordsRead::default();
        let mut skipped = Vec::new();
        let r = chat(Vec::new());
        let a = file_records(
            &r,
            "/x/a.jsonl",
            None,
            THREAD.as_bytes(),
            5000,
            &mut read,
            &mut skipped,
        );
        assert_eq!(a.len(), 3);
        let b = file_records(
            &r,
            "/x/b.jsonl",
            None,
            concat!(
                r#"{"ts":"1","user":"d","text":"fresh"}"#,
                "\n",
                r#"{"ts":"2","user":"e","text":"reply","thread":"1"}"#,
                "\n",
            )
            .as_bytes(),
            5000,
            &mut read,
            &mut skipped,
        );
        assert_eq!(b[0].text, "d: fresh");
        assert_eq!(b[1].text, "e: reply\n\n[context] d: fresh");
    }

    fn sections(content: &str) -> (Vec<Document>, Vec<String>) {
        let r = m::FileRecords {
            format: m::RecordFormat::MarkdownSections,
            id: "heading".into(),
            time: None,
            author: None,
            text: vec!["{body}".into()],
            thread: None,
            filters: Vec::new(),
        };
        let mut skipped = Vec::new();
        let docs = file_records(
            &r,
            "/x/n.md",
            None,
            content.as_bytes(),
            5000,
            &mut RecordsRead::default(),
            &mut skipped,
        );
        (docs, skipped)
    }

    #[test]
    fn front_matter_is_not_a_setext_heading_and_setext_headings_open_sections() {
        let (docs, skipped) = sections(
            "---\ntitle: Guide\n---\n\nIntro.\n\nSetup\n-----\n\nInstall.\n\n- item\n---\n\nUsage\n---\nRun.\n",
        );
        assert!(skipped.is_empty(), "{skipped:?}");
        assert_eq!(keys(&docs), ["/x/n.md#Setup", "/x/n.md#Usage"]);
        assert_eq!(docs[0].text, "Install.\n\n- item\n---");
        assert_eq!(docs[1].text, "Run.");
    }

    #[test]
    fn a_fence_closes_only_on_its_own_character_at_least_as_long() {
        let (docs, _) = sections(
            "## One\n\n````\n```\n## inside\n```\n~~~\n## still inside\n````\n\n## Two\n\nx\n",
        );
        assert_eq!(keys(&docs), ["/x/n.md#One", "/x/n.md#Two"]);
        assert!(docs[0].text.contains("## still inside"), "{}", docs[0].text);
    }

    #[test]
    fn a_repeated_heading_is_numbered_past_a_heading_that_already_has_the_number() {
        let (docs, _) = sections("## Notes\na\n## Notes-2\nb\n## Notes\nc\n");
        assert_eq!(
            keys(&docs),
            ["/x/n.md#Notes", "/x/n.md#Notes-2", "/x/n.md#Notes-3"]
        );
        assert_eq!(docs[2].text, "c");
    }

    #[test]
    fn a_byte_order_mark_is_ignored_and_a_file_without_sections_is_named() {
        let (docs, _) = sections("\u{feff}## First\nx\n");
        assert_eq!(keys(&docs), ["/x/n.md#First"]);
        let (docs, skipped) = sections("# Title\n\nNo level-two heading.\n");
        assert!(docs.is_empty());
        assert_eq!(skipped, ["/x/n.md: no ## section; skipped"]);
        let (docs, skipped) = read(&chat(Vec::new()), "\u{feff}{\"ts\":\"1\",\"text\":\"x\"}\n");
        assert_eq!(keys(&docs), ["/x/chat.jsonl#1"]);
        assert!(skipped.is_empty(), "{skipped:?}");
    }

    #[test]
    fn the_second_record_of_a_thread_carries_the_first_as_context() {
        let (docs, skipped) = read(
            &chat(Vec::new()),
            concat!(
                r#"{"ts":"1","user":"ana","time":"t1","text":"Shall we ship on Friday?"}"#,
                "\n",
                r#"{"ts":"2","user":"ben","time":"t2","text":"Yes.","thread":"1"}"#,
                "\n",
                r#"{"ts":"3","user":"cem","text":"Unrelated.","thread":"9"}"#,
                "\n",
            ),
        );
        assert!(skipped.is_empty(), "{skipped:?}");
        assert_eq!(
            docs.iter().map(|d| d.key.as_str()).collect::<Vec<_>>(),
            ["/x/chat.jsonl#1", "/x/chat.jsonl#2", "/x/chat.jsonl#3"]
        );
        assert_eq!(docs[0].text, "ana: Shall we ship on Friday?");
        assert_eq!(
            docs[1].text,
            "ben: Yes.\n\n[context] ana: Shall we ship on Friday?"
        );
        assert_eq!(docs[2].text, "cem: Unrelated.");
        assert_eq!(docs[0].published.as_deref(), Some("t1"));
        assert_eq!(docs[2].published, None);
        assert!(docs.iter().all(|d| d.origin == Origin::FileRecord));
        assert_eq!(docs[0].title.as_deref(), Some("chat.jsonl"));
    }

    #[test]
    fn filters_include_or_exclude_records_and_a_line_that_is_not_json_is_skipped_and_named() {
        let only_eng = m::RecordFilter {
            field: "channel".into(),
            values: vec!["eng".into(), "ops".into()],
            include: true,
        };
        let no_bots = m::RecordFilter {
            field: "user".into(),
            values: vec!["bot".into()],
            include: false,
        };
        let (docs, skipped) = read(
            &chat(vec![only_eng, no_bots]),
            concat!(
                r#"{"ts":"1","channel":"eng","user":"ana","text":"a"}"#,
                "\n",
                r#"{"ts":"2","channel":"dm","user":"ana","text":"b"}"#,
                "\n",
                "not json\n",
                r#"{"ts":"3","channel":"ops","user":"bot","text":"c"}"#,
                "\n",
                r#"{"ts":"4","user":"ana","text":"no channel"}"#,
                "\n",
                r#"{"ts":"5","channel":"ops","user":"ben","text":"e"}"#,
                "\n",
            ),
        );
        assert_eq!(
            docs.iter().map(|d| d.key.as_str()).collect::<Vec<_>>(),
            ["/x/chat.jsonl#1", "/x/chat.jsonl#5"]
        );
        assert_eq!(skipped, ["/x/chat.jsonl: line 3 is not JSON; skipped"]);
    }

    #[test]
    fn a_markdown_file_is_one_record_per_level_two_section() {
        let r = m::FileRecords {
            format: m::RecordFormat::MarkdownSections,
            id: "heading".into(),
            time: None,
            author: None,
            text: vec!["{heading}".into(), "{body}".into()],
            thread: None,
            filters: Vec::new(),
        };
        let mut skipped = Vec::new();
        let docs = file_records(
            &r,
            "/x/notes.md",
            None,
            "# Title\n\nIntro.\n\n## One\n\nFirst.\n\n### Deeper\n\nStill one.\n\n~~~\n## fenced\n~~~\n\n## Two\nSecond.\n"
                .as_bytes(),
            5000,
            &mut RecordsRead::default(),
            &mut skipped,
        );
        assert_eq!(
            docs.iter()
                .map(|d| (d.key.as_str(), d.text.as_str()))
                .collect::<Vec<_>>(),
            [
                (
                    "/x/notes.md#One",
                    "One\n\nFirst.\n\n### Deeper\n\nStill one.\n\n~~~\n## fenced\n~~~"
                ),
                ("/x/notes.md#Two", "Two\n\nSecond."),
            ]
        );
    }
}

#[cfg(test)]
mod max_pages_tests {
    use super::*;

    fn capped(max_pages: i64, last: i64) -> Vec<String> {
        let p = m::Paging {
            style: m::PageStyle::Token,
            param: "page".into(),
            next: Some("next".into()),
            max_pages,
        };
        let mut capped = Vec::new();
        for _ in 0..2 {
            walk(
                &mut |input: &Value| {
                    let n = input.get("page").and_then(Value::as_i64).unwrap_or(1);
                    let next = if n < last { json!(n + 1) } else { Value::Null };
                    Ok(json!({"items": [n], "next": next}))
                },
                "t.op",
                &json!({}),
                "items",
                Some(&p),
                &mut capped,
            )
            .unwrap();
        }
        capped
    }

    #[test]
    fn a_walk_that_reaches_max_pages_with_pages_left_says_so_once() {
        assert_eq!(capped(2, 3), ["t.op: max_pages 2 reached with pages left"]);
    }

    #[test]
    fn a_walk_whose_last_page_is_its_max_pages_th_is_not_capped() {
        assert!(capped(3, 3).is_empty());
        assert!(capped(10, 3).is_empty());
    }
}
