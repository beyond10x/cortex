//! `story:ekr-pin-0-0-43`: cortex runs against EKR 0.0.43 and reads a store outcome by EKR's own
//! name and exit status, not by its English text, and files a fetched web page as `!Url` evidence.
//!
//! - An unknown publication outcome is exit 3 with the refusal `publication-outcome-unknown`: a
//!   stand-in `ekr` in front of the real one answers it after `apply-extraction` with the message
//!   after the name reworded, and the run still fails as `store-outcome-unknown`, exit 4. The
//!   0.0.34 shape (exit 1, the English message and no name) is an ordinary apply failure now.
//! - An unseeded store is exit 2 with the refusal `ekr.views.NotSeeded`: a stand-in answers a
//!   `postgres` store from a SQLite file, its unseeded `head` with the reason reworded, and
//!   `cortex create` seeds it. The 0.0.34 shape (exit 1, `the lineage has no seed`) is an error.
//! - A run over web pages stores each page's evidence as `{"Url": <url>}` in `ekr snapshot`.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::common::{ekr, executable, World};
use serde_json::Value;

/// A stand-in `ekr` in front of the real one. While `<root>/apply` holds `unknown`,
/// `apply-extraction` applies, then answers exit 3 `publication-outcome-unknown` with a message no
/// EKR has printed; while it holds `fault`, it applies nothing and answers EKR 0.0.34's fault, exit
/// 1, with the English message and no name. Every other verb goes to the real `ekr`.
fn apply_ekr(w: &World) -> PathBuf {
    let path = w.bin.join("ekr-apply");
    let body = r#"R="@R@"; EKR="@EKR@"
if [ "$1" = apply-extraction ]; then
  case "$(cat "$R/apply" 2>/dev/null)" in
    unknown) "$EKR" "$@" > /dev/null || exit 1
      echo "ekr: publication-outcome-unknown: worded otherwise by a later EKR" >&2; exit 3 ;;
    fault) echo "ekr: publication outcome is unknown; resolve the exact retained occurrence" >&2
      exit 1 ;;
  esac
fi
exec "$EKR" "$@"
"#
    .replace("@R@", &w.root.display().to_string())
    .replace("@EKR@", &ekr().display().to_string());
    executable(&path, &body);
    path
}

/// A stand-in `ekr` that answers a `postgres` store from `<root>/pg-as-sqlite.sqlite` through the
/// real `ekr`. Before that file exists, `head` answers as `<root>/unseeded` says: `named` is exit 2
/// `ekr.views.NotSeeded` with a reason no EKR has printed, `text` is EKR 0.0.34's exit 1 `ekr: the
/// lineage has no seed`.
fn pg_ekr(w: &World) -> PathBuf {
    let path = w.bin.join("ekr-pg");
    let body = r#"R="@R@"; EKR="@EKR@"; DB="$R/pg-as-sqlite.sqlite"
if [ "$EKR_BACKEND" = postgres ]; then
  if [ "$1" = head ] && [ ! -e "$DB" ]; then
    case "$(cat "$R/unseeded")" in
      named) echo "ekr: ekr.views.NotSeeded: worded otherwise by a later EKR" >&2; exit 2 ;;
      text) echo "ekr: the lineage has no seed" >&2; exit 1 ;;
    esac
  fi
  EKR_BACKEND=sqlite; EKR_STORE="$DB"; export EKR_BACKEND EKR_STORE
fi
exec "$EKR" "$@"
"#
    .replace("@R@", &w.root.display().to_string())
    .replace("@EKR@", &ekr().display().to_string());
    executable(&path, &body);
    path
}

/// `w`'s web spec of instance `name`, its `ekr.bin` replaced by `bin`, with `tail` appended.
fn spec_with(w: &World, name: &str, bin: &Path, tail: &str) -> PathBuf {
    let path = w.spec(name, "conn_test");
    let text = std::fs::read_to_string(&path).unwrap();
    let real = format!("bin: \"{}\"", ekr().display());
    assert!(text.contains(&real), "{text}");
    let text = text.replace(&real, &format!("bin: \"{}\"", bin.display())) + tail;
    std::fs::write(&path, text).unwrap();
    path
}

fn cortex(w: &World, args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(args)
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .env_remove("EKR_STAGE")
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// The first JSON line `stdout` holds: the command's own line.
fn first_line(stdout: &str) -> Value {
    stdout
        .lines()
        .next()
        .and_then(|l| serde_json::from_str(l).ok())
        .unwrap_or(Value::Null)
}

fn create(w: &World, spec: &Path) -> (Option<i32>, Value, String) {
    let (code, out, err) = cortex(
        w,
        &["create", "--spec", spec.to_str().unwrap(), "--no-units"],
    );
    let line = out
        .lines()
        .last()
        .and_then(|l| serde_json::from_str(l).ok())
        .unwrap_or(Value::Null);
    (code, line, format!("{out}{err}"))
}

/// `ekr snapshot` of instance `name`'s SQLite store.
fn snapshot(w: &World, name: &str) -> Value {
    let dir = w.home.join("instances").join(name);
    let out = Command::new(ekr())
        .arg("snapshot")
        .env("EKR_HOST", dir.join("host.json"))
        .env("EKR_BACKEND", "sqlite")
        .env("EKR_STORE", dir.join("store.sqlite"))
        .env_remove("EKR_STAGE")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn an_unknown_outcome_is_read_from_exit_3_and_its_name_whatever_its_message_says() {
    let w = World::new();
    let spec = spec_with(&w, "t", &apply_ekr(&w), "");
    let (code, line, all) = create(&w, &spec);
    assert_eq!(code, Some(0), "{line} {all}");

    std::fs::write(w.root.join("apply"), "unknown").unwrap();
    let (code, out, err) = cortex(&w, &["run", "t/news"]);
    let ran = first_line(&out);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (Some(4), Some("store-outcome-unknown")),
        "{out}{err}"
    );
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("worded otherwise by a later EKR"),
        "{reason}"
    );

    // EKR 0.0.34's shape, the English message on a fault with no name, is no unknown outcome.
    std::fs::write(w.root.join("apply"), "fault").unwrap();
    let (code, out, err) = cortex(&w, &["run", "t/news"]);
    let ran = first_line(&out);
    assert_ne!(code, Some(4), "{out}{err}");
    assert_ne!(
        ran["outcome"].as_str(),
        Some("store-outcome-unknown"),
        "{out}{err}"
    );
}

#[test]
fn an_unseeded_store_is_read_from_exit_2_and_its_name_whatever_its_reason_says() {
    let w = World::new();
    let config = w.root.join("pg/app.json");
    std::fs::create_dir_all(config.parent().unwrap()).unwrap();
    std::fs::write(w.root.join("pg/password.txt"), "not-a-secret").unwrap();
    let document = serde_json::json!({
        "format": "ekr.postgres/1",
        "connection_file": "app.dsn",
        "password_file": w.root.join("pg/password.txt"),
        "schema": "ekr",
        "database_connections": 100,
        "replicas": 1,
        "reserved_connections": 10,
    });
    std::fs::write(&config, document.to_string()).unwrap();
    let store = format!(
        "store: {{backend: postgres, value: {{config: \"{}\"}}}}\n",
        config.display()
    );
    let bin = pg_ekr(&w);

    // EKR 0.0.34's shape is not read as an unseeded store: create refuses and seeds nothing.
    std::fs::write(w.root.join("unseeded"), "text").unwrap();
    let (code, line, all) = create(&w, &spec_with(&w, "pgtext", &bin, &store));
    assert_ne!(code, Some(0), "{line} {all}");
    assert!(all.contains("the lineage has no seed"), "{all}");
    assert!(!w.root.join("pg-as-sqlite.sqlite").exists(), "{all}");

    std::fs::write(w.root.join("unseeded"), "named").unwrap();
    let (code, line, all) = create(&w, &spec_with(&w, "pg", &bin, &store));
    assert_eq!(
        (code, line["outcome"].as_str()),
        (Some(0), Some("created")),
        "{all}"
    );
    assert!(w.root.join("pg-as-sqlite.sqlite").is_file(), "{all}");
}

#[test]
fn a_fetched_web_page_is_stored_as_url_evidence() {
    let w = World::new();
    let spec = w.spec("t", "conn_test");
    let (code, line, all) = create(&w, &spec);
    assert_eq!(code, Some(0), "{line} {all}");
    let (code, out, err) = cortex(&w, &["run", "t/news"]);
    let ran = first_line(&out);
    assert_eq!(
        (
            code,
            ran["outcome"].as_str(),
            ran["detail"]["documents_applied"].as_i64()
        ),
        (Some(0), Some("ran"), Some(2)),
        "{out}{err}"
    );

    let snapshot = snapshot(&w, "t");
    let sources: Vec<&Value> = snapshot["graph"]["graph"]["evidence"]
        .as_object()
        .expect("an evidence map")
        .values()
        .map(|item| &item["source"])
        .collect();
    let urls: std::collections::BTreeSet<&str> =
        sources.iter().filter_map(|s| s["Url"].as_str()).collect();
    assert_eq!(
        urls,
        ["https://example.org/a", "https://example.org/b"].into(),
        "{sources:?}"
    );
    assert!(
        sources
            .iter()
            .all(|s| s.get("HumanStatement").is_none() || !s.to_string().contains("https://")),
        "a web page is no statement: {sources:?}"
    );
}
