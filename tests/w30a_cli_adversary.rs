//! Adversary cases for unit `cli` of wave 20261030a (`story:unit-names-unique`,
//! `story:spec-integer-bounds`, `story:durable-home-writes`). Each case is ignored with the finding
//! it shows, so the branch stays green; `cargo test --test w30a_cli_adversary -- --ignored` runs
//! them.

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::{executable, World};
use serde_json::Value;

/// `cortex` with `--home home`: the exit code, the last JSON line of stdout and stderr.
fn at(w: &World, home: &Path, args: &[&str]) -> (Option<i32>, Value, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .arg("--home")
        .arg(home)
        .args(args)
        .env_remove("CORTEX_HOME")
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let last = stdout.lines().last().unwrap_or("null");
    (
        out.status.code(),
        serde_json::from_str(last).unwrap_or(Value::Null),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn assert_created(r: &(Option<i32>, Value, String)) {
    assert_eq!(
        (r.0, r.1["outcome"].as_str()),
        (Some(0), Some("created")),
        "{} {}",
        r.1,
        r.2
    );
}

/// The world's spec of instance `t`, with a failure hook that appends its arguments to
/// `hook.log`.
fn spec_with_hook(w: &World) -> PathBuf {
    let hook = w.root.join("hook");
    executable(
        &hook,
        &format!("echo \"$@\" >> \"{}/hook.log\"\n", w.root.display()),
    );
    let spec = w.spec("t", "conn_test");
    let mut text = std::fs::read_to_string(&spec).unwrap();
    text.push_str(&format!(
        "monitoring: {{hook: {{command: [\"{}\"], timeout_s: 10}}}}\n",
        hook.display()
    ));
    std::fs::write(&spec, text).unwrap();
    spec
}

/// Rewrites instance `t`'s frozen copy with `from` replaced by `to`: the copy an instance created
/// by cortex 0.6.1, which accepted any `refresh_after_days` of 0 or more, holds.
fn refreeze(w: &World, from: &str, to: &str) {
    let path = w.home.join("instances/t/instance.yaml");
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains(from), "{text}");
    std::fs::write(&path, text.replace(from, to)).unwrap();
}

/// The `state` `cortex sources` lists for `id`.
fn source_state(w: &World, id: &str) -> Value {
    let (code, _, err) = at(w, &w.home, &["sources"]);
    assert_eq!(code, Some(0), "{err}");
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(["--home", w.home.to_str().unwrap(), "sources"])
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .find(|r| r["source_id"] == id)
        .map(|r| r["state"].clone())
        .unwrap_or(Value::Null)
}

/// A `refresh_after_days` of 40000 never overflowed (`40000 * 86_400_000` is far inside `i64`),
/// and cortex 0.6.1 created and ran such an instance. The bound now sits in `spec::check`, which
/// reads the frozen copy, so after the upgrade every run of that instance fails. The repository's
/// rule for a new spec refusal (`spec::load_given`, `a_frozen_spec_with_a_reserved_source_name_
/// still_parses`) is that a frozen copy keeps loading.
#[test]
#[ignore = "adversary: an instance whose frozen copy holds refresh_after_days 40000 (accepted and \
            run by 0.6.1, no overflow) stops running after the upgrade"]
fn adv_w30a_an_instance_frozen_with_a_large_refresh_window_still_runs_after_the_upgrade() {
    let w = World::new();
    let spec = w.spec("t", "conn_test");
    assert_created(&at(
        &w,
        &w.home,
        &["create", "--spec", spec.to_str().unwrap(), "--no-units"],
    ));
    refreeze(&w, "refresh_after_days: 0,", "refresh_after_days: 40000,");
    let (code, ran, err) = at(&w, &w.home, &["run", "t/news"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (Some(0), Some("ran")),
        "{ran} {err}"
    );
}

/// The failure hook is "called once for each run that fails" (`spec-file.md`, `monitoring.hook`).
/// A timer run of an instance whose frozen copy the new bound refuses fails, and two such runs
/// disable the source, but the hook reads `monitoring` from that same refused copy and is never
/// called: the operator's alerting is silent while the source is switched off.
#[test]
#[ignore = "adversary: a timer run that fails on a frozen copy the new bound refuses calls no \
            failure hook, and two of them disable the source silently"]
fn adv_w30a_a_timer_run_failing_on_a_refused_frozen_copy_calls_the_failure_hook() {
    let w = World::new();
    let spec = spec_with_hook(&w);
    assert_created(&at(
        &w,
        &w.home,
        &["create", "--spec", spec.to_str().unwrap(), "--no-units"],
    ));
    refreeze(&w, "refresh_after_days: 0,", "refresh_after_days: 40000,");
    let mut runs = Vec::new();
    for _ in 0..2 {
        let (code, out, err) = at(&w, &w.home, &["run", "t/news", "--record-failure"]);
        runs.push(format!("{code:?} {out} {err}"));
    }
    let state = source_state(&w, "t/news");
    assert!(
        !w.lines("hook.log").is_empty(),
        "no hook call; source state {state}; runs: {runs:#?}"
    );
}

/// The CHANGELOG says such an instance is refused "until the value in that file is brought into
/// range". After the timer's two failing runs the source is disabled, so bringing the value into
/// range does not put the instance back on its schedule; the entry names no `cortex source
/// enable`.
#[test]
#[ignore = "adversary: the CHANGELOG's hand fix (bring the frozen value into range) leaves the \
            source the refused runs disabled; it does not run on its timer again"]
fn adv_w30a_the_changelog_hand_fix_puts_the_instance_back_on_its_schedule() {
    let w = World::new();
    let spec = w.spec("t", "conn_test");
    assert_created(&at(
        &w,
        &w.home,
        &["create", "--spec", spec.to_str().unwrap(), "--no-extract"],
    ));
    refreeze(&w, "refresh_after_days: 0,", "refresh_after_days: 40000,");
    for _ in 0..2 {
        at(&w, &w.home, &["run", "t/news", "--record-failure"]);
    }
    // The fix the CHANGELOG gives.
    refreeze(&w, "refresh_after_days: 40000,", "refresh_after_days: 0,");
    assert_eq!(
        source_state(&w, "t/news"),
        Value::from("Enabled"),
        "after the hand fix the source stays off its timer"
    );
}

/// The world's spec of instance `instance` with its one source named `source`, at a file of its
/// own.
fn spec_of(w: &World, instance: &str, source: &str) -> PathBuf {
    let text = std::fs::read_to_string(w.spec(instance, "conn_test"))
        .unwrap()
        .replace("  - name: news\n", &format!("  - name: {source}\n"));
    let path = w.root.join(format!("{instance}--{source}.yaml"));
    std::fs::write(&path, text).unwrap();
    path
}

fn exec_of(w: &World, unit: &str) -> Option<String> {
    std::fs::read_to_string(w.units.join(unit))
        .ok()?
        .lines()
        .find(|l| l.starts_with("ExecStart="))
        .map(str::to_string)
}

/// Instance `a` with source `b-c` in one home and instance `a-b` with source `c` in another, on
/// one systemd user manager, name one unit. `Systemd::foreign` asks the first home for an
/// instance `a-b`, finds none, reads the unit as orphaned and lets the second home take it over:
/// the first home's source no longer runs.
#[test]
#[ignore = "adversary: across two homes, instance a-b/c takes over the live unit cortex-a-b-c of \
            instance a/b-c in the other home (Systemd::foreign asks for the requesting instance)"]
fn adv_w30a_a_colliding_unit_of_a_live_instance_in_another_home_is_not_taken_over() {
    let w = World::new();
    let other = w.root.join("home2");
    assert_created(&at(
        &w,
        &w.home,
        &[
            "create",
            "--spec",
            spec_of(&w, "a", "b-c").to_str().unwrap(),
            "--no-extract",
        ],
    ));
    let before = exec_of(&w, "cortex-a-b-c.service").expect("the unit of a/b-c");
    assert!(before.contains(" run a/b-c "), "{before}");
    let second = at(
        &w,
        &other,
        &[
            "create",
            "--spec",
            spec_of(&w, "a-b", "c").to_str().unwrap(),
            "--no-extract",
        ],
    );
    assert_eq!(
        exec_of(&w, "cortex-a-b-c.service"),
        Some(before),
        "the second home overwrote the first home's live unit: {} {}",
        second.1,
        second.2
    );
}

/// Every regular file of `dir`, by name, with its text.
fn texts(dir: &Path) -> BTreeMap<String, String> {
    std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| {
            (
                e.file_name().into_string().unwrap(),
                std::fs::read_to_string(e.path()).unwrap(),
            )
        })
        .collect()
}

/// The `restored` summary the unit wrote (`spec/domains/instance.yaml`, `operating.md`): a state
/// copy that cannot be written back "leaves the store restored and the state as it was". The copy
/// is written back one file at a time, so a write that fails on a later file (here a non-text
/// entry, standing in for a disk that fills on the second file) leaves `state/` half put back:
/// `news.json` at the snapshot, while the restore says `state_restored: false`.
#[test]
#[ignore = "adversary: a state copy that fails part-way leaves state/ half restored, though \
            restore reports state_restored false and the docs say state/ is as it was"]
fn adv_w30a_a_state_copy_that_fails_part_way_leaves_state_as_it_was() {
    let w = World::new();
    let spec = w.spec("t", "conn_test");
    assert_created(&at(
        &w,
        &w.home,
        &["create", "--spec", spec.to_str().unwrap()],
    ));
    let dir = w.home.join("instances/t");
    let applying = |n: u32| {
        std::fs::write(
            w.root.join("answer.json"),
            common::answer(&common::PAGES.replace(
                "also develops the Gadget runtime",
                &format!("now develops the Gadget runtime, release {n}"),
            )),
        )
        .unwrap();
        let (code, ran, err) = at(&w, &w.home, &["run", "t/news"]);
        assert_eq!(code, Some(0), "{ran} {err}");
    };
    applying(0);
    applying(1);
    let mut held: Vec<String> = std::fs::read_dir(dir.join("snapshots-state"))
        .unwrap()
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| !n.starts_with('.'))
        .filter_map(|n| n.strip_suffix(".json").map(str::to_string))
        .collect();
    held.sort();
    let before_second = held.last().expect("a snapshot").clone();
    let copy = dir.join(format!("snapshots-state/{before_second}.json"));
    let mut doc: Value = serde_json::from_str(&std::fs::read_to_string(&copy).unwrap()).unwrap();
    assert!(doc["files"].get("news.json").is_some(), "{doc}");
    doc["files"]["zz-unwritable.json"] = Value::from(1);
    std::fs::write(&copy, serde_json::to_string(&doc).unwrap()).unwrap();
    let live = texts(&dir.join("state"));
    assert_ne!(
        live.get("news.json").map(String::as_str),
        doc["files"]["news.json"].as_str(),
        "the second run changed news.json"
    );

    let (code, restored, err) = at(&w, &w.home, &["restore", "t", &before_second]);
    assert_eq!(
        (code, restored["outcome"].as_str()),
        (Some(0), Some("restored")),
        "{restored} {err}"
    );
    assert_eq!(restored["detail"]["state_restored"], false, "{restored}");
    assert!(restored["detail"]["state_failed"].is_string(), "{restored}");
    assert_eq!(
        texts(&dir.join("state")),
        live,
        "state/ is not as it was: {restored}"
    );
}
