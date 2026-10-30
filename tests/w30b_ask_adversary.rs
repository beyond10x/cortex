//! Adversarial cases for `story:ask-body-limit-test-race` (wave 20261030b): the ask service reads
//! a refused body to its end (up to 16 MiB, for at most 10 s) before its 413, and the eval `mcp`
//! runner's redacting server (`ask::proxy`) reads up to 16 MiB of the declared body before its 401,
//! 404, 405 and 413. These cases attack the limits at their exact edges, slow and lying senders,
//! chunked bodies, the refusals that still come before the body is read, and the cost of holding
//! connections open in a drain.

mod common;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use common::executable;
use cortex_cli::ask::{proxy, ReaderCommand};
use cortex_cli::redact::{compile, Redactor};
use cortex_model::instance as m;
use serde_json::{json, Value};

const TOKEN_ENV: &str = "W30B_ADV_GATEWAY_TOKEN";
const TOKEN: &str = "w30b-adversary-gateway-token";
const HEADER: &str = "x-asker";
const MIB: usize = 1024 * 1024;
const ASK_LIMIT: usize = MIB;
const RECORD_LIMIT: usize = 64 * 1024;
const DRAIN_LIMIT: usize = 16 * MIB;

// ---------------------------------------------------------------------------------------------
// The world of `cortex ask serve`: a stand-in gateway and a stand-in `ekr mcp`.

/// A stand-in OpenAI-compatible gateway that answers every request with one text reply.
fn gateway() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let reply = json!({
        "id": "final",
        "object": "chat.completion",
        "choices": [{
            "index": 0,
            "finish_reason": "stop",
            "message": {"role": "assistant", "content": "Yes [node:n-1]."}
        }],
        "usage": {"prompt_tokens": 10, "completion_tokens": 1}
    })
    .to_string();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            let reply = reply.clone();
            std::thread::spawn(move || {
                let mut reader = BufReader::new(&stream);
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
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}",
                    reply.len()
                );
            });
        }
    });
    url
}

struct Service {
    _tmp: tempfile::TempDir,
    child: Child,
    addr: String,
}

impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn scratch(prefix: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(prefix)
        .tempdir_in(
            std::env::var_os("CARGO_TARGET_TMPDIR")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir),
        )
        .unwrap()
}

/// Starts `cortex ask serve` against a stand-in gateway and a stand-in `ekr mcp`.
fn serve() -> Service {
    let url = gateway();
    let tmp = scratch("cortex-w30b-adv");
    let root = tmp.path().to_path_buf();
    std::fs::create_dir_all(root.join("bin")).unwrap();
    let ekr = root.join("bin/ekr");
    executable(
        &ekr,
        r#"[ "$1" = mcp ] || exit 2
while IFS= read -r line; do
  id=$(printf '%s' "$line" | head -c 200 | sed -n 's/.*"id":\([0-9][0-9]*\).*/\1/p')
  case "$line" in
    *'"method":"initialize"'*) printf '{"jsonrpc":"2.0","id":%s,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"ekr","version":"0"}}}\n' "$id" ;;
    *'"method":"tools/list"'*) printf '{"jsonrpc":"2.0","id":%s,"result":{"tools":[{"name":"search","description":"Search","inputSchema":{"type":"object"}}]}}\n' "$id" ;;
    *'"method":"tools/call"'*) printf '{"jsonrpc":"2.0","id":%s,"result":{"content":[{"type":"text","text":"a result"}]}}\n' "$id" ;;
  esac
done
"#,
    );
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
  limits: {{max_steps: 4, max_result_bytes: 4096, max_seconds: 30, per_identity_per_minute: 1000}}
"#,
            ekr = ekr.display(),
        ),
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["ask", "serve", "--spec"])
        .arg(&spec)
        .env("CORTEX_HOME", root.join("home"))
        .env(TOKEN_ENV, TOKEN)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let ready: Value = serde_json::from_str(line.trim())
        .unwrap_or_else(|_| panic!("`cortex ask serve` did not start: {line}"));
    let addr = ready["listening"].as_str().unwrap().to_string();
    Service {
        _tmp: tmp,
        child,
        addr,
    }
}

/// `{"<key>":"xxx…"}` with `prefix` before the key's value, exactly `total` bytes long.
fn padded(prefix: &str, total: usize) -> Vec<u8> {
    let fill = total - prefix.len() - 2;
    let mut body = prefix.as_bytes().to_vec();
    body.extend(std::iter::repeat_n(b'x', fill));
    body.extend_from_slice(b"\"}");
    assert_eq!(body.len(), total);
    body
}

/// One request on a connection of its own, the whole body written before anything is read: the
/// response's first line, or how the connection failed.
fn send_then_read(
    addr: &str,
    method: &str,
    path: &str,
    extra: &str,
    body: &[u8],
) -> Result<String, String> {
    let mut s = TcpStream::connect(addr).map_err(|e| format!("connect: {e}"))?;
    s.set_read_timeout(Some(Duration::from_secs(25))).unwrap();
    s.set_write_timeout(Some(Duration::from_secs(25))).unwrap();
    let head = format!(
        "{method} {path} HTTP/1.1\r\nhost: {addr}\r\n{extra}content-type: application/json\r\n\
         content-length: {}\r\nconnection: close\r\n\r\n",
        body.len()
    );
    s.write_all(head.as_bytes())
        .and_then(|()| s.write_all(body))
        .map_err(|e| format!("write: {e}"))?;
    let mut out = Vec::new();
    s.read_to_end(&mut out).map_err(|e| format!("read: {e}"))?;
    Ok(String::from_utf8_lossy(&out)
        .lines()
        .next()
        .unwrap_or("")
        .to_string())
}

fn id(identity: &str) -> String {
    format!("{HEADER}: {identity}\r\n")
}

// ---------------------------------------------------------------------------------------------
// The service: the limits at their exact edges

/// A question body of exactly 1 MiB is admitted; one byte more is refused with 413, and the client
/// still sending it reads the refusal.
#[test]
fn an_ask_body_of_exactly_one_mib_is_admitted_and_one_byte_more_is_refused() {
    let service = serve();
    let at = send_then_read(
        &service.addr,
        "POST",
        "/ask",
        &id("ivo"),
        &padded("{\"question\":\"", ASK_LIMIT),
    );
    assert_eq!(at.as_deref(), Ok("HTTP/1.1 200 OK"), "exactly {ASK_LIMIT}");
    let over = send_then_read(
        &service.addr,
        "POST",
        "/ask",
        &id("ivo"),
        &padded("{\"question\":\"", ASK_LIMIT + 1),
    );
    assert_eq!(
        over.as_deref(),
        Ok("HTTP/1.1 413 Payload Too Large"),
        "one byte over"
    );
}

/// A vote or a close body of exactly 64 KiB is read and judged on its content (not 413); one byte
/// more is refused with 413.
#[test]
fn a_record_body_of_exactly_64_kib_is_admitted_and_one_byte_more_is_refused() {
    let service = serve();
    for path in ["/ask/no-such-answer/vote", "/review/no-such-review/close"] {
        let prefix = "{\"value\":\"up\",\"comment\":\"";
        let at = send_then_read(
            &service.addr,
            "POST",
            path,
            &id("ivo"),
            &padded(prefix, RECORD_LIMIT),
        )
        .unwrap_or_else(|e| panic!("{path}, exactly {RECORD_LIMIT}: {e}"));
        assert!(
            !at.contains(" 413 ") && at.starts_with("HTTP/1.1 4"),
            "{path}, exactly {RECORD_LIMIT}: {at}"
        );
        let over = send_then_read(
            &service.addr,
            "POST",
            path,
            &id("ivo"),
            &padded(prefix, RECORD_LIMIT + 1),
        );
        assert_eq!(
            over.as_deref(),
            Ok("HTTP/1.1 413 Payload Too Large"),
            "{path}, one byte over"
        );
    }
}

/// The drain's own bound: a body of exactly 16 MiB is read to its end, so the client still sending
/// it reads the 413, on `/ask` and on a record route.
#[test]
fn a_body_of_exactly_the_drain_limit_still_reads_the_refusal() {
    let service = serve();
    let body = vec![b'x'; DRAIN_LIMIT];
    for path in ["/ask", "/ask/no-such-answer/vote"] {
        let out = send_then_read(&service.addr, "POST", path, &id("ivo"), &body);
        assert_eq!(
            out.as_deref(),
            Ok("HTTP/1.1 413 Payload Too Large"),
            "{path}"
        );
    }
}

/// A caller without the identity header still sending an over-limit body reads its 401 on every
/// route that reads a body.
#[test]
fn an_unidentified_client_still_sending_an_over_limit_body_reads_its_401() {
    let service = serve();
    let body = vec![b'x'; 3 * MIB];
    for path in [
        "/ask",
        "/ask/no-such-answer/vote",
        "/review/no-such-review/close",
    ] {
        let out = send_then_read(&service.addr, "POST", path, "", &body);
        let out = out.unwrap_or_else(|e| panic!("{path}: {e}"));
        assert!(out.starts_with("HTTP/1.1 401"), "{path}: {out}");
    }
}

// ---------------------------------------------------------------------------------------------
// The service: slow, lying and endless senders

/// Opens a connection that declares `declared` bytes for `/ask`, sends `sent` of them and then
/// stops sending without closing. Returns the stream once the bytes are written.
fn stalled_sender(addr: &str, declared: usize, sent: &[u8]) -> TcpStream {
    let mut s = TcpStream::connect(addr).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(25))).unwrap();
    s.set_write_timeout(Some(Duration::from_secs(25))).unwrap();
    let head = format!(
        "POST /ask HTTP/1.1\r\nhost: {addr}\r\n{HEADER}: slow\r\ncontent-type: application/json\r\n\
         content-length: {declared}\r\nconnection: close\r\n\r\n"
    );
    s.write_all(head.as_bytes()).unwrap();
    s.write_all(sent).unwrap();
    s
}

fn first_line(mut s: TcpStream) -> Result<String, String> {
    let mut out = Vec::new();
    s.read_to_end(&mut out).map_err(|e| format!("read: {e}"))?;
    Ok(String::from_utf8_lossy(&out)
        .lines()
        .next()
        .unwrap_or("")
        .to_string())
}

/// A sender that declares 2 MiB, sends one byte past the 1 MiB limit and then stops is answered
/// 413 within the drain time (10 s), not held until it gives up.
#[test]
fn a_sender_that_stops_past_the_limit_is_answered_within_the_drain_time() {
    let service = serve();
    let sent = vec![b'x'; ASK_LIMIT + 1];
    let s = stalled_sender(&service.addr, 2 * MIB, &sent);
    let from = Instant::now();
    let line = first_line(s);
    let took = from.elapsed();
    assert_eq!(line.as_deref(), Ok("HTTP/1.1 413 Payload Too Large"));
    assert!(took < Duration::from_secs(13), "the 413 took {took:?}");
    eprintln!("stalled sender: 413 after {took:?}");
}

/// The resource bound of the drain: 200 senders, each held in its 10 s drain at once, do not
/// starve a legitimate question. It is answered within 5 s while they are held, and each of them
/// is answered 413 within the drain time.
#[test]
fn senders_held_in_the_drain_do_not_starve_a_legitimate_question() {
    const N: usize = 200;
    let service = serve();
    let sent = Arc::new(vec![b'x'; ASK_LIMIT + 1]);
    let (tx, rx) = mpsc::channel();
    let mut handles = Vec::new();
    for _ in 0..N {
        let (addr, sent, tx) = (service.addr.clone(), sent.clone(), tx.clone());
        handles.push(std::thread::spawn(move || {
            let s = stalled_sender(&addr, 2 * MIB, &sent);
            let written = Instant::now();
            tx.send(()).unwrap();
            let line = first_line(s);
            (line, written.elapsed())
        }));
    }
    for i in 0..N {
        rx.recv_timeout(Duration::from_secs(10))
            .unwrap_or_else(|_| panic!("only {i} of {N} senders wrote their bytes"));
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .unwrap();
    let from = Instant::now();
    let response = client
        .post(format!("http://{}/ask", service.addr))
        .header(HEADER, "legit")
        .header("content-type", "application/json")
        .body(json!({"question": "Now?"}).to_string())
        .send()
        .expect("the legitimate question is answered");
    let status = response.status().as_u16();
    let text = response.text().unwrap_or_default();
    let legit = from.elapsed();
    assert_eq!(status, 200, "{text}");
    assert!(text.contains("event: done"), "{text}");
    assert!(
        legit < Duration::from_secs(5),
        "the question took {legit:?}"
    );
    let mut worst = Duration::ZERO;
    let mut lost = Vec::new();
    for h in handles {
        let (line, took) = h.join().unwrap();
        worst = worst.max(took);
        match line {
            Ok(l) if l.starts_with("HTTP/1.1 413") => {}
            other => lost.push(format!("{other:?}")),
        }
    }
    eprintln!("{N} held senders: legitimate question {legit:?}, slowest 413 {worst:?}");
    assert!(
        lost.is_empty(),
        "{} lost: {:?}",
        lost.len(),
        &lost[..lost.len().min(3)]
    );
    assert!(worst < Duration::from_secs(13), "slowest 413 {worst:?}");
}

/// A chunked body that never ends is read up to the drain limit and then refused: the client,
/// still writing, reads the 413 well within the drain time.
#[test]
fn a_chunked_body_that_never_ends_is_refused_at_the_drain_limit() {
    let service = serve();
    let mut s = TcpStream::connect(&service.addr).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
    let head = format!(
        "POST /ask HTTP/1.1\r\nhost: {}\r\n{HEADER}: ivo\r\ncontent-type: application/json\r\n\
         transfer-encoding: chunked\r\n\r\n",
        service.addr
    );
    s.write_all(head.as_bytes()).unwrap();
    let mut writer = s.try_clone().unwrap();
    let pump = std::thread::spawn(move || {
        let chunk = vec![b'x'; 64 * 1024];
        let mut sent = 0usize;
        // Stops at the first failed write, or after 256 MiB.
        while sent < 256 * MIB {
            let head = format!("{:x}\r\n", chunk.len());
            if writer
                .write_all(head.as_bytes())
                .and_then(|()| writer.write_all(&chunk))
                .and_then(|()| writer.write_all(b"\r\n"))
                .is_err()
            {
                break;
            }
            sent += chunk.len();
        }
        sent
    });
    let from = Instant::now();
    let mut line = String::new();
    let read = BufReader::new(&s).read_line(&mut line);
    let took = from.elapsed();
    let _ = s.shutdown(std::net::Shutdown::Both);
    let sent = pump.join().unwrap();
    eprintln!("endless chunked body: {line:?} after {took:?}, {sent} bytes written");
    assert!(read.is_ok(), "{read:?}");
    assert!(line.starts_with("HTTP/1.1 413"), "{line:?}");
    assert!(took < Duration::from_secs(10), "{took:?}");
}

/// A client that sends `expect: 100-continue` waits for the server before it sends its body. The
/// service tells it to go on (100) or refuses at once; it must not leave it waiting.
#[test]
fn an_expect_continue_client_is_not_left_waiting_by_the_service() {
    let service = serve();
    let mut s = TcpStream::connect(&service.addr).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(8))).unwrap();
    let head = format!(
        "POST /ask HTTP/1.1\r\nhost: {}\r\n{HEADER}: ivo\r\ncontent-type: application/json\r\n\
         content-length: {}\r\nexpect: 100-continue\r\n\r\n",
        service.addr,
        2 * MIB
    );
    s.write_all(head.as_bytes()).unwrap();
    let mut line = String::new();
    let read = BufReader::new(&s).read_line(&mut line);
    assert!(read.is_ok(), "{read:?}");
    assert!(
        line.starts_with("HTTP/1.1 100") || line.starts_with("HTTP/1.1 413"),
        "{line:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// The service: refusals that still come before the body is read

/// Every refusal the service makes, not only those of the three body routes, reaches a client
/// still sending its body: an unknown path (`/ask/`, a trailing slash), a wrong method (`GET
/// /ask`), and the `GET /review` route, none of which read the body.
#[test]
fn a_client_still_sending_its_body_reads_the_answers_of_routes_that_read_no_body() {
    let service = serve();
    let body = vec![b'x'; 4 * MIB];
    let cases = [
        ("POST", "/ask/", "HTTP/1.1 404"),
        ("GET", "/ask", "HTTP/1.1 405"),
        ("GET", "/review", "HTTP/1.1 200"),
    ];
    let mut lost = Vec::new();
    for (method, path, want) in cases {
        match send_then_read(&service.addr, method, path, &id("ivo"), &body) {
            Ok(line) if line.starts_with(want) => {}
            other => lost.push(format!("{method} {path}: {other:?}")),
        }
    }
    assert!(lost.is_empty(), "{lost:?}");
}

// ---------------------------------------------------------------------------------------------
// The redacting MCP server of the eval `mcp` runner

fn redactor() -> Redactor {
    compile(Some(&m::RedactionPolicy {
        classes: vec![m::RedactionClass::Email],
        rules: Vec::new(),
        known_names: None,
        rare_limit: None,
        refuse_if_left: None,
    }))
    .expect("a valid policy")
    .expect("a policy that redacts")
}

fn stand_in_reader(root: &Path) -> ReaderCommand {
    let path = root.join("ekr");
    executable(
        &path,
        r#"[ "$1" = mcp ] || exit 2
while IFS= read -r line; do
  id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9][0-9]*\).*/\1/p')
  case "$line" in
    *'"method":"initialize"'*) printf '{"jsonrpc":"2.0","id":%s,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"ekr","version":"0"}}}\n' "$id" ;;
    *'"method":"tools/list"'*) printf '{"jsonrpc":"2.0","id":%s,"result":{"tools":[{"name":"search","inputSchema":{"type":"object"}}]}}\n' "$id" ;;
  esac
done
"#,
    );
    ReaderCommand {
        program: path,
        args: vec!["mcp".into()],
        env: Vec::new(),
        remove: Vec::new(),
    }
}

/// The address, path and bearer of the server a `--mcp-config` document names.
struct Proxy {
    addr: String,
    path: String,
    bearer: String,
}

/// Runs `f` against a redacting server for one case.
fn with_proxy<T>(f: impl FnOnce(&Proxy) -> T) -> T {
    let tmp = scratch("cortex-w30b-proxy");
    let reader = stand_in_reader(tmp.path());
    let r = redactor();
    let deadline = Instant::now() + Duration::from_secs(120);
    proxy::serve(&r, &reader, "ekr", "q", deadline, |_, config| {
        let url = config["mcpServers"]["ekr"]["url"]
            .as_str()
            .unwrap()
            .strip_prefix("http://")
            .unwrap()
            .to_string();
        let at = url.find('/').unwrap();
        f(&Proxy {
            addr: url[..at].to_string(),
            path: url[at..].to_string(),
            bearer: config["mcpServers"]["ekr"]["headers"]["Authorization"]
                .as_str()
                .unwrap()
                .to_string(),
        })
    })
    .expect("the server starts")
    .answer
}

/// A `ping` padded with spaces to exactly `total` bytes.
fn ping(total: usize) -> Vec<u8> {
    let mut body = br#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#.to_vec();
    body.resize(total, b' ');
    body
}

/// A request of exactly 1 MiB is served; one byte more is refused with 413, and the client still
/// sending it reads the refusal.
#[test]
fn a_proxy_request_of_exactly_one_mib_is_served_and_one_byte_more_is_refused() {
    let lines = with_proxy(|p| {
        let auth = format!("authorization: {}\r\n", p.bearer);
        [MIB, MIB + 1].map(|n| send_then_read(&p.addr, "POST", &p.path, &auth, &ping(n)))
    });
    assert_eq!(lines[0].as_deref(), Ok("HTTP/1.1 200 OK"), "exactly 1 MiB");
    assert_eq!(
        lines[1].as_deref(),
        Ok("HTTP/1.1 413 Payload Too Large"),
        "one byte over"
    );
}

/// A client that frames its body with `transfer-encoding: chunked` and writes it whole before it
/// reads gets the server's refusal, as a client declaring `content-length` now does.
#[test]
#[ignore = "story:ignored-adversary-limits: the proxy drains only a declared content-length; a chunked body is left unread and the 401 is lost"]
fn a_proxy_client_sending_a_chunked_body_reads_its_refusal() {
    let out = with_proxy(|p| {
        let mut s = TcpStream::connect(&p.addr).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
        s.set_write_timeout(Some(Duration::from_secs(20))).unwrap();
        let mut request = format!(
            "POST {} HTTP/1.1\r\nhost: {}\r\ntransfer-encoding: chunked\r\n\r\n",
            p.path, p.addr
        )
        .into_bytes();
        let chunk = vec![b'x'; 64 * 1024];
        for _ in 0..64 {
            request.extend_from_slice(format!("{:x}\r\n", chunk.len()).as_bytes());
            request.extend_from_slice(&chunk);
            request.extend_from_slice(b"\r\n");
        }
        request.extend_from_slice(b"0\r\n\r\n");
        s.write_all(&request).map_err(|e| format!("write: {e}"))?;
        let mut out = String::new();
        s.read_to_string(&mut out)
            .map_err(|e| format!("read: {e}"))?;
        Ok::<_, String>(out.lines().next().unwrap_or("").to_string())
    });
    assert!(
        out.as_ref().is_ok_and(|l| l.starts_with("HTTP/1.1 401")),
        "{out:?}"
    );
}

/// A client that sends `expect: 100-continue` waits for the server before it sends its body. A
/// server that will refuse the request answers it at once (RFC 9110 10.1.1); the proxy did before
/// this change, for an unauthenticated request and for one above 1 MiB.
#[test]
fn a_proxy_refusal_is_not_held_back_from_an_expect_continue_client() {
    let seen = with_proxy(|p| {
        let auth = format!("authorization: {}\r\n", p.bearer);
        [("", "401"), (auth.as_str(), "413")].map(|(auth, want)| {
            let mut s = TcpStream::connect(&p.addr).unwrap();
            s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let head = format!(
                "POST {} HTTP/1.1\r\nhost: {}\r\n{auth}content-length: {}\r\n\
                 expect: 100-continue\r\n\r\n",
                p.path,
                p.addr,
                2 * MIB
            );
            s.write_all(head.as_bytes()).unwrap();
            let from = Instant::now();
            let mut line = String::new();
            let read = BufReader::new(&s).read_line(&mut line);
            (want, read.map(|_| line), from.elapsed())
        })
    });
    for (want, line, took) in seen {
        assert!(
            line.as_ref()
                .is_ok_and(|l| l.starts_with(&format!("HTTP/1.1 {want}"))),
            "{want}: {line:?} after {took:?}"
        );
    }
}

/// The cost of an unauthenticated flood: 200 connections that declare 16 MiB and send none of it
/// are each held in the drain, but the case's own client is still served within 2 s.
#[test]
fn unauthenticated_connections_held_in_the_drain_do_not_starve_the_case() {
    const N: usize = 200;
    let (served, held) = with_proxy(|p| {
        let mut flood = Vec::new();
        for _ in 0..N {
            let mut s = TcpStream::connect(&p.addr).unwrap();
            let head = format!(
                "POST {} HTTP/1.1\r\nhost: {}\r\ncontent-length: {DRAIN_LIMIT}\r\n\r\n",
                p.path, p.addr
            );
            s.write_all(head.as_bytes()).unwrap();
            flood.push(s);
        }
        std::thread::sleep(Duration::from_millis(500));
        let auth = format!("authorization: {}\r\n", p.bearer);
        let from = Instant::now();
        let line = send_then_read(&p.addr, "POST", &p.path, &auth, &ping(64));
        let served = (line, from.elapsed());
        // How many of the flood were answered in the 2 s after.
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut answered = 0;
        for s in &flood {
            let left = deadline.saturating_duration_since(Instant::now());
            s.set_read_timeout(Some(left.max(Duration::from_millis(1))))
                .unwrap();
            let mut buf = [0u8; 16];
            if matches!((&*s).read(&mut buf), Ok(n) if n > 0) {
                answered += 1;
            }
        }
        (served, N - answered)
    });
    eprintln!(
        "{N} unauthenticated declared-16-MiB connections: case client {:?} after {:?}, {held} still held after 2 s",
        served.0, served.1
    );
    assert_eq!(served.0.as_deref(), Ok("HTTP/1.1 200 OK"));
    assert!(served.1 < Duration::from_secs(2), "{:?}", served.1);
}
