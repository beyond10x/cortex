//! `story:small-robustness`: a model document holding a `"!"` key, EKR's stderr carrying a
//! credential, an `EKR_*` variable inherited by a store `ekr` cortex starts itself, and a host
//! document that is not an object. Each once panicked or leaked; each is now an error, masked or
//! stripped. (The fifth item, two structured sources sharing identities, is in
//! tests/structured_files.rs.)

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{ekr, executable, World, EKR_SESSION_SHIM};

/// A stand-in `ekr` in front of the real one. Every invocation logs its verb and
/// `EKR_FULL_REPLAY` (or `unset`) to `ekr-env.log`; an `ekr session` serves each request as an
/// invocation of its own ([`EKR_SESSION_SHIM`]). With the file `refuse-apply`, `apply-extraction`
/// fails with a credential on stderr, and with the file `refuse-<verb>` so does that verb; with
/// the file `example.json`, `example` prints it.
const EKR: &str = r#"R="@ROOT@"
echo "$1 ${EKR_FULL_REPLAY-unset}" >> "$R/ekr-env.log"
@SHIM@
if { [ "$1" = apply-extraction ] && [ -e "$R/refuse-apply" ]; } || [ -e "$R/refuse-$1" ]; then
  echo "ekr: the store refused the batch: password=hunter2secret" >&2
  exit 1
fi
if [ "$1" = example ] && [ -e "$R/example.json" ]; then cat "$R/example.json"; exit 0; fi
unset EKR_FULL_REPLAY
exec "@EKR@" "$@"
"#;

/// A stand-in `claude` that answers one fact per evidence id it was given, each under the key
/// `"!"`: a tag with no name.
const BARE_TAG: &str = r#"ids=$(grep -oE 'evidence id [0-9a-f-]{36}' | cut -d' ' -f3)
facts=""
for id in $ids; do
  facts="$facts{\"!\":{\"subject\":{\"node_type\":\"Organization\",\"aliases\":[\"Example Labs\"]},\"property\":\"website\",\"value\":{\"value_kind\":\"String\",\"value\":\"example.org\"},\"evidence\":[\"$id\"]}},"
done
facts="${facts%,}"
cat <<EOF
{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{"format":"ekr.extraction-document/1","ontology":{"node_types":[{"name":"Organization","parents":[],"abstract_type":false,"properties":[{"name":"website","value":{"value_kind":"String"},"cardinality":"One","required":false}]}],"edge_types":[]},"entities":[{"node_type":"Organization","aliases":["Example Labs"]}],"facts":[$facts]}}
EOF
"#;

/// The world with the stand-in `ekr` at `bin/ekr`.
fn world() -> World {
    let w = World::new();
    executable(
        &w.bin.join("ekr"),
        &EKR.replace("@ROOT@", &w.root.display().to_string())
            .replace("@SHIM@", EKR_SESSION_SHIM)
            .replace("@EKR@", &ekr().display().to_string()),
    );
    w
}

/// The spec of instance `name`, its `ekr` the stand-in.
fn spec(w: &World, name: &str) -> PathBuf {
    let path = w.spec(name, "conn_test");
    let text = std::fs::read_to_string(&path).unwrap().replace(
        &ekr().display().to_string(),
        &w.bin.join("ekr").display().to_string(),
    );
    std::fs::write(&path, text).unwrap();
    path
}

const REPLAY: [(&str, &str); 1] = [("EKR_FULL_REPLAY", "1")];

fn env(pairs: &[(&'static str, &'static str)]) -> Vec<(&'static str, &'static Path)> {
    pairs.iter().map(|(k, v)| (*k, Path::new(*v))).collect()
}

fn create(w: &World, name: &str) {
    let path = spec(w, name);
    let (code, created) = w.cortex_env(
        &["create", "--spec", path.to_str().unwrap(), "--no-units"],
        &env(&REPLAY),
    );
    assert_eq!(code, 0, "{created}");
}

/// `EKR_FULL_REPLAY` set in cortex's environment does not reach an `ekr` that opens the store
/// directly: not the session, not a verb it serves, not `apply-extraction`. (The pinned binary's
/// own `example` and `schema`, which open no store, are left as they are.)
#[test]
fn an_inherited_ekr_full_replay_does_not_reach_a_direct_path_store_ekr() {
    let w = world();
    create(&w, "r");
    let (code, ran) = w.cortex_env(&["run", "r/news"], &env(&REPLAY));
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let log = w.lines("ekr-env.log");
    let store: Vec<&String> = log
        .iter()
        .filter(|l| !l.starts_with("example ") && !l.starts_with("schema "))
        .collect();
    for verb in ["session ", "apply-extraction "] {
        assert!(store.iter().any(|l| l.starts_with(verb)), "{log:?}");
    }
    let leaked: Vec<&&String> = store.iter().filter(|l| !l.ends_with(" unset")).collect();
    assert!(leaked.is_empty(), "EKR_FULL_REPLAY reached {leaked:?}");
}

/// Every file under `dir` whose text contains `needle`.
fn holding(dir: &Path, needle: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(holding(&path, needle));
        } else if std::fs::read(&path).is_ok_and(|b| String::from_utf8_lossy(&b).contains(needle)) {
            found.push(path);
        }
    }
    found
}

/// A credential-shaped string in what EKR wrote to stderr when an apply failed is masked in the
/// run's report and in what the run recorded: no file of the instance (its status, its
/// `cortex.log`) holds it.
#[test]
fn a_credential_in_ekrs_stderr_is_masked_in_the_report_and_the_log() {
    let w = world();
    create(&w, "m");
    std::fs::write(w.root.join("refuse-apply"), "").unwrap();
    let (code, ran) = w.cortex(&["run", "m/news"]);
    assert_ne!(code, 0, "{ran}");
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("the store refused the batch"), "{ran}");
    assert!(!ran.to_string().contains("hunter2secret"), "{ran}");
    assert!(reason.contains("[masked:"), "{ran}");
    let instance = w.home.join("instances/m");
    assert!(
        !holding(&instance, "the store refused the batch").is_empty(),
        "the run recorded its reason somewhere under {}",
        instance.display()
    );
    let leaked = holding(&instance, "hunter2secret");
    assert!(leaked.is_empty(), "the credential reached {leaked:?}");
}

/// D38, the class of the masked-stderr case above: an `ekr` that `cortex quality` runs outside a
/// source run (`ekr sample`) and that fails with a credential on stderr is masked in the answer
/// and in every file of the instance, as a run's store verb is.
#[test]
fn a_credential_in_ekrs_stderr_outside_a_run_is_masked_too() {
    let w = world();
    create(&w, "q");
    std::fs::write(w.root.join("refuse-sample"), "").unwrap();
    let (code, out) = w.cortex(&["quality", "q", "--sample", "5"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("sample-failed")),
        "{out}"
    );
    assert!(
        out.to_string().contains("the store refused the batch"),
        "{out}"
    );
    assert!(!out.to_string().contains("hunter2secret"), "{out}");
    assert!(out.to_string().contains("[masked:"), "{out}");
    let leaked = holding(&w.home.join("instances/q"), "hunter2secret");
    assert!(leaked.is_empty(), "the credential reached {leaked:?}");
}

/// An `ekr example ekr.cli-host/1` that prints a document that is not an object, or whose
/// `authority` or `validation_profile` is not one, fails `create` with an error; it does not panic.
#[test]
fn a_host_document_that_is_not_an_object_is_an_error() {
    for (n, example) in [
        "[1]",
        r#"{"tenant":"t","authority":1}"#,
        r#"{"tenant":"t","authority":{"validation_profile":"open"}}"#,
    ]
    .into_iter()
    .enumerate()
    {
        let w = world();
        std::fs::write(w.root.join("example.json"), example).unwrap();
        let name = format!("h{n}");
        let path = spec(&w, &name);
        let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
            .args(["create", "--spec", path.to_str().unwrap(), "--no-units"])
            .env("CORTEX_HOME", &w.home)
            .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
            .env("CORTEX_CLAUDE", w.bin.join("claude"))
            .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
            .env("CORTEX_UNIT_DIR", &w.units)
            .output()
            .unwrap();
        let said = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            matches!(out.status.code(), Some(1 | 2)),
            "{example}: {:?} {said}",
            out.status
        );
        assert!(!said.contains("panicked"), "{example}: {said}");
        assert!(said.contains("is not an object"), "{example}: {said}");
    }
}

/// A model document holding an object key `"!"`, a tag with no name, is refused as an extraction
/// error naming it; the run does not panic, and nothing is applied.
#[test]
fn a_bare_tag_key_in_the_model_document_is_an_extraction_error() {
    let w = world();
    create(&w, "t");
    executable(&w.bin.join("claude"), BARE_TAG);
    let head = w.head("t");
    let (code, ran) = w.cortex(&["run", "t/news"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("extraction-failed")),
        "{ran}"
    );
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("\"!\""), "{reason}");
    assert_eq!(w.head("t"), head, "nothing was applied");
}
