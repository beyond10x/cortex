//! What a run leaves for an operator besides its printed line and `cortex.log`: each source's
//! status (its last run's outcome, time and reason, and its runs by outcome), the metric file and
//! the hook (`cortex.instance.MonitoringSpec`).
//!
//! A source's status is `status/<source>.json` in the instance directory, beside
//! `registry.json`: it keeps the source's runs by outcome, which the registry does not hold.
//! Every writer of an instance's status files and its metric file holds `status/.lock` for that
//! write only (milliseconds, never during a run or a hook), so a count is not lost to a
//! concurrent writer.

use std::collections::BTreeMap;
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use cortex_model::instance as m;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::home::write_atomic;
use crate::instance::Layout;

/// How long a hook may run when the spec names no `timeout_s`.
pub const HOOK_TIMEOUT_S: u64 = 10;

/// The metric file in the instance directory.
pub const METRICS: &str = "metrics.prom";

/// Every outcome a source's run records, in the order the spec declares them.
pub const OUTCOMES: [m::RunOutcome; 10] = [
    m::RunOutcome::Ran,
    m::RunOutcome::FetchFailed,
    m::RunOutcome::ExtractionFailed,
    m::RunOutcome::ApplyRefused,
    m::RunOutcome::StoreOutcomeUnknown,
    m::RunOutcome::ConnectionUnavailable,
    m::RunOutcome::BatchTooLarge,
    m::RunOutcome::Terminated,
    m::RunOutcome::ModelUnavailable,
    m::RunOutcome::Disabled,
];

pub fn outcome_name(outcome: m::RunOutcome) -> &'static str {
    match outcome {
        m::RunOutcome::Ran => "ran",
        m::RunOutcome::FetchFailed => "fetch-failed",
        m::RunOutcome::ExtractionFailed => "extraction-failed",
        m::RunOutcome::ApplyRefused => "apply-refused",
        m::RunOutcome::StoreOutcomeUnknown => "store-outcome-unknown",
        m::RunOutcome::ConnectionUnavailable => "connection-unavailable",
        m::RunOutcome::BatchTooLarge => "batch-too-large",
        m::RunOutcome::Terminated => "terminated",
        m::RunOutcome::ModelUnavailable => "model-unavailable",
        m::RunOutcome::Disabled => "disabled",
    }
}

fn outcome_of(name: &str) -> Option<m::RunOutcome> {
    OUTCOMES.into_iter().find(|o| outcome_name(*o) == name)
}

/// `status/<source>.json`, format `cortex.source-status/1`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Status {
    pub format: String,
    pub last_outcome: Option<String>,
    /// When the last run started, in Unix milliseconds.
    pub last_run_at: Option<i64>,
    pub last_reason: Option<String>,
    /// The source's runs by outcome, since its first status was written.
    #[serde(default)]
    pub runs: BTreeMap<String, u64>,
    /// How long the last run took, in milliseconds, to the moment it recorded its outcome.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_duration_ms: Option<i64>,
    /// The last run's documents by kind (`read`, `applied`): present only when it ended `ran`, or
    /// applied a batch and then failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_documents: Option<BTreeMap<String, u64>>,
    /// The documents of every run that reported them, by kind (`read`, `applied`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub documents: BTreeMap<String, u64>,
    /// The model tokens those runs reported, by kind (`input`, `cached`, `output`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tokens: BTreeMap<String, u64>,
}

/// What a run that ended `ran`, or applied a batch and then failed, did, as `metrics.prom` reports
/// it: the documents it read (the new or changed ones it selected, `SourceRan.documents_new`) and
/// applied (`0` for a run that was undone), as counts, and the model tokens its answers reported,
/// each `None` when an answer did not report it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Figures {
    pub read: u64,
    pub applied: u64,
    /// `input_tokens` and `cache_creation_input_tokens`.
    pub input_tokens: Option<u64>,
    /// `cache_read_input_tokens`.
    pub cached_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

/// The kinds of documents a run reports, in the order the metric file lists them.
const DOCUMENT_KINDS: [&str; 2] = ["read", "applied"];
/// The kinds of model tokens a run reports, in the order the metric file lists them.
const TOKEN_KINDS: [&str; 3] = ["input", "cached", "output"];

impl Status {
    /// Sets the last run's duration and figures, and adds the figures to the totals, whatever the
    /// outcome. A run that failed before it applied a batch (no `figures`) reports no documents.
    fn add(&mut self, duration_ms: Option<i64>, figures: Option<&Figures>) {
        self.last_duration_ms = duration_ms;
        self.last_documents = figures.map(|f| {
            DOCUMENT_KINDS
                .into_iter()
                .zip([f.read, f.applied])
                .map(|(k, n)| (k.to_string(), n))
                .collect()
        });
        let Some(f) = figures else {
            return;
        };
        for (kind, n) in DOCUMENT_KINDS.into_iter().zip([f.read, f.applied]) {
            let total = self.documents.entry(kind.into()).or_default();
            *total = total.saturating_add(n);
        }
        let tokens = [f.input_tokens, f.cached_tokens, f.output_tokens];
        for (kind, n) in TOKEN_KINDS.into_iter().zip(tokens) {
            let total = self.tokens.entry(kind.into()).or_default();
            *total = total.saturating_add(n.unwrap_or(0));
        }
    }
}

fn path(layout: &Layout, source: &str) -> PathBuf {
    layout.dir.join("status").join(format!("{source}.json"))
}

/// The status of `source` (its name within the instance); empty when none was written. A file
/// that cannot be read or parsed is an error naming it.
pub fn read(layout: &Layout, source: &str) -> Result<Status, String> {
    let file = path(layout, source);
    let bytes = match std::fs::read(&file) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Status::default()),
        Err(e) => return Err(format!("cannot read {}: {e}", file.display())),
    };
    serde_json::from_slice(&bytes)
        .map_err(|e| format!("{} is no source status: {e}", file.display()))
}

/// [`read`], with a file that cannot be read or parsed answered as an empty status and named on
/// stderr: what a listing or the metric file shows, never what a run records on.
pub fn load(layout: &Layout, source: &str) -> Status {
    read(layout, source).unwrap_or_else(|e| {
        eprintln!("cortex: {e}");
        Status::default()
    })
}

/// Sets the last run fields of `data` from its status file.
pub fn overlay(layout: &Layout, data: &mut m::SourceData) {
    let status = load(layout, &data.name);
    data.last_outcome = status.last_outcome.as_deref().and_then(outcome_of);
    data.last_run_at = status
        .last_run_at
        .map(|ms| cortex_model::primitives::Timestamp(crate::sources::rfc3339(ms)));
    data.last_reason = status.last_reason;
}

/// Records a run of `source` that started at `at_ms`: its outcome, time and reason, and one more
/// run under its outcome. The reason is masked (`crate::mask`) before it is written, as a key is
/// in `cortex.log`: a fetch failure may quote a URL.
pub fn record(
    layout: &Layout,
    source: &str,
    outcome: m::RunOutcome,
    at_ms: i64,
    reason: Option<&str>,
) -> Result<(), String> {
    record_run(layout, source, outcome, at_ms, reason, None, None)
}

/// [`record`], with the run's duration and, for a run that ended `ran`, its figures.
pub fn record_run(
    layout: &Layout,
    source: &str,
    outcome: m::RunOutcome,
    at_ms: i64,
    reason: Option<&str>,
    duration_ms: Option<i64>,
    figures: Option<&Figures>,
) -> Result<(), String> {
    // A status that cannot be read is left as it is: rewriting it would start its counts again.
    let mut status = read(layout, source)?;
    status.add(duration_ms, figures);
    status.format = "cortex.source-status/1".into();
    status.last_outcome = Some(outcome_name(outcome).into());
    status.last_run_at = Some(at_ms);
    status.last_reason = reason.map(|r| crate::mask::mask(r).0);
    *status.runs.entry(outcome_name(outcome).into()).or_default() += 1;
    write_atomic(
        &path(layout, source),
        serde_json::to_string_pretty(&status)
            .expect("plain JSON")
            .as_bytes(),
    )
}

/// The outcome a hook is called with for a run that answered `outcome`: the failures only, and
/// `disabled` instead when the run's recorded failure disabled the source. An unknown store
/// outcome is reported too, though no failure is recorded for it.
pub fn hook_outcome(outcome: m::RunOutcome, disabled: bool) -> Option<m::RunOutcome> {
    match outcome {
        m::RunOutcome::FetchFailed
        | m::RunOutcome::ExtractionFailed
        | m::RunOutcome::ApplyRefused
        | m::RunOutcome::StoreOutcomeUnknown
        | m::RunOutcome::ConnectionUnavailable
        | m::RunOutcome::BatchTooLarge
        | m::RunOutcome::Terminated
        | m::RunOutcome::ModelUnavailable => Some(if disabled {
            m::RunOutcome::Disabled
        } else {
            outcome
        }),
        m::RunOutcome::Ran | m::RunOutcome::Disabled => None,
    }
}

/// One run of `source_id` that answered `outcome`, as the source's status, the metric file and
/// the hook report it. `sources` are the ids of the instance's sources, for the metric file;
/// `disabled` says the run's recorded failure disabled the source. Nothing here changes the run's
/// outcome: what cannot be written, and a hook that fails, is recorded in `cortex.log`.
pub struct Run<'a> {
    pub source_id: &'a str,
    pub outcome: m::RunOutcome,
    pub at_ms: i64,
    pub reason: Option<&'a str>,
    pub disabled: bool,
    /// How long the run took, in milliseconds.
    pub duration_ms: Option<i64>,
    /// What a run that ended `ran` did; `None` for every other outcome.
    pub figures: Option<Figures>,
}

pub fn report(
    layout: &Layout,
    monitoring: Option<&m::MonitoringSpec>,
    sources: &[String],
    run: &Run,
) {
    let name = run
        .source_id
        .split_once('/')
        .map_or(run.source_id, |(_, n)| n);
    let failed = |what: &str, e: String| {
        let mut line = json!({"at": run.at_ms, "source": run.source_id});
        line[what] = json!({"failed": e});
        layout.log_line(&line);
    };
    let metrics = monitoring.is_some_and(|o| o.metrics == Some(true));
    match lock(layout) {
        Err(e) => failed("status", e),
        Ok(_held) => {
            let recorded = record_run(
                layout,
                name,
                run.outcome,
                run.at_ms,
                run.reason,
                run.duration_ms,
                run.figures.as_ref(),
            );
            if let Err(e) = recorded {
                failed("status", e);
            }
            if metrics {
                if let Err(e) = write_metrics(layout, sources) {
                    failed("metrics", e);
                }
            }
        }
    }
    let Some(monitoring) = monitoring else {
        return;
    };
    let called = monitoring
        .hook
        .as_ref()
        .zip(hook_outcome(run.outcome, run.disabled));
    if let Some((hook, outcome)) = called {
        let result = call_hook(layout, hook, run.source_id, outcome);
        layout.log_line(&json!({
            "at": crate::instance::now_ms(),
            "source": run.source_id,
            "outcome": outcome_name(outcome),
            "hook": result.json(),
        }));
    }
}

/// Holds `status/.lock` of the instance until the file returned is dropped: the lock of the
/// instance's status files and metric file, taken for one write.
fn lock(layout: &Layout) -> Result<std::fs::File, String> {
    let dir = layout.dir.join("status");
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let path = dir.join(".lock");
    let file = std::fs::File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .map_err(|e| format!("cannot open {}: {e}", path.display()))?;
    file.lock()
        .map_err(|e| format!("cannot lock {}: {e}", path.display()))?;
    Ok(file)
}

/// A label value in the Prometheus text format.
fn label(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// The metric file of the instance's `sources` (their ids, `<instance>/<name>`), from their
/// status files.
pub fn metrics_text(layout: &Layout, sources: &[String]) -> String {
    let statuses: Vec<(String, Status)> = sources
        .iter()
        .map(|id| {
            let name = id.split_once('/').map_or(id.as_str(), |(_, n)| n);
            (label(id), load(layout, name))
        })
        .collect();
    let mut text = String::from(
        "# HELP cortex_source_runs_total Runs of a cortex source, by outcome.\n\
         # TYPE cortex_source_runs_total counter\n",
    );
    for (id, status) in &statuses {
        for outcome in OUTCOMES {
            let name = outcome_name(outcome);
            let n = status.runs.get(name).copied().unwrap_or(0);
            text.push_str(&format!(
                "cortex_source_runs_total{{source=\"{id}\",outcome=\"{name}\"}} {n}\n"
            ));
        }
    }
    text.push_str(
        "# HELP cortex_source_last_run_timestamp_seconds When the source's last run started, in \
         Unix seconds.\n\
         # TYPE cortex_source_last_run_timestamp_seconds gauge\n",
    );
    for (id, status) in &statuses {
        if let Some(ms) = status.last_run_at {
            text.push_str(&format!(
                "cortex_source_last_run_timestamp_seconds{{source=\"{id}\"}} {}.{:03}\n",
                ms.div_euclid(1000),
                ms.rem_euclid(1000)
            ));
        }
    }
    text.push_str(
        "# HELP cortex_source_last_run_duration_seconds How long the source's last run took.\n\
         # TYPE cortex_source_last_run_duration_seconds gauge\n",
    );
    for (id, status) in &statuses {
        if let Some(ms) = status.last_duration_ms {
            text.push_str(&format!(
                "cortex_source_last_run_duration_seconds{{source=\"{id}\"}} {}.{:03}\n",
                ms.div_euclid(1000),
                ms.rem_euclid(1000)
            ));
        }
    }
    text.push_str(
        "# HELP cortex_source_last_run_documents The documents the source's last run read and \
         applied, when it ended ran or applied a batch before it failed.\n\
         # TYPE cortex_source_last_run_documents gauge\n",
    );
    for (id, status) in &statuses {
        if let Some(last) = &status.last_documents {
            for kind in DOCUMENT_KINDS {
                let n = last.get(kind).copied().unwrap_or(0);
                text.push_str(&format!(
                    "cortex_source_last_run_documents{{source=\"{id}\",kind=\"{kind}\"}} {n}\n"
                ));
            }
        }
    }
    text.push_str(
        "# HELP cortex_source_documents_total The documents the source's runs read and applied.\n\
         # TYPE cortex_source_documents_total counter\n",
    );
    for (id, status) in &statuses {
        for kind in DOCUMENT_KINDS {
            let n = status.documents.get(kind).copied().unwrap_or(0);
            text.push_str(&format!(
                "cortex_source_documents_total{{source=\"{id}\",kind=\"{kind}\"}} {n}\n"
            ));
        }
    }
    text.push_str(
        "# HELP cortex_source_model_tokens_total The model tokens the source's runs reported.\n\
         # TYPE cortex_source_model_tokens_total counter\n",
    );
    for (id, status) in &statuses {
        for kind in TOKEN_KINDS {
            let n = status.tokens.get(kind).copied().unwrap_or(0);
            text.push_str(&format!(
                "cortex_source_model_tokens_total{{source=\"{id}\",kind=\"{kind}\"}} {n}\n"
            ));
        }
    }
    text
}

/// Rewrites the metric file whole: a temporary file, then a rename.
pub fn write_metrics(layout: &Layout, sources: &[String]) -> Result<(), String> {
    write_atomic(
        &layout.dir.join(METRICS),
        metrics_text(layout, sources).as_bytes(),
    )
}

/// What became of a hook call (`cortex.instance.HookResult`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookResult {
    Called,
    /// It could not start (the reason), or exited with a status other than 0.
    Failed {
        status: Option<i32>,
        reason: Option<String>,
    },
    TimedOut,
}

impl HookResult {
    pub fn json(&self) -> serde_json::Value {
        match self {
            HookResult::Called => json!({"result": "called"}),
            HookResult::Failed { status, reason } => {
                let mut v = json!({"result": "failed"});
                if let Some(status) = status {
                    v["status"] = json!(status);
                }
                if let Some(reason) = reason {
                    v["reason"] = json!(reason);
                }
                v
            }
            HookResult::TimedOut => json!({"result": "timed-out"}),
        }
    }
}

/// The program a hook's `command[0]` names: a relative path holding a `/` is read against the
/// directory the spec was created from (`meta.json`'s `spec_dir`), as seed paths are; a bare
/// name is looked up on `PATH`; an absolute path is used as written.
fn hook_program(layout: &Layout, program: &str) -> PathBuf {
    let path = Path::new(program);
    if path.is_relative() && program.contains('/') {
        if let Ok(meta) = layout.load_meta() {
            return meta.spec_dir.join(path);
        }
    }
    path.to_path_buf()
}

/// How long a killed hook's process group may take to be gone before the call returns anyway.
const GROUP_GONE: Duration = Duration::from_secs(2);

/// Kills every process of the hook's process group, the hook and whatever it started, reaps the
/// hook and waits until no process of the group is left, `GROUP_GONE` at most: a kill is
/// delivered asynchronously, so a process of the group may still run when the signal is sent,
/// and a process forked while it was sent is killed by the next one.
fn end_group(child: &mut std::process::Child) {
    use rustix::process::{kill_process_group, test_kill_process_group, Pid, Signal};
    let pid = Pid::from_child(child);
    let _ = kill_process_group(pid, Signal::KILL);
    let _ = child.wait();
    let until = Instant::now() + GROUP_GONE;
    while test_kill_process_group(pid).is_ok() && Instant::now() < until {
        let _ = kill_process_group(pid, Signal::KILL);
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Calls `hook` with the source id and the outcome as its last two arguments, in the instance
/// directory and in a process group of its own, and waits at most its timeout: the hook's process
/// group is killed then. The timeout bounds the whole call. A process the hook started and left
/// running after it exited is not waited for, and is not killed.
pub fn call_hook(
    layout: &Layout,
    hook: &m::RunHook,
    source_id: &str,
    outcome: m::RunOutcome,
) -> HookResult {
    let Some((program, args)) = hook.command.split_first() else {
        return HookResult::Failed {
            status: None,
            reason: Some("the hook names no command".into()),
        };
    };
    let timeout = Duration::from_secs(
        hook.timeout_s
            .map(|t| t.min(crate::spec::MAX_HOOK_TIMEOUT_S))
            .and_then(|t| u64::try_from(t).ok())
            .unwrap_or(HOOK_TIMEOUT_S),
    );
    let started = Instant::now();
    // A given spec is refused above a day (`spec::bounds_refusal`); a frozen copy from before the
    // rule is capped at a day here, and an `Instant` past its range is never waited for.
    let deadline = started
        .checked_add(timeout)
        .unwrap_or_else(|| started + Duration::from_secs(HOOK_TIMEOUT_S));
    let spawned = Command::new(hook_program(layout, program))
        .args(args)
        .arg(source_id)
        .arg(outcome_name(outcome))
        .current_dir(&layout.dir)
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(e) => {
            return HookResult::Failed {
                status: None,
                reason: Some(format!("cannot start {program}: {e}")),
            }
        }
    };
    // Read stderr on its own thread, so a hook that writes much cannot block on a full pipe. The
    // pipe stays open as long as any process holding it runs, a background child of the hook
    // included, so it is read until the deadline at most.
    let (sent, stderr) = mpsc::channel();
    if let Some(mut pipe) = child.stderr.take() {
        std::thread::spawn(move || {
            let mut text = String::new();
            let _ = pipe.read_to_string(&mut text);
            let _ = sent.send(text);
        });
    }
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() >= deadline => {
                end_group(&mut child);
                break None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => {
                end_group(&mut child);
                return HookResult::Failed {
                    status: None,
                    reason: Some(format!("cannot wait for {program}: {e}")),
                };
            }
        }
    };
    let Some(status) = status else {
        return HookResult::TimedOut;
    };
    if status.success() {
        return HookResult::Called;
    }
    let left = deadline.saturating_duration_since(Instant::now());
    let stderr = stderr
        .recv_timeout(left)
        .ok()
        // Masked before the cut, so the cut cannot leave a credential shape too short to match.
        .map(|t| {
            crate::mask::mask(t.trim())
                .0
                .chars()
                .take(2000)
                .collect::<String>()
        })
        .filter(|t| !t.is_empty());
    HookResult::Failed {
        status: status.code(),
        reason: stderr,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_label_value_escapes_backslash_quote_and_newline() {
        assert_eq!(label("a\\b\"c\nd"), "a\\\\b\\\"c\\nd");
    }

    #[test]
    fn the_metric_file_lists_every_outcome_of_every_source_and_its_last_run() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path().to_path_buf());
        record(
            &layout,
            "news",
            m::RunOutcome::FetchFailed,
            1_500,
            Some("x"),
        )
        .unwrap();
        record(
            &layout,
            "news",
            m::RunOutcome::FetchFailed,
            2_250,
            Some("y"),
        )
        .unwrap();
        let text = metrics_text(&layout, &["t/news".into(), "t/other".into()]);
        assert!(text
            .contains("cortex_source_runs_total{source=\"t/news\",outcome=\"fetch-failed\"} 2\n"));
        assert!(text.contains("cortex_source_runs_total{source=\"t/news\",outcome=\"ran\"} 0\n"));
        assert!(text.contains("cortex_source_runs_total{source=\"t/other\",outcome=\"ran\"} 0\n"));
        assert!(
            text.contains("cortex_source_last_run_timestamp_seconds{source=\"t/news\"} 2.250\n")
        );
        assert!(!text.contains("last_run_timestamp_seconds{source=\"t/other\"}"));
        let status = load(&layout, "news");
        assert_eq!(status.last_reason.as_deref(), Some("y"));
    }

    /// `story:run-metrics`: a run that ended `ran` sets the last run's documents and adds them
    /// and its tokens to the totals; a run that did not clears the last run's documents and adds
    /// nothing; a token count no answer reported adds nothing.
    #[test]
    fn the_metric_file_reports_the_last_runs_figures_and_their_totals() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path().to_path_buf());
        let figures = Figures {
            read: 3,
            applied: 2,
            input_tokens: Some(105),
            cached_tokens: None,
            output_tokens: Some(40),
        };
        let ran = |at, duration| {
            record_run(
                &layout,
                "news",
                m::RunOutcome::Ran,
                at,
                None,
                Some(duration),
                Some(&figures),
            )
            .unwrap()
        };
        ran(1_000, 1_250);
        ran(5_000, 750);
        let text = metrics_text(&layout, &["t/news".into()]);
        for line in [
            "cortex_source_last_run_duration_seconds{source=\"t/news\"} 0.750\n",
            "cortex_source_last_run_documents{source=\"t/news\",kind=\"read\"} 3\n",
            "cortex_source_last_run_documents{source=\"t/news\",kind=\"applied\"} 2\n",
            "cortex_source_documents_total{source=\"t/news\",kind=\"read\"} 6\n",
            "cortex_source_documents_total{source=\"t/news\",kind=\"applied\"} 4\n",
            "cortex_source_model_tokens_total{source=\"t/news\",kind=\"input\"} 210\n",
            "cortex_source_model_tokens_total{source=\"t/news\",kind=\"cached\"} 0\n",
            "cortex_source_model_tokens_total{source=\"t/news\",kind=\"output\"} 80\n",
        ] {
            assert!(text.contains(line), "{line}{text}");
        }
        record_run(
            &layout,
            "news",
            m::RunOutcome::Terminated,
            9_000,
            Some("stopped"),
            Some(2_000),
            None,
        )
        .unwrap();
        let text = metrics_text(&layout, &["t/news".into()]);
        assert!(
            !text.contains("cortex_source_last_run_documents{"),
            "{text}"
        );
        assert!(text.contains("cortex_source_documents_total{source=\"t/news\",kind=\"read\"} 6\n"));
        assert!(text.contains("cortex_source_last_run_duration_seconds{source=\"t/news\"} 2.000\n"));
        assert!(
            text.contains("cortex_source_runs_total{source=\"t/news\",outcome=\"terminated\"} 1\n")
        );
        assert_eq!(
            hook_outcome(m::RunOutcome::Terminated, false),
            Some(m::RunOutcome::Terminated)
        );
    }

    /// `story:partial-run-counts`: a run that applied a batch and then failed reports its figures
    /// as a run that ended `ran` does: the last run's documents, and its documents and tokens
    /// added to the totals.
    #[test]
    fn a_failed_run_with_figures_reports_and_adds_them() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path().to_path_buf());
        let figures = Figures {
            read: 4,
            applied: 1,
            input_tokens: Some(100),
            cached_tokens: Some(7),
            output_tokens: Some(40),
        };
        for outcome in [
            m::RunOutcome::ModelUnavailable,
            m::RunOutcome::BatchTooLarge,
        ] {
            record_run(
                &layout,
                "news",
                outcome,
                1_000,
                Some("stopped"),
                Some(500),
                Some(&figures),
            )
            .unwrap();
        }
        let text = metrics_text(&layout, &["t/news".into()]);
        for line in [
            "cortex_source_last_run_documents{source=\"t/news\",kind=\"read\"} 4\n",
            "cortex_source_last_run_documents{source=\"t/news\",kind=\"applied\"} 1\n",
            "cortex_source_documents_total{source=\"t/news\",kind=\"read\"} 8\n",
            "cortex_source_documents_total{source=\"t/news\",kind=\"applied\"} 2\n",
            "cortex_source_model_tokens_total{source=\"t/news\",kind=\"input\"} 200\n",
            "cortex_source_model_tokens_total{source=\"t/news\",kind=\"cached\"} 14\n",
            "cortex_source_model_tokens_total{source=\"t/news\",kind=\"output\"} 80\n",
        ] {
            assert!(text.contains(line), "{line}{text}");
        }
    }

    /// `story:store-outcome-unknown-not-counted`: the hook is called for an unknown store outcome,
    /// and the metric file counts it, though the scheduler records no failure for it.
    #[test]
    fn an_unknown_store_outcome_is_reported_to_the_hook_and_the_metric_file() {
        assert_eq!(
            hook_outcome(m::RunOutcome::StoreOutcomeUnknown, false),
            Some(m::RunOutcome::StoreOutcomeUnknown)
        );
        assert_eq!(
            outcome_of("store-outcome-unknown"),
            Some(m::RunOutcome::StoreOutcomeUnknown)
        );
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path().to_path_buf());
        record(
            &layout,
            "news",
            m::RunOutcome::StoreOutcomeUnknown,
            1_500,
            Some("x"),
        )
        .unwrap();
        let text = metrics_text(&layout, &["t/news".into()]);
        assert!(text.contains(
            "cortex_source_runs_total{source=\"t/news\",outcome=\"store-outcome-unknown\"} 1\n"
        ));
    }

    /// A status file that cannot be parsed is an error naming it, and a run's record leaves it as
    /// it is instead of starting the counts again; a missing one is an empty status.
    #[test]
    fn a_corrupt_status_file_is_an_error_and_is_not_rewritten() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = Layout::new(tmp.path().to_path_buf());
        assert_eq!(read(&layout, "news").unwrap().runs.len(), 0);
        let file = path(&layout, "news");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, b"").unwrap();
        let e = read(&layout, "news").unwrap_err();
        assert!(e.contains(&file.display().to_string()), "{e}");
        let e = record(&layout, "news", m::RunOutcome::Ran, 1, None).unwrap_err();
        assert!(e.contains(&file.display().to_string()), "{e}");
        assert_eq!(std::fs::read(&file).unwrap(), b"");
    }
}
