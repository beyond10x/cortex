//! Security review, pass 1, of unit `tail` of wave 20261030a: integrity checks of the invariants
//! the unit's stories rest on, each against the `cortex` binary.
//!
//! 1. No credential that `ekr` writes to stderr is kept unmasked in what a run reports or records,
//!    also when the session's stderr is cut to its last 16 KiB (`src/ekr.rs` `STDERR_KEPT`).
//! 2. Pruning `runs/` removes only run directories inside the instance's own `runs/`: a symlink
//!    named as a run directory, and a symlink inside a pruned one, are not followed.
//! 3. An inherited `EKR_STORE`, `EKR_HOST`, `EKR_BACKEND` or `EKR_STAGE` does not redirect the
//!    store a run opens.
//! 4. A `connectors` call ends within its deadline, also when the child exits and something it
//!    started still holds its output; an out-of-range deadline setting does not panic.
//! 5. The 16 MiB cap holds for a lookup file reached through a symlink, and a lookup that is a
//!    special file (a FIFO) fails the fetch instead of blocking the run.

mod common;

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use common::{ekr, executable, World, LISTED};

const DAY_MS: u128 = 86_400_000;

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

/// What `cortex <args>` exited with and printed, or `None` for the exit code when it was still
/// running after `limit` (it is then killed).
struct Ran {
    code: Option<i32>,
    stdout: String,
    stderr: String,
    took: Duration,
}

fn cortex_within(w: &World, args: &[&str], env: &[(&str, &str)], limit: Duration) -> Ran {
    let started = Instant::now();
    let mut child = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(args)
        .envs(env.iter().copied())
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let pipe = |mut p: Box<dyn Read + Send>| {
        std::thread::spawn(move || {
            let mut s = Vec::new();
            let _ = p.read_to_end(&mut s);
            String::from_utf8_lossy(&s).into_owned()
        })
    };
    let out = pipe(Box::new(child.stdout.take().unwrap()));
    let err = pipe(Box::new(child.stderr.take().unwrap()));
    let code = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status.code();
        }
        if started.elapsed() > limit {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let took = started.elapsed();
    Ran {
        code,
        stdout: out.join().unwrap(),
        stderr: err.join().unwrap(),
        took,
    }
}

/// Every file under `dir` whose bytes contain `needle`.
fn holding(dir: &Path, needle: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            found.extend(holding(&path, needle));
        } else if kind.is_file()
            && std::fs::read(&path).is_ok_and(|b| String::from_utf8_lossy(&b).contains(needle))
        {
            found.push(path);
        }
    }
    found
}

fn alive(pid: &str) -> bool {
    Command::new("kill")
        .args(["-0", pid.trim()])
        .stderr(Stdio::null())
        .status()
        .unwrap()
        .success()
}

/// The world's spec of `name`, its `ekr` the stand-in at `bin/ekr`, with `extra` appended.
fn spec_with_bin(w: &World, name: &str, extra: &str) -> PathBuf {
    let path = w.spec(name, "conn_test");
    let mut text = std::fs::read_to_string(&path).unwrap().replace(
        &ekr().display().to_string(),
        &w.bin.join("ekr").display().to_string(),
    );
    text.push_str(extra);
    std::fs::write(&path, text).unwrap();
    path
}

fn create(w: &World, spec: &Path) {
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
}

// ---------------------------------------------------------------------------------------------
// Invariant 1: masking runs before the cut.

/// How much of an `ekr session`'s stderr cortex keeps, from its end (`src/ekr.rs` `STDERR_KEPT`).
const STDERR_KEPT: usize = 16 * 1024;

/// A stand-in `ekr`: with the file `break-session`, `ekr session` writes `@LINE@` and then
/// `@PAD@` bytes of `x` to stderr and exits 1; everything else is the real `ekr`.
const BREAKS_SESSION: &str = r#"R="@ROOT@"
if [ "$1" = session ] && [ -e "$R/break-session" ]; then
  printf 'ekr: cannot open the store: %s\n' "@LINE@" >&2
  head -c @PAD@ /dev/zero | tr '\0' x >&2
  exit 1
fi
exec "@EKR@" "$@"
"#;

/// A session that ends writing a credential-shaped assignment to stderr, followed by enough output
/// that the 16 KiB window cortex keeps starts inside the credential's name: what is kept must not
/// hold the credential's value. `error_text` (`src/extract.rs`) masks before it cuts for this
/// reason; `Pipe::finish` (`src/ekr.rs`) masks what the drain already cut from the front.
#[test]
fn a_credential_cut_by_the_session_stderr_window_is_not_kept_unmasked() {
    let w = World::new();
    // Assembled at runtime so no credential-shaped literal sits in the source.
    let value = format!("{}{}", "hunter2", "Qz7vX9wK");
    let line = format!("pass{}={value}", "word");
    // The window starts 6 bytes into the line ("passwo" is cut), so the name no longer matches.
    let kept_of_line = line.len() - 6 + 1;
    let pad = STDERR_KEPT - kept_of_line;
    executable(
        &w.bin.join("ekr"),
        &BREAKS_SESSION
            .replace("@ROOT@", &w.root.display().to_string())
            .replace("@LINE@", &line)
            .replace("@PAD@", &pad.to_string())
            .replace("@EKR@", &ekr().display().to_string()),
    );
    let spec = spec_with_bin(&w, "m", "");
    create(&w, &spec);
    std::fs::write(w.root.join("break-session"), "").unwrap();

    let ran = cortex_within(&w, &["run", "m/news"], &[], Duration::from_secs(120));
    assert!(ran.code.is_some(), "the run did not end");
    let said = format!("{}\n{}", ran.stdout, ran.stderr);
    let reached = said.contains("session ended") || !holding(&w.home, "session ended").is_empty();
    assert!(reached, "the run did not report the ended session: {said}");
    let printed: Vec<&str> = said.lines().filter(|l| l.contains(&value)).collect();
    let files = holding(&w.home, &value);
    assert!(
        printed.is_empty() && files.is_empty(),
        "the credential's value is kept unmasked: printed in {} line(s) (first starts {:?}), \
         written to {files:?}",
        printed.len(),
        printed
            .first()
            .map(|l| &l[..l.find("xxxx").unwrap_or(l.len()).min(400)])
    );
}

// ---------------------------------------------------------------------------------------------
// Invariant 2: pruning stays inside `runs/`.

/// A symlink under `runs/` named as an old run directory, and a symlink inside an old run
/// directory, both pointing outside the instance: pruning removes the old directory and neither
/// target nor anything in it.
#[test]
fn pruning_follows_no_symlink_out_of_the_runs_directory() {
    let w = World::new();
    let spec = {
        let path = w.spec("p", "conn_test");
        let mut text = std::fs::read_to_string(&path).unwrap();
        text.push_str("runs: {keep_days: 1}\n");
        std::fs::write(&path, &text).unwrap();
        path
    };
    create(&w, &spec);
    let runs = w.home.join("instances").join("p").join("runs");
    std::fs::create_dir_all(&runs).unwrap();
    let outside = w.root.join("outside");
    let inner = w.root.join("outside-inner");
    for dir in [&outside, &inner] {
        std::fs::create_dir_all(dir.join("1000-news")).unwrap();
        std::fs::write(dir.join("keep.txt"), "kept").unwrap();
        std::fs::write(dir.join("1000-news").join("keep.txt"), "kept").unwrap();
    }
    let old = now_ms() - 3 * DAY_MS;
    let linked = format!("{old}-news");
    std::os::unix::fs::symlink(&outside, runs.join(&linked)).unwrap();
    let real = format!("{}-news", old + 1);
    std::fs::create_dir_all(runs.join(&real).join("batch-1")).unwrap();
    std::os::unix::fs::symlink(&inner, runs.join(&real).join("batch-1").join("link")).unwrap();

    let (code, ran) = w.cortex(&["run", "p/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");

    assert!(
        !runs.join(&real).exists(),
        "the old run directory was not pruned"
    );
    for dir in [&outside, &inner] {
        assert!(
            dir.join("keep.txt").is_file(),
            "{} lost a file",
            dir.display()
        );
        assert!(
            dir.join("1000-news").join("keep.txt").is_file(),
            "{} lost a directory",
            dir.display()
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Invariant 3: inherited `EKR_*` does not redirect the store.

/// A run with `EKR_STORE`, `EKR_HOST` and `EKR_BACKEND` naming another store, and an `EKR_STAGE`
/// that is no stage, inherited: the run opens its own store and applies to it, and the other
/// store is never created.
#[test]
fn an_inherited_ekr_store_host_backend_or_stage_does_not_redirect_the_run() {
    let w = World::new();
    let spec = w.spec("s", "conn_test");
    create(&w, &spec);
    let other = w.root.join("other-tenant");
    std::fs::create_dir_all(&other).unwrap();
    let store = other.join("store.sqlite");
    let host = other.join("host.json");
    let before = w.head("s");
    let ran = cortex_within(
        &w,
        &["run", "s/news"],
        &[
            ("EKR_STORE", store.to_str().unwrap()),
            ("EKR_HOST", host.to_str().unwrap()),
            ("EKR_BACKEND", "sqlite"),
            ("EKR_STAGE", "not-a-stage"),
        ],
        Duration::from_secs(120),
    );
    assert_eq!(ran.code, Some(0), "{}\n{}", ran.stdout, ran.stderr);
    assert!(
        w.head("s") > before,
        "the run did not apply to its own store"
    );
    assert!(!store.exists(), "the run wrote the inherited EKR_STORE");
}

// ---------------------------------------------------------------------------------------------
// Invariant 4: the deadline ends the call.

/// A `connectors` whose connection is ready and whose `invoke` answers at once and exits, leaving
/// a process it started (its pid in `grandchild`) holding its stdout for `@HOLD@` seconds.
const LEAVES_A_CHILD: &str = r#"R="@ROOT@"
case "$*" in
  *"connections list"*) printf '{"ok":true,"result":{"connections":[{"adapter":"tavily",@LISTED@,"connection":"conn_test","state":"ready","revision":"rev1"}]}}' ;;
  "invoke tavily/main "*) sleep @HOLD@ & echo $! > "$R/grandchild"; cat "$R/answer.json" ;;
  *) echo '{"ok":false}'; exit 2 ;;
esac
"#;

/// The call's deadline bounds the whole call: a `connectors` that exits but leaves a process of
/// its group holding its output does not hold the run past the deadline, and that process is
/// gone afterwards. `wait_with` (`src/terminate.rs`) stops checking the deadline once the direct
/// child exited and then reads its pipes to their end with no bound.
#[test]
fn a_connectors_process_left_holding_the_output_does_not_hold_the_call_past_its_deadline() {
    let w = World::new();
    let hold = 10;
    executable(
        &w.bin.join("connectors"),
        &LEAVES_A_CHILD
            .replace("@ROOT@", &w.root.display().to_string())
            .replace("@LISTED@", LISTED)
            .replace("@HOLD@", &hold.to_string()),
    );
    let spec = w.spec("d", "conn_test");
    create(&w, &spec);
    let ran = cortex_within(
        &w,
        &["run", "d/news", "--fetch-only"],
        &[("CORTEX_CONNECTORS_TIMEOUT_S", "1")],
        Duration::from_secs(60),
    );
    let grandchild = std::fs::read_to_string(w.root.join("grandchild")).expect("invoke ran");
    let left = alive(&grandchild);
    assert!(
        ran.took < Duration::from_secs(4),
        "the call with a 1 s deadline held the run {:?} (the left process holds its output {hold} s)",
        ran.took
    );
    assert!(!left, "the process connectors left is still running");
}

/// A deadline setting past what an `Instant` can hold is not a panic after the child was spawned:
/// `wait_within` computes `Instant::now() + limit`, which panics on overflow, and the spawned
/// `connectors` is then left unreaped and unbounded.
#[test]
fn an_out_of_range_deadline_setting_does_not_panic_the_run() {
    let w = World::new();
    let spec = w.spec("o", "conn_test");
    create(&w, &spec);
    let ran = cortex_within(
        &w,
        &["run", "o/news"],
        &[("CORTEX_CONNECTORS_TIMEOUT_S", "9223372036854775807")],
        Duration::from_secs(120),
    );
    assert!(
        !ran.stderr.contains("panicked"),
        "cortex panicked: {}",
        ran.stderr
    );
    assert_ne!(ran.code, Some(101), "{}", ran.stderr);
}

// ---------------------------------------------------------------------------------------------
// Invariant 5: the read cap and special files.

/// A `files` source `chat` over `chat/*.jsonl` whose records map authors through `lookup`.
fn lookup_world(lookup: &str) -> World {
    let w = World::new();
    std::fs::create_dir_all(w.root.join("chat")).unwrap();
    std::fs::write(
        w.root.join("chat").join("export.jsonl"),
        "{\"id\":\"1\",\"user\":\"U1\",\"text\":\"hi\"}\n",
    )
    .unwrap();
    let spec = w.root.join("l.yaml");
    std::fs::write(
        &spec,
        format!(
            "format: cortex.instance/1\nname: l\ndescription: Test brain.\nekr: {{version: \"0.0.34\", bin: \"{ekr}\"}}\nseed: {{documents: []}}\nmodel: {{model: claude-haiku-4-5-20251001, budget_usd: \"1\", timeout_s: 60}}\nsources:\n  - name: chat\n    schedule: daily\n    settings:\n      kind: files\n      value:\n        paths: [chat]\n        glob: \"*.jsonl\"\n        records: {{format: JsonLines, id: id, author: user, text: [\"{{text}}\"], lookup: {lookup}, filters: []}}\n    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}\nserve: {{view_port: 18993}}\n",
            ekr = ekr().display()
        ),
    )
    .unwrap();
    create(&w, &spec);
    w
}

/// A lookup reached through a symlink to a file over 16 MiB fails the fetch on the cap.
#[test]
fn the_cap_holds_for_a_lookup_reached_through_a_symlink() {
    let w = lookup_world("people.json");
    let big = w.root.join("big.json");
    let f = std::fs::File::create(&big).unwrap();
    f.set_len(16 * 1024 * 1024 + 1).unwrap();
    std::os::unix::fs::symlink(&big, w.root.join("people.json")).unwrap();
    let ran = cortex_within(&w, &["run", "l/chat"], &[], Duration::from_secs(60));
    assert_eq!(ran.code, Some(1), "{}\n{}", ran.stdout, ran.stderr);
    assert!(ran.stdout.contains("16 MiB cap"), "{}", ran.stdout);
}

/// A lookup that is a FIFO with no writer fails the fetch; it does not block the run in `open`.
#[test]
fn a_lookup_that_is_a_fifo_fails_the_fetch_instead_of_blocking_the_run() {
    let w = lookup_world("people.json");
    let fifo = w.root.join("people.json");
    let made = Command::new("mkfifo").arg(&fifo).status().unwrap();
    assert!(made.success());
    let ran = cortex_within(&w, &["run", "l/chat"], &[], Duration::from_secs(15));
    assert!(
        ran.code.is_some(),
        "the run was still blocked after {:?} on a lookup that is a FIFO",
        ran.took
    );
    assert_ne!(ran.code, Some(0), "{}", ran.stdout);
}
