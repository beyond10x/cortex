//! `story:review-connection-pagination`, finding 8 of the 0.7.2 review: a connection named by id
//! is found on any page of `connectors connections list`, which Connectors 0.65 pages with
//! `next_cursor` and `--cursor`. `cortex create` finds it there, and a run revalidates it there,
//! so timer runs no longer fail on it and disable the source. A list that names a cursor a second
//! time ends the read with a reason instead of a loop.
//!
//! The stand-in `connectors` answers `connections list` with the file `page1.json` for the first
//! page and `page2.json` for any `--cursor`, `STATE` replaced by the connection's state (`pending`
//! until a revalidation succeeded), and logs every list call to `lists.log`.

use std::path::PathBuf;

use crate::common::{executable, World, LISTED};
use serde_json::Value;

fn world() -> World {
    let w = World::new();
    let root = w.root.display().to_string();
    executable(
        &w.bin.join("connectors"),
        &format!(
            r#"R="{root}"
case "$*" in
  *"connections list"*)
    echo "$*" >> "$R/lists.log"
    if [ -e "$R/revalidated" ]; then S=ready; else S=pending; fi
    case "$*" in *"--cursor "*) P=page2.json ;; *) P=page1.json ;; esac
    sed "s/STATE/$S/" "$R/$P" ;;
  *"connections revalidate"*)
    echo "$*" >> "$R/revalidations.log"; touch "$R/revalidated"
    echo '{{"ok":true,"result":{{}}}}' ;;
  *"connections status"*) printf '{{"ok":true,"result":{{"connection":{{"summary":{{"state":"ready"}}}}}}}}' ;;
  "invoke tavily/main "*)
    echo "$*" >> "$R/invocations.log"; cat > /dev/null
    cat "$R/answer.json" ;;
  *) echo '{{"ok":false}}'; exit 2 ;;
esac
"#
        ),
    );
    w
}

/// The `connections list` entry of `id`, its state left for the stand-in to fill.
fn entry(id: &str) -> String {
    format!(
        r#"{{"adapter":"tavily",{LISTED},"connection":"{id}","state":"STATE","revision":"rev1"}}"#
    )
}

/// One page of `connections list` holding `ids`, naming `next` as its further page.
fn page(ids: &[&str], next: Option<&str>) -> String {
    let entries: Vec<String> = ids.iter().map(|id| entry(id)).collect();
    let next = next
        .map(|c| format!(r#","next_cursor":"{c}""#))
        .unwrap_or_default();
    format!(
        r#"{{"ok":true,"result":{{"connections":[{}]{next}}}}}"#,
        entries.join(",")
    )
}

/// Sets the two pages the stand-in answers.
fn pages(w: &World, first: String, second: String) {
    std::fs::write(w.root.join("page1.json"), first).unwrap();
    std::fs::write(w.root.join("page2.json"), second).unwrap();
}

/// `conn_test` on the second page, after a page that lists another connection.
fn on_page_two(w: &World) {
    pages(
        w,
        page(&["conn_other"], Some("c1")),
        page(&["conn_test"], None),
    );
}

/// `conn_test` on the first and only page.
fn on_page_one(w: &World) {
    pages(w, page(&["conn_other", "conn_test"], None), page(&[], None));
}

/// Clears the logs and the revalidation, so a case counts from here.
fn reset(w: &World) {
    for log in ["revalidations.log", "lists.log", "invocations.log"] {
        let _ = std::fs::remove_file(w.root.join(log));
    }
    let _ = std::fs::remove_file(w.root.join("revalidated"));
}

/// `cortex create` of instance `name` over the world's web source naming `conn_test`: its exit
/// status, its answer, and the instance's directory.
fn create(w: &World, name: &str) -> (i32, Value, PathBuf) {
    let spec = w.spec(name, "conn_test");
    let (code, out) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    (code, out, w.home.join("instances").join(name))
}

/// `cortex run <source> --record-failure`, as the timers run it: its exit status and the run's
/// own line, the first it prints.
fn run(w: &World, source: &str) -> (i32, Value) {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["run", source, "--record-failure"])
        .env("CORTEX_REVALIDATE_PAUSE_MS", "5")
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let line = stdout
        .lines()
        .next()
        .and_then(|l| serde_json::from_str(l).ok())
        .unwrap_or(Value::Null);
    assert!(
        out.status.code() != Some(2),
        "cortex run {source} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code().unwrap_or(-1), line)
}

/// The `sources` row of `source`.
fn row(w: &World, source: &str) -> Value {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_cortex"))
        .arg("sources")
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .find(|r| r["source_id"] == source)
        .unwrap_or_else(|| panic!("a row for {source}"))
}

fn asked_page_two(w: &World) -> bool {
    w.lines("lists.log")
        .iter()
        .any(|l| l.contains("--cursor c1"))
}

/// Lookup: `cortex create` finds an id that only the second page lists.
#[test]
fn create_finds_an_id_on_the_second_page() {
    let w = world();
    on_page_two(&w);
    let (code, out, dir) = create(&w, "a");
    assert_eq!(code, 0, "{out}");
    assert!(dir.is_dir(), "{out}");
    assert!(asked_page_two(&w), "{:?}", w.lines("lists.log"));
}

/// Revalidation: an instance created while its id was on the first page keeps running when the id
/// moves to the second page. Two runs as the timer starts them both revalidate `conn_test` and
/// apply; the source stays enabled with no failure counted.
#[test]
fn a_run_revalidates_an_id_on_the_second_page_and_the_source_stays_enabled() {
    let w = world();
    on_page_one(&w);
    let (code, out, _) = create(&w, "b");
    assert_eq!(code, 0, "{out}");
    on_page_two(&w);
    for n in 1..=2 {
        reset(&w);
        let (code, ran) = run(&w, "b/news");
        assert_eq!(
            (code, ran["outcome"].as_str()),
            (0, Some("ran")),
            "run {n}: {ran}"
        );
        let revalidations = w.lines("revalidations.log");
        assert_eq!(revalidations.len(), 1, "run {n}: {revalidations:?}");
        assert!(
            revalidations[0].contains("--connection conn_test"),
            "run {n}: {revalidations:?}"
        );
        assert!(asked_page_two(&w), "run {n}: {:?}", w.lines("lists.log"));
        assert!(!w.lines("invocations.log").is_empty(), "run {n} fetched");
    }
    let r = row(&w, "b/news");
    assert_eq!(r["state"], "Enabled", "{r}");
    assert_eq!(r["consecutive_failures"], 0, "{r}");
}

/// A list whose second page names the first page's cursor again ends the read with a reason that
/// names the cursor, after two pages, instead of a loop; nothing is fetched.
#[test]
fn a_repeated_cursor_ends_the_read_with_a_reason() {
    let w = world();
    on_page_one(&w);
    let (code, out, _) = create(&w, "c");
    assert_eq!(code, 0, "{out}");
    pages(
        &w,
        page(&["conn_other"], Some("c1")),
        page(&["conn_other"], Some("c1")),
    );
    reset(&w);
    let (code, ran) = run(&w, "c/news");
    assert_ne!(code, 0, "{ran}");
    assert_ne!(ran["outcome"].as_str(), Some("ran"), "{ran}");
    let text = ran.to_string();
    assert!(
        text.contains("cursor c1") && text.contains("second time"),
        "{ran}"
    );
    assert_eq!(w.lines("lists.log").len(), 2, "{:?}", w.lines("lists.log"));
    assert!(w.lines("invocations.log").is_empty(), "nothing was fetched");

    // `cortex create` fails on the same list, naming the cursor too.
    let spec = w.spec("d", "conn_test");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["create", "--spec", spec.to_str().unwrap(), "--no-units"])
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(
        stderr.contains("cursor c1") && stderr.contains("second time"),
        "{stderr}"
    );
}
