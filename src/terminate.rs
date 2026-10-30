//! SIGTERM for `cortex run` (`cortex.instance.RunSource`, outcome `terminated`): a Kubernetes
//! Job's deadline or a `systemctl stop` sends it. Once [`install`] ran, the first SIGTERM only
//! sets a flag, which the run reads at its safe points ([`requested`]): while it waits for a
//! `connectors` call of its fetch or a model call, which it then ends ([`wait`]), between the
//! pages and files of a fetch, before it uses what it fetched, before each batch, and in the
//! waits before a call is tried again ([`sleep`]). The run answers `terminated`, records its
//! status and the metric file and exits 8. A second SIGTERM ends the process at once with
//! [`EXIT_SECOND_SIGNAL`], recording nothing.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

/// The exit status of a process a second SIGTERM ended: the shell's 128 + 15.
pub const EXIT_SECOND_SIGNAL: i32 = 143;

fn flag() -> &'static Arc<AtomicBool> {
    static FLAG: OnceLock<Arc<AtomicBool>> = OnceLock::new();
    FLAG.get_or_init(|| Arc::new(AtomicBool::new(false)))
}

/// Handles SIGTERM from now on: the first sets the flag [`requested`] reads, a second exits.
pub fn install() -> Result<(), String> {
    use signal_hook::consts::SIGTERM;
    // Registered first, so it reads the flag as it was before this signal set it.
    signal_hook::flag::register_conditional_shutdown(SIGTERM, EXIT_SECOND_SIGNAL, flag().clone())
        .map_err(|e| format!("cannot handle SIGTERM: {e}"))?;
    signal_hook::flag::register(SIGTERM, flag().clone())
        .map_err(|e| format!("cannot handle SIGTERM: {e}"))?;
    Ok(())
}

/// Whether the process was sent SIGTERM since [`install`].
pub fn requested() -> bool {
    flag().load(Ordering::SeqCst)
}

/// How long a child ended on SIGTERM (a model call, a `connectors` call) may take to exit before
/// its process group is killed.
pub const GRACE: Duration = Duration::from_secs(5);

/// How a child process the run waited for ended ([`wait`]).
pub enum Waited {
    Done(std::process::Output),
    /// It could not be waited for, or its input not sent: why.
    Failed(String),
    /// The run was sent SIGTERM, and the child was ended.
    Terminated,
    /// The child was still running at its deadline ([`wait_within`]): it was killed, with its
    /// process group, and reaped.
    TimedOut,
}

/// Sends `input`, when given, to `child`'s piped stdin and waits for it, reading its output on
/// threads of their own, until it exits or the run is sent SIGTERM ([`requested`]). Then the
/// child's process group (a child that runs in a group of its own, and what it started) is sent
/// SIGTERM, and SIGKILL after [`GRACE`] ([`end`]). A child that exits before reading its input
/// closes the pipe; its exit status and stderr say why, so a broken pipe is not the error
/// reported. `name` names the child in a failure.
pub fn wait(child: &mut std::process::Child, input: Option<&[u8]>, name: &str) -> Waited {
    wait_with(child, input, name, false, requested, None).0
}

/// [`wait`], at most `limit` from now: a child still running then is sent SIGKILL, with its
/// process group, reaped, and answered as [`Waited::TimedOut`].
pub fn wait_within(
    child: &mut std::process::Child,
    input: Option<&[u8]>,
    name: &str,
    limit: Duration,
) -> Waited {
    let deadline = Instant::now() + limit;
    wait_with(child, input, name, false, requested, Some(deadline)).0
}

/// [`wait`], except that a child found already exited when the run's SIGTERM is seen is read,
/// not dropped: what it started is ended ([`end`]), its output comes back as [`Waited::Done`],
/// and `true` says the run was sent SIGTERM as it finished, so the caller stops. A model call
/// that had finished keeps what its answer says it cost.
pub fn wait_finished(
    child: &mut std::process::Child,
    input: Option<&[u8]>,
    name: &str,
) -> (Waited, bool) {
    wait_with(child, input, name, true, requested, None)
}

/// [`wait`], [`wait_finished`] and [`wait_within`], with `requested` read for the run's SIGTERM
/// and the child killed once it runs past `deadline`.
fn wait_with(
    child: &mut std::process::Child,
    input: Option<&[u8]>,
    name: &str,
    keep_finished: bool,
    requested: impl Fn() -> bool,
    deadline: Option<Instant>,
) -> (Waited, bool) {
    use std::io::{Read, Write};
    let mut seen = false;
    let writer = input.map(|input| {
        let mut stdin = child.stdin.take().expect("piped stdin");
        let bytes = input.to_vec();
        std::thread::spawn(move || stdin.write_all(&bytes))
    });
    let read = |pipe: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            if let Some(mut pipe) = pipe {
                let _ = pipe.read_to_end(&mut bytes);
            }
            bytes
        })
    };
    let stdout = read(
        child
            .stdout
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let stderr = read(
        child
            .stderr
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let status = loop {
        match child.try_wait() {
            // A stop of the whole unit signals the child too: one that ended after the run was
            // sent SIGTERM ended because of it, whatever its status says. Unless `keep_finished`:
            // then what it wrote is read, and the caller told the run was sent SIGTERM.
            Ok(Some(status)) if requested() => {
                end(child);
                if !keep_finished {
                    return (Waited::Terminated, false);
                }
                seen = true;
                break status;
            }
            Ok(Some(status)) => break status,
            Ok(None) if requested() => {
                end(child);
                return (Waited::Terminated, false);
            }
            Ok(None) if deadline.is_some_and(|d| Instant::now() >= d) => {
                kill(child);
                return (Waited::TimedOut, false);
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => {
                end(child);
                return (Waited::Failed(format!("{name} did not finish: {e}")), false);
            }
        }
    };
    if let Some(writer) = writer {
        match writer.join() {
            Ok(Err(e)) if e.kind() != std::io::ErrorKind::BrokenPipe => {
                return (
                    Waited::Failed(format!("cannot send the input of {name}: {e}")),
                    seen,
                );
            }
            _ => {}
        }
    }
    let out = std::process::Output {
        status,
        stdout: stdout.join().unwrap_or_default(),
        stderr: stderr.join().unwrap_or_default(),
    };
    (Waited::Done(out), seen)
}

/// Ends a child and its process group: SIGTERM, then SIGKILL after [`GRACE`].
pub fn end(child: &mut std::process::Child) {
    use rustix::process::{kill_process, kill_process_group, Pid, Signal};
    let pid = Pid::from_child(child);
    let signal = |s: Signal| {
        let _ = kill_process_group(pid, s);
        let _ = kill_process(pid, s);
    };
    signal(Signal::TERM);
    let until = Instant::now() + GRACE;
    while Instant::now() < until {
        if !matches!(child.try_wait(), Ok(None)) {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    signal(Signal::KILL);
    let _ = child.wait();
}

/// Kills a child and its process group at once (SIGKILL), and reaps it.
fn kill(child: &mut std::process::Child) {
    use rustix::process::{kill_process, kill_process_group, Pid, Signal};
    let pid = Pid::from_child(child);
    let _ = kill_process_group(pid, Signal::KILL);
    let _ = kill_process(pid, Signal::KILL);
    let _ = child.wait();
}

/// Sleeps `pause`, or less when the run is sent SIGTERM: whether it was ([`requested`]).
pub fn sleep(pause: Duration) -> bool {
    let until = Instant::now() + pause;
    while Instant::now() < until && !requested() {
        std::thread::sleep(
            until
                .saturating_duration_since(Instant::now())
                .min(Duration::from_millis(20)),
        );
    }
    requested()
}

#[cfg(test)]
mod tests {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    use super::{wait_with, Waited};

    /// A child that printed `out` and exited, not yet reaped: what a run sent SIGTERM finds when
    /// its model call had finished first. Left unreaped on purpose: the test's `wait_with` reaps it.
    #[allow(clippy::zombie_processes)]
    fn exited(out: &str) -> std::process::Child {
        let child = Command::new("sh")
            .args(["-c", &format!("printf '%s' '{out}'")])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let until = Instant::now() + Duration::from_secs(10);
        loop {
            let stat = std::fs::read_to_string(format!("/proc/{}/stat", child.id())).unwrap();
            let state = stat
                .rsplit_once(") ")
                .and_then(|(_, rest)| rest.chars().next());
            if state == Some('Z') {
                return child;
            }
            assert!(Instant::now() < until, "the child never exited");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// `story:run-cost-counted-on-refusal`: a child found exited once the run was sent SIGTERM is
    /// ended as terminated by [`super::wait`], but read by [`super::wait_finished`], which says
    /// that SIGTERM was seen, so a model call that had finished keeps what it cost.
    #[test]
    fn a_child_that_exited_before_sigterm_was_seen_is_read_when_asked() {
        let mut child = exited(r#"{"total_cost_usd":0.4}"#);
        let (waited, seen) = wait_with(&mut child, None, "sh", false, || true, None);
        assert!(matches!(waited, Waited::Terminated) && !seen);

        let mut child = exited(r#"{"total_cost_usd":0.4}"#);
        let (waited, seen) = wait_with(&mut child, None, "sh", true, || true, None);
        let Waited::Done(out) = waited else {
            panic!("the finished child's output is read");
        };
        assert!(seen);
        assert_eq!(out.stdout, br#"{"total_cost_usd":0.4}"#);

        let mut child = exited("ok");
        let (waited, seen) = wait_with(&mut child, None, "sh", true, || false, None);
        assert!(matches!(waited, Waited::Done(_)) && !seen);
    }
}
