//! Fetching one source's documents: web pages through the Connectors `tavily` provider, records of
//! any Connectors operation, or local files.

use std::collections::BTreeMap;
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

/// Sets the value at a dotted path, creating the objects on the way.
fn set_at(v: &mut Value, path: &str, value: Value) {
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
/// failed in; the other kinds use neither.
pub fn fetch(
    settings: &m::SourceSettings,
    connectors: &Connectors,
    base: &Path,
    window: &Window,
    child_failures: &BTreeMap<String, u32>,
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
        m::SourceSettings::Files(f) => fetch_files(f, base).map(documents),
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
                        failures.remove(&key);
                        children
                    }
                    Err(e) => {
                        let runs = failures.entry(key.clone()).or_insert(0);
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
                hash: None,
            });
        }
    }
    Ok(docs)
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
