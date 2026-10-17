//! The HTTP side of `cortex ask serve`: `POST /ask` answers `text/event-stream`, one SSE event per
//! `cortex.ask.AskEvent`, in the generated wire form (`model_map::ask_event`).

use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::Router;
use cortex_model::ask as ma;
use serde_json::{json, Value};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt;

use super::limit::RateLimiter;
use super::{answer, final_event, Settings};

struct Service {
    settings: Settings,
    limiter: Mutex<RateLimiter>,
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

async fn ask(State(service): State<Arc<Service>>, headers: HeaderMap, body: Bytes) -> Response {
    let header = &service.settings.ask.identity_header;
    let identity = headers
        .get(header.as_str())
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string);
    let Some(identity) = identity else {
        return refusal(
            StatusCode::UNAUTHORIZED,
            final_event(&ma::AskOutcome::Unidentified {
                error: ma::Unidentified {
                    header: header.clone(),
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
    let question = match serde_json::from_slice::<Value>(&body) {
        Ok(v) => match v.get("question").and_then(Value::as_str).map(str::trim) {
            Some(q) if !q.is_empty() => q.to_string(),
            _ => return bad_request("the body names no `question`"),
        },
        Err(_) => return bad_request("the body is not JSON"),
    };
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let service = service.clone();
    tokio::task::spawn_blocking(move || {
        let outcome = answer(&service.settings, &identity, &question, &mut |event| {
            tx.send(event).is_ok()
        });
        let _ = tx.send(final_event(&outcome));
    });
    let stream = UnboundedReceiverStream::new(rx).map(|event| Ok::<_, Infallible>(sse(&event)));
    Sse::new(stream).into_response()
}

/// Binds `ask.listen`, prints `{"listening": "<address>"}` on one line, and answers until the
/// process is stopped.
pub fn serve(settings: Settings) -> Result<(), String> {
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
        let service = Arc::new(Service {
            settings,
            limiter: Mutex::new(RateLimiter::new(per_minute)),
        });
        let app = Router::new().route("/ask", post(ask)).with_state(service);
        println!("{}", json!({"listening": address.to_string()}));
        axum::serve(listener, app)
            .await
            .map_err(|e| format!("the service stopped: {e}"))
    })
}
