//! The `connectors` binary: which connections exist, and invoking an admitted operation. cortex
//! never sees a credential; Connectors signs every call.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

use serde_json::Value;

use crate::terminate::Waited;

pub struct Connectors {
    pub bin: PathBuf,
    /// Whether an invocation answered: from then on a revalidation that stays unavailable fails
    /// the read as [`InvokeError::Failed`], since the run has fetched documents.
    answered: AtomicBool,
}

static BIN: OnceLock<PathBuf> = OnceLock::new();

/// Records the `connectors` binary the command line names (`--connectors`), once, for [`bin`].
pub fn set_bin(bin: PathBuf) {
    let _ = BIN.set(bin);
}

/// The `connectors` binary this process uses: the one [`set_bin`] recorded, else
/// `CORTEX_CONNECTORS`, else `connectors` on `PATH`.
pub fn bin() -> PathBuf {
    BIN.get().cloned().unwrap_or_else(|| {
        std::env::var_os("CORTEX_CONNECTORS")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("connectors"))
    })
}

/// How a spec names a Connectors connection: by its id, or by a profile of its adapter, resolved
/// with [`Connectors::resolve`] each time cortex needs the connection.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reference {
    Id(String),
    Profile(String),
}

impl Reference {
    /// The reference of a spec's `connection` and `profile`, of which `spec::check` admits exactly
    /// one; a profile is taken when both are given.
    pub fn of(connection: Option<&String>, profile: Option<&String>) -> Self {
        match (profile, connection) {
            (Some(profile), _) => Self::Profile(profile.clone()),
            (None, connection) => Self::Id(connection.cloned().unwrap_or_default()),
        }
    }

    /// The reference as a reason names it: `<adapter>:<id>` or `<adapter>:profile <profile>`.
    pub fn named(&self, adapter: &str) -> String {
        match self {
            Self::Id(id) => format!("{adapter}:{id}"),
            Self::Profile(profile) => format!("{adapter}:profile {profile}"),
        }
    }
}

impl From<&str> for Reference {
    fn from(id: &str) -> Self {
        Self::Id(id.to_string())
    }
}

/// The one connection of `listed`, a `connections list --adapter <adapter>` result, whose
/// `adapter` is `adapter`, whose `profile` is `profile` and whose `state` is not `revoked`: its id.
/// None, more than one, or a list with a further page (`next_cursor`), which cortex does not read,
/// is refused with a reason naming the adapter, the profile, the count and every matching id.
pub fn pick(adapter: &str, profile: &str, listed: &Value) -> Result<String, String> {
    let Some(entries) = listed["connections"].as_array() else {
        return Err(format!(
            "{adapter}: profile {profile}: connections list answered no connections array"
        ));
    };
    let ids: std::collections::BTreeSet<&str> = entries
        .iter()
        .filter(|c| {
            c["adapter"].as_str() == Some(adapter)
                && c["profile"].as_str() == Some(profile)
                && c["state"].as_str() != Some("revoked")
        })
        .filter_map(|c| c["connection"].as_str())
        .collect();
    let named = ids.iter().copied().collect::<Vec<_>>().join(", ");
    let count = ids.len();
    let further = listed["next_cursor"].as_str().is_some();
    match ids.into_iter().next() {
        Some(id) if count == 1 && !further => Ok(id.to_string()),
        _ if further => Err(format!(
            "{adapter}: profile {profile}: {count} live connections on the first page of \
             connections list ({named}), which has further pages cortex does not read; name the \
             connection by id"
        )),
        None => Err(format!(
            "{adapter}: profile {profile}: 0 live connections; run connectors connections connect \
             --adapter {adapter} --profile {profile}"
        )),
        Some(_) => Err(format!(
            "{adapter}: profile {profile}: {count} live connections ({named}); revoke all but \
             one, or name the connection by id"
        )),
    }
}

/// Why an invocation answered nothing.
#[derive(Debug)]
pub enum InvokeError {
    /// Connectors refused at admission: no such connection, adapter or operation here.
    Missing(String),
    /// Connectors refused at admission because the connection's validation evidence lapsed.
    Lapsed(String),
    /// Anything else: an outage, a refused or failed provider call, an unreadable answer.
    Failed(String),
    /// Connectors answered a retryable failure (`kind` `operational`, `next_action`
    /// `retry_explicitly`) to every attempt of a revalidation, before any invocation answered.
    Unavailable(String),
    /// The run was sent SIGTERM ([`crate::terminate`]) during a call, which was ended, or during a
    /// wait between attempts: what it stopped.
    Terminated(String),
}

impl std::fmt::Display for InvokeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing(m) => write!(f, "connection missing: {m}"),
            Self::Lapsed(m) => write!(f, "connection evidence lapsed: {m}"),
            Self::Failed(m) => write!(f, "connectors failed: {m}"),
            Self::Unavailable(m) => write!(f, "connection unavailable: {m}"),
            Self::Terminated(m) => f.write_str(m),
        }
    }
}

/// The end of the message [`envelope`] writes for a `timeout` at `admission`: Connectors' own
/// deadline ran out. The operation is a read, so repeating it costs at most one more provider call.
const TIMEOUT_AT_ADMISSION: &str = ": timeout at admission";

fn envelope(doc: Value, what: &str) -> Result<Value, InvokeError> {
    if doc["ok"] == Value::Bool(true) {
        return Ok(doc["result"].clone());
    }
    let data = &doc["error"]["data"];
    let code = data["code"].as_str().unwrap_or("unknown");
    let stage = data["stage"].as_str().unwrap_or("unknown");
    let message = format!("{what}: {code} at {stage}");
    if code == "not_granted" && stage == "admission" {
        return Err(InvokeError::Lapsed(message));
    }
    if code == "not_found" && stage == "admission" {
        Err(InvokeError::Missing(message))
    } else {
        Err(InvokeError::Failed(message))
    }
}

/// Error codes that refuse an operation, whatever else the envelope says: never retried.
const REFUSALS: [&str; 5] = [
    "not_found",
    "not_granted",
    "revoked",
    "unauthenticated",
    "forbidden",
];

/// Whether `doc` is a failure Connectors asks the caller to repeat: `kind` `operational` with
/// `next_action` `retry_explicitly` (such as `unavailable` at `dispatch`), and no refusal.
fn retryable(doc: &Value) -> bool {
    let data = &doc["error"]["data"];
    doc["ok"] != Value::Bool(true)
        && data["kind"].as_str() == Some("operational")
        && data["next_action"].as_str() == Some("retry_explicitly")
        && !REFUSALS.contains(&data["code"].as_str().unwrap_or_default())
}

/// A whole-number setting from the environment, `default` when it is unset or unreadable.
fn setting(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

impl Connectors {
    pub fn new(bin: PathBuf) -> Self {
        Self {
            bin,
            answered: AtomicBool::new(false),
        }
    }

    fn json(&self, args: &[&str], what: &str) -> Result<Value, InvokeError> {
        envelope(self.answer(args, None, what)?, what)
    }

    /// [`Self::json`], with a [`retryable`] failure answered as [`InvokeError::Unavailable`].
    fn json_or_unavailable(&self, args: &[&str], what: &str) -> Result<Value, InvokeError> {
        let doc = self.answer(args, None, what)?;
        let again = retryable(&doc);
        match envelope(doc, what) {
            Err(InvokeError::Failed(message)) if again => Err(InvokeError::Unavailable(message)),
            other => other,
        }
    }

    /// The JSON document `connectors` answered, success or refusal. `input`, when given, is the
    /// child's standard input, written whole and closed; otherwise it reads none. The call runs in
    /// a process group of its own and is waited for as a model call is
    /// ([`crate::terminate::wait`]): a run sent SIGTERM ends it, and its group, and answers
    /// [`InvokeError::Terminated`].
    fn answer(
        &self,
        args: &[&str],
        input: Option<&[u8]>,
        what: &str,
    ) -> Result<Value, InvokeError> {
        use std::os::unix::process::CommandExt;
        if crate::terminate::requested() {
            return Err(InvokeError::Terminated(format!(
                "the {what} call was not made"
            )));
        }
        let mut child = Command::new(&self.bin)
            .args(args)
            .args(["--output", "json"])
            // A group of its own is not the terminal's foreground group: its stdin is a pipe or
            // nothing, never the terminal.
            .process_group(0)
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| InvokeError::Failed(format!("cannot run connectors: {e}")))?;
        let out = match crate::terminate::wait(&mut child, input, "connectors") {
            Waited::Done(out) => out,
            Waited::Failed(e) => return Err(InvokeError::Failed(e)),
            Waited::Terminated => {
                return Err(InvokeError::Terminated(format!(
                    "the {what} call was ended"
                )))
            }
        };
        // A refusal's JSON envelope goes to stderr, with nothing on stdout.
        let answer = if out.stdout.iter().all(u8::is_ascii_whitespace) {
            &out.stderr
        } else {
            &out.stdout
        };
        serde_json::from_slice(answer)
            .map_err(|e| InvokeError::Failed(format!("{what}: no JSON answer: {e}")))
    }

    /// The id `reference` names among `adapter`'s connections: an id as given, a profile through
    /// [`pick`] over `connections list --adapter <adapter>` (a retryable failure of that read is
    /// tried again as in [`Self::attempts`]), its refusal as
    /// [`InvokeError::Missing`]. A profile resolved once is answered again from this process's
    /// memory, so one command uses one connection throughout.
    pub fn resolve(&self, adapter: &str, reference: &Reference) -> Result<String, InvokeError> {
        type Resolved =
            std::sync::Mutex<std::collections::BTreeMap<(PathBuf, String, String), String>>;
        static RESOLVED: OnceLock<Resolved> = OnceLock::new();
        let profile = match reference {
            Reference::Id(id) => return Ok(id.clone()),
            Reference::Profile(profile) => profile,
        };
        let key = (self.bin.clone(), adapter.to_string(), profile.clone());
        let resolved = RESOLVED.get_or_init(Default::default);
        if let Some(id) = resolved.lock().expect("not poisoned").get(&key) {
            return Ok(id.clone());
        }
        let listed = self.attempts(|| {
            self.json_or_unavailable(
                &["connections", "list", "--adapter", adapter],
                "connections list",
            )
        })?;
        let id = pick(adapter, profile, &listed).map_err(InvokeError::Missing)?;
        resolved
            .lock()
            .expect("not poisoned")
            .insert(key, id.clone());
        Ok(id)
    }

    /// The state of `connection` among `adapter`'s connections, or `None` when it is not listed.
    pub fn connection_state(
        &self,
        adapter: &str,
        connection: &str,
    ) -> Result<Option<String>, InvokeError> {
        let listed = self.json(
            &["connections", "list", "--adapter", adapter],
            "connections list",
        )?;
        Ok(listed["connections"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|c| c["connection"].as_str() == Some(connection))
            .map(|c| c["state"].as_str().unwrap_or("unknown").to_string()))
    }

    /// Renews a connection whose validation evidence lapsed (`pending`), so a read is admitted.
    /// Connectors keeps evidence for a short lifetime (60 s for the shipped HTTP adapters), and
    /// refuses reads with `not_granted` after it; a scheduled run therefore revalidates first.
    pub fn ensure_ready(&self, adapter: &str, connection: &str) -> Result<(), InvokeError> {
        self.revalidate(adapter, connection, false).map(|_| ())
    }

    /// Revalidates `connection`; unless `force`, only when it is not `ready`. A retryable failure
    /// of the revalidation or of the list read before it ([`retryable`]) is tried again, at most
    /// `CORTEX_REVALIDATE_ATTEMPTS` attempts in all (4, at least 1), pausing
    /// `CORTEX_REVALIDATE_PAUSE_MS` (5000) before the second and doubling after it; one that
    /// remains is [`InvokeError::Unavailable`], naming the last error. A refusal is not retried.
    /// Answers the connection's `connections list` entry, read before the revalidation.
    fn revalidate(
        &self,
        adapter: &str,
        connection: &str,
        force: bool,
    ) -> Result<Value, InvokeError> {
        self.attempts(|| self.revalidate_once(adapter, connection, force))
    }

    /// Runs `attempt` until it is not [`InvokeError::Unavailable`], at most
    /// `CORTEX_REVALIDATE_ATTEMPTS` times (4, at least 1), pausing `CORTEX_REVALIDATE_PAUSE_MS`
    /// (5000) before the second and doubling after it; an unavailable answer that remains names the
    /// number of attempts. Shared by the revalidation and the profile resolution.
    fn attempts<T>(
        &self,
        mut attempt_once: impl FnMut() -> Result<T, InvokeError>,
    ) -> Result<T, InvokeError> {
        let attempts = setting("CORTEX_REVALIDATE_ATTEMPTS", 4).max(1);
        let mut pause = Duration::from_millis(setting("CORTEX_REVALIDATE_PAUSE_MS", 5000));
        let mut attempt = 1;
        loop {
            match attempt_once() {
                Err(InvokeError::Unavailable(m)) if attempt < attempts => {
                    if crate::terminate::sleep(pause) {
                        return Err(InvokeError::Terminated(format!(
                            "it waited to try again after {m}"
                        )));
                    }
                    pause = pause.saturating_mul(2);
                    attempt += 1;
                }
                Err(InvokeError::Unavailable(m)) => {
                    return Err(InvokeError::Unavailable(format!(
                        "{m}, after {attempts} attempts"
                    )))
                }
                other => return other,
            }
        }
    }

    /// One attempt of [`Self::revalidate`].
    fn revalidate_once(
        &self,
        adapter: &str,
        connection: &str,
        force: bool,
    ) -> Result<Value, InvokeError> {
        let listed = self.json_or_unavailable(
            &["connections", "list", "--adapter", adapter],
            "connections list",
        )?;
        let Some(found) = listed["connections"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|c| c["connection"].as_str() == Some(connection))
        else {
            return Err(InvokeError::Missing(format!(
                "{adapter}: no connection {connection}"
            )));
        };
        let found = found.clone();
        if !force && found["state"].as_str() == Some("ready") {
            return Ok(found);
        }
        let revision = found["revision"].as_str().unwrap_or_default().to_string();
        let what = "connections revalidate";
        let doc = self.answer(
            &[
                "connections",
                "revalidate",
                "--adapter",
                adapter,
                "--connection",
                connection,
                "--expected-revision",
                &revision,
            ],
            None,
            what,
        )?;
        // Connectors can apply a revalidation and still be unable to confirm it
        // (`outcome_unknown` at publication); it then names reading the status as the next step.
        let unconfirmed = doc["error"]["data"]["next_action"].as_str() == Some("retry_status");
        let again = retryable(&doc);
        match envelope(doc, what) {
            Err(InvokeError::Failed(message)) if unconfirmed => self
                .await_ready(adapter, connection, &message)
                .map(|()| found),
            Err(InvokeError::Failed(message)) if again => Err(InvokeError::Unavailable(message)),
            other => other.map(|_| found),
        }
    }

    /// Reads `connection`'s status until it is `ready`, up to `CORTEX_STATUS_POLLS` reads (12)
    /// `CORTEX_STATUS_POLL_MS` apart (5000); otherwise fails naming `cause` and the last state.
    fn await_ready(&self, adapter: &str, connection: &str, cause: &str) -> Result<(), InvokeError> {
        let polls = setting("CORTEX_STATUS_POLLS", 12).max(1);
        let pause = Duration::from_millis(setting("CORTEX_STATUS_POLL_MS", 5000));
        let mut state = String::from("unknown");
        for read in 0..polls {
            if read > 0 && crate::terminate::sleep(pause) {
                return Err(InvokeError::Terminated(format!(
                    "it waited to read the status of {connection} again"
                )));
            }
            let status = self.json(
                &[
                    "connections",
                    "status",
                    "--adapter",
                    adapter,
                    "--connection",
                    connection,
                ],
                "connections status",
            )?;
            state = status["connection"]["summary"]["state"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            if state == "ready" {
                return Ok(());
            }
        }
        Err(InvokeError::Failed(format!(
            "{cause}; connections status still {state} after {polls} reads"
        )))
    }

    /// Invokes `operation` and answers the adapter's own result, which must be a JSON object; a
    /// lapse is renewed and a timeout at admission retried, each at most once. The call is
    /// Connectors 0.60.0's `invoke <connector>/<instance> <operation>`: the target comes from the
    /// connection's `connections list` entry ([`target`]), the schema and revision are the
    /// current description's, and the input goes on standard input. Connectors wraps the answer
    /// as `{connector, instance, operation, revision, result}` ([`result`]).
    /// A revalidation still unavailable after its retries is [`InvokeError::Unavailable`] until an
    /// invocation of this process answered, and [`InvokeError::Failed`] after it: a run that has
    /// fetched documents fails as it always did.
    pub fn invoke(
        &self,
        adapter: &str,
        connection: &str,
        operation: &str,
        input: &Value,
    ) -> Result<Value, InvokeError> {
        let entry = self
            .revalidate(adapter, connection, false)
            .map_err(|e| self.after_fetch(e))?;
        let target = target(adapter, connection, &entry)?;
        let input = serde_json::to_vec(input).expect("plain JSON");
        // Evidence listed as ready can lapse before the read is admitted; renew once and retry.
        // A timeout at admission is Connectors' deadline running out on a read; retry that once
        // too. Each retry is spent at most once, so the loop runs at most three invocations.
        let (mut renew, mut timeout) = (true, true);
        let answer = loop {
            match self.invoke_once(adapter, &target, connection, operation, &input) {
                Err(InvokeError::Lapsed(_)) if renew => {
                    renew = false;
                    self.revalidate(adapter, connection, true)
                        .map_err(|e| self.after_fetch(e))?;
                }
                Err(InvokeError::Failed(m)) if timeout && m.ends_with(TIMEOUT_AT_ADMISSION) => {
                    timeout = false;
                }
                other => break other?,
            }
        };
        self.answered.store(true, Ordering::Relaxed);
        result(&format!("{adapter}.{operation}"), answer)
    }

    /// `error`, an [`InvokeError::Unavailable`] made [`InvokeError::Failed`] once an invocation
    /// answered.
    fn after_fetch(&self, error: InvokeError) -> InvokeError {
        match error {
            InvokeError::Unavailable(m) if self.answered.load(Ordering::Relaxed) => {
                InvokeError::Failed(m)
            }
            other => other,
        }
    }

    /// One `connectors invoke <target> <operation> --connection <connection> --input-stdin`, the
    /// one spawn of a call: no description is read first, since `--schema` and `--revision` left
    /// out are the current description's. No field of `input` is in the argv.
    fn invoke_once(
        &self,
        adapter: &str,
        target: &str,
        connection: &str,
        operation: &str,
        input: &[u8],
    ) -> Result<Value, InvokeError> {
        let what = format!("{adapter}.{operation}");
        let doc = self.answer(
            &[
                "invoke",
                target,
                operation,
                "--connection",
                connection,
                "--input-stdin",
            ],
            Some(input),
            &what,
        )?;
        envelope(doc, &what)
    }
}

/// The `invoke` target `<connector>/<instance>` of a `connections list` entry, which carries
/// `connector` and `instance` beside `adapter` from Connectors 0.60.0 on. An entry without them is
/// an older Connectors, whose removed invoke form cortex no longer calls: refused naming the entry.
fn target(adapter: &str, connection: &str, entry: &Value) -> Result<String, InvokeError> {
    let field = |name: &str| entry[name].as_str().filter(|v| !v.is_empty());
    match (field("connector"), field("instance")) {
        (Some(connector), Some(instance)) => Ok(format!("{connector}/{instance}")),
        _ => Err(InvokeError::Failed(format!(
            "{adapter}: the connections list entry of {connection} names no connector and \
             instance; cortex invokes through Connectors 0.60.0 or newer"
        ))),
    }
}

/// The JSON type of `value`, as a reason names it.
fn json_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

/// The provider's `result` of an `invoke` answer (`InvokeResult`), which a source reads as a JSON
/// object; any other JSON type, or none, is refused naming it.
fn result(what: &str, mut answer: Value) -> Result<Value, InvokeError> {
    match answer.get_mut("result").map(Value::take) {
        Some(inner @ Value::Object(_)) => Ok(inner),
        Some(other) => Err(InvokeError::Failed(format!(
            "{what}: the result is {}, not an object",
            json_type(&other)
        ))),
        None => Err(InvokeError::Failed(format!(
            "{what}: the answer has no result"
        ))),
    }
}

/// A provider's answer body: the catalog engine wraps an HTTP answer as `{status, body}`.
pub fn body(result: &Value) -> &Value {
    match result.get("body") {
        Some(body) if body.is_object() || body.is_array() => body,
        _ => result,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn entry(adapter: &str, id: &str, profile: &str, state: &str) -> Value {
        json!({"adapter": adapter, "instance_id": "i", "connection": id, "profile": profile,
               "revision": "r1", "state": state})
    }

    #[test]
    fn a_profile_resolves_to_its_one_live_connection() {
        let listed = json!({"connections": [
            entry("tavily", "conn_A", "tavily.other", "ready"),
            entry("tavily", "conn_B", "tavily.api-key", "pending"),
            entry("tavily", "conn_R", "tavily.api-key", "revoked"),
        ]});
        assert_eq!(
            pick("tavily", "tavily.api-key", &listed),
            Ok("conn_B".to_string())
        );
    }

    #[test]
    fn two_live_connections_of_a_profile_are_refused_naming_both() {
        let listed = json!({"connections": [
            entry("tavily", "conn_C", "tavily.api-key", "ready"),
            entry("tavily", "conn_B", "tavily.api-key", "pending"),
        ]});
        let why = pick("tavily", "tavily.api-key", &listed).unwrap_err();
        for part in ["tavily", "tavily.api-key", " 2 ", "conn_B", "conn_C"] {
            assert!(why.contains(part), "{part:?} in {why}");
        }
    }

    #[test]
    fn no_live_connection_of_a_profile_is_refused_with_the_count_0() {
        let listed = json!({"connections": [
            entry("tavily", "conn_A", "tavily.other", "ready"),
            entry("tavily", "conn_R", "tavily.api-key", "revoked"),
        ]});
        let why = pick("tavily", "tavily.api-key", &listed).unwrap_err();
        for part in ["tavily", "tavily.api-key", " 0 "] {
            assert!(why.contains(part), "{part:?} in {why}");
        }
        assert!(!why.contains("conn_R"), "{why}");
    }

    /// An entry without a `profile`, or of another adapter, is no match: a stand-in or an older
    /// Connectors that omits the field never lends its connection to a profile.
    #[test]
    fn an_entry_without_the_profile_or_of_another_adapter_is_no_match() {
        let listed = json!({"connections": [
            {"adapter": "tavily", "connection": "conn_N", "state": "ready", "revision": "r"},
            entry("exa", "conn_X", "tavily.api-key", "ready"),
        ]});
        let why = pick("tavily", "tavily.api-key", &listed).unwrap_err();
        assert!(why.contains(" 0 "), "{why}");
    }

    /// The first page is all cortex reads: with a further page, one match is not known to be the
    /// only one.
    #[test]
    fn a_list_with_a_further_page_is_refused() {
        let listed = json!({"connections": [entry("tavily", "conn_B", "tavily.api-key", "ready")],
                            "next_cursor": "c1"});
        let why = pick("tavily", "tavily.api-key", &listed).unwrap_err();
        for part in ["tavily.api-key", "conn_B", "further pages"] {
            assert!(why.contains(part), "{part:?} in {why}");
        }
    }

    #[test]
    fn a_list_without_a_connections_array_is_refused() {
        assert!(pick("tavily", "p", &json!({})).is_err());
    }

    /// `resolve` answers an id as given without asking Connectors, and lists a profile's adapter
    /// once per process: the second resolution of the same profile does not list again.
    #[test]
    fn resolve_lists_a_profile_once_and_never_for_an_id() {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("connectors");
        let log = dir.path().join("calls.log");
        let script = format!(
            "#!/bin/sh\necho \"$*\" >> \"{}\"\nprintf '%s' '{}'\n",
            log.display(),
            json!({"ok": true, "result": {"connections": [
                entry("tavily", "conn_B", "tavily.api-key", "ready")]}})
        );
        let status = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(r#"printf '%s' "$2" > "$1" && chmod 755 "$1""#)
            .arg("sh")
            .arg(&bin)
            .arg(&script)
            .status()
            .unwrap();
        assert!(status.success());
        let connectors = Connectors::new(bin);
        assert_eq!(
            connectors
                .resolve("tavily", &Reference::from("conn_test"))
                .unwrap(),
            "conn_test"
        );
        assert!(!log.exists(), "an id asked Connectors");
        let profile = Reference::Profile("tavily.api-key".into());
        assert_eq!(connectors.resolve("tavily", &profile).unwrap(), "conn_B");
        assert_eq!(connectors.resolve("tavily", &profile).unwrap(), "conn_B");
        let calls = std::fs::read_to_string(&log).unwrap();
        assert_eq!(calls.lines().count(), 1, "{calls}");
        assert!(
            calls.contains("connections list --adapter tavily"),
            "{calls}"
        );
    }

    /// `story:connectors-invoke-0-60`: the target comes from the list entry's `connector` and
    /// `instance`, not from the adapter; an entry without them is refused naming the version.
    #[test]
    fn the_invoke_target_is_the_entrys_connector_and_instance() {
        let mut listed = entry("tavily-alias", "conn_A", "p", "ready");
        listed["connector"] = json!("tavily");
        listed["instance"] = json!("main");
        assert_eq!(
            target("tavily-alias", "conn_A", &listed).unwrap(),
            "tavily/main"
        );
        for missing in ["connector", "instance"] {
            let mut old = listed.clone();
            old.as_object_mut().unwrap().remove(missing);
            let why = target("tavily-alias", "conn_A", &old)
                .unwrap_err()
                .to_string();
            for part in ["conn_A", "Connectors 0.60.0 or newer"] {
                assert!(why.contains(part), "{part:?} in {why}");
            }
        }
    }

    /// R3-10: an object `result` is the answer; any other JSON type is refused naming it.
    #[test]
    fn a_result_that_is_no_object_is_refused_naming_its_json_type() {
        let answer = |result: Value| {
            json!({"connector": "tavily", "instance": "main", "operation": "op",
                   "revision": "r", "result": result})
        };
        assert_eq!(
            result("tavily.op", answer(json!({"records": []}))).unwrap(),
            json!({"records": []})
        );
        for (value, named) in [
            (json!([1]), "an array"),
            (json!("{\"records\":[]}"), "a string"),
            (json!(3), "a number"),
            (json!(true), "a boolean"),
            (Value::Null, "null"),
        ] {
            let why = result("tavily.op", answer(value)).unwrap_err().to_string();
            assert!(
                why.contains("tavily.op: the result is ") && why.contains(named),
                "{named:?} in {why}"
            );
        }
        let why = result("tavily.op", json!({"connector": "tavily"}))
            .unwrap_err()
            .to_string();
        assert!(why.contains("no result"), "{why}");
    }

    #[test]
    fn a_spec_reference_takes_its_profile_or_its_id() {
        let (id, profile) = (Some("conn_A".to_string()), Some("p".to_string()));
        assert_eq!(
            Reference::of(id.as_ref(), None),
            Reference::Id("conn_A".into())
        );
        assert_eq!(
            Reference::of(None, profile.as_ref()),
            Reference::Profile("p".into())
        );
        assert_eq!(
            Reference::Profile("p".into()).named("tavily"),
            "tavily:profile p"
        );
        assert_eq!(Reference::from("conn_A").named("tavily"), "tavily:conn_A");
    }

    /// `story:connections-revalidate-retry`: only an operational failure Connectors asks to repeat
    /// is retried; a refusal never is, whatever its `next_action`.
    #[test]
    fn only_an_operational_failure_to_retry_explicitly_is_retryable() {
        let failure = |code: &str, kind: &str, next: &str| {
            json!({"ok": false, "error": {"code": "failure", "data": {
                "code": code, "kind": kind, "next_action": next, "stage": "dispatch"}}})
        };
        assert!(retryable(&failure(
            "unavailable",
            "operational",
            "retry_explicitly"
        )));
        assert!(!retryable(&failure(
            "unavailable",
            "operational",
            "retry_status"
        )));
        assert!(!retryable(&failure(
            "unauthenticated",
            "operational",
            "repair_connection"
        )));
        assert!(!retryable(&failure(
            "unavailable",
            "policy",
            "retry_explicitly"
        )));
        for refusal in REFUSALS {
            assert!(
                !retryable(&failure(refusal, "operational", "retry_explicitly")),
                "{refusal}"
            );
        }
        assert!(!retryable(&json!({"ok": true, "result": {}})));
    }
}
