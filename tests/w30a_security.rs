//! Security review of wave 20261030a: the `cortex ask serve` invariants of
//! `story:ask-error-text`, `story:ask-service-bounds` and the always-on mask
//! (`story:mask-quoted-credentials`), driven through the real binary.
//!
//! The world: a stand-in OpenAI-compatible gateway in this process (one thread per connection,
//! a fixed status, body and delay) and a stand-in `ekr` whose `mcp` speaks MCP over stdio and
//! records its pid in `<root>/ekr.pids`. Credential-shaped values are assembled at runtime so no
//! such literal sits in the source.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::json;

const TOKEN_ENV: &str = "W30A_SEC_GATEWAY_TOKEN";
const HEADER: &str = "x-asker";

/// A stand-in gateway answering every request with `status` and `body` after `delay`.
struct Gateway {
    url: String,
    asked: Arc<AtomicUsize>,
}

impl Gateway {
    fn start(status: &'static str, body: String, delay: Duration) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let asked = Arc::new(AtomicUsize::new(0));
        let seen = asked.clone();
        let body = Arc::new(body);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { return };
                let (seen, body) = (seen.clone(), body.clone());
                std::thread::spawn(move || {
                    drain_request(&stream);
                    seen.fetch_add(1, Ordering::SeqCst);
                    std::thread::sleep(delay);
                    let _ = write!(
                        stream,
                        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                        body.len()
                    );
                });
            }
        });
        Self { url, asked }
    }
}

fn drain_request(stream: &TcpStream) {
    let mut reader = BufReader::new(stream);
    let mut length = 0;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                length = value.trim().parse().unwrap_or(0);
            }
        }
    }
    let mut body = vec![0; length];
    let _ = reader.read_exact(&mut body);
}

fn executable(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, format!("#!/bin/sh\n{body}")).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// A stand-in `ekr mcp`: answers `initialize` and `tools/list`, and records its pid.
fn stand_in_ekr(root: &Path) -> PathBuf {
    let path = root.join("bin/ekr");
    let body = r#"R="@ROOT@"
[ "$1" = mcp ] || exit 2
echo $$ >> "$R/ekr.pids"
while IFS= read -r line; do
  id=$(printf '%s' "$line" | head -c 200 | sed -n 's/.*"id":\([0-9][0-9]*\).*/\1/p')
  case "$line" in
    *'"method":"initialize"'*) printf '{"jsonrpc":"2.0","id":%s,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"ekr","version":"0"}}}\n' "$id" ;;
    *'"method":"tools/list"'*) printf '{"jsonrpc":"2.0","id":%s,"result":{"tools":[{"name":"search","description":"Search","inputSchema":{"type":"object"}}]}}\n' "$id" ;;
  esac
done
"#
    .replace("@ROOT@", &root.display().to_string());
    executable(&path, body.as_str());
    path
}

struct Service {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    child: Child,
    addr: String,
}

impl Service {
    fn reader_pids(&self) -> Vec<u32> {
        std::fs::read_to_string(self.root.join("ekr.pids"))
            .unwrap_or_default()
            .lines()
            .filter_map(|l| l.trim().parse().ok())
            .collect()
    }

    /// The readers this service started that are still running (a zombie is not running).
    fn live_readers(&self) -> Vec<u32> {
        self.reader_pids()
            .into_iter()
            .filter(|pid| {
                std::fs::read_to_string(format!("/proc/{pid}/stat"))
                    .ok()
                    .and_then(|s| {
                        s.rsplit_once(')')
                            .map(|(_, rest)| rest.trim_start().to_string())
                    })
                    .is_some_and(|rest| !rest.starts_with('Z'))
            })
            .collect()
    }

    fn stderr(&self) -> String {
        std::fs::read_to_string(self.root.join("service.stderr")).unwrap_or_default()
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        for pid in self.reader_pids() {
            if Path::new(&format!("/proc/{pid}")).exists() {
                let _ = Command::new("kill")
                    .arg("-9")
                    .arg(pid.to_string())
                    .stderr(Stdio::null())
                    .status();
            }
        }
    }
}

/// Starts `cortex ask serve` with its standard error in `<root>/service.stderr`.
fn serve(gateway: &Gateway, limits: &str, ask_extra: &str) -> Service {
    let tmp = tempfile::Builder::new()
        .prefix("cortex-w30a-sec")
        .tempdir_in(
            std::env::var_os("CARGO_TARGET_TMPDIR")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir),
        )
        .unwrap();
    let root = tmp.path().to_path_buf();
    std::fs::create_dir_all(root.join("bin")).unwrap();
    let ekr = stand_in_ekr(&root);
    let spec = root.join("asker.yaml");
    std::fs::write(
        &spec,
        format!(
            r#"format: cortex.instance/1
name: asker
description: Test brain.
ekr: {{version: "0.0.34", bin: "{ekr}"}}
seed: {{documents: []}}
model:
  model: test-model
  budget_tokens: 100000
  timeout_s: 60
  gateway: {{base_url: "{url}", token_env: {TOKEN_ENV}}}
sources: []
serve: {{view_port: 18999}}
ask:
  listen: "127.0.0.1:0"
  identity_header: {HEADER}
  limits: {limits}
{ask_extra}
"#,
            ekr = ekr.display(),
            url = gateway.url,
        ),
    )
    .unwrap();
    let stderr = std::fs::File::create(root.join("service.stderr")).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["ask", "serve", "--spec"])
        .arg(&spec)
        .env("CORTEX_HOME", root.join("home"))
        .env(TOKEN_ENV, "w30a-sec-gateway-token")
        .stdout(Stdio::piped())
        .stderr(Stdio::from(stderr))
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let ready: serde_json::Value = serde_json::from_str(line.trim())
        .unwrap_or_else(|_| panic!("`cortex ask serve` did not start: {line:?}"));
    let addr = ready["listening"].as_str().unwrap().to_string();
    Service {
        _tmp: tmp,
        root,
        child,
        addr,
    }
}

/// Asks `question` as `identity` and answers the whole SSE text.
fn ask(service: &Service, identity: &str, question: &str) -> String {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap()
        .post(format!("http://{}/ask", service.addr))
        .header(HEADER, identity)
        .header("content-type", "application/json")
        .body(json!({"question": question}).to_string())
        .send()
        .unwrap()
        .text()
        .unwrap()
}

/// Posts a question as `identity` on a raw connection, reads until `marker`, and closes it.
fn leave_after(service: &Service, identity: &str, marker: &str) {
    let body = json!({"question": "Leave?"}).to_string();
    let mut stream = TcpStream::connect(&service.addr).unwrap();
    write!(
        stream,
        "POST /ask HTTP/1.1\r\nhost: x\r\n{HEADER}: {identity}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let mut seen = Vec::new();
    let mut buf = [0; 4096];
    while !String::from_utf8_lossy(&seen).contains(marker) {
        let n = stream.read(&mut buf).unwrap();
        assert!(n > 0, "the stream ended before {marker:?}");
        seen.extend_from_slice(&buf[..n]);
    }
    drop(stream);
}

const LIMITS: &str =
    "{max_steps: 4, max_result_bytes: 4096, max_seconds: 30, per_identity_per_minute: 10}";

/// `story:ask-error-text` moved a gateway's failure body out of the client's `error` event and
/// onto a new standard-error line (`src/ask/serve.rs`, `cortex: ask: request <id>: <detail>`).
/// That line is written as the gateway sent it: a credential shape the always-on mask covers
/// (here an `sk-` API key a gateway echoes in its 401) reaches the service's log unmasked. Every
/// other place that text could reach is masked (`src/ask/records.rs` `masked`).
#[test]
fn a_gateway_failure_reaches_neither_the_client_nor_the_log_with_a_credential_in_it() {
    let key = format!("sk-{}", "Qx7".repeat(9));
    let gateway = Gateway::start(
        "401 Unauthorized",
        json!({"error": {"message": format!("Incorrect API key provided: {key}")}}).to_string(),
        Duration::ZERO,
    );
    let service = serve(&gateway, LIMITS, "");
    let answer = ask(&service, "ann", "What is a widget?");
    assert!(answer.contains("model-failed"), "{answer}");
    assert!(!answer.contains(&key), "the client got the key: {answer}");
    let log = service.stderr();
    assert!(
        log.contains("cortex: ask: request "),
        "no detail line was logged: {log}"
    );
    assert!(
        !log.contains(&key),
        "the service's log carries the gateway's key unmasked: {log}"
    );
}

/// `website/docs/ask-service.md` ("At most `max_in_flight` readers run at once") and the
/// CHANGELOG ("`ask.max_in_flight` caps the answers worked on at once") bound the readers and
/// answers in flight over every identity. A reader who leaves while the model is asked frees the
/// answer's slot at once (`Events::left`), but the answer's thread keeps its `ekr mcp` reader and
/// its gateway call until the call returns, up to `max_seconds`. So clients who each leave after
/// "asking the model" hold any number of readers and model calls with `max_in_flight: 1`; the
/// identity header is whatever the client sends, so the per-identity minute limit bounds nothing
/// across them.
#[test]
fn readers_of_departed_answers_stay_within_max_in_flight() {
    let gateway = Gateway::start(
        "200 OK",
        json!({
            "choices": [{"index": 0, "finish_reason": "stop",
                "message": {"role": "assistant", "content": "Yes."}}],
            "usage": {"prompt_tokens": 10, "completion_tokens": 1}
        })
        .to_string(),
        Duration::from_secs(6),
    );
    let service = serve(&gateway, LIMITS, "  max_in_flight: 1");
    for identity in ["ann", "ben", "cee"] {
        leave_after(&service, identity, "asking the model");
        // Let the answer's wait notice the reader left (it polls every 25 ms).
        std::thread::sleep(Duration::from_millis(200));
    }
    let started = Instant::now();
    while gateway.asked.load(Ordering::SeqCst) < 3 && started.elapsed() < Duration::from_secs(3)
    {
        std::thread::sleep(Duration::from_millis(20));
    }
    let live = service.live_readers();
    let calls = gateway.asked.load(Ordering::SeqCst);
    assert!(
        live.len() <= 1 && calls <= 1,
        "max_in_flight 1, yet {} readers run ({live:?}) and the gateway was asked {calls} times \
         at once",
        live.len()
    );
}
