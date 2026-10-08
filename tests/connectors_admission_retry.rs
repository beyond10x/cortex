//! A Connectors invocation refused with `timeout` at `admission` was never sent to the provider, so
//! a run retries it once before it answers `fetch-failed`. A stand-in `connectors` times out at
//! admission for as many invocations as the file `timeouts` holds lines, then answers.

mod common;

use common::{executable, World};

/// The world's stand-in `connectors`, with `operations invoke` timing out at admission while the
/// file `timeouts` holds a line; each such invocation removes one.
fn world() -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(
        &w.bin.join("connectors"),
        &format!(
            r#"R="{root}"
case "$*" in
  *"connections list"*) printf '{{"ok":true,"result":{{"connections":[{{"adapter":"tavily","connection":"conn_test","state":"ready","revision":"rev1"}}]}}}}' ;;
  *"connections revalidate"*) echo "$*" >> "$R/revalidations.log"; echo '{{"ok":true,"result":{{}}}}' ;;
  *"connections status"*) printf '{{"ok":true,"result":{{"connection":{{"summary":{{"state":"ready"}}}}}}}}' ;;
  *"operations describe"*) echo "$*" >> "$R/describes.log"; echo '{{"ok":true,"result":{{"schema":"s","revision":"r"}}}}' ;;
  *"operations invoke"*)
    echo "$*" >> "$R/invocations.log"
    if [ -s "$R/timeouts" ]; then
      sed -i '1d' "$R/timeouts"
      echo '{{"ok":false,"error":{{"code":"failure","data":{{"code":"timeout","stage":"admission"}}}}}}' >&2; exit 1
    fi
    cat "$R/answer.json" ;;
  *) echo '{{"ok":false}}'; exit 2 ;;
esac
"#
        ),
    );
    w
}

fn created(w: &World, name: &str, timeouts: usize) {
    let spec = w.spec(name, "conn_test");
    let (code, out) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{out}");
    std::fs::write(w.root.join("timeouts"), "t\n".repeat(timeouts)).unwrap();
}

#[test]
fn an_invocation_that_times_out_at_admission_once_is_retried_and_the_run_applies() {
    let w = world();
    created(&w, "a", 1);
    let (code, ran) = w.cortex(&["run", "a/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
    let invocations = w.lines("invocations.log");
    assert_eq!(invocations.len(), 2, "retried exactly once");
    assert_eq!(invocations[0], invocations[1], "the same input both times");
    assert_eq!(w.lines("describes.log").len(), 2, "a fresh describe");
}

#[test]
fn an_invocation_that_times_out_at_admission_twice_answers_fetch_failed_after_one_retry() {
    let w = world();
    created(&w, "b", 2);
    let (code, ran) = w.cortex(&["run", "b/news"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("fetch-failed")),
        "{ran}"
    );
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("tavily.websearch.search: timeout at admission"),
        "{reason}"
    );
    assert_eq!(w.lines("invocations.log").len(), 2, "one retry, no more");
}
