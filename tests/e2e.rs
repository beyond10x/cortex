//! End to end through the real `cortex` binary and a real `ekr`, with stand-in `connectors`,
//! `claude` and `systemctl` written at runtime (`tests/common`). A missing `ekr` fails the test; it
//! never skips.

mod common;

use common::{answer, World, PAGES};

#[test]
fn an_instance_is_created_run_twice_and_removed() {
    let w = World::new();
    let spec = w.spec("t", "conn_test");

    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap()]);
    assert_eq!(
        (code, created["outcome"].as_str()),
        (0, Some("created")),
        "{created}"
    );
    assert!(w.units.join("cortex-t-news.timer").is_file());
    assert!(w.units.join("cortex-t-view.service").is_file());
    assert!(w
        .lines("systemctl.log")
        .iter()
        .any(|l| l == "--user enable --now cortex-t-news.timer"));
    let seeded = w.head("t");

    let (code, ran) = w.cortex(&["run", "t/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_new"], 2, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
    assert_eq!(
        ran["detail"]["facts_refused"], 1,
        "the forged citation is refused: {ran}"
    );
    assert_eq!(
        ran["detail"]["parts_rejected"], 0,
        "EKR rejected nothing: {ran}"
    );
    let invoked = w.lines("invocations.log").join("\n");
    assert!(
        invoked.contains("--adapter tavily") && invoked.contains("--operation websearch.search"),
        "the web source invokes the websearch contract: {invoked}"
    );
    assert!(invoked.contains(r#""content":"full""#), "{invoked}");
    // The connection was pending, so the run revalidated it once before reading.
    let revalidated = w.lines("revalidations.log");
    assert_eq!(revalidated.len(), 1, "{revalidated:?}");
    assert!(revalidated[0].contains("--connection conn_test --expected-revision rev1"));
    assert!(w.head("t") > seeded, "the run committed to the store");

    // The model call is isolated: no tools, no user settings, no API key.
    assert_eq!(w.lines("claude-calls.log").len(), 1);
    let args = w.lines("claude-args.log").join(" ");
    assert!(
        args.contains("-p --tools  --setting-sources  --strict-mcp-config"),
        "{args}"
    );
    assert_eq!(w.lines("claude-env.log"), ["0"]);

    // Nothing changed upstream: nothing new, no model call, no commit.
    let head = w.head("t");
    let (code, again) = w.cortex(&["run", "t/news"]);
    assert_eq!(
        (code, again["detail"]["documents_new"].as_i64()),
        (0, Some(0)),
        "{again}"
    );
    assert_eq!(w.lines("claude-calls.log").len(), 1);
    assert_eq!(w.head("t"), head);

    // A changed page is extracted again, and re-declaring known types applies cleanly.
    std::fs::write(
        w.root.join("answer.json"),
        answer(&PAGES.replace(
            "also develops the Gadget runtime",
            "now develops the Gizmo runtime",
        )),
    )
    .unwrap();
    let (code, changed) = w.cortex(&["run", "t/news"]);
    assert_eq!(
        (code, changed["detail"]["documents_applied"].as_i64()),
        (0, Some(1)),
        "{changed}"
    );
    assert_eq!(
        changed["detail"]["parts_rejected"], 0,
        "re-declared types apply cleanly: {changed}"
    );

    let (code, taken) = w.cortex(&["create", "--spec", spec.to_str().unwrap()]);
    assert_eq!(
        (code, taken["outcome"].as_str()),
        (1, Some("name-taken")),
        "{taken}"
    );

    let (code, removed) = w.cortex(&["remove", "t"]);
    assert_eq!(
        (code, removed["outcome"].as_str()),
        (0, Some("removed")),
        "{removed}"
    );
    assert!(!w.units.join("cortex-t-news.timer").exists());
    let (code, again) = w.cortex(&["remove", "t"]);
    assert_eq!(
        (code, again["outcome"].as_str()),
        (1, Some("wrong-state")),
        "{again}"
    );
}

#[test]
fn a_connection_connectors_does_not_list_creates_nothing() {
    let w = World::new();
    let spec = w.spec("u", "conn_unknown");
    let (code, out) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(
        (code, out["outcome"].as_str()),
        (1, Some("connection-missing")),
        "{out}"
    );
    assert_eq!(out["detail"]["connection"], "tavily:conn_unknown");
    assert!(!w.home.join("instances/u").exists());
}

#[test]
fn two_failed_runs_in_a_row_disable_a_source_until_it_is_enabled() {
    let w = World::new();
    let spec = w.spec("f", "conn_test");
    let (code, _) = w.cortex(&["create", "--spec", spec.to_str().unwrap()]);
    assert_eq!(code, 0);
    std::fs::write(w.root.join("fail"), "").unwrap();

    let (code, first) = w.cortex(&["run", "f/news", "--record-failure"]);
    assert_eq!(
        (code, first["outcome"].as_str()),
        (1, Some("counted")),
        "{first}"
    );
    let (_, second) = w.cortex(&["run", "f/news", "--record-failure"]);
    assert_eq!(second["outcome"], "disabled", "{second}");
    assert!(w
        .lines("systemctl.log")
        .iter()
        .any(|l| l == "--user disable --now cortex-f-news.timer"));

    let (code, refused) = w.cortex(&["run", "f/news"]);
    assert_eq!(
        (code, refused["outcome"].as_str()),
        (1, Some("disabled")),
        "{refused}"
    );

    std::fs::remove_file(w.root.join("fail")).unwrap();
    let (code, enabled) = w.cortex(&["source", "enable", "f/news"]);
    assert_eq!(
        (code, enabled["outcome"].as_str()),
        (0, Some("enabled")),
        "{enabled}"
    );
    let (code, ran) = w.cortex(&["run", "f/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
}

#[test]
fn a_revalidation_whose_outcome_is_unknown_is_settled_by_reading_the_status() {
    let w = World::new();
    let spec = w.spec("u", "conn_test");
    let (code, _) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0);
    // Connectors applied the revalidation but could not confirm it (outcome_unknown at
    // publication, next_action retry_status); the status read that follows says ready.
    std::fs::write(w.root.join("unknown"), "").unwrap();
    let (code, ran) = w.cortex(&["run", "u/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(w.lines("revalidations.log").len(), 1);
    assert!(!w.lines("status.log").is_empty(), "the status was read");
}

#[test]
fn a_revalidation_whose_outcome_stays_unknown_fails_the_fetch_naming_the_state() {
    let w = World::new();
    let spec = w.spec("p", "conn_test");
    let (code, _) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0);
    std::fs::write(w.root.join("unknown"), "").unwrap();
    std::fs::write(w.root.join("stays-pending"), "").unwrap();
    let (code, ran) = w.cortex_env(
        &["run", "p/news"],
        &[
            ("CORTEX_STATUS_POLLS", std::path::Path::new("2")),
            ("CORTEX_STATUS_POLL_MS", std::path::Path::new("10")),
        ],
    );
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("fetch-failed")),
        "{ran}"
    );
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("outcome_unknown") && reason.contains("pending"),
        "{reason}"
    );
    assert_eq!(
        w.lines("status.log").len(),
        2,
        "polled as many times as allowed"
    );
}

#[test]
fn evidence_that_lapses_before_admission_is_renewed_once_and_the_read_retried() {
    let w = World::new();
    let spec = w.spec("l", "conn_test");
    let (code, _) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0);
    // Listed ready, refused at admission once (on stderr, as the CLI does).
    std::fs::write(w.root.join("revalidated"), "").unwrap();
    std::fs::write(w.root.join("lapse"), "").unwrap();
    let (code, ran) = w.cortex(&["run", "l/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
    assert_eq!(
        w.lines("revalidations.log").len(),
        1,
        "renewed exactly once"
    );
    assert_eq!(
        w.lines("invocations.log").len(),
        2,
        "the read was retried once"
    );
}

/// The last line of an instance's `cortex.log`.
fn last_log_line(w: &World, name: &str) -> serde_json::Value {
    let log = w.home.join("instances").join(name).join("cortex.log");
    let text = std::fs::read_to_string(&log).unwrap();
    serde_json::from_str(text.lines().last().expect("a log line")).unwrap()
}

#[test]
fn a_model_answer_without_a_cost_is_reported_as_no_cost_never_as_zero() {
    let w = World::new();
    let spec = w.spec("c", "conn_test");
    let (code, _) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0);
    std::fs::write(w.root.join("no-cost"), "").unwrap();
    let (code, ran) = w.cortex(&["run", "c/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
    assert_eq!(ran["detail"]["cost_usd"], serde_json::Value::Null, "{ran}");
    let line = last_log_line(&w, "c");
    assert_eq!(line["cost_usd"], serde_json::Value::Null, "{line}");
}

#[test]
fn a_run_that_calls_no_model_costs_zero_and_a_costed_one_reports_its_cost() {
    let w = World::new();
    let spec = w.spec("z", "conn_test");
    let (code, _) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0);
    let (_, ran) = w.cortex(&["run", "z/news"]);
    assert_eq!(ran["detail"]["cost_usd"], "0.0100", "{ran}");
    assert_eq!(last_log_line(&w, "z")["cost_usd"], 0.01);
    let (_, again) = w.cortex(&["run", "z/news"]);
    assert_eq!(again["detail"]["documents_new"], 0, "{again}");
    assert_eq!(again["detail"]["cost_usd"], "0.0000", "{again}");
    assert_eq!(last_log_line(&w, "z")["cost_usd"], 0.0);
}

#[test]
fn the_codex_backend_fails_extraction_naming_the_story_that_builds_it() {
    let w = World::new();
    let spec = w.spec("x", "conn_test");
    let text = std::fs::read_to_string(&spec).unwrap();
    let from = "model: {model: claude-haiku-4-5-20251001,";
    assert!(text.contains(from), "{text}");
    std::fs::write(
        &spec,
        text.replace(from, "model: {model: gpt-5-codex, backend: Codex,"),
    )
    .unwrap();
    let (code, created) = w.cortex(&["create", "--spec", spec.to_str().unwrap(), "--no-units"]);
    assert_eq!(
        (code, created["outcome"].as_str()),
        (0, Some("created")),
        "{created}"
    );
    let codex = w.bin.join("codex");
    let (code, ran) = w.cortex(&["--codex", codex.to_str().unwrap(), "run", "x/news"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("extraction-failed")),
        "{ran}"
    );
    let reason = ran["detail"]["reason"].as_str().unwrap_or_default();
    assert!(reason.contains("story:codex-model-backend"), "{ran}");
    assert!(
        w.lines("claude-calls.log").is_empty(),
        "claude was not asked"
    );
}

#[test]
fn the_codex_binary_is_a_flag_beside_the_claude_binary() {
    let w = World::new();
    let codex = w.bin.join("codex");
    let (code, _) = w.cortex(&["--codex", codex.to_str().unwrap(), "list"]);
    assert_eq!(code, 0);
}

/// A structured source is accepted, registered as `Structured`, survives a registry reload, and,
/// reading files, fails its run naming the story that builds file input. Written by the wave's
/// adversary (pass 1); `story:structured-source` runs the Connectors input (`tests/structured.rs`).
#[test]
fn a_structured_source_is_registered_round_trips_and_does_not_run() {
    let w = World::new();
    std::fs::create_dir_all(w.root.join("records")).unwrap();
    let path = w.spec("st", "conn_test");
    let text = std::fs::read_to_string(&path).unwrap();
    let structured = r#"  - name: people
    schedule: daily
    settings:
      kind: structured
      value:
        input:
          from: files
          value: {paths: [records], glob: "**/*.json"}
        records: "$.people"
        mapping:
          node_type: Person
          id: "$.id"
          name: "$.name"
          aliases: []
          properties: []
          relations: []
        dropped: Supersede
    policy: {refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}
serve:"#;
    std::fs::write(&path, text.replacen("serve:", structured, 1)).unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(
        (code, created["outcome"].as_str()),
        (0, Some("created")),
        "{created}"
    );
    let registry: serde_json::Value =
        serde_json::from_slice(&std::fs::read(w.home.join("registry.json")).unwrap()).unwrap();
    let kinds: Vec<_> = registry["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| (s["name"].clone(), s["kind"].clone()))
        .collect();
    assert!(
        kinds.contains(&(serde_json::json!("people"), serde_json::json!("Structured"))),
        "{registry}"
    );
    let (code, ran) = w.cortex(&["run", "st/people"]);
    assert_eq!(
        (code, ran["outcome"].as_str()),
        (1, Some("fetch-failed")),
        "{ran}"
    );
    assert!(
        ran["detail"]["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("story:structured-from-files-and-drops"),
        "{ran}"
    );
    let (code, ran) = w.cortex(&["run", "st/news"]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
}
