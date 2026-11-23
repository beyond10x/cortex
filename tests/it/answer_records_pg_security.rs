//! Conformance checks of the records connection's TLS contract (`spec/domains/ask.yaml`,
//! `cortex.ask.PostgresRecords`): "TLS is required unless it says `sslmode=disable`". A stand-in
//! server on loopback declines TLS (answers `N` to the SSLRequest) and then asks for the password
//! in clear text; the records connection must end there, before it sends its startup message or
//! the role's password over a connection without TLS.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::time::Duration;

use cortex_cli::ask::records::postgres::PgRecords;

const PASSWORD: &str = "records-secret-never-in-clear";
/// The SSLRequest message: length 8, code 80877103.
const SSL_REQUEST: [u8; 8] = [0, 0, 0, 8, 0x04, 0xd2, 0x16, 0x2f];

/// What the stand-in saw after it declined TLS: whether a startup message came in clear text, and
/// whether the password did.
#[derive(Debug, Default)]
struct Seen {
    startup_in_clear: bool,
    password_in_clear: bool,
}

/// Accepts one connection, declines TLS, and when the client goes on in clear text asks for the
/// password in clear text (AuthenticationCleartextPassword) and reads the answer.
fn declining_server() -> (u16, mpsc::Receiver<Seen>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut seen = Seen::default();
        if let Ok((mut s, _)) = listener.accept() {
            s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
            let mut first = [0u8; 8];
            if s.read_exact(&mut first).is_ok() {
                let mut len = u32::from_be_bytes(first[..4].try_into().unwrap()) as usize;
                if first == SSL_REQUEST {
                    let _ = s.write_all(b"N");
                    let mut head = [0u8; 4];
                    if s.read_exact(&mut head).is_ok() {
                        len = u32::from_be_bytes(head) as usize;
                        seen.startup_in_clear = true;
                    } else {
                        let _ = tx.send(seen);
                        return;
                    }
                    let mut rest = vec![0u8; len.saturating_sub(4)];
                    let _ = s.read_exact(&mut rest);
                } else {
                    // No SSLRequest at all: the startup went in clear from the first byte.
                    seen.startup_in_clear = true;
                    let mut rest = vec![0u8; len.saturating_sub(8)];
                    let _ = s.read_exact(&mut rest);
                }
                // AuthenticationCleartextPassword.
                let _ = s.write_all(&[b'R', 0, 0, 0, 8, 0, 0, 0, 3]);
                let mut buf = vec![0u8; 512];
                if let Ok(n) = s.read(&mut buf) {
                    seen.password_in_clear = String::from_utf8_lossy(&buf[..n]).contains(PASSWORD);
                }
            }
        }
        let _ = tx.send(seen);
    });
    (port, rx)
}

fn probe(url_of: impl Fn(u16) -> String) -> (Result<(), String>, Seen) {
    let (port, rx) = declining_server();
    let opened =
        PgRecords::connect(&url_of(port), "cortex_ask_records", None, PASSWORD).map(|_| ());
    let seen = rx.recv_timeout(Duration::from_secs(40)).unwrap_or_default();
    (opened, seen)
}

fn assert_no_clear_text(what: &str, opened: Result<(), String>, seen: Seen) {
    assert!(opened.is_err(), "{what}: a server without TLS was accepted");
    assert!(
        !seen.password_in_clear,
        "{what}: the role's password was sent over a connection without TLS"
    );
    assert!(
        !seen.startup_in_clear,
        "{what}: the startup message was sent over a connection without TLS after the server \
         declined it"
    );
}

/// The control: no `sslmode` in the URL, TLS required, nothing in clear text.
#[test]
fn a_records_url_without_sslmode_sends_nothing_in_clear_text() {
    let (opened, seen) = probe(|port| format!("postgres://cortex_ask@127.0.0.1:{port}/ekr"));
    assert_no_clear_text("no sslmode", opened, seen);
}

/// `sslmode=prefer` asks for TLS and is not `disable`: the contract requires TLS, so a server that
/// declines it must not get a connection in clear text.
#[test]
fn a_records_url_with_sslmode_prefer_never_falls_back_to_clear_text() {
    let (opened, seen) =
        probe(|port| format!("postgres://cortex_ask@127.0.0.1:{port}/ekr?sslmode=prefer"));
    assert_no_clear_text("sslmode=prefer", opened, seen);
}

/// The text `sslmode=` inside another parameter's value sets no mode: TLS stays required.
#[test]
fn sslmode_text_inside_another_parameter_does_not_lift_the_tls_requirement() {
    let (opened, seen) = probe(|port| {
        format!("postgres://cortex_ask@127.0.0.1:{port}/ekr?application_name=sslmode=require")
    });
    assert_no_clear_text("application_name=sslmode=require", opened, seen);
}
