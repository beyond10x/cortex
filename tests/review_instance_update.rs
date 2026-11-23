//! Findings 12 to 16 of the 0.7.2 review (`story:review-instance-update`): `update`,
//! `source add` and `remove` keep the registry, the scheduler and the frozen spec in agreement,
//! and report every step that did not complete. Each test drives the binary with a stand-in
//! `systemctl` that logs its arguments and refuses `disable` while the file `deny-disable` exists.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{ekr, executable, World};
use serde_json::Value;

/// A world whose `systemctl` refuses `disable` while `<root>/deny-disable` exists.
fn world() -> World {
    let w = World::new();
    executable(
        &w.bin.join("systemctl"),
        &format!(
            "R=\"{root}\"\necho \"$*\" >> \"$R/systemctl.log\"\ncase \"$*\" in *is-active*) exit 3;; *disable*) [ -e \"$R/deny-disable\" ] && exit 1;; esac\nexit 0\n",
            root = w.root.display()
        ),
    );
    w
}

/// A spec of instance `t` with one web source per `(name, schedule)` and the viewer on `port`.
fn spec(w: &World, file: &str, sources: &[(&str, &str)], port: u16) -> PathBuf {
    let sources: String = sources
        .iter()
        .map(|(name, schedule)| {
            format!(
                r#"  - name: {name}
    schedule: {schedule}
    settings:
      kind: web
      value:
        connection: conn_test
        input:
          input: search
          value:
            queries: [widget engine]
            policy: {{topic: news, max_results: 3, include_domains: [], exclude_domains: []}}
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
"#
            )
        })
        .collect();
    let path = w.root.join(file);
    std::fs::write(
        &path,
        format!(
            r#"format: cortex.instance/1
name: t
description: Test brain.
ekr: {{version: "0.0.43", bin: "{ekr}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
{sources}serve: {{view_port: {port}}}
"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    path
}

/// `cortex <args>` in the world's home: its exit status and every line it printed, as JSON.
fn lines(w: &World, args: &[&str]) -> (i32, Vec<Value>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(args)
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .output()
        .unwrap();
    let rows = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap_or(Value::Null))
        .collect();
    (
        out.status.code().unwrap_or(-1),
        rows,
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// The rows `cortex sources` prints, by source id.
fn sources(w: &World) -> Vec<Value> {
    let (code, rows, err) = lines(w, &["sources"]);
    assert_eq!(code, 0, "{err}");
    rows
}

fn source(w: &World, id: &str) -> Option<Value> {
    sources(w).into_iter().find(|r| r["source_id"] == id)
}

fn create(w: &World, spec: &Path, units: bool) {
    let mut args = vec!["create", "--spec", spec.to_str().unwrap(), "--no-extract"];
    if !units {
        args.push("--no-units");
    }
    let (code, created) = w.cortex(&args);
    assert_eq!(
        (code, created["outcome"].as_str()),
        (0, Some("created")),
        "{created}"
    );
}

fn update(w: &World, spec: &Path) -> Value {
    let (code, updated) = w.cortex(&["update", "t", "--spec", spec.to_str().unwrap()]);
    assert_eq!(
        (code, updated["outcome"].as_str()),
        (0, Some("updated")),
        "{updated}"
    );
    updated
}

/// Finding 12: a source the replacement spec drops leaves the registry, and its timer is stopped
/// and its unit files deleted; the source it adds is registered and scheduled.
#[test]
fn an_update_that_drops_a_source_stops_its_timer_and_takes_it_out_of_the_registry() {
    let w = world();
    let first = spec(
        &w,
        "first.yaml",
        &[("news", "daily"), ("clips", "daily")],
        18997,
    );
    create(&w, &first, true);
    assert!(w.units.join("cortex-t-clips.timer").is_file());

    let second = spec(
        &w,
        "second.yaml",
        &[("news", "daily"), ("arrivals", "daily")],
        18997,
    );
    let updated = update(&w, &second);
    assert_eq!(
        updated["detail"]["removed_sources"],
        serde_json::json!(["t/clips"]),
        "{updated}"
    );
    assert_eq!(
        updated["detail"]["added_sources"],
        serde_json::json!(["t/arrivals"]),
        "{updated}"
    );
    assert!(updated["detail"].get("units").is_none(), "{updated}");

    let ids: Vec<String> = sources(&w)
        .iter()
        .map(|r| r["source_id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(ids, ["t/arrivals", "t/news"]);
    for unit in ["cortex-t-clips.timer", "cortex-t-clips.service"] {
        assert!(!w.units.join(unit).exists(), "{unit} is left");
    }
    assert!(w.units.join("cortex-t-arrivals.timer").is_file());
    assert!(w
        .lines("systemctl.log")
        .iter()
        .any(|l| l == "--user disable --now cortex-t-clips.timer"));

    // The dropped source no longer runs, and a run of it counts nothing.
    let (code, gone) = w.cortex(&["run", "t/clips", "--record-failure"]);
    assert_eq!(
        (code, gone["outcome"].as_str()),
        (1, Some("no-such-source")),
        "{gone}"
    );
    assert_eq!(sources(&w).len(), 2);
}

/// Finding 12, retry: a dropped source whose timer cannot be stopped keeps its row and its unit
/// files, the update names the failure, and the next update stops and removes it.
#[test]
fn a_dropped_source_whose_timer_cannot_be_stopped_is_kept_for_the_next_update() {
    let w = world();
    let first = spec(
        &w,
        "first.yaml",
        &[("news", "daily"), ("clips", "daily")],
        18997,
    );
    create(&w, &first, true);
    let second = spec(&w, "second.yaml", &[("news", "daily")], 18997);

    std::fs::write(w.root.join("deny-disable"), "").unwrap();
    let updated = update(&w, &second);
    let failed = updated["detail"]["units"]["failed"].to_string();
    assert!(
        failed.contains("disable --now cortex-t-clips.timer"),
        "{updated}"
    );
    assert_eq!(updated["detail"]["removed_sources"], serde_json::json!([]));
    assert!(source(&w, "t/clips").is_some());
    assert!(w.units.join("cortex-t-clips.timer").is_file());

    std::fs::remove_file(w.root.join("deny-disable")).unwrap();
    let updated = update(&w, &second);
    assert_eq!(
        updated["detail"]["removed_sources"],
        serde_json::json!(["t/clips"]),
        "{updated}"
    );
    assert!(source(&w, "t/clips").is_none());
    assert!(!w.units.join("cortex-t-clips.timer").exists());
}

/// Finding 13: a source the update keeps takes its new schedule in the registry, as its timer
/// does, and keeps its runs and its failures.
#[test]
fn an_update_refreshes_a_kept_sources_schedule_and_keeps_its_counters() {
    let w = world();
    let first = spec(&w, "first.yaml", &[("news", "daily")], 18997);
    create(&w, &first, true);
    let (code, ran) = w.cortex(&["run", "t/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    let (code, _, err) = lines(
        &w,
        &["source", "record-failure", "t/news", "--reason", "boom"],
    );
    assert_eq!(code, 0, "{err}");
    let before = source(&w, "t/news").unwrap();
    assert_eq!(
        (&before["runs"], &before["consecutive_failures"]),
        (&Value::from(1), &Value::from(1)),
        "{before}"
    );

    let second = spec(&w, "second.yaml", &[("news", "hourly")], 18997);
    update(&w, &second);
    let timer = std::fs::read_to_string(w.units.join("cortex-t-news.timer")).unwrap();
    assert!(timer.contains("OnCalendar=hourly"), "{timer}");
    let after = source(&w, "t/news").unwrap();
    assert_eq!(after["schedule"], "hourly", "{after}");
    for field in [
        "runs",
        "consecutive_failures",
        "state",
        "last_outcome",
        "last_run_at",
        "last_reason",
    ] {
        assert_eq!(after[field], before[field], "{field}: {after}");
    }
}

/// Finding 14: a changed `serve.view_port` is applied: `cortex list` shows it, and the viewer
/// unit is written on it and restarted.
#[test]
fn an_update_that_changes_the_view_port_moves_the_viewer() {
    let w = world();
    let first = spec(&w, "first.yaml", &[("news", "daily")], 18997);
    create(&w, &first, true);
    let calls = w.lines("systemctl.log").len();

    let second = spec(&w, "second.yaml", &[("news", "daily")], 18998);
    let updated = update(&w, &second);
    assert!(updated["detail"].get("units").is_none(), "{updated}");
    let (code, listed, err) = lines(&w, &["list"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(listed[0]["view"], "http://127.0.0.1:18998/", "{listed:?}");
    let unit = std::fs::read_to_string(w.units.join("cortex-t-view.service")).unwrap();
    assert!(unit.contains("view --port 18998"), "{unit}");
    let after: Vec<String> = w.lines("systemctl.log").into_iter().skip(calls).collect();
    assert!(
        after
            .iter()
            .any(|l| l == "--user restart cortex-t-view.service"),
        "{after:?}"
    );

    // An instance without units takes the port too.
    let third = spec(&w, "third.yaml", &[("news", "daily")], 18996);
    let (code, updated) = w.cortex(&[
        "update",
        "t",
        "--spec",
        third.to_str().unwrap(),
        "--no-units",
    ]);
    assert_eq!(code, 0, "{updated}");
    let (_, listed, _) = lines(&w, &["list"]);
    assert_eq!(listed[0]["view"], "http://127.0.0.1:18996/", "{listed:?}");
}

/// Finding 15: `source add` for a source the registry holds changes nothing and is refused, and a
/// source of an instance the home does not hold, or holds removed, is refused.
#[test]
fn source_add_keeps_an_existing_source_and_refuses_a_missing_parent() {
    let w = world();
    let first = spec(&w, "first.yaml", &[("news", "daily")], 18997);
    create(&w, &first, false);
    let (code, ran) = w.cortex(&["run", "t/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    for _ in 0..2 {
        let (code, _, err) = lines(&w, &["source", "record-failure", "t/news", "--reason", "r"]);
        assert_eq!(code, 0, "{err}");
    }
    let before = source(&w, "t/news").unwrap();
    assert_eq!(before["state"], "Disabled", "{before}");
    assert_eq!(before["runs"], 1, "{before}");

    let add = |instance: &str, id: &str, name: &str| {
        w.cortex(&[
            "source",
            "add",
            "--instance-name",
            instance,
            "--source-id",
            id,
            "--name",
            name,
            "--kind",
            "Web",
            "--schedule",
            "daily",
        ])
    };
    let (code, again) = add("t", "t/news", "news");
    assert_eq!(
        (code, again["outcome"].as_str()),
        (1, Some("source-exists")),
        "{again}"
    );
    assert_eq!(again["detail"]["source_id"], "t/news", "{again}");
    assert_eq!(source(&w, "t/news").unwrap(), before);
    let (code, refused) = w.cortex(&["run", "t/news"]);
    assert_eq!(
        (code, refused["outcome"].as_str()),
        (3, Some("disabled")),
        "{refused}"
    );

    let (code, orphan) = add("ghost", "ghost/x", "x");
    assert_eq!(
        (code, orphan["outcome"].as_str()),
        (1, Some("no-such-instance")),
        "{orphan}"
    );
    assert_eq!(orphan["detail"]["name"], "ghost", "{orphan}");
    assert!(source(&w, "ghost/x").is_none());

    let (code, removed) = w.cortex(&["remove", "t"]);
    assert_eq!(code, 0, "{removed}");
    let (code, late) = add("t", "t/late", "late");
    assert_eq!(
        (code, late["outcome"].as_str()),
        (1, Some("not-active")),
        "{late}"
    );
    assert!(source(&w, "t/late").is_none());
}

/// Finding 16: a `remove` whose `systemctl disable --now` fails answers `stop-failed` with a
/// non-zero status naming the failure, keeps the instance active and its unit files, and a
/// second `remove` finishes the job.
#[test]
fn a_remove_whose_units_cannot_be_stopped_says_so_and_can_be_retried() {
    let w = world();
    let first = spec(&w, "first.yaml", &[("news", "daily")], 18997);
    create(&w, &first, true);

    std::fs::write(w.root.join("deny-disable"), "").unwrap();
    let (code, refused) = w.cortex(&["remove", "t"]);
    assert_eq!(
        (code, refused["outcome"].as_str()),
        (1, Some("stop-failed")),
        "{refused}"
    );
    let reason = refused["detail"]["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("disable --now cortex-t-news.timer"),
        "{refused}"
    );
    assert!(w.units.join("cortex-t-news.timer").is_file());
    assert!(w.units.join("cortex-t-view.service").is_file());
    let (_, listed, _) = lines(&w, &["list"]);
    assert_eq!(listed[0]["state"], "Active", "{listed:?}");

    std::fs::remove_file(w.root.join("deny-disable")).unwrap();
    let (code, removed) = w.cortex(&["remove", "t"]);
    assert_eq!(
        (code, removed["outcome"].as_str()),
        (0, Some("removed")),
        "{removed}"
    );
    assert!(!w.units.join("cortex-t-news.timer").exists());
    assert!(!w.units.join("cortex-t-view.service").exists());
}
