//! Adversary cases for unit `unit` of wave 20261030b (`story:unit-takeover-across-homes`): a unit
//! another home's instance holds under a colliding name is refused, naming the unit, that home and
//! the instance there whose unit it is, and a home's own units are never refused.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::World;
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

fn assert_outcome(r: &(Option<i32>, Value, String), outcome: &str) {
    assert_eq!(
        (r.0, r.1["outcome"].as_str()),
        (Some(0), Some(outcome)),
        "{} {}",
        r.1,
        r.2
    );
}

/// The world's spec of instance `instance` with its one source named `source` and the viewer port
/// `port`, at a file of its own.
fn spec_of(w: &World, instance: &str, source: &str, port: u16) -> PathBuf {
    let text = std::fs::read_to_string(w.spec(instance, "conn_test"))
        .unwrap()
        .replace("  - name: news\n", &format!("  - name: {source}\n"))
        .replace("view_port: 18999", &format!("view_port: {port}"));
    let path = w.root.join(format!("{instance}--{source}.yaml"));
    std::fs::write(&path, text).unwrap();
    path
}

fn unit_text(w: &World, unit: &str) -> Option<String> {
    std::fs::read_to_string(w.units.join(unit)).ok()
}

fn create(w: &World, home: &Path, instance: &str, source: &str, port: u16, units: bool) {
    let spec = spec_of(w, instance, source, port);
    let mut args = vec!["create", "--spec", spec.to_str().unwrap(), "--no-extract"];
    if !units {
        args.push("--no-units");
    }
    assert_outcome(&at(w, home, &args), "created");
}

/// The refusal of a command that would write `unit`: exit 2, naming the unit, the home `home` and
/// `holder`, the instance there whose unit it is.
fn assert_refused(r: &(Option<i32>, Value, String), unit: &str, home: &Path, holder: &str) {
    let (code, out, err) = r;
    assert_eq!(*code, Some(2), "{out} {err}");
    for part in [unit, home.to_str().unwrap(), holder] {
        assert!(
            err.contains(part),
            "the refusal does not name {part:?}: {err}"
        );
    }
}

/// The first home holds instance `a-b` (source `x`) and instance `a` (source `b-c`, unit
/// `cortex-a-b-c`); the two name no common unit, so one home holds both. A second home's `create`
/// of `a-b` with source `c` would write `cortex-a-b-c`, which is source `a/b-c`'s. The refusal
/// must name that holder; telling the operator to remove the instance `a-b` there sends them to
/// remove an instance that does not hold the unit, after which the refusal still stands.
#[test]
fn adv_w30b_the_holder_is_named_when_the_other_home_also_has_an_instance_of_the_name() {
    let w = World::new();
    let other = w.root.join("home2");
    create(&w, &w.home, "a-b", "x", 18990, true);
    create(&w, &w.home, "a", "b-c", 18991, true);
    let before = unit_text(&w, "cortex-a-b-c.service").expect("the unit of a/b-c");
    assert!(before.contains(" run a/b-c "), "{before}");

    let spec = spec_of(&w, "a-b", "c", 18992);
    let second = at(
        &w,
        &other,
        &["create", "--spec", spec.to_str().unwrap(), "--no-extract"],
    );
    assert_eq!(unit_text(&w, "cortex-a-b-c.service"), Some(before));
    assert_refused(&second, "cortex-a-b-c.", &w.home, "source a/b-c");
}

/// The viewer of instance `a-b` in one home is `cortex-a-b-view.service`; instance `a` with source
/// `b-view` in another home names the same service. `create` there refuses, naming the viewer, and
/// the first home's viewer is left as it was. Without the viewer half of `unit_holder`, the
/// second home takes the live viewer over.
#[test]
fn adv_w30b_a_source_unit_colliding_with_another_homes_live_viewer_is_refused() {
    let w = World::new();
    let other = w.root.join("home2");
    create(&w, &w.home, "a-b", "news", 18993, true);
    let before = unit_text(&w, "cortex-a-b-view.service").expect("the viewer of a-b");

    let spec = spec_of(&w, "a", "b-view", 18994);
    let second = at(
        &w,
        &other,
        &["create", "--spec", spec.to_str().unwrap(), "--no-extract"],
    );
    assert_eq!(unit_text(&w, "cortex-a-b-view.service"), Some(before));
    assert_refused(
        &second,
        "cortex-a-b-view.service",
        &w.home,
        "the viewer of instance a-b",
    );
    assert!(
        unit_text(&w, "cortex-a-b-view.timer").is_none(),
        "a timer was written beside the refused service"
    );
}

/// The second home holds `a-b/c` and wrote `cortex-a-b-c`; the first home holds `a/b-c` without
/// units. `update` of `a-b` in its own home is not refused (the unit is its own), and `update` of
/// `a` in the first home is, naming source `a-b/c`.
#[test]
fn adv_w30b_a_homes_own_update_is_not_refused_by_a_colliding_instance_elsewhere() {
    let w = World::new();
    let other = w.root.join("home2");
    create(&w, &w.home, "a", "b-c", 18995, false);
    create(&w, &other, "a-b", "c", 18996, true);
    let own = spec_of(&w, "a-b", "c", 18996);
    let updated = at(
        &w,
        &other,
        &["update", "a-b", "--spec", own.to_str().unwrap()],
    );
    assert_outcome(&updated, "updated");
    assert!(
        updated.1["detail"]["units"].get("failed").is_none(),
        "{}",
        updated.1
    );

    let theirs = spec_of(&w, "a", "b-c", 18995);
    let refused = at(
        &w,
        &w.home,
        &["update", "a", "--spec", theirs.to_str().unwrap()],
    );
    assert_refused(&refused, "cortex-a-b-c.", &other, "source a-b/c");
}
