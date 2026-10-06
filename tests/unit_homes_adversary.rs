//! Adversary cases for issue #26: the forms a home's path takes, units an earlier cortex wrote, and
//! a refusal's remedy. Each home owns exactly the units it wrote, whatever form its path is given
//! in, and the remedy a refusal names clears it.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::World;
use serde_json::Value;

const UNITS: [&str; 3] = [
    "cortex-t-news.service",
    "cortex-t-news.timer",
    "cortex-t-view.service",
];

/// `cortex` with `home` given as `--home` (`Some`), or through `CORTEX_HOME` (`None` here, `env`
/// set), run in `cwd`: the exit code, the last JSON line and stderr.
fn cortex(
    w: &World,
    home: Option<&str>,
    env_home: Option<&str>,
    cwd: &Path,
    args: &[&str],
) -> (Option<i32>, Value, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cortex"));
    if let Some(home) = home {
        cmd.arg("--home").arg(home);
    }
    cmd.args(args)
        .current_dir(cwd)
        .env_remove("CORTEX_HOME")
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units);
    if let Some(h) = env_home {
        cmd.env("CORTEX_HOME", h);
    }
    let out = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let last = stdout.lines().last().unwrap_or("null");
    (
        out.status.code(),
        serde_json::from_str(last).unwrap_or(Value::Null),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn at(w: &World, home: &Path, args: &[&str]) -> (Option<i32>, Value, String) {
    cortex(w, Some(home.to_str().unwrap()), None, &w.root, args)
}

fn create(w: &World, home: &Path) -> (Option<i32>, Value, String) {
    let spec = w.spec("t", "conn_test");
    at(
        w,
        home,
        &["create", "--spec", spec.to_str().unwrap(), "--no-extract"],
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
    assert!(r.1["detail"].get("units").is_none(), "{}", r.1);
}

fn units(w: &World) -> Vec<(String, String)> {
    UNITS
        .iter()
        .map(|u| {
            let text = std::fs::read_to_string(w.units.join(u)).unwrap_or_default();
            (u.to_string(), text)
        })
        .collect()
}

/// Rewrites every unit of `t` as an earlier cortex wrote it: no `X-CortexHome=` line.
fn strip_home_lines(w: &World, which: &[&str]) {
    for unit in which {
        let path = w.units.join(unit);
        let text = std::fs::read_to_string(&path).unwrap();
        let old: String = text
            .lines()
            .filter(|l| !l.starts_with("X-CortexHome="))
            .map(|l| format!("{l}\n"))
            .collect();
        std::fs::write(&path, old).unwrap();
    }
}

/// Removes `t` in `home` and asserts every unit of `t` is gone and none is reported kept.
fn removes_all_its_units(w: &World, home: &Path) {
    let (code, out, stderr) = at(w, home, &["remove", "t"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (Some(0), Some("removed")),
        "{out} {stderr}"
    );
    assert!(
        out["detail"].get("units").is_none(),
        "remove kept or failed on its own home's units: {out}"
    );
    for unit in UNITS {
        assert!(
            !w.units.join(unit).exists(),
            "{unit} of the home's own instance is left after its remove"
        );
    }
}

/// Asserts `r` is a create that took over every unit of `t` from `old` and wrote this `home` into
/// each.
fn assert_taken_over(w: &World, r: &(Option<i32>, Value, String), old: &Path, home: &Path) {
    assert_eq!(
        (r.0, r.1["outcome"].as_str()),
        (Some(0), Some("created")),
        "{} {}",
        r.1,
        r.2
    );
    assert!(r.1["detail"]["units"].get("failed").is_none(), "{}", r.1);
    let taken: Vec<&str> = r.1["detail"]["units"]["taken_over"]
        .as_array()
        .unwrap_or_else(|| panic!("no units.taken_over: {}", r.1))
        .iter()
        .filter_map(Value::as_str)
        .collect();
    for unit in UNITS {
        let entry = format!("{unit} (home {})", old.display());
        assert!(taken.contains(&entry.as_str()), "{entry} not in {taken:?}");
        let text = std::fs::read_to_string(w.units.join(unit)).unwrap();
        let homes: Vec<&str> = text
            .lines()
            .filter_map(|l| l.strip_prefix("X-CortexHome="))
            .collect();
        assert_eq!(homes, [home.to_str().unwrap()], "{unit}:\n{text}");
    }
    let service = std::fs::read_to_string(w.units.join("cortex-t-news.service")).unwrap();
    assert!(
        service.contains(&format!("--home \"{}\"", home.display())),
        "{service}"
    );
}

/// The units of a home that was deleted are orphaned: no cortex command of that home can clear
/// them. A create in another home takes them over, records its own home in each and names each
/// in `units.taken_over`, with the home it took it from.
#[test]
fn a_create_takes_over_the_units_of_a_home_that_was_deleted() {
    let w = World::new();
    // A scratch home took the units (as every home could before #26), then was deleted.
    let scratch = w.root.join("home-scratch");
    assert_created(&create(&w, &scratch));
    std::fs::remove_dir_all(&scratch).unwrap();

    let r = create(&w, &w.home);
    assert_taken_over(&w, &r, &scratch, &w.home);
    // They are this home's now: its remove deletes them.
    removes_all_its_units(&w, &w.home);
}

/// A home that exists but holds no instance of the name, or holds it removed, has orphaned the
/// units of that name, and another home's create takes them over. An earlier cortex's units
/// (no `X-CortexHome=`) are orphaned the same way.
#[test]
fn a_create_takes_over_the_units_a_living_home_holds_no_instance_for() {
    for (label, old_units) in [("recorded", false), ("earlier cortex", true)] {
        for registry in ["none", "removed"] {
            let w = World::new();
            let scratch = w.root.join("home-scratch");
            assert_created(&create(&w, &scratch));
            if old_units {
                strip_home_lines(&w, &UNITS);
            }
            let path = scratch.join("registry.json");
            match registry {
                "none" => std::fs::write(
                    &path,
                    r#"{"format": "cortex.registry/1", "instances": [], "sources": []}"#,
                )
                .unwrap(),
                _ => {
                    let text = std::fs::read_to_string(&path).unwrap();
                    let removed = text.replace("\"Active\"", "\"Removed\"");
                    assert_ne!(text, removed, "{label}: no Active instance in {text}");
                    std::fs::write(&path, removed).unwrap();
                }
            }
            let r = create(&w, &w.home);
            assert!(r.0 == Some(0), "{label}, {registry}: {} {}", r.1, r.2);
            assert_taken_over(&w, &r, &scratch, &w.home);
        }
    }
}

/// A home whose registry cannot be read may hold the instance: its units are refused, and the
/// refusal says why rather than claiming the instance is there.
#[test]
fn the_units_of_a_home_whose_registry_cannot_be_read_are_refused() {
    let w = World::new();
    let scratch = w.root.join("home-scratch");
    assert_created(&create(&w, &scratch));
    std::fs::write(scratch.join("registry.json"), "not json").unwrap();
    let before = units(&w);

    let (code, out, stderr) = create(&w, &w.home);
    assert_eq!(code, Some(2), "{out} {stderr}");
    assert!(stderr.contains(scratch.to_str().unwrap()), "{stderr}");
    assert!(stderr.contains("registry cannot be read"), "{stderr}");
    assert!(
        !stderr.contains("has an instance of the same name"),
        "{stderr}"
    );
    assert_eq!(units(&w), before);
}

/// A timer orphaned by a deleted home is not enabled by another home's `source enable`; the
/// answer names the command that takes it over here, not a remove in the deleted home.
#[test]
fn source_enable_names_the_takeover_for_an_orphaned_timer() {
    let w = World::new();
    let scratch = w.root.join("home-scratch");
    assert_created(&create(&w, &scratch));
    std::fs::remove_dir_all(&scratch).unwrap();
    let spec = w.spec("t", "conn_test");
    let r = at(
        &w,
        &w.home,
        &[
            "create",
            "--spec",
            spec.to_str().unwrap(),
            "--no-extract",
            "--no-units",
        ],
    );
    assert_eq!(r.0, Some(0), "{} {}", r.1, r.2);
    let before = units(&w);
    for _ in 0..2 {
        let (code, out, stderr) = at(
            &w,
            &w.home,
            &["source", "record-failure", "t/news", "--reason", "r"],
        );
        assert_eq!(code, Some(0), "{out} {stderr}");
    }
    let (code, out, stderr) = at(&w, &w.home, &["source", "enable", "t/news"]);
    assert_eq!(code, Some(0), "{out} {stderr}");
    let failed = out["detail"]["units"]["failed"]
        .as_str()
        .unwrap_or_default();
    assert!(failed.contains(scratch.to_str().unwrap()), "{out}");
    assert!(failed.contains("update"), "{out}");
    assert!(!failed.contains("Remove the instance there"), "{out}");
    assert_eq!(units(&w), before);
}

/// A home path holding a newline would be cut at the newline in every unit it writes, and the home
/// would no longer recognise its own units. Every command refuses such a home before it writes
/// anything, and says why.
#[test]
fn a_home_whose_path_holds_a_newline_is_refused() {
    let w = World::new();
    let spec = w.spec("t", "conn_test");
    let spec = spec.to_str().unwrap();
    for home in ["home\nlive", "home\rlive"] {
        let home = w.root.join(home);
        for args in [
            &["list"][..],
            &["create", "--spec", spec, "--no-extract"],
            &["remove", "t"],
            &["mcp-line", "t"],
            &["schema"],
        ] {
            let (code, out, stderr) = at(&w, &home, args);
            assert_eq!(code, Some(2), "{args:?}: {out} {stderr}");
            assert!(stderr.contains("line break"), "{args:?}: {stderr}");
            assert!(!home.exists(), "{args:?} created the home");
        }
    }
    assert_eq!(
        std::fs::read_dir(&w.units)
            .map(|d| d.count())
            .unwrap_or_default(),
        0,
        "a unit was written"
    );
}

/// The home path is one of several values cortex writes into a unit; a schedule holding a line
/// break would add a line of its own to the timer. No unit is written with one.
#[test]
fn a_schedule_holding_a_line_break_writes_no_unit() {
    let w = World::new();
    let spec = w.spec("t", "conn_test");
    let text = std::fs::read_to_string(&spec).unwrap();
    let injected = text.replace("schedule: daily", "schedule: \"daily\\nOnBootSec=1\"");
    assert_ne!(text, injected);
    std::fs::write(&spec, injected).unwrap();
    let (code, out, stderr) = at(
        &w,
        &w.home,
        &["create", "--spec", spec.to_str().unwrap(), "--no-extract"],
    );
    assert_eq!(code, Some(0), "{out} {stderr}");
    let failed = out["detail"]["units"]["failed"]
        .as_str()
        .unwrap_or_default();
    assert!(failed.contains("line break"), "{out}");
    assert!(!w.units.join("cortex-t-news.timer").exists());
    assert!(!w.units.join("cortex-t-news.service").exists());
}

/// A home path holding spaces and a double quote owns the units it wrote.
#[test]
fn a_home_whose_path_holds_spaces_and_a_quote_removes_its_own_units() {
    let w = World::new();
    let home = w.root.join("my \"live\" home");
    assert_created(&create(&w, &home));
    let spec = w.spec("t", "conn_test");
    let (code, out, stderr) = at(
        &w,
        &home,
        &["update", "t", "--spec", spec.to_str().unwrap()],
    );
    assert_eq!(code, Some(0), "{out} {stderr}");
    removes_all_its_units(&w, &home);
}

/// One home, named by a symlink, with a trailing slash, as a relative `--home` and through
/// `CORTEX_HOME`, is one home: none of those forms refuses or keeps its units.
#[test]
fn one_home_in_every_form_owns_its_units() {
    let w = World::new();
    assert_created(&create(&w, &w.home));
    let spec = w.spec("t", "conn_test");
    let spec = spec.to_str().unwrap();
    let link = w.root.join("link-to-home");
    std::os::unix::fs::symlink(&w.home, &link).unwrap();
    let trailing = format!("{}/", w.home.display());
    let dotted = format!("{}/./", w.root.join("bin/..").join("home").display());

    for (label, home, env, cwd) in [
        (
            "symlink",
            Some(link.to_str().unwrap()),
            None,
            w.root.clone(),
        ),
        (
            "trailing slash",
            Some(trailing.as_str()),
            None,
            w.root.clone(),
        ),
        ("dot-dot", Some(dotted.as_str()), None, w.root.clone()),
        ("relative", Some("home"), None, w.root.clone()),
        (
            "relative from below",
            Some("../home"),
            None,
            w.root.join("bin"),
        ),
        (
            "CORTEX_HOME",
            None,
            Some(w.home.to_str().unwrap()),
            w.root.clone(),
        ),
    ] {
        let (code, out, stderr) = cortex(&w, home, env, &cwd, &["update", "t", "--spec", spec]);
        assert_eq!(code, Some(0), "{label}: {out} {stderr}");
    }
    // The last update wrote the units; a symlinked remove still deletes them.
    let (code, out, stderr) = cortex(
        &w,
        Some(link.to_str().unwrap()),
        None,
        &w.root,
        &["remove", "t"],
    );
    assert_eq!(code, Some(0), "{out} {stderr}");
    assert!(out["detail"].get("units").is_none(), "{out}");
    for unit in UNITS {
        assert!(!w.units.join(unit).exists(), "{unit} is left");
    }
}

/// `<root>/home` and `<root>/home2` share a prefix and are two homes.
#[test]
fn two_homes_whose_paths_share_a_prefix_are_two_homes() {
    let w = World::new();
    assert_created(&create(&w, &w.home));
    let before = units(&w);
    let other: PathBuf = PathBuf::from(format!("{}2", w.home.display()));
    let (code, out, stderr) = create(&w, &other);
    assert_eq!(code, Some(2), "{out} {stderr}");
    assert_eq!(units(&w), before);
}

/// A viewer an earlier cortex wrote names its home only through `EKR_HOST`. A second home's
/// `remove` of an instance of the same name keeps it; nothing else in the suite reaches the
/// `EKR_HOST` branch alone, since the source service is always checked first.
#[test]
fn an_old_viewer_is_attributed_to_the_home_of_its_ekr_host() {
    let w = World::new();
    assert_created(&create(&w, &w.home));
    strip_home_lines(&w, &["cortex-t-view.service"]);
    let viewer = std::fs::read_to_string(w.units.join("cortex-t-view.service")).unwrap();

    let scratch = w.root.join("home-scratch");
    let spec = w.spec("t", "conn_test");
    let r = at(
        &w,
        &scratch,
        &[
            "create",
            "--spec",
            spec.to_str().unwrap(),
            "--no-extract",
            "--no-units",
        ],
    );
    assert_eq!(r.0, Some(0), "{} {}", r.1, r.2);
    let (code, out, stderr) = at(&w, &scratch, &["remove", "t"]);
    assert_eq!(code, Some(0), "{out} {stderr}");
    assert_eq!(
        std::fs::read_to_string(w.units.join("cortex-t-view.service")).unwrap_or_default(),
        viewer,
        "a second home's remove deleted an old viewer of another home: {out}"
    );
    assert!(
        out["detail"]["units"]["kept"]
            .to_string()
            .contains("cortex-t-view.service"),
        "{out}"
    );
}

/// `remove` of an instance whose unit files were deleted by hand succeeds and reports nothing
/// kept.
#[test]
fn remove_after_the_units_were_deleted_by_hand() {
    let w = World::new();
    assert_created(&create(&w, &w.home));
    for unit in UNITS {
        std::fs::remove_file(w.units.join(unit)).unwrap();
    }
    removes_all_its_units(&w, &w.home);
}

/// An install an earlier cortex made (no `X-CortexHome=` in any unit) is updated by this cortex,
/// which records the home in each unit it rewrites and keeps the `ExecStart`, and is removed by
/// it in full; its timer's own failure handling still disables it.
#[test]
fn an_install_from_an_earlier_cortex_is_updated_and_removed_by_its_own_home() {
    let w = World::new();
    assert_created(&create(&w, &w.home));
    strip_home_lines(&w, &UNITS);
    let exec = |w: &World| {
        std::fs::read_to_string(w.units.join("cortex-t-news.service"))
            .unwrap()
            .lines()
            .find(|l| l.starts_with("ExecStart="))
            .map(str::to_string)
    };
    let before = exec(&w);

    // The timer runs `<home>/bin/cortex --home <home> run t/news --record-failure` before any
    // update: two failures disable its own timer.
    let calls = w.lines("systemctl.log").len();
    for _ in 0..2 {
        let (code, out, stderr) = at(
            &w,
            &w.home,
            &["source", "record-failure", "t/news", "--reason", "r"],
        );
        assert_eq!(code, Some(0), "{out} {stderr}");
        assert!(out["detail"].get("units").is_none(), "{out}");
    }
    let disabled: Vec<String> = w.lines("systemctl.log").into_iter().skip(calls).collect();
    assert!(
        disabled
            .iter()
            .any(|c| c.contains("disable --now cortex-t-news.timer")),
        "{disabled:?}"
    );
    let (code, out, stderr) = at(&w, &w.home, &["source", "enable", "t/news"]);
    assert_eq!(code, Some(0), "{out} {stderr}");

    let spec = w.spec("t", "conn_test");
    let (code, out, stderr) = at(
        &w,
        &w.home,
        &["update", "t", "--spec", spec.to_str().unwrap()],
    );
    assert_eq!(code, Some(0), "{out} {stderr}");
    assert!(out["detail"].get("units").is_none(), "{out}");
    assert_eq!(exec(&w), before);
    for unit in ["cortex-t-news.service", "cortex-t-news.timer"] {
        let text = std::fs::read_to_string(w.units.join(unit)).unwrap();
        assert!(
            text.lines()
                .any(|l| l == format!("X-CortexHome={}", w.home.display())),
            "{unit}:\n{text}"
        );
    }
    removes_all_its_units(&w, &w.home);
}
