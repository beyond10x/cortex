//! Two homes, one systemd user unit directory (both default to `~/.config/systemd/user`), and an
//! instance of the same name in each (issue #26). Each home records its own path in the units it
//! writes; no command of one home writes, starts, stops or deletes a unit another home wrote.
//! `create` refuses before it creates anything, naming the other home, and `remove` deletes only
//! its own home's units.

mod common;

use std::path::{Path, PathBuf};

use common::World;
use serde_json::Value;

/// A second home beside the world's own.
fn scratch(w: &World) -> PathBuf {
    w.root.join("home-scratch")
}

fn at(home: &Path, args: &[&str]) -> Vec<String> {
    let mut out = vec!["--home".to_string(), home.display().to_string()];
    out.extend(args.iter().map(|a| a.to_string()));
    out
}

/// `cortex` in `home`: the exit code, the last JSON line and stderr.
fn cortex_in(w: &World, home: &Path, args: &[&str]) -> (Option<i32>, Value, String) {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(at(home, args))
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

const UNITS: [&str; 3] = [
    "cortex-t-news.service",
    "cortex-t-news.timer",
    "cortex-t-view.service",
];

/// Every unit file of instance `t`, by name.
fn units(w: &World) -> Vec<(String, String)> {
    UNITS
        .iter()
        .map(|u| {
            let text = std::fs::read_to_string(w.units.join(u)).unwrap_or_default();
            (u.to_string(), text)
        })
        .collect()
}

/// The `systemctl` calls made since `from` lines were logged.
fn calls_since(w: &World, from: usize) -> Vec<String> {
    w.lines("systemctl.log").into_iter().skip(from).collect()
}

/// Instance `t` created in the world's home with its units.
fn live(w: &World) -> Vec<(String, String)> {
    let spec = w.spec("t", "conn_test");
    let (code, out, stderr) = cortex_in(
        w,
        &w.home,
        &["create", "--spec", spec.to_str().unwrap(), "--no-extract"],
    );
    assert_eq!(
        (code, out["outcome"].as_str()),
        (Some(0), Some("created")),
        "{out} {stderr}"
    );
    assert!(out["detail"].get("units").is_none(), "{out}");
    let units = units(w);
    for (unit, text) in &units {
        assert!(!text.is_empty(), "{unit} was not written");
    }
    units
}

#[test]
fn a_home_records_its_own_path_in_every_unit_it_writes() {
    let w = World::new();
    for (unit, text) in live(&w) {
        assert!(
            text.lines()
                .any(|l| l == format!("X-CortexHome={}", w.home.display())),
            "{unit} does not name its home {}:\n{text}",
            w.home.display()
        );
    }
}

#[test]
fn a_second_homes_create_refuses_the_units_of_another_home_and_names_it() {
    let w = World::new();
    let before = live(&w);
    let calls = w.lines("systemctl.log").len();
    let spec = w.spec("t", "conn_test");
    let b = scratch(&w);
    let (code, out, stderr) = cortex_in(
        &w,
        &b,
        &["create", "--spec", spec.to_str().unwrap(), "--no-extract"],
    );
    assert_eq!(code, Some(2), "{out} {stderr}");
    assert!(
        stderr.contains(&w.home.display().to_string()),
        "the refusal does not name the home the units belong to: {stderr}"
    );
    assert_eq!(units(&w), before, "the other home's units changed");
    assert_eq!(calls_since(&w, calls), Vec::<String>::new());
    assert!(
        !b.join("instances/t").exists(),
        "a refused create left an instance"
    );

    // `--no-units` writes no unit, so it is not refused.
    let (code, out, stderr) = cortex_in(
        &w,
        &b,
        &[
            "create",
            "--spec",
            spec.to_str().unwrap(),
            "--no-extract",
            "--no-units",
        ],
    );
    assert_eq!(
        (code, out["outcome"].as_str()),
        (Some(0), Some("created")),
        "{out} {stderr}"
    );

    // Nor does `update` in that home write over them.
    let (code, out, stderr) = cortex_in(&w, &b, &["update", "t", "--spec", spec.to_str().unwrap()]);
    assert_eq!(code, Some(2), "{out} {stderr}");
    assert!(stderr.contains(&w.home.display().to_string()), "{stderr}");
    assert_eq!(units(&w), before, "update changed the other home's units");
}

#[test]
fn a_unit_written_before_homes_were_recorded_belongs_to_the_home_its_service_runs() {
    // A unit an earlier cortex wrote carries no `X-CortexHome=`: a source's units belong to the
    // `--home` of its service's `ExecStart`, a viewer to the instance directory of its `EKR_HOST`.
    let w = World::new();
    live(&w);
    for unit in UNITS {
        let path = w.units.join(unit);
        let text = std::fs::read_to_string(&path).unwrap();
        let old: String = text
            .lines()
            .filter(|l| !l.starts_with("X-CortexHome="))
            .map(|l| format!("{l}\n"))
            .collect();
        std::fs::write(&path, old).unwrap();
    }
    let before = units(&w);
    let spec = w.spec("t", "conn_test");
    let (code, out, stderr) = cortex_in(
        &w,
        &scratch(&w),
        &["create", "--spec", spec.to_str().unwrap(), "--no-extract"],
    );
    assert_eq!(code, Some(2), "{out} {stderr}");
    assert!(stderr.contains(&w.home.display().to_string()), "{stderr}");
    assert_eq!(units(&w), before, "the other home's units changed");
}

#[test]
fn no_command_of_a_second_home_stops_starts_or_deletes_the_units_of_another() {
    let w = World::new();
    let before = live(&w);
    let spec = w.spec("t", "conn_test");
    let b = scratch(&w);
    let (code, out, stderr) = cortex_in(
        &w,
        &b,
        &[
            "create",
            "--spec",
            spec.to_str().unwrap(),
            "--no-extract",
            "--no-units",
        ],
    );
    assert_eq!(
        (code, out["outcome"].as_str()),
        (Some(0), Some("created")),
        "{out} {stderr}"
    );
    let calls = w.lines("systemctl.log").len();

    // A run that applies something takes a snapshot; restoring it restarts the viewer it owns.
    let (code, out, stderr) = cortex_in(&w, &b, &["run", "t/news"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (Some(0), Some("ran")),
        "{out} {stderr}"
    );
    let snapshot = std::fs::read_dir(b.join("instances/t/snapshots"))
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .find_map(|n| n.strip_suffix(".sqlite").map(str::to_string))
        .expect("a snapshot");
    let (code, out, stderr) = cortex_in(&w, &b, &["restore", "t", &snapshot]);
    assert_eq!(code, Some(0), "{out} {stderr}");

    // Two failures disable the source, and `enable` enables it: both name its timer.
    for _ in 0..2 {
        let (code, out, stderr) = cortex_in(
            &w,
            &b,
            &["source", "record-failure", "t/news", "--reason", "r"],
        );
        assert_eq!(code, Some(0), "{out} {stderr}");
    }
    let (code, out, stderr) = cortex_in(&w, &b, &["source", "enable", "t/news"]);
    assert_eq!(code, Some(0), "{out} {stderr}");

    let (code, out, stderr) = cortex_in(&w, &b, &["remove", "t"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (Some(0), Some("removed")),
        "{out} {stderr}"
    );
    let kept = out["detail"]["units"]["kept"].to_string();
    assert!(
        kept.contains(&w.home.display().to_string()),
        "remove does not say whose units it kept: {out}"
    );

    assert_eq!(units(&w), before, "the other home's units changed");
    let touched: Vec<String> = calls_since(&w, calls)
        .into_iter()
        .filter(|c| c.contains("cortex-t-"))
        .collect();
    assert_eq!(
        touched,
        Vec::<String>::new(),
        "systemctl touched the other home's units"
    );

    // The live home still removes its own.
    let (code, out, stderr) = cortex_in(&w, &w.home, &["remove", "t"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (Some(0), Some("removed")),
        "{out} {stderr}"
    );
    for unit in UNITS {
        assert!(!w.units.join(unit).exists(), "{unit} is left");
    }
}
