//! Adversary cases for the admission-timeout retry: what is not a timeout at admission is not
//! retried, and the timeout retry composes with the renew-on-lapse retry in at most three
//! invocations. The stand-in `connectors` answers each `operations invoke` by the next line of the
//! file `script` (`lapsed`, `timeout`, `provider-timeout`), or with a result once it is empty.

mod common;

use common::{executable, World};

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
  *"operations describe"*) echo '{{"ok":true,"result":{{"schema":"s","revision":"r"}}}}' ;;
  *"operations invoke"*)
    echo "$*" >> "$R/invocations.log"
    next=""
    if [ -s "$R/script" ]; then next=$(head -n1 "$R/script"); sed -i '1d' "$R/script"; fi
    case "$next" in
      lapsed) echo '{{"ok":false,"error":{{"code":"failure","data":{{"code":"not_granted","stage":"admission"}}}}}}' >&2; exit 1 ;;
      timeout) echo '{{"ok":false,"error":{{"code":"failure","data":{{"code":"timeout","stage":"admission"}}}}}}' >&2; exit 1 ;;
      provider-timeout) echo '{{"ok":false,"error":{{"code":"failure","data":{{"code":"timeout","stage":"provider"}}}}}}' >&2; exit 1 ;;
    esac
    cat "$R/answer.json" ;;
  *) echo '{{"ok":false}}'; exit 2 ;;
esac
"#
        ),
    );
    w
}

fn created(w: &World, name: &str, script: &[&str]) {
    let spec = w.spec(name, "conn_test");
    let (code, out) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{out}");
    let mut body = script.join("\n");
    if !body.is_empty() {
        body.push('\n');
    }
    std::fs::write(w.root.join("script"), body).unwrap();
}

#[test]
fn adv_a_timeout_at_the_provider_stage_is_not_retried() {
    let w = world();
    created(&w, "a", &["provider-timeout"]);
    let (code, ran) = w.cortex(&["run", "a/news"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("fetch-failed")),
        "{ran}"
    );
    assert_eq!(w.lines("invocations.log").len(), 1, "no retry at provider");
}

#[test]
fn adv_lapsed_then_timeout_then_answer_applies_in_three_invocations() {
    let w = world();
    created(&w, "b", &["lapsed", "timeout"]);
    let (code, ran) = w.cortex(&["run", "b/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(w.lines("invocations.log").len(), 3);
    assert_eq!(w.lines("revalidations.log").len(), 1);
}

#[test]
fn adv_timeout_then_lapsed_then_answer_applies_in_three_invocations() {
    let w = world();
    created(&w, "c", &["timeout", "lapsed"]);
    let (code, ran) = w.cortex(&["run", "c/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(w.lines("invocations.log").len(), 3);
    assert_eq!(w.lines("revalidations.log").len(), 1);
}

#[test]
fn adv_lapsed_timeout_lapsed_fails_after_three_invocations() {
    let w = world();
    created(
        &w,
        "d",
        &["lapsed", "timeout", "lapsed", "timeout", "lapsed"],
    );
    let (code, ran) = w.cortex(&["run", "d/news"]);
    assert_eq!(code, 1, "{ran}");
    assert_eq!(w.lines("invocations.log").len(), 3, "bounded at three");
    assert_eq!(w.lines("revalidations.log").len(), 1, "renewed once");
}

#[test]
fn adv_timeout_timeout_timeout_fails_after_two_with_the_base_reason() {
    let w = world();
    created(&w, "e", &["timeout", "timeout", "timeout"]);
    let (code, ran) = w.cortex(&["run", "e/news"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("fetch-failed")),
        "{ran}"
    );
    assert_eq!(w.lines("invocations.log").len(), 2);
    assert_eq!(
        ran["detail"]["reason"].as_str(),
        Some("tavily.websearch.search: timeout at admission"),
        "{ran}"
    );
}
