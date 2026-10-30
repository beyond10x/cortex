//! The spec forms of the 1.0 types, held to the published schema
//! (`website/static/schemas/instance-spec.schema.json`): every `store` form it accepts parses,
//! every form it refuses is refused and the refusal names `store`, a gate bound that is no decimal
//! is refused, both SQLite forms freeze, update into each other and run, and a source named after
//! a state file cortex keeps is refused.
//!
//! The cases were written by the wave's adversary (pass 2) and moved here.

mod common;

use std::path::PathBuf;
use std::process::Command;

use common::{ekr, World};
use cortex_cli::spec;
use cortex_model::instance as m;
use serde_json::Value;
/// A minimal valid spec with `store` written as given (a YAML flow mapping, or `null`).
fn spec_text(name: &str, store: &str) -> String {
    format!(
        r#"format: cortex.instance/1
name: {name}
description: Test brain.
ekr: {{version: "0.0.30", bin: "{ekr}"}}
seed: {{documents: []}}
model: {{model: claude-haiku-4-5-20251001, budget_usd: "1", timeout_s: 60}}
sources:
  - name: news
    schedule: daily
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
serve: {{view_port: 18997}}
store: {store}
"#,
        ekr = ekr().display()
    )
}

fn parsed_store(store: &str) -> Result<Option<m::StoreSpec>, String> {
    spec::parse(&spec_text("s", store)).map(|s| s.store)
}

/// `cortex` without `World::cortex`'s refusal of exit 2: the exit code and stderr.
fn cortex_raw(w: &World, args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
        .args(args)
        .env("CORTEX_HOME", &w.home)
        .env("CORTEX_CONNECTORS", w.bin.join("connectors"))
        .env("CORTEX_CLAUDE", w.bin.join("claude"))
        .env("CORTEX_SYSTEMCTL", w.bin.join("systemctl"))
        .env("CORTEX_UNIT_DIR", &w.units)
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn write(w: &World, file: &str, text: &str) -> PathBuf {
    let path = w.root.join(file);
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn every_sqlite_form_the_published_schema_accepts_parses_to_sqlite() {
    // The published schema's `sqlite` alternative requires only `backend`; `value` is an
    // optional object whose one property `path` is an optional string.
    assert_eq!(
        parsed_store("{backend: sqlite}"),
        Ok(Some(m::StoreSpec::Sqlite(None)))
    );
    assert_eq!(
        parsed_store("{backend: sqlite, value: {}}"),
        Ok(Some(m::StoreSpec::Sqlite(Some(m::SqliteStore {
            path: None
        }))))
    );
    assert_eq!(
        parsed_store("{backend: sqlite, value: {path: brain.db}}"),
        Ok(Some(m::StoreSpec::Sqlite(Some(m::SqliteStore {
            path: Some("brain.db".into())
        }))))
    );
    assert_eq!(
        parsed_store("{backend: postgres, value: {config: pg.yaml}}"),
        Ok(Some(m::StoreSpec::Postgres(m::PostgresStore {
            config: "pg.yaml".into(),
            connection: None,
            schema_connection: None,
        })))
    );
    assert_eq!(
        parsed_store(
            "{backend: postgres, value: {config: pg.json, \
             connection: {adapter: pg, connection: app}, \
             schema_connection: {adapter: pg, connection: owner}}}"
        ),
        Ok(Some(m::StoreSpec::Postgres(m::PostgresStore {
            config: "pg.json".into(),
            connection: Some(m::ConnectionRef {
                adapter: "pg".into(),
                connection: Some("app".into()),
                profile: None,
            }),
            schema_connection: Some(m::ConnectionRef {
                adapter: "pg".into(),
                connection: Some("owner".into()),
                profile: None,
            }),
        })))
    );
}

#[test]
fn every_store_form_the_published_schema_refuses_is_refused() {
    for store in [
        "{backend: sqlite, value: null}",
        "{backend: postgres}",
        "{backend: postgres, value: {}}",
        "{backend: postgres, value: null}",
        "{backend: mysql}",
        "{backend: mysql, value: {}}",
        "{backend: Sqlite}",
        "{value: {}}",
        "{backend: sqlite, value: {path: null}}",
        "{backend: sqlite, value: {path: a.db, extra: 1}}",
        "{backend: sqlite, extra: 1}",
    ] {
        assert!(parsed_store(store).is_err(), "{store} was accepted");
    }
}

#[test]
fn a_gate_bound_the_published_schema_refuses_as_a_decimal_is_refused() {
    // website/static/schemas/instance-spec.schema.json gives `GateCheck.min` and `.max` the
    // decimal pattern `^-?(0|[1-9][0-9]*)(\.[0-9]+)?$`, so the schema refuses each of these.
    for bound in [
        r#"min: "abc""#,
        r#"max: "1,5""#,
        r#"min: """#,
        r#"max: "NaN""#,
    ] {
        let text = spec_text("g", "{backend: sqlite}")
            + &format!("gate:\n  checks:\n    - {{measure: facts_applied, {bound}}}\n");
        let parsed = spec::parse(&text);
        assert!(parsed.is_err(), "the gate check `{bound}` was accepted");
        // The refusal names the field to fix.
        let field = format!("gate.checks[0].{}", &bound[..3]);
        let err = parsed.err().unwrap_or_default();
        assert!(err.contains(&field), "{bound}: {err} does not name {field}");
    }
}

#[test]
fn a_gate_bound_the_published_schema_accepts_as_a_decimal_is_accepted() {
    for bound in [r#"min: "0""#, r#"max: "-1.25""#, r#"min: "10""#] {
        let text = spec_text("g", "{backend: sqlite}")
            + &format!("gate:\n  checks:\n    - {{measure: facts_applied, {bound}}}\n");
        assert!(
            spec::parse(&text).is_ok(),
            "the gate check `{bound}` was refused"
        );
    }
}

#[test]
fn a_refused_store_names_the_store_field() {
    // The operator has to find the line to fix: a refusal of `store` says `store`.
    for store in [
        "{backend: sqlite, value: null}",
        "{backend: postgres}",
        "{backend: mysql}",
    ] {
        let w = World::new();
        let path = write(&w, "s.yaml", &spec_text("s", store));
        let (code, _, stderr) = cortex_raw(
            &w,
            &["create", "--spec", path.to_str().unwrap(), "--no-units"],
        );
        assert_eq!(code, Some(2), "{store}: {stderr}");
        assert!(
            stderr.contains("store"),
            "the refusal of {store} does not name `store`: {stderr}"
        );
    }
}

#[test]
fn both_sqlite_forms_freeze_update_into_each_other_and_run() {
    let w = World::new();
    let short = spec_text("rt", "{backend: sqlite}");
    let path = write(&w, "rt.yaml", &short);
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(
        (code, created["outcome"].as_str()),
        (0, Some("created")),
        "{created}"
    );
    let frozen = w.home.join("instances/rt/instance.yaml");
    assert_eq!(std::fs::read_to_string(&frozen).unwrap(), short);

    // A sqlite `value.path` is not supported yet (wave 20261005c, U1 fix 1, F2/F3): the update is
    // refused and the frozen spec and the store stay as they were.
    let with_path = write(
        &w,
        "rt.yaml",
        &spec_text("rt", "{backend: sqlite, value: {path: brain.db}}"),
    );
    let (code, refused) = w.cortex(&[
        "update",
        "rt",
        "--spec",
        with_path.to_str().unwrap(),
        "--no-units",
    ]);
    assert_eq!(
        (code, refused["outcome"].as_str()),
        (1, Some("seed-change-refused")),
        "{refused}"
    );
    assert_eq!(std::fs::read_to_string(&frozen).unwrap(), short);

    for store in ["{backend: sqlite, value: {}}", "{backend: sqlite}"] {
        let text = spec_text("rt", store);
        let path = write(&w, "rt.yaml", &text);
        let (code, updated) = w.cortex(&[
            "update",
            "rt",
            "--spec",
            path.to_str().unwrap(),
            "--no-units",
        ]);
        assert_eq!(
            (code, updated["outcome"].as_str()),
            (0, Some("updated")),
            "{store}: {updated}"
        );
        assert_eq!(std::fs::read_to_string(&frozen).unwrap(), text, "{store}");
        let (code, ran) = w.cortex(&["run", "rt/news"]);
        assert_eq!(
            (code, ran["outcome"].as_str()),
            (0, Some("ran")),
            "{store}: {ran}"
        );
    }
    let registry: Value =
        serde_json::from_slice(&std::fs::read(w.home.join("registry.json")).unwrap()).unwrap();
    assert_eq!(registry["instances"][0]["name"], "rt", "{registry}");
    assert_eq!(registry["instances"][0]["state"], "Active", "{registry}");
}

/// `spec_text` with its one source named `source` instead of `news`.
fn spec_with_source(name: &str, source: &str) -> String {
    spec_text(name, "{backend: sqlite}").replace("- name: news", &format!("- name: {source}"))
}

#[test]
fn a_source_named_after_a_state_file_cortex_keeps_is_refused_naming_it() {
    // A source's seen documents are `state/<source>.json`; cortex keeps the known entity names in
    // `state/entities.json` and the seed's seen documents in `state/seed.json`. A source of either
    // name would share that file (issue #27), so a spec given to create, update or adopt is refused.
    for reserved in ["entities", "seed"] {
        let w = World::new();
        let given = write(&w, "r.yaml", &spec_with_source("r", reserved));
        let given = given.to_str().unwrap();
        let named =
            |stderr: &str| stderr.contains(&format!("{reserved:?}")) && stderr.contains("reserved");

        let (code, _, stderr) = cortex_raw(&w, &["create", "--spec", given, "--no-units"]);
        assert_eq!(code, Some(2), "create with a source {reserved:?}: {stderr}");
        assert!(
            named(&stderr),
            "create does not name {reserved:?} as reserved: {stderr}"
        );
        assert!(
            !w.home.join("instances/r").exists(),
            "create left an instance"
        );

        let (code, _, stderr) = cortex_raw(
            &w,
            &[
                "adopt",
                "--spec",
                given,
                "--store",
                "absent.sqlite",
                "--no-units",
            ],
        );
        assert_eq!(code, Some(2), "adopt with a source {reserved:?}: {stderr}");
        assert!(
            named(&stderr),
            "adopt does not name {reserved:?} as reserved: {stderr}"
        );

        let plain = write(&w, "plain.yaml", &spec_with_source("r", "news"));
        let (code, created) =
            w.cortex(&["create", "--spec", plain.to_str().unwrap(), "--no-units"]);
        assert_eq!(
            (code, created["outcome"].as_str()),
            (0, Some("created")),
            "{created}"
        );
        let frozen = std::fs::read_to_string(w.home.join("instances/r/instance.yaml")).unwrap();
        let (code, _, stderr) = cortex_raw(&w, &["update", "r", "--spec", given, "--no-units"]);
        assert_eq!(code, Some(2), "update to a source {reserved:?}: {stderr}");
        assert!(
            named(&stderr),
            "update does not name {reserved:?} as reserved: {stderr}"
        );
        assert_eq!(
            std::fs::read_to_string(w.home.join("instances/r/instance.yaml")).unwrap(),
            frozen,
            "a refused update changed the frozen spec"
        );
    }
}

#[test]
fn a_frozen_spec_with_a_reserved_source_name_still_parses() {
    // The refusal is made where an operator gives a spec file, not where an instance reads its
    // frozen copy: an instance created before the rule keeps loading, so every other source of it
    // keeps running and `update` to a renamed source is possible.
    for reserved in ["entities", "seed"] {
        assert!(
            spec::parse(&spec_with_source("r", reserved)).is_ok(),
            "{reserved}"
        );
    }
}

/// The spec of instance `name` with `refresh_after_days`, `view_port` and, when given, a failure
/// hook running `hook` with `timeout_s`, each written as given.
fn spec_with_numbers(
    name: &str,
    refresh_after_days: &str,
    view_port: &str,
    hook: Option<(&str, &str)>,
) -> String {
    let mut text = spec_text(name, "{backend: sqlite}")
        .replace(
            "refresh_after_days: 0,",
            &format!("refresh_after_days: {refresh_after_days},"),
        )
        .replace(
            "serve: {view_port: 18997}",
            &format!("serve: {{view_port: {view_port}}}"),
        );
    if let Some((command, timeout_s)) = hook {
        text.push_str(&format!(
            "monitoring: {{hook: {{command: [\"{command}\"], timeout_s: {timeout_s}}}}}\n"
        ));
    }
    text
}

#[test]
fn an_out_of_range_number_is_refused_by_create_naming_the_field() {
    // `refresh_after_days` is multiplied into milliseconds, a hook's `timeout_s` is added to an
    // `Instant` and `view_port` is a TCP port: out of range they overflowed or wrapped at run time.
    let max = i64::MAX.to_string();
    let cases: Vec<(String, &str)> = [
        ("36501", "18997", None),
        (max.as_str(), "18997", None),
        ("-1", "18997", None),
        ("0", "65536", None),
        ("0", "70000", None),
        ("0", "-1", None),
        ("0", "18997", Some(("/bin/true", "0"))),
        ("0", "18997", Some(("/bin/true", "86401"))),
        ("0", "18997", Some(("/bin/true", max.as_str()))),
    ]
    .into_iter()
    .map(|(refresh, port, hook)| {
        let field = match (refresh, port, hook) {
            (_, _, Some(_)) => "monitoring.hook.timeout_s",
            ("0", _, None) => "serve.view_port",
            _ => "sources.news.policy.refresh_after_days",
        };
        (spec_with_numbers("n", refresh, port, hook), field)
    })
    .collect();
    for (text, field) in cases {
        // A negative or zero value fails the check every spec is read with; one above the range
        // fails only where an operator gives the spec (`spec::bounds_refusal`).
        assert!(
            spec::parse(&text).map_or(true, |s| spec::bounds_refusal(&s).is_some()),
            "{field} accepted:\n{text}"
        );
        let w = World::new();
        let path = write(&w, "n.yaml", &text);
        let (code, _, stderr) = cortex_raw(
            &w,
            &[
                "create",
                "--spec",
                path.to_str().unwrap(),
                "--no-units",
                "--no-extract",
            ],
        );
        assert_eq!(code, Some(2), "{field}: {stderr}");
        assert!(
            stderr.contains(field),
            "the refusal does not name {field}: {stderr}"
        );
        assert!(
            !w.home.join("instances/n").exists(),
            "a refused create left its instance directory ({field})"
        );
    }
}

#[test]
fn a_frozen_copy_above_a_bound_keeps_running_and_updates_only_into_range() {
    // cortex 0.6.1 accepted any `refresh_after_days` of 0 or more, any hook `timeout_s` above 0
    // and any `view_port`. Its frozen copy keeps loading: a run saturates the refresh window and
    // caps the hook at a day. An `update` is held to the bounds and names the field.
    let max = i64::MAX.to_string();
    let frozen = spec_with_numbers("f", &max, "70000", Some(("/bin/true", max.as_str())));
    let parsed = spec::parse(&frozen).expect("the frozen copy parses");
    assert!(spec::bounds_refusal(&parsed).is_some());

    let w = World::new();
    let hook = w.root.join("hook");
    common::executable(
        &hook,
        &format!("echo \"$@\" >> \"{}/hook.log\"\n", w.root.display()),
    );
    let given = spec_with_numbers("f", "0", "18997", Some((hook.to_str().unwrap(), "10")));
    let path = write(&w, "f.yaml", &given);
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
    let copy = w.home.join("instances/f/instance.yaml");
    let text = std::fs::read_to_string(&copy).unwrap();
    for from in ["refresh_after_days: 0", "timeout_s: 10"] {
        assert!(
            text.contains(from),
            "{from} not in the frozen copy:\n{text}"
        );
    }
    std::fs::write(
        &copy,
        text.replace(
            "refresh_after_days: 0",
            &format!("refresh_after_days: {max}"),
        )
        .replace("timeout_s: 10", &format!("timeout_s: {max}")),
    )
    .unwrap();
    for _ in 0..2 {
        let (code, ran) = w.cortex(&["run", "f/news"]);
        assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    }
    std::fs::write(w.root.join("fail"), "").unwrap();
    let (code, _, stderr) = cortex_raw(&w, &["run", "f/news"]);
    assert!(code.is_some_and(|c| c != 0 && c != 2), "{code:?}: {stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");
    assert_eq!(w.lines("hook.log"), ["f/news fetch-failed"]);
    std::fs::remove_file(w.root.join("fail")).unwrap();

    let above = write(
        &w,
        "above.yaml",
        &spec_with_numbers("f", "40000", "18997", None),
    );
    let (code, _, stderr) = cortex_raw(
        &w,
        &[
            "update",
            "f",
            "--spec",
            above.to_str().unwrap(),
            "--no-units",
        ],
    );
    assert_eq!(code, Some(2), "{stderr}");
    assert!(
        stderr.contains("sources.news.policy.refresh_after_days"),
        "{stderr}"
    );
    let (code, updated) = w.cortex(&[
        "update",
        "f",
        "--spec",
        path.to_str().unwrap(),
        "--no-units",
    ]);
    assert_eq!(
        (code, updated["outcome"].as_str()),
        (0, Some("updated")),
        "{updated}"
    );
}

#[test]
fn the_largest_accepted_numbers_run_without_overflow() {
    // 36500 days, port 65535 (and 0, "take a free port") and a hook allowed 86400 seconds are
    // accepted; a run reads the refresh window of seen documents and calls the hook with them.
    for port in ["0", "65535"] {
        assert!(spec::parse(&spec_with_numbers("b", "36500", port, None)).is_ok());
    }
    let w = World::new();
    let hook = w.root.join("hook");
    common::executable(
        &hook,
        &format!("echo \"$@\" >> \"{}/hook.log\"\n", w.root.display()),
    );
    let text = spec_with_numbers(
        "b",
        "36500",
        "65535",
        Some((hook.to_str().unwrap(), "86400")),
    );
    let path = write(&w, "b.yaml", &text);
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(
        (code, created["outcome"].as_str()),
        (0, Some("created")),
        "{created}"
    );
    assert_eq!(
        created["detail"]["view"], "http://127.0.0.1:65535/",
        "{created}"
    );
    // The second run holds every seen document to the 36500-day window.
    for _ in 0..2 {
        let (code, ran) = w.cortex(&["run", "b/news"]);
        assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    }
    std::fs::write(w.root.join("fail"), "").unwrap();
    let (code, _, stderr) = cortex_raw(&w, &["run", "b/news"]);
    assert!(code.is_some_and(|c| c != 0 && c != 2), "{code:?}: {stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");
    assert_eq!(w.lines("hook.log"), ["b/news fetch-failed"]);
}
