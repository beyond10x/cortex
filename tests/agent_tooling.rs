//! `examples/agent-tooling.yaml`, the first web instance (`story:first-web-instance`), is created
//! as the story's Instance table describes it: its seed ontology, a daily news search for three
//! queries, a weekly crawl of the Model Context Protocol specification, and $0.50 per run. Each
//! case creates it from a copy of `examples/`, with the stand-in `connectors`, `claude` and
//! `systemctl` of `tests/common`; no search and no model call leaves the machine.

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::{answer, ekr, World};
use serde_json::{json, Value};

const NAME: &str = "agent-tooling";
/// The connection id the example names, to be replaced by a real one; the stand-in lists it.
const CONNECTION: &str = "conn_replace_with_your_tavily_connection_id";
const QUERIES: [&str; 3] = [
    "Model Context Protocol specification change",
    "Claude Code release",
    "Rust AI agent framework release",
];
const SPECIFICATION: &str = "https://modelcontextprotocol.io/specification";

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let dest = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &dest);
        } else {
            std::fs::copy(entry.path(), &dest).unwrap();
        }
    }
}

/// A world with the example created in it, timers included (written to the world's unit
/// directory, with the stand-in `systemctl`), and the `HOME` its commands run with.
fn created() -> (World, PathBuf) {
    let w = World::new();
    std::fs::write(w.root.join("connection"), CONNECTION).unwrap();
    // The example names no `ekr.bin`, so cortex takes the pinned binary under `$HOME`.
    let user = w.root.join("user");
    let pinned = user.join(".cache/cortex/bin/0.0.31/bin");
    std::fs::create_dir_all(&pinned).unwrap();
    std::os::unix::fs::symlink(ekr(), pinned.join("ekr")).unwrap();
    let dir = w.root.join("spec");
    copy_dir(&manifest().join("examples/seed"), &dir.join("seed"));
    let spec = dir.join("agent-tooling.yaml");
    std::fs::copy(manifest().join("examples/agent-tooling.yaml"), &spec).unwrap();

    let (code, out) = w.cortex_env(
        &["create", "--spec", spec.to_str().unwrap()],
        &[("HOME", user.as_path())],
    );
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["outcome"], "created", "{out}");
    (w, user)
}

fn run(w: &World, user: &Path, source: &str) -> Value {
    let (code, out) = w.cortex_env(&["run", &format!("{NAME}/{source}")], &[("HOME", user)]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(out["outcome"], "ran", "{out}");
    assert!(
        out["detail"]["documents_applied"].as_u64().unwrap() > 0,
        "{out}"
    );
    out
}

/// The `--input-json` of every `operations invoke` of `operation` the stand-in recorded; the
/// global `--output json` follows it.
fn inputs(w: &World, operation: &str) -> Vec<Value> {
    w.lines("invocations.log")
        .iter()
        .filter(|line| line.contains(&format!("--operation {operation} ")))
        .map(|line| {
            let (_, input) = line.split_once("--input-json ").unwrap();
            let input = input.strip_suffix(" --output json").unwrap_or(input);
            serde_json::from_str(input).unwrap_or_else(|e| panic!("{e}: {line}"))
        })
        .collect()
}

/// The value after `flag` in the first `claude` call the stand-in recorded.
fn claude_arg(w: &World, flag: &str) -> String {
    let calls = w.lines("claude-args.log");
    let first = calls.first().expect("a model call");
    let words: Vec<&str> = first.split(' ').collect();
    let at = words.iter().position(|w| *w == flag).expect(flag);
    words[at + 1].to_string()
}

/// Every name under `key` in the store's ontology (`ekr ontology`).
fn ontology_names(w: &World, key: &str) -> BTreeSet<String> {
    let dir = w.home.join("instances").join(NAME);
    let out = Command::new(ekr())
        .arg("ontology")
        .env("EKR_HOST", dir.join("host.json"))
        .env("EKR_BACKEND", "sqlite")
        .env("EKR_STORE", dir.join("store.sqlite"))
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let ontology: Value = serde_json::from_slice(&out.stdout).unwrap();
    let mut names = BTreeSet::new();
    collect(&ontology, key, &mut names);
    names
}

fn collect(v: &Value, key: &str, names: &mut BTreeSet<String>) {
    match v {
        Value::Object(o) => {
            for (k, child) in o {
                if k == key {
                    for item in child.as_array().into_iter().flatten() {
                        if let Some(name) = item["name"].as_str() {
                            names.insert(name.to_string());
                        }
                    }
                }
                collect(child, key, names);
            }
        }
        Value::Array(items) => items.iter().for_each(|i| collect(i, key, names)),
        _ => {}
    }
}

fn on_calendar(w: &World, source: &str) -> String {
    let timer = w.units.join(format!("cortex-{NAME}-{source}.timer"));
    let text = std::fs::read_to_string(&timer).unwrap();
    text.lines()
        .find_map(|l| l.strip_prefix("OnCalendar="))
        .unwrap_or_else(|| panic!("no OnCalendar in {}", timer.display()))
        .to_string()
}

#[test]
fn the_agent_tooling_example_creates_with_its_seed_ontology_and_two_timers() {
    let (w, _) = created();

    let nodes = ontology_names(&w, "node_types");
    for name in ["Specification", "Release", "Project", "Organization"] {
        assert!(nodes.contains(name), "{name} missing from {nodes:?}");
    }
    let edges = ontology_names(&w, "edge_types");
    for name in ["RELEASES", "SPECIFIES", "DEPENDS_ON"] {
        assert!(edges.contains(name), "{name} missing from {edges:?}");
    }

    // The search runs every day, the crawl once a week.
    assert_eq!(on_calendar(&w, "news"), "*-*-* 00:30:00");
    assert_eq!(on_calendar(&w, "mcp-specification"), "Wed *-*-* 01:30:00");
}

#[test]
fn the_search_source_asks_for_the_week_s_news_on_three_queries_within_0_50_usd() {
    let (w, user) = created();
    run(&w, &user, "news");

    let searches = inputs(&w, "websearch.search");
    let queries: BTreeSet<&str> = searches
        .iter()
        .map(|i| i["query"].as_str().unwrap())
        .collect();
    assert_eq!(queries, BTreeSet::from(QUERIES), "{searches:?}");
    for input in &searches {
        assert_eq!(input["topic"], "news", "{input}");
        assert_eq!(input["time_range"], "week", "{input}");
        assert_eq!(input["max_results"], 5, "{input}");
    }

    let budget: f64 = claude_arg(&w, "--max-budget-usd").parse().unwrap();
    assert_eq!(budget, 0.5);
}

#[test]
fn the_crawl_source_reads_up_to_20_pages_of_the_mcp_specification_two_links_deep() {
    let (w, user) = created();
    let pages = json!({"pages": [{
        "url": format!("{SPECIFICATION}/2025-06-18"),
        "title": "Specification",
        "content": "Example Labs develops the Widget engine.",
    }]});
    std::fs::write(w.root.join("answer.json"), answer(&pages.to_string())).unwrap();
    run(&w, &user, "mcp-specification");

    let crawls = inputs(&w, "websearch.crawl");
    assert_eq!(crawls.len(), 1, "{crawls:?}");
    let crawl = &crawls[0];
    assert_eq!(crawl["url"], SPECIFICATION, "{crawl}");
    assert_eq!(crawl["limit"], 20, "{crawl}");
    assert_eq!(crawl["max_depth"], 2, "{crawl}");
}
