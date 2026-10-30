//! Security conformance cases for `story:ask-redaction`: with a redaction policy no original value
//! of a redacted class, known name or rule reaches the model, a `refuse_if_left` result is
//! withheld, and the redacting MCP server of the eval `mcp` runner (`ask::proxy`) serves only the
//! case it was started for. The names and addresses are synthetic.

mod common;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use common::executable;
use cortex_cli::ask::{proxy, shown, ReaderCommand};
use cortex_cli::redact::{compile, Redactor};
use cortex_model::instance as m;
use serde_json::{json, Value};

/// The document the stand-in reader's one tool answers.
const PERSONAL: &str = "Zorblat leads the widget team, write to help@acme.example.";

fn redactor(policy: m::RedactionPolicy) -> Redactor {
    compile(Some(&policy))
        .expect("a valid policy")
        .expect("a policy that redacts")
}

/// Hides email addresses and the known name `Zorblat`.
fn email_and_name() -> Redactor {
    redactor(m::RedactionPolicy {
        classes: vec![m::RedactionClass::Email],
        rules: Vec::new(),
        known_names: Some(vec!["Zorblat".into()]),
        rare_limit: None,
        refuse_if_left: None,
    })
}

fn scratch() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("cortex-ask-sec")
        .tempdir_in(
            std::env::var_os("CARGO_TARGET_TMPDIR")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir),
        )
        .expect("a scratch directory")
}

/// A stand-in `ekr mcp` speaking MCP over stdio: one tool whose result is [`PERSONAL`].
fn stand_in_reader(root: &Path) -> ReaderCommand {
    let path = root.join("ekr");
    executable(
        &path,
        &format!(
            r#"[ "$1" = mcp ] || exit 2
while IFS= read -r line; do
  id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9][0-9]*\).*/\1/p')
  case "$line" in
    *'"method":"initialize"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"protocolVersion":"2025-06-18","capabilities":{{"tools":{{}}}},"serverInfo":{{"name":"ekr","version":"0"}}}}}}\n' "$id" ;;
    *'"method":"tools/list"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"tools":[{{"name":"search","inputSchema":{{"type":"object"}}}}]}}}}\n' "$id" ;;
    *'"method":"tools/call"'*) printf '{{"jsonrpc":"2.0","id":%s,"result":{{"content":[{{"type":"text","text":"{PERSONAL}"}}]}}}}\n' "$id" ;;
  esac
done
"#
        ),
    );
    ReaderCommand {
        program: path,
        args: vec!["mcp".into()],
        env: Vec::new(),
        remove: Vec::new(),
    }
}

/// The `host:port` and the path of the server the `--mcp-config` document names.
fn url_of(config: &Value) -> (String, String) {
    let url = config["mcpServers"]["ekr"]["url"]
        .as_str()
        .expect("an http server")
        .strip_prefix("http://")
        .expect("http")
        .to_string();
    let at = url.find('/').expect("a path");
    (url[..at].to_string(), url[at..].to_string())
}

/// One JSON-RPC `POST` to `addr` at `path`: the whole HTTP response.
fn post(addr: &str, path: &str, body: &Value) -> String {
    let mut s = TcpStream::connect(addr).expect("connect");
    // A server that never ends its response fails the case instead of hanging it.
    s.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
    let body = body.to_string();
    write!(
        s,
        "POST {path} HTTP/1.1\r\nhost: {addr}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
    let mut out = String::new();
    let _ = s.read_to_string(&mut out);
    out
}

/// The `--mcp-config` document goes on `claude`'s command line (`extract.rs`, `--mcp-config
/// <json>`), which every local user reads in `/proc/<pid>/cmdline`. What it holds alone must not
/// be enough to read the store through the redacting server: today the path token is all the
/// server checks, and it is in that document.
#[test]
fn the_command_line_config_alone_does_not_open_the_store() {
    let tmp = scratch();
    let reader = stand_in_reader(tmp.path());
    let r = email_and_name();
    let deadline = Instant::now() + Duration::from_secs(60);
    let served = proxy::serve(
        &r,
        &reader,
        "ekr",
        "What does the team do?",
        deadline,
        |_, config| {
            // Another local process, knowing only what the command line shows.
            let argv_visible = config.to_string();
            let config: Value = serde_json::from_str(&argv_visible).unwrap();
            let (addr, path) = url_of(&config);
            post(
                &addr,
                &path,
                &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                    "params": {"name": "search", "arguments": {"query": "x"}}}),
            )
        },
    )
    .expect("the server starts");
    assert!(
        !served.answer.contains("leads the widget team"),
        "a client holding only the command-line config read the store: {}",
        served.answer
    );
}

/// The server and the reader stop when the case's model call returns: a connection that never
/// sends a request (another local process, or a stuck client) must not hold the case, and the
/// reader with it, open. Today `serve` waits for the 30-second read timeout of each held
/// connection, and a client that sends a byte every few seconds holds it without bound.
#[test]
fn the_server_stops_when_the_case_ends_even_with_a_silent_connection_open() {
    let tmp = scratch();
    let reader = stand_in_reader(tmp.path());
    let r = email_and_name();
    let deadline = Instant::now() + Duration::from_secs(120);
    let started = Instant::now();
    let served = proxy::serve(&r, &reader, "ekr", "q", deadline, |_, config| {
        let (addr, _) = url_of(config);
        let held = TcpStream::connect(addr).expect("connect");
        // Let the server accept it and block reading its request line.
        std::thread::sleep(Duration::from_millis(300));
        held
    })
    .expect("the server starts");
    let took = started.elapsed();
    drop(served.answer);
    assert!(
        took < Duration::from_secs(5),
        "serve returned {took:?} after the case ended: a silent connection held the server"
    );
}

/// Each text of a batch round-trips: restoring what the model was shown of it gives the text
/// back. A store text that already holds placeholder-shaped text (`[Email-1]`), read after an
/// earlier tool result was given that placeholder, is shown unchanged and then restored to the
/// earlier result's value, so an answer quoting it attributes another document's address.
#[test]
fn a_placeholder_shaped_store_text_in_a_later_result_is_not_restored_to_another_value() {
    let r = email_and_name();
    let mut batch = r.batch();
    let first = "write to help@acme.example";
    batch.reserve(first);
    assert_eq!(batch.replace(first), "write to [Email-1]");
    let later = "the mail template reads Dear [Email-1],";
    batch.reserve(later);
    let mut seen = Value::String(batch.replace(later));
    batch.restore(&mut seen);
    assert_eq!(
        seen,
        Value::String(later.into()),
        "the later text came back with another document's value"
    );
}

/// A withheld result is shown to the model as a tool error only; its values were never given to
/// the model under any placeholder, so no placeholder may restore to them. Today `shown` replaces
/// before it checks, so the withheld text's values join the batch under sequential, guessable
/// placeholders, and an answer naming `[Email-1]` is restored to an address from a withheld result.
#[test]
fn a_value_of_a_withheld_result_is_never_restored() {
    // The rule's replacement is itself an address the `Email` detector finds, so the result is
    // withheld whatever else it holds.
    let r = redactor(m::RedactionPolicy {
        classes: vec![m::RedactionClass::Email],
        rules: vec![m::RedactionRule {
            name: "Team".into(),
            pattern: "widget team".into(),
            replacement: "ops@acme.example".into(),
        }],
        known_names: None,
        rare_limit: None,
        refuse_if_left: Some(vec![m::RedactionClass::Email]),
    });
    let mut batch = r.batch();
    let (text, _, withheld) = shown(Some(&mut batch), PERSONAL, 10_000);
    assert!(withheld, "the case needs a withheld result: {text}");
    assert!(!text.contains("help@acme.example"));
    let mut answer = Value::String("Write to [Email-1].".into());
    batch.restore(&mut answer);
    assert!(
        !answer.as_str().unwrap().contains("help@acme.example"),
        "a guessed placeholder restored a value of a withheld result: {answer}"
    );
}

/// `story:eval-proxy-bounds`, R1-07: each connection the server accepts is kept in its open list
/// so the end of the case can shut it down, and must leave that list when it is answered. Today
/// none leaves until the case ends, so a client making many short requests holds one descriptor
/// per request, and at the descriptor limit `accept` fails and the server only sleeps.
#[test]
fn two_thousand_sequential_connections_leave_the_open_list_bounded() {
    let tmp = scratch();
    let reader = stand_in_reader(tmp.path());
    let r = email_and_name();
    let deadline = Instant::now() + Duration::from_secs(120);
    let served = proxy::serve(&r, &reader, "ekr", "q", deadline, |_, config| {
        let (addr, path) = url_of(config);
        for i in 0..2_000 {
            let mut s = TcpStream::connect(&addr).expect("connect");
            s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            write!(s, "GET {path} HTTP/1.1\r\nhost: {addr}\r\n\r\n").unwrap();
            // Read to the end: the server closes the connection after it answered.
            let mut out = String::new();
            let _ = s.read_to_string(&mut out);
            assert!(
                out.starts_with("HTTP/1.1 401"),
                "connection {i} was not answered: {out:?}"
            );
        }
    })
    .expect("the server starts");
    // Each client read to the end of its connection, which the server closes only after it left
    // the list: none is left.
    assert_eq!(
        served.open_connections(),
        0,
        "{} connections were still in the open list after 2,000 answered ones",
        served.open_connections()
    );
}

/// `story:eval-proxy-bounds`, R1-08: the request line and the header lines are read up to a cap
/// of 64 KiB, checked as they are read and before the bearer check. A header line that reaches
/// the cap is refused at once, before any further read; today the server reads on, without a
/// cap, until the line ends or its 30-second read timeout.
#[test]
fn a_header_line_over_the_cap_is_refused_before_any_further_read() {
    let tmp = scratch();
    let reader = stand_in_reader(tmp.path());
    let r = email_and_name();
    let deadline = Instant::now() + Duration::from_secs(120);
    let served = proxy::serve(&r, &reader, "ekr", "q", deadline, |_, config| {
        let (addr, path) = url_of(config);
        let mut s = TcpStream::connect(&addr).expect("connect");
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        // 64 KiB of request line and one header line that does not end, and nothing after it:
        // a server that reads on waits for the rest.
        let head = format!("POST {path} HTTP/1.1\r\nx-pad: ");
        let pad = "a".repeat((64 << 10) - head.len());
        s.write_all(head.as_bytes()).unwrap();
        s.write_all(pad.as_bytes()).unwrap();
        let started = Instant::now();
        let mut out = String::new();
        let _ = s.read_to_string(&mut out);
        (out, started.elapsed())
    })
    .expect("the server starts");
    let (out, took) = served.answer;
    assert!(
        out.starts_with("HTTP/1.1 431"),
        "a header line at the cap was not refused (after {took:?}): {out:?}"
    );
    assert!(took < Duration::from_secs(2), "the refusal took {took:?}");
}
