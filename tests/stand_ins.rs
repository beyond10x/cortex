//! The stand-ins the end-to-end tests write at runtime run the moment they are written: running one
//! never fails with `Text file busy` (ETXTBSY), however many other tests are spawning processes.

mod common;

use std::path::Path;
use std::process::Command;

use common::executable;

/// ETXTBSY on Linux.
const TEXT_FILE_BUSY: i32 = 26;

/// Every thread writes a stand-in and runs it at once, over and over, while every other thread does
/// the same: each spawn forks the test process, and a fork taken while a stand-in is still open for
/// writing is the race that once failed a gate.
#[test]
fn a_stand_in_runs_the_moment_it_is_written_while_other_threads_spawn() {
    const THREADS: usize = 16;
    const ROUNDS: usize = 150;
    let dir = tempfile::Builder::new()
        .prefix("cortex-stand-ins")
        .tempdir_in(
            std::env::var_os("CARGO_TARGET_TMPDIR")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(std::env::temp_dir),
        )
        .unwrap();
    let busy: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..THREADS)
            .map(|t| {
                let dir = dir.path();
                scope.spawn(move || {
                    let mut busy = Vec::new();
                    for round in 0..ROUNDS {
                        let path = dir.join(format!("stand-in-{t}-{round}"));
                        executable(&path, "exit 0\n");
                        match Command::new(&path).status() {
                            Ok(status) => assert!(status.success(), "{}", path.display()),
                            Err(e) if e.raw_os_error() == Some(TEXT_FILE_BUSY) => {
                                busy.push(format!("{}: {e}", path.display()))
                            }
                            Err(e) => panic!("{}: {e}", path.display()),
                        }
                    }
                    busy
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    });
    assert!(
        busy.is_empty(),
        "{} of {} stand-ins were busy when run:\n{}",
        busy.len(),
        THREADS * ROUNDS,
        busy.join("\n")
    );
}

/// `common::executable` is the one place a test makes an executable, so the guarantee above covers
/// every stand-in: no other test file sets an executable mode or writes a `#!` line itself.
#[test]
fn every_stand_in_is_written_by_common_executable() {
    let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let forbidden = ["\"chmod\"", "0o7", "0o5", "#!/"];
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&tests).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs")
            || path.file_name() == Some("stand_ins.rs".as_ref())
        {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        for (n, line) in text.lines().enumerate() {
            if forbidden.iter().any(|f| line.contains(f)) {
                found.push(format!("{}:{}: {}", path.display(), n + 1, line.trim()));
            }
        }
    }
    assert!(
        found.is_empty(),
        "make stand-ins with common::executable:\n{}",
        found.join("\n")
    );
}
