//! The end-to-end world every binary-level test shares: a real `ekr`, and stand-in `connectors`,
//! `claude` and `systemctl` written at runtime. A missing `ekr` fails the test; it never skips.
//!
//! Each test file that drives the binary declares `mod common;` and builds a [`World`].

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

pub fn ekr() -> PathBuf {
    let bin = std::env::var_os("CORTEX_TEST_EKR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").expect("HOME"))
                .join(".cache/cortex/bin/0.0.34/bin/ekr")
        });
    assert!(
        bin.is_file(),
        "the end-to-end test needs ekr 0.0.34 at {} (set CORTEX_TEST_EKR)",
        bin.display()
    );
    bin
}

/// Lines that make a `/bin/sh` stand-in `ekr` serve `ekr session` itself: each request line
/// `{"argv":[…]}` runs the stand-in again as the one-shot `ekr <argv>`, its standard input closed,
/// and is answered `{"exit":…,"stdout":…,"stderr":…}`, as the real `ekr session` answers. A
/// stand-in that logs or fails verbs by its arguments then sees each verb a run sends its session as
/// one process of its own. Put them where the stand-in should see `session` itself: first, to log
/// only the verbs, or after its own log line, to log the session too. An argument holding `"` or a
/// line break is not supported; the stdout of a verb answered through it must be JSON.
pub const EKR_SESSION_SHIM: &str = r#"if [ "$1" = session ]; then
  E=$(mktemp "${0%/*}/.ekr-session-stderr.XXXXXX") || exit 1
  while IFS= read -r REQ; do
    L=$(printf '%s' "$REQ" | sed -e 's/^{"argv":\["//' -e 's/"\]}$//' -e 's/","/\n/g')
    set --
    while IFS= read -r A; do set -- "$@" "$A"; done <<EOF_ARGV
$L
EOF_ARGV
    O=$("$0" "$@" < /dev/null 2> "$E"); X=$?
    O=$(printf '%s' "$O" | tr '\n' ' ')
    case "$O" in *[![:space:]]*) ;; *) O=null ;; esac
    S=$(sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' -e 's/\t/\\t/g' "$E" | sed -e ':a' -e 'N' -e '$!ba' -e 's/\n/\\n/g')
    printf '{"exit":%s,"stdout":%s,"stderr":"%s"}\n' "$X" "$O" "$S"
  done
  rm -f "$E"
  exit 0
fi
"#;

/// Writes a `/bin/sh` stand-in at `path` and makes it executable.
///
/// A child shell writes the file, never this process: a descriptor open for writing here would be
/// copied into the child of every spawn another test thread makes meanwhile, and running the file
/// fails with `Text file busy` until that child execs. Writing to a temporary name and renaming does
/// not help; the renamed file is the same inode, still open for writing.
pub fn executable(path: &Path, body: &str) {
    let status = Command::new("/bin/sh")
        .arg("-c")
        .arg(r#"printf '%s' "$2" > "$1" && chmod 755 "$1""#)
        .arg("sh")
        .arg(path)
        .arg(format!("#!/bin/sh\n{body}"))
        .status()
        .unwrap();
    assert!(status.success(), "cannot write {}", path.display());
}

/// The connector and instance a stand-in's `connections list` entry names beside its adapter, as
/// Connectors 0.60.0 lists them; `invoke` is called on `<connector>/<instance>`.
pub const LISTED: &str = r#""connector":"tavily","instance":"main""#;

/// Connectors 0.60.0's `InvokeResult` envelope around an adapter answer: `result.result` is the
/// answer `pages`, a JSON text, as the JSON value it holds.
pub fn answer(pages: &str) -> String {
    let pages: Value = serde_json::from_str(pages).expect("the pages are JSON");
    serde_json::json!({"ok": true, "result": {"connector": "tavily", "instance": "main",
        "operation": "websearch.search", "revision": "r", "result": pages}})
    .to_string()
}

pub struct World {
    _tmp: tempfile::TempDir,
    pub root: PathBuf,
    pub home: PathBuf,
    pub units: PathBuf,
    pub bin: PathBuf,
}

pub const PAGES: &str = r#"{"results":[
 {"url":"https://example.org/a","title":"A","description":"Example Labs ships Widget.","content":"Example Labs develops the Widget engine. Its website is example.org.","content_truncated":false,"published":null,"score":"0.9"},
 {"url":"https://example.org/b","title":"B","description":"Gadget news.","content":"Example Labs also develops the Gadget runtime.","content_truncated":false,"published":null,"score":"0.5"}
],"complete":false,"truncation":["provider_limit"],"provenance":{"instance":"t","profile":"tavily/2026-10","received_at":"2026-10-05T00:00:00.000Z"}}"#;

impl World {
    pub fn new() -> Self {
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
        std::fs::write(root.join("answer.json"), answer(PAGES)).unwrap();
        // connectors (0.60.0): one tavily connection of `tavily/main`, `conn_test` or the id in the
        // file `connection`, pending until revalidated; `invoke` logs its argv to invocations.log
        // and its standard input to inputs.log, and answers answer.json, or fails when the file
        // `fail` exists. Any other command line, `operations …` included, is refused with exit 2:
        // 0.60.0 removed the `operations` invoke, and cortex calls no other `operations` command.
        executable(
            &bin.join("connectors"),
            &format!(
                r#"R="{root}"
case "$*" in
  *"connections list"*) if [ -e "$R/revalidated" ]; then S=ready; else S=pending; fi; C=conn_test; if [ -e "$R/connection" ]; then C=$(cat "$R/connection"); fi; printf '{{"ok":true,"result":{{"connections":[{{"adapter":"tavily",{listed},"connection":"%s","state":"%s","revision":"rev1"}}]}}}}' "$C" "$S" ;;
  *"connections revalidate"*)
    echo "$*" >> "$R/revalidations.log"; touch "$R/revalidated"
    if [ -e "$R/unknown" ]; then echo '{{"ok":false,"error":{{"code":"failure","data":{{"code":"outcome_unknown","kind":"operational","next_action":"retry_status","stage":"publication"}}}}}}' >&2; exit 1; fi
    echo '{{"ok":true,"result":{{}}}}' ;;
  *"connections status"*) echo "$*" >> "$R/status.log"; if [ -e "$R/revalidated" ] && [ ! -e "$R/stays-pending" ]; then S=ready; else S=pending; fi; printf '{{"ok":true,"result":{{"connection":{{"summary":{{"state":"%s"}}}}}}}}' "$S" ;;
  "invoke tavily/main "*)
    echo "$*" >> "$R/invocations.log"; cat >> "$R/inputs.log"; echo >> "$R/inputs.log"
    if [ -e "$R/lapse" ]; then rm "$R/lapse"; echo '{{"ok":false,"error":{{"code":"failure","data":{{"code":"not_granted","stage":"admission"}}}}}}' >&2; exit 1; fi
    if [ -e "$R/fail" ]; then echo '{{"ok":false,"error":{{"code":"failure","data":{{"code":"unavailable","stage":"execution"}}}}}}'; exit 1; fi
    cat "$R/answer.json" ;;
  *) echo '{{"ok":false}}'; exit 2 ;;
esac
"#,
                root = root.display(),
                listed = LISTED,
            ),
        );
        // claude: records its arguments and whether ANTHROPIC_API_KEY reached it, then answers
        // one fact per evidence id it was given and one fact citing a forged id. Its answer carries
        // `total_cost_usd` 0.01, or no cost at all when the file `no-cost` exists.
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
COST='"total_cost_usd":0.01,'
if [ -e "$R/no-cost" ]; then COST=''; fi
cat <<EOF
{{"type":"result","subtype":"success","is_error":false,$COST"structured_output":{{"format":"ekr.extraction-document/1","ontology":{{"node_types":[{{"name":"Organization","parents":[],"abstract_type":false,"properties":[{{"name":"website","value":{{"value_kind":"String"}},"cardinality":"One","required":false}}]}},{{"name":"Product","parents":[],"abstract_type":false,"properties":[]}}],"edge_types":[{{"name":"DEVELOPS","source_types":["Organization"],"target_types":["Product"],"cardinality":"Many","properties":[]}}]}},"entities":[{{"node_type":"Organization","aliases":["Example Labs"]}},{{"node_type":"Product","aliases":["Widget engine"]}}],"facts":[$facts]}}}}
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

    pub fn spec(&self, name: &str, connection: &str) -> PathBuf {
        let path = self.root.join(format!("{name}.yaml"));
        std::fs::write(
            &path,
            format!(
                r#"format: cortex.instance/1
name: {name}
description: Test brain.
ekr: {{version: "0.0.34", bin: "{ekr}"}}
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

    pub fn cortex(&self, args: &[&str]) -> (i32, Value) {
        self.cortex_env(args, &[])
    }

    /// `cortex` with `env` set on top of the world's own variables.
    pub fn cortex_env(&self, args: &[&str], env: &[(&str, &Path)]) -> (i32, Value) {
        let out = Command::new(env!("CARGO_BIN_EXE_cortex"))
            .args(args)
            .envs(env.iter().copied())
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

    pub fn lines(&self, file: &str) -> Vec<String> {
        std::fs::read_to_string(self.root.join(file))
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }

    pub fn head(&self, name: &str) -> i64 {
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
