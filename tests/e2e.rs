//! End to end through the real `cortex` binary and a real `ekr`, with stand-in `connectors`,
//! `claude` and `systemctl` written at runtime. A missing `ekr` fails the test; it never skips.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn ekr() -> PathBuf {
    let bin = std::env::var_os("CORTEX_TEST_EKR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").expect("HOME"))
                .join(".cache/company-brain-v3/bin/0.0.30/bin/ekr")
        });
    assert!(
        bin.is_file(),
        "the end-to-end test needs ekr 0.0.30 at {} (set CORTEX_TEST_EKR)",
        bin.display()
    );
    bin
}

fn executable(path: &Path, body: &str) {
    std::fs::write(path, format!("#!/bin/sh\n{body}")).unwrap();
    let status = Command::new("chmod").arg("755").arg(path).status().unwrap();
    assert!(status.success());
}

struct World {
    _tmp: tempfile::TempDir,
    root: PathBuf,
    home: PathBuf,
    units: PathBuf,
    bin: PathBuf,
}

const PAGES: &str = r#"{"results":[
 {"url":"https://example.org/a","title":"A","description":"Example Labs ships Widget.","content":"Example Labs develops the Widget engine. Its website is example.org.","content_truncated":false,"published":null,"score":"0.9"},
 {"url":"https://example.org/b","title":"B","description":"Gadget news.","content":"Example Labs also develops the Gadget runtime.","content_truncated":false,"published":null,"score":"0.5"}
],"complete":false,"truncation":["provider_limit"],"provenance":{"instance":"t","profile":"tavily/2026-10","received_at":"2026-10-05T00:00:00.000Z"}}"#;

impl World {
    fn new() -> Self {
        let tmp = tempfile::Builder::new()
            .prefix("cortex-e2e")
            .tempdir_in(
                std::env::var_os("CARGO_TARGET_TMPDIR")
                    .map(PathBuf::from)
                    .unwrap_or_else(std::env::temp_dir),
            )
            .unwrap();
        let root = tmp.path().to_path_buf();
        let bin = root.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(root.join("pages.json"), PAGES).unwrap();
        // connectors: one live tavily connection `conn_test`; `invoke` answers pages.json, or
        // fails when the file `fail` exists.
        executable(
            &bin.join("connectors"),
            &format!(
                r#"R="{root}"
case "$*" in
  *"connections list"*) echo '{{"ok":true,"result":{{"connections":[{{"adapter":"tavily","connection":"conn_test","state":"ready"}}]}}}}' ;;
  *"operations describe"*) echo '{{"ok":true,"result":{{"schema":"s","revision":"r"}}}}' ;;
  *"operations invoke"*)
    echo "$*" >> "$R/invocations.log"
    if [ -e "$R/fail" ]; then echo '{{"ok":false,"error":{{"code":"failure","data":{{"code":"unavailable","stage":"execution"}}}}}}'; exit 1; fi
    printf '{{"ok":true,"result":%s}}' "$(cat "$R/pages.json")" ;;
  *) echo '{{"ok":false}}'; exit 2 ;;
esac
"#,
                root = root.display()
            ),
        );
        // claude: records its arguments and whether ANTHROPIC_API_KEY reached it, then answers
        // one fact per evidence id it was given and one fact citing a forged id.
        executable(
            &bin.join("claude"),
            &format!(
                r#"R="{root}"
echo call >> "$R/claude-calls.log"
printf "%s\n" "$*" | tr "\n" " " >> "$R/claude-args.log"; echo >> "$R/claude-args.log"
env | grep -c '^ANTHROPIC_API_KEY=' >> "$R/claude-env.log"
ids=$(grep -oE 'evidence id [0-9a-f-]{{36}}' | cut -d' ' -f3)
facts=""
for id in $ids; do
  facts="$facts{{\"!Relation\":{{\"subject\":{{\"node_type\":\"Organization\",\"aliases\":[\"Example Labs\"]}},\"relation\":\"DEVELOPS\",\"object\":{{\"node_type\":\"Product\",\"aliases\":[\"Widget engine\"]}},\"evidence\":[\"$id\"]}}}},"
done
facts="$facts{{\"!Property\":{{\"subject\":{{\"node_type\":\"Organization\",\"aliases\":[\"Example Labs\"]}},\"property\":\"website\",\"value\":{{\"value_kind\":\"String\",\"value\":\"forged.example\"}},\"evidence\":[\"00000000-0000-4000-8000-00000000dead\"]}}}}"
cat <<EOF
{{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01,"structured_output":{{"format":"ekr.extraction-document/1","ontology":{{"node_types":[{{"name":"Organization","parents":[],"abstract_type":false,"properties":[{{"name":"website","value":{{"value_kind":"String"}},"cardinality":"One","required":false}}]}},{{"name":"Product","parents":[],"abstract_type":false,"properties":[]}}],"edge_types":[{{"name":"DEVELOPS","source_types":["Organization"],"target_types":["Product"],"cardinality":"Many","properties":[]}}]}},"entities":[{{"node_type":"Organization","aliases":["Example Labs"]}},{{"node_type":"Product","aliases":["Widget engine"]}}],"facts":[$facts]}}}}
EOF
"#,
                root = root.display()
            ),
        );
        executable(
            &bin.join("systemctl"),
            &format!("echo \"$*\" >> \"{}/systemctl.log\"\n", root.display()),
        );
        Self {
            home: root.join("home"),
            units: root.join("units"),
            bin,
            root,
            _tmp: tmp,
        }
    }

    fn spec(&self, name: &str, connection: &str) -> PathBuf {
        let path = self.root.join(format!("{name}.yaml"));
        std::fs::write(
            &path,
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
        connection: {connection}
        input:
          input: search
          value:
            queries: [widget engine]
            policy: {{topic: news, max_results: 3, include_domains: [], exclude_domains: []}}
    policy: {{refresh_after_days: 0, change: ContentHash, max_documents_per_run: 10, max_chars_per_document: 5000}}
serve: {{view_port: 18999}}
"#,
                ekr = ekr().display()
            ),
        )
        .unwrap();
        path
    }

    fn cortex(&self, args: &[&str]) -> (i32, Value) {
        let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
            .args(args)
            .env("CORTEX_HOME", &self.home)
            .env("CORTEX_CONNECTORS", self.bin.join("connectors"))
            .env("CORTEX_CLAUDE", self.bin.join("claude"))
            .env("CORTEX_SYSTEMCTL", self.bin.join("systemctl"))
            .env("CORTEX_UNIT_DIR", &self.units)
            .env("ANTHROPIC_API_KEY", "must-not-reach-claude")
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        let last = stdout.lines().last().unwrap_or("null");
        let value = serde_json::from_str(last).unwrap_or(Value::Null);
        assert!(
            out.status.code() != Some(2),
            "cortex {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        (out.status.code().unwrap_or(-1), value)
    }

    fn lines(&self, file: &str) -> Vec<String> {
        std::fs::read_to_string(self.root.join(file))
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }

    fn head(&self, name: &str) -> i64 {
        let dir = self.home.join("instances").join(name);
        let out = Command::new(ekr())
            .arg("head")
            .env("EKR_HOST", dir.join("host.json"))
            .env("EKR_BACKEND", "sqlite")
            .env("EKR_STORE", dir.join("store.sqlite"))
            .output()
            .unwrap();
        let head: Value = serde_json::from_slice(&out.stdout).unwrap();
        head["revision"].as_i64().unwrap()
    }
}

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
        w.root.join("pages.json"),
        PAGES.replace(
            "also develops the Gadget runtime",
            "now develops the Gizmo runtime",
        ),
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
