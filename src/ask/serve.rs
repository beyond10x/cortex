//! The HTTP side of `cortex ask serve`: `POST /ask` answers `text/event-stream`, one SSE event per
//! `cortex.ask.AskEvent`, in the generated wire form (`model_map::ask_event`), and keeps the answer
//! as a record (`records::Keeper`) before its last event. `POST /ask/{answer_id}/vote` casts or
//! changes the caller's vote, `GET /review` lists the open review items and
//! `POST /review/{review_id}/close` closes one; each needs the identity header and answers JSON.
//! Records past `ask.retention_days` are purged before the ready line and every 24 hours after.

use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::body::Bytes;
use axum::extract::rejection::BytesRejection;
use axum::extract::DefaultBodyLimit;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use cortex_model::ask as ma;
use serde_json::{json, Value};
use tokio::sync::Semaphore;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt;

use super::limit::RateLimiter;
use super::records::{self, Asked, Keeper, VoteRefused};
use super::{answer, final_event, Settings};

struct Service {
    settings: Settings,
    limiter: Mutex<RateLimiter>,
    /// One permit per answer in flight, over every identity (`ask.max_in_flight`).
    in_flight: Arc<Semaphore>,
    keeper: Keeper,
}

/// Now, in Unix milliseconds.
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

/// The caller's identity: the trimmed value of the header the spec names, when not empty.
fn identity_of(service: &Service, headers: &HeaderMap) -> Option<String> {
    headers
        .get(service.settings.ask.identity_header.as_str())
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

/// A JSON answer of the record routes.
fn json_response(status: StatusCode, body: Value) -> Response {
    (
        status,
        [("content-type", "application/json")],
        body.to_string(),
    )
        .into_response()
}

/// A JSON refusal of the record routes: `{"error": <reason>, "message": <words>}`.
fn json_error(status: StatusCode, error: &str, message: impl Into<String>) -> Response {
    json_response(status, json!({"error": error, "message": message.into()}))
}

fn unidentified(service: &Service) -> Response {
    json_error(
        StatusCode::UNAUTHORIZED,
        "unidentified",
        format!(
            "the request carries no `{}` header",
            service.settings.ask.identity_header
        ),
    )
}

/// Writes `line` to the service's standard error with every credential shape masked
/// (`crate::mask`). A line can carry text from outside: a gateway's error body, a store's error
/// (a connection string). Every log line of the ask service goes through here; a test of this
/// module holds that no other line is written.
fn log(line: &str) {
    eprintln!("{}", crate::mask::mask(line).0);
}

/// A failure of the records: 500 `store-failed` with a new request id and a fixed message. The
/// detail (a database path, SQL text) goes to standard error under that id, never to the client.
fn store_failed(detail: String) -> Response {
    let request = uuid::Uuid::now_v7().to_string();
    log(&format!(
        "cortex: answer records: request {request}: {detail}"
    ));
    json_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        json!({
            "error": "store-failed",
            "message": "the answer records could not be read or written",
            "request_id": request,
        }),
    )
}

/// Runs `work` on the blocking pool: the records are SQLite.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, Box<Response>> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|e| Box::new(store_failed(format!("the store call stopped: {e}"))))
}

/// The SSE event of one `cortex.ask.AskEvent`: its name, and the event as one JSON object.
fn sse(event: &ma::AskEvent) -> Event {
    let name = match event {
        ma::AskEvent::Status(_) => "status",
        ma::AskEvent::Tool(_) => "tool",
        ma::AskEvent::Delta(_) => "delta",
        ma::AskEvent::Discard(_) => "discard",
        ma::AskEvent::Done(_) => "done",
        ma::AskEvent::Error(_) => "error",
    };
    let data = serde_json::to_string(&crate::model_map::ask_event(event))
        .expect("a generated wire type serialises");
    Event::default().event(name).data(data)
}

/// A refusal before the answer starts: `status`, and a stream of the one `error` event.
fn refusal(status: StatusCode, event: ma::AskEvent) -> Response {
    let retry_after = match &event {
        ma::AskEvent::Error(e) => e.retry_after,
        _ => None,
    };
    let stream = tokio_stream::iter([Ok::<_, Infallible>(sse(&event))]);
    let mut response = (status, Sse::new(stream)).into_response();
    if let Some(seconds) = retry_after {
        response
            .headers_mut()
            .insert("retry-after", HeaderValue::from(seconds));
    }
    response
}

fn bad_request(message: &str) -> Response {
    refusal(
        StatusCode::BAD_REQUEST,
        ma::AskEvent::Error(ma::AskError {
            reason: "bad-request".into(),
            message: message.into(),
            retry_after: None,
        }),
    )
}

/// The most bytes of a request body: a question is text, and a body above this is refused.
const BODY_LIMIT: usize = 1024 * 1024;

/// The most bytes of a body of the record routes (a vote, a close): a body above this is refused
/// with 413.
const RECORD_BODY_LIMIT: usize = 64 * 1024;

/// The body of a record route, or its refusal: 413 above [`RECORD_BODY_LIMIT`], 400 when it
/// cannot be read.
fn record_body(body: Result<Bytes, BytesRejection>) -> Result<Bytes, Box<Response>> {
    match body {
        Ok(body) => Ok(body),
        Err(rejection) if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE => {
            Err(Box::new(json_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "bad-request",
                format!("the body is above {RECORD_BODY_LIMIT} bytes"),
            )))
        }
        Err(_) => Err(Box::new(json_error(
            StatusCode::BAD_REQUEST,
            "bad-request",
            "the body cannot be read",
        ))),
    }
}

/// The stream of one answer: its events to the reader, and the tool calls made for its record.
struct Events {
    tx: tokio::sync::mpsc::UnboundedSender<ma::AskEvent>,
    tool_calls: Vec<ma::AskTool>,
}

impl super::Stream for Events {
    fn emit(&mut self, event: ma::AskEvent) -> bool {
        if let ma::AskEvent::Tool(tool) = &event {
            self.tool_calls.push(tool.clone());
        }
        self.tx.send(event).is_ok()
    }

    fn gone(&self) -> bool {
        self.tx.is_closed()
    }
}

async fn ask(
    State(service): State<Arc<Service>>,
    headers: HeaderMap,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    let header = &service.settings.ask.identity_header;
    let Some(identity) = identity_of(&service, &headers) else {
        return refusal(
            StatusCode::UNAUTHORIZED,
            final_event(&ma::AskOutcome::Unidentified {
                error: ma::Unidentified {
                    header: header.clone(),
                },
            }),
        );
    };
    let body = match body {
        Ok(body) => body,
        Err(rejection) if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE => {
            return refusal(
                StatusCode::PAYLOAD_TOO_LARGE,
                ma::AskEvent::Error(ma::AskError {
                    reason: "bad-request".into(),
                    message: format!("the body is above {BODY_LIMIT} bytes"),
                    retry_after: None,
                }),
            )
        }
        Err(_) => return bad_request("the body cannot be read"),
    };
    let question = match serde_json::from_slice::<Value>(&body) {
        Ok(v) => match v.get("question").and_then(Value::as_str).map(str::trim) {
            Some(q) if !q.is_empty() => q.to_string(),
            _ => return bad_request("the body names no `question`"),
        },
        Err(_) => return bad_request("the body is not JSON"),
    };
    // A question still holding a class `refuse_if_left` names is refused before it spends a
    // question of the minute, and before anything is sent.
    if let Some(why) = super::question_refused(&service.settings, &question) {
        return bad_request(&why);
    }
    // Past `max_in_flight` answers the question is refused at once, before it spends a question
    // of the minute. The permit lives exactly as long as the answer's blocking thread: an answer
    // whose reader left still holds its `ekr mcp` reader and its model call until that call
    // replies or `max_seconds` passes, so it keeps its slot until then.
    let Ok(permit) = service.in_flight.clone().try_acquire_owned() else {
        return refusal(
            StatusCode::TOO_MANY_REQUESTS,
            final_event(&ma::AskOutcome::Busy {
                error: ma::Busy {
                    max_in_flight: service.settings.max_in_flight() as i64,
                },
            }),
        );
    };
    let admitted = service
        .limiter
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .admit(&identity, Instant::now());
    if let Err(retry_after) = admitted {
        return refusal(
            StatusCode::TOO_MANY_REQUESTS,
            final_event(&ma::AskOutcome::RateLimited {
                error: ma::RateLimited {
                    identity,
                    retry_after,
                },
            }),
        );
    }
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let service = service.clone();
    let (started, at_ms) = (Instant::now(), now_ms());
    tokio::task::spawn_blocking(move || {
        // Owned by this thread and nothing else, so no path gives the slot back before the
        // answer's reader is killed and its model call has ended.
        let _permit = permit;
        let answer_id = uuid::Uuid::now_v7().to_string();
        let mut stream = Events {
            tx,
            tool_calls: Vec::new(),
        };
        let outcome = answer(
            &service.settings,
            &answer_id,
            &identity,
            &question,
            &mut stream,
        );
        let Events { tx, tool_calls } = stream;
        // The answer's id is its request id: the detail of a failure (a gateway's body, a program
        // path) goes to standard error under it, masked, and the client's `error` names it.
        if let Some(detail) = super::detail(&outcome) {
            log(&format!("cortex: ask: request {answer_id}: {detail}"));
        }
        let mut last = final_event(&outcome);
        if let ma::AskEvent::Error(e) = &mut last {
            e.message.push_str(&format!(" (request {answer_id})"));
        }
        let asked = Asked {
            answer_id,
            identity,
            question,
            tool_calls,
            latency_ms: started.elapsed().as_millis() as i64,
            at_ms,
        };
        // The record is kept before the last event, so a reader who saw it can vote on it.
        let request = asked.answer_id.clone();
        if let Err(e) = service.keeper.record(asked, &outcome) {
            log(&format!("cortex: answer records: request {request}: {e}"));
        }
        let _ = tx.send(last);
    });
    let stream = UnboundedReceiverStream::new(rx).map(|event| Ok::<_, Infallible>(sse(&event)));
    Sse::new(stream).into_response()
}

/// `POST /ask/{answer_id}/vote` with `{"value": "up"|"down", "comment"?: "…"}`: the vote as kept.
async fn vote(
    State(service): State<Arc<Service>>,
    Path(answer_id): Path<String>,
    headers: HeaderMap,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    let Some(identity) = identity_of(&service, &headers) else {
        return unidentified(&service);
    };
    let body = match record_body(body) {
        Ok(body) => body,
        Err(refused) => return *refused,
    };
    let Ok(body) = serde_json::from_slice::<Value>(&body) else {
        return json_error(
            StatusCode::BAD_REQUEST,
            "bad-request",
            "the body is not JSON",
        );
    };
    let Some(value) = body
        .get("value")
        .and_then(Value::as_str)
        .and_then(records::value_of)
    else {
        return json_error(
            StatusCode::BAD_REQUEST,
            "bad-request",
            "`value` is `up` or `down`",
        );
    };
    let comment = match body.get("comment") {
        None | Some(Value::Null) => None,
        Some(Value::String(c)) => Some(c.clone()),
        Some(_) => {
            return json_error(
                StatusCode::BAD_REQUEST,
                "bad-request",
                "`comment` is a string",
            )
        }
    };
    let keeper = service.keeper.clone();
    let voted = match blocking(move || keeper.vote(&answer_id, &identity, value, comment)).await {
        Ok(voted) => voted,
        Err(response) => return *response,
    };
    match voted {
        Ok(vote) => json_response(
            StatusCode::OK,
            json!({
                "vote_id": vote.vote_id.0,
                "answer_id": vote.answer_id.0,
                "identity": vote.identity,
                "value": records::value_name(vote.value),
                "comment": vote.comment,
                "at": vote.at.0,
            }),
        ),
        Err(VoteRefused::NoSuchAnswer) => json_error(
            StatusCode::NOT_FOUND,
            "no-such-answer",
            "no answer record carries that id",
        ),
        Err(VoteRefused::Store(reason)) => store_failed(reason),
    }
}

/// `GET /review`: `{"reviews": [...]}`, the open review items, oldest first.
async fn review(State(service): State<Arc<Service>>, headers: HeaderMap) -> Response {
    if identity_of(&service, &headers).is_none() {
        return unidentified(&service);
    }
    let keeper = service.keeper.clone();
    match blocking(move || keeper.open_reviews()).await {
        Ok(Ok(items)) => {
            let reviews: Vec<Value> = items
                .into_iter()
                .map(|item| {
                    json!({
                        "review_id": item.review_id.0,
                        "answer_id": item.answer_id.0,
                        "cause": records::cause_name(item.cause),
                        "opened_at": item.opened_at.0,
                    })
                })
                .collect();
            json_response(StatusCode::OK, json!({ "reviews": reviews }))
        }
        Ok(Err(reason)) => store_failed(reason),
        Err(response) => *response,
    }
}

/// An optional list of strings that are not blank in a close's body: absent is empty, anything
/// else is the refusal's message.
fn expectations(body: &Value, key: &str) -> Result<Vec<String>, String> {
    let refused = || format!("`{key}` is not a list of strings that are not blank");
    match body.get(key) {
        None | Some(Value::Null) => Ok(vec![]),
        Some(Value::Array(items)) => items
            .iter()
            .map(|i| match i.as_str() {
                Some(s) if !s.trim().is_empty() => Ok(s.to_string()),
                _ => Err(refused()),
            })
            .collect(),
        Some(_) => Err(refused()),
    }
}

/// `POST /review/{review_id}/close` with `{"reason": "…", "expect_facts": […], "expect_nodes":
/// […]}`: the item as kept after it.
async fn close(
    State(service): State<Arc<Service>>,
    Path(review_id): Path<String>,
    headers: HeaderMap,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    if identity_of(&service, &headers).is_none() {
        return unidentified(&service);
    }
    let body = match record_body(body) {
        Ok(body) => body,
        Err(refused) => return *refused,
    };
    let body = serde_json::from_slice::<Value>(&body).ok();
    let expected = body.as_ref().map(|b| {
        expectations(b, "expect_facts")
            .and_then(|facts| expectations(b, "expect_nodes").map(|nodes| (facts, nodes)))
    });
    let reason = body
        .as_ref()
        .and_then(|b| {
            b.get("reason")
                .and_then(Value::as_str)
                .map(str::trim)
                .map(str::to_string)
        })
        .filter(|r| !r.is_empty());
    let Some(reason) = reason else {
        return json_error(
            StatusCode::BAD_REQUEST,
            "bad-request",
            "the body names no `reason`",
        );
    };
    let (facts, nodes) = match expected {
        Some(Ok(lists)) => lists,
        Some(Err(message)) => return json_error(StatusCode::BAD_REQUEST, "bad-request", message),
        None => (vec![], vec![]),
    };
    let keeper = service.keeper.clone();
    let closed = match blocking(move || keeper.close(&review_id, &reason, facts, nodes)).await {
        Ok(Ok(closed)) => closed,
        Ok(Err(reason)) => return store_failed(reason),
        Err(response) => return *response,
    };
    match closed {
        (ma::CloseReviewOutcome::Closed { .. }, Some(item)) => json_response(
            StatusCode::OK,
            json!({
                "review_id": item.data.review_id.0,
                "answer_id": item.data.answer_id.0,
                "cause": records::cause_name(item.data.cause),
                "state": records::review_state_name(item.state),
                "opened_at": item.data.opened_at.0,
                "close_reason": item.data.close_reason,
                "closed_at": item.data.closed_at.map(|t| t.0),
                "expect_facts": item.data.expect_facts.unwrap_or_default(),
                "expect_nodes": item.data.expect_nodes.unwrap_or_default(),
            }),
        ),
        (ma::CloseReviewOutcome::Closed { review_closed }, None) => store_failed(format!(
            "the review item {} was not kept",
            review_closed.review_id.0
        )),
        (ma::CloseReviewOutcome::WrongState { error }, _) => json_error(
            StatusCode::CONFLICT,
            "review-not-open",
            format!("the review item {} is closed already", error.review_id.0),
        ),
        (ma::CloseReviewOutcome::NoSuchReview { .. }, _) => json_error(
            StatusCode::NOT_FOUND,
            "no-such-review",
            "no review item carries that id",
        ),
    }
}

/// How often the service purges expired records after the purge it makes when it starts.
const PURGE_EVERY: Duration = Duration::from_secs(24 * 3600);

/// Binds `ask.listen`, prints `{"listening": "<address>"}` on one line, and answers until the
/// process is stopped. Records past retention are purged before the ready line (a failure there
/// stops the service) and every 24 hours after.
pub fn serve(settings: Settings, keeper: Keeper) -> Result<(), String> {
    keeper
        .purge(now_ms())
        .map_err(|e| format!("answer records: the purge failed: {e}"))?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()
        .map_err(|e| format!("cannot start the runtime: {e}"))?;
    runtime.block_on(async move {
        let listen = settings.ask.listen.clone();
        let listener = tokio::net::TcpListener::bind(&listen)
            .await
            .map_err(|e| format!("ask.listen: cannot bind {listen}: {e}"))?;
        let address = listener
            .local_addr()
            .map_err(|e| format!("ask.listen: {e}"))?;
        let per_minute = settings.ask.limits.per_identity_per_minute as usize;
        let in_flight = Arc::new(Semaphore::new(settings.max_in_flight()));
        let service = Arc::new(Service {
            settings,
            limiter: Mutex::new(RateLimiter::new(per_minute)),
            in_flight,
            keeper,
        });
        let keeper = service.keeper.clone();
        tokio::spawn(async move {
            let mut every =
                tokio::time::interval_at(tokio::time::Instant::now() + PURGE_EVERY, PURGE_EVERY);
            loop {
                every.tick().await;
                let keeper = keeper.clone();
                match tokio::task::spawn_blocking(move || keeper.purge(now_ms())).await {
                    Ok(Ok(_)) => {}
                    Ok(Err(e)) => log(&format!("cortex: answer records: the purge failed: {e}")),
                    Err(e) => log(&format!("cortex: answer records: the purge stopped: {e}")),
                }
            }
        });
        let record_limit = || DefaultBodyLimit::max(RECORD_BODY_LIMIT);
        let app = Router::new()
            .route("/ask", post(ask))
            .route("/ask/{answer_id}/vote", post(vote).layer(record_limit()))
            .route("/review", get(review).layer(record_limit()))
            .route(
                "/review/{review_id}/close",
                post(close).layer(record_limit()),
            )
            .layer(DefaultBodyLimit::max(BODY_LIMIT))
            .with_state(service);
        println!("{}", json!({"listening": address.to_string()}));
        axum::serve(listener, app)
            .await
            .map_err(|e| format!("the service stopped: {e}"))
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    /// Every `.rs` file under `dir`, with its text.
    fn sources(dir: &Path, into: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                sources(&path, into);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let text = std::fs::read_to_string(&path).unwrap();
                into.push((path.display().to_string(), text));
            }
        }
    }

    /// The class of the security review's finding at `serve.rs:296`: a line the ask service
    /// writes to standard error can carry a gateway's body or a store's error, and must be
    /// masked. So the one write to standard error under `src/ask/` is [`super::log`]'s, which
    /// masks; a new log line written any other way fails here.
    #[test]
    fn every_log_line_of_the_ask_service_goes_through_the_mask() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ask");
        let mut files = Vec::new();
        sources(&dir, &mut files);
        assert!(files.len() > 5, "src/ask/ was not read: {files:?}");
        // Split, so this test's own text is not a match.
        let writes = [concat!("eprint", "ln!("), concat!("eprint", "!(")];
        let found: Vec<(String, usize)> = files
            .iter()
            .flat_map(|(path, text)| {
                text.lines()
                    .enumerate()
                    .filter(|(_, line)| writes.iter().any(|w| line.contains(w)))
                    .map(move |(n, _)| (path.clone(), n + 1))
            })
            .collect();
        let serve = &files
            .iter()
            .find(|(p, _)| p.ends_with("serve.rs"))
            .unwrap()
            .1;
        let masked = format!(
            "fn log(line: &str) {{\n    {}\"{{}}\", crate::mask::mask(line).0);",
            writes[0]
        );
        assert!(
            found.len() == 1 && serve.contains(&masked),
            "a line goes to standard error without `log`'s mask: {found:?}"
        );
    }
}
