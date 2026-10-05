//! A `connectors` source reads every page of a paged operation, calls a child operation once per
//! record, and asks only for what changed since its last successful run
//! (`story:connectors-source-walks`).

mod common;

use std::path::PathBuf;

use common::{ekr, executable, World};
use cortex_cli::instance::now_ms;
use cortex_cli::sources::rfc3339;
use serde_json::Value;

const DAY_MS: i64 = 86_400_000;

/// A stand-in `connectors` with one ready connection and two operations. `issues.list` serves
/// three pages by token (`cursor`): no cursor answers I-1 and I-2 with `next` t2, t2 answers I-3
/// and I-4 with `next` t3, t3 answers I-5 and I-6 with no `next`. `comments.list` serves two pages
/// by token (`after`) for the `issue` it is given: the first comment with `next_after` c2, then
/// the second with none. Every invocation's arguments go to `issues.log` or `comments.log`.
const CONNECTORS: &str = r#"R="@ROOT@"
issue() {
  printf '{"key":"%s","summary":"Issue %s"}' "$1" "$1"
}
case "$*" in
  *"connections list"*) echo '{"ok":true,"result":{"connections":[{"adapter":"tracker","connection":"conn_test","state":"ready","revision":"rev1"}]}}' ;;
  *"connections revalidate"*) echo '{"ok":true,"result":{}}' ;;
  *"operations describe"*) echo '{"ok":true,"result":{"schema":"s","revision":"r"}}' ;;
  *"--operation issues.list"*)
    echo "$*" >> "$R/issues.log"
    case "$*" in
      *'"cursor":"t3"'*) PAGE="$(issue I-5),$(issue I-6)"; NEXT='' ;;
      *'"cursor":"t2"'*) PAGE="$(issue I-3),$(issue I-4)"; NEXT=',"next":"t3"' ;;
      *) PAGE="$(issue I-1),$(issue I-2)"; NEXT=',"next":"t2"' ;;
    esac
    printf '{"ok":true,"result":{"adapter":"tracker","operation":"issues.list","revision":"r","result":{"issues":[%s]%s}}}\n' "$PAGE" "$NEXT" ;;
  *"--operation comments.list"*)
    echo "$*" >> "$R/comments.log"
    I=$(printf '%s' "$*" | sed -n 's/.*"issue":"\([^"]*\)".*/\1/p')
    case "$*" in
      *'"after":"c2"'*) BODY="second note on $I"; NEXT='' ;;
      *) BODY="first note on $I"; NEXT=',"next_after":"c2"' ;;
    esac
    printf '{"ok":true,"result":{"adapter":"tracker","operation":"comments.list","revision":"r","result":{"comments":[{"body":"%s"}]%s}}}\n' "$BODY" "$NEXT" ;;
  *) echo '{"ok":false}'; exit 2 ;;
esac
"#;

fn world() -> World {
    let w = World::new();
    executable(
        &w.bin.join("connectors"),
        &CONNECTORS.replace("@ROOT@", &w.root.display().to_string()),
    );
    w
}

/// The connection part of a stand-in `connectors` whose operations are given separately.
const HEAD: &str = r#"R="@ROOT@"
case "$*" in
  *"connections list"*) echo '{"ok":true,"result":{"connections":[{"adapter":"tracker","connection":"conn_test","state":"ready","revision":"rev1"}]}}' ;;
  *"connections revalidate"*) echo '{"ok":true,"result":{}}' ;;
  *"operations describe"*) echo '{"ok":true,"result":{"schema":"s","revision":"r"}}' ;;
"#;

const TAIL: &str = r#"  *) echo '{"ok":false}'; exit 2 ;;
esac
"#;

/// A provider that answers `issues.list` from `db` (lines `stamp<TAB>key<TAB>summary`), sorted by
/// stamp then key, keeping the lines with `updated_since <= stamp < updated_until`, in pages of
/// `pagesize` lines (all of them when absent) by `page` number. When the file
/// `move-after-first-page` exists, serving page 1 updates I-1 (new stamp, " moved" summary), as a
/// write landing between two page requests does.
const FILTERING: &str = r#"  *"--operation issues.list"*)
    echo "$*" >> "$R/issues.log"
    SINCE=$(printf '%s' "$*" | sed -n 's/.*"updated_since":"\([^"]*\)".*/\1/p')
    UNTIL=$(printf '%s' "$*" | sed -n 's/.*"updated_until":"\([^"]*\)".*/\1/p')
    PAGE=$(printf '%s' "$*" | sed -n 's/.*"page":\([0-9]*\).*/\1/p'); [ -n "$PAGE" ] || PAGE=1
    SIZE=$(cat "$R/pagesize" 2>/dev/null || echo 1000)
    FIRST=$(( (PAGE - 1) * SIZE + 1 )); LAST=$(( PAGE * SIZE ))
    ITEMS=$(sort "$R/db" | awk -F'\t' -v s="$SINCE" -v u="$UNTIL" -v f="$FIRST" -v l="$LAST" '
      ($1 "" >= s "") && (u == "" || $1 "" < u "") { n++; if (n >= f && n <= l) { printf "%s{\"key\":\"%s\",\"summary\":\"%s\"}", (c++ ? "," : ""), $2, $3 } }')
    if [ -e "$R/move-after-first-page" ] && [ "$PAGE" = 1 ]; then
      rm "$R/move-after-first-page"
      NOW=$(date -u +%Y-%m-%dT%H:%M:%SZ)
      awk -F'\t' -v OFS='\t' -v t="$NOW" '$2 == "I-1" { $1 = t; $3 = $3 " moved" } { print }' "$R/db" > "$R/db.new"
      mv "$R/db.new" "$R/db"
    fi
    printf '{"ok":true,"result":{"adapter":"tracker","operation":"issues.list","revision":"r","result":{"issues":[%s]}}}\n' "$ITEMS" ;;
"#;

/// `issues.list` answers I-1 alone; `comments.list` answers every line of `comments` as one
/// comment body, in one page.
const COMMENTS_FILE: &str = r#"  *"--operation issues.list"*)
    echo "$*" >> "$R/issues.log"
    printf '{"ok":true,"result":{"adapter":"tracker","operation":"issues.list","revision":"r","result":{"issues":[{"key":"I-1","summary":"Issue I-1"}]}}}\n' ;;
  *"--operation comments.list"*)
    echo "$*" >> "$R/comments.log"
    ITEMS=$(awk '{ printf "%s{\"body\":\"%s\"}", (NR > 1 ? "," : ""), $0 }' "$R/comments")
    printf '{"ok":true,"result":{"adapter":"tracker","operation":"comments.list","revision":"r","result":{"comments":[%s]}}}\n' "$ITEMS" ;;
"#;

/// `issues.list` by token `cursor`: no cursor answers I-1 and I-2 with `next` t2; t2 answers an
/// empty page that still names `next` t3; t3 answers I-5 and I-6 and no `next`.
const EMPTY_MIDDLE_PAGE: &str = r#"  *"--operation issues.list"*)
    echo "$*" >> "$R/issues.log"
    case "$*" in
      *'"cursor":"t3"'*) PAGE='{"key":"I-5","summary":"Issue I-5"},{"key":"I-6","summary":"Issue I-6"}'; NEXT='' ;;
      *'"cursor":"t2"'*) PAGE=''; NEXT=',"next":"t3"' ;;
      *) PAGE='{"key":"I-1","summary":"Issue I-1"},{"key":"I-2","summary":"Issue I-2"}'; NEXT=',"next":"t2"' ;;
    esac
    printf '{"ok":true,"result":{"adapter":"tracker","operation":"issues.list","revision":"r","result":{"issues":[%s]%s}}}\n' "$PAGE" "$NEXT" ;;
"#;

/// `issues.list` answers I-1 alone; `comments.list` by token `after` cycles: no token answers
/// "first" with c2, c2 answers "second" with c3, c3 answers "third" with c2 again.
const CHILD_CYCLE: &str = r#"  *"--operation issues.list"*)
    echo "$*" >> "$R/issues.log"
    printf '{"ok":true,"result":{"adapter":"tracker","operation":"issues.list","revision":"r","result":{"issues":[{"key":"I-1","summary":"Issue I-1"}]}}}\n' ;;
  *"--operation comments.list"*)
    echo "$*" >> "$R/comments.log"
    case "$*" in
      *'"after":"c3"'*) BODY=third; NEXT=c2 ;;
      *'"after":"c2"'*) BODY=second; NEXT=c3 ;;
      *) BODY=first; NEXT=c2 ;;
    esac
    printf '{"ok":true,"result":{"adapter":"tracker","operation":"comments.list","revision":"r","result":{"comments":[{"body":"%s note"}],"next_after":"%s"}}}\n' "$BODY" "$NEXT" ;;
"#;

/// `issues.list` answers I-1 to I-3; `comments.list` answers one comment, and fails for the issue
/// named in the file `fail-for` while it exists.
const CHILD_FAILS: &str = r#"  *"--operation issues.list"*)
    echo "$*" >> "$R/issues.log"
    printf '{"ok":true,"result":{"adapter":"tracker","operation":"issues.list","revision":"r","result":{"issues":[{"key":"I-1","summary":"Issue I-1"},{"key":"I-2","summary":"Issue I-2"},{"key":"I-3","summary":"Issue I-3"}]}}}\n' ;;
  *"--operation comments.list"*)
    echo "$*" >> "$R/comments.log"
    I=$(printf '%s' "$*" | sed -n 's/.*"issue":"\([^"]*\)".*/\1/p')
    if [ -e "$R/fail-for" ] && [ "$(cat "$R/fail-for")" = "$I" ]; then
      echo '{"ok":false,"error":{"code":"failure","data":{"code":"unavailable","stage":"execution"}}}'; exit 1
    fi
    printf '{"ok":true,"result":{"adapter":"tracker","operation":"comments.list","revision":"r","result":{"comments":[{"body":"note on %s"}]}}}\n' "$I" ;;
"#;

/// A world whose stand-in `connectors` answers `operations`.
fn world_with(operations: &str) -> World {
    let w = World::new();
    let body = format!("{HEAD}{operations}{TAIL}");
    executable(
        &w.bin.join("connectors"),
        &body.replace("@ROOT@", &w.root.display().to_string()),
    );
    w
}

const WINDOWED: &str = r#"inputs:
          - {updated_since: "{since}", updated_until: "{until}"}
        records: issues
        id: key
        text: ["{key}: {summary}"]"#;

/// Creates instance `name` with one `connectors` source, `tracker`, whose settings continue with
/// `rest` (the lines after `operation`, at eight spaces).
fn create_source(
    w: &World,
    name: &str,
    rest: &str,
    refresh_after_days: i64,
    max_chars: i64,
    max_documents: i64,
) {
    let path = w.root.join(format!("{name}.yaml"));
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
  - name: tracker
    schedule: daily
    settings:
      kind: connectors
      value:
        adapter: tracker
        connection: conn_test
        operation: issues.list
        {rest}
    policy: {{refresh_after_days: {refresh_after_days}, change: ContentHash, max_documents_per_run: {max_documents}, max_chars_per_document: {max_chars}}}
serve: {{view_port: 18996}}
"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
}

fn state_path(w: &World, name: &str) -> PathBuf {
    instance(w, name).join("state").join("tracker.json")
}

/// Moves every document's last application `days` back, as that many days passing would.
fn age(w: &World, name: &str, days: i64) {
    let mut seen = state(w, name);
    for doc in seen["documents"].as_object_mut().unwrap().values_mut() {
        let at = doc["applied_at"].as_i64().unwrap();
        doc["applied_at"] = Value::from(at - days * DAY_MS);
    }
    std::fs::write(
        state_path(w, name),
        serde_json::to_vec_pretty(&seen).unwrap(),
    )
    .unwrap();
}

fn write_db(w: &World, rows: &[(String, &str, &str)]) {
    let text: String = rows
        .iter()
        .map(|(stamp, key, summary)| format!("{stamp}\t{key}\t{summary}\n"))
        .collect();
    std::fs::write(w.root.join("db"), text).unwrap();
}

fn sleep_past_a_second() {
    std::thread::sleep(std::time::Duration::from_millis(1_100));
}

/// A spec with one paged `connectors` source, `tracker`, whose child call reads each issue's
/// comments, and the given policy numbers.
fn spec(w: &World, name: &str, refresh_after_days: i64, max_documents: i64) -> PathBuf {
    let path = w.root.join(format!("{name}.yaml"));
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
  - name: tracker
    schedule: daily
    settings:
      kind: connectors
      value:
        adapter: tracker
        connection: conn_test
        operation: issues.list
        inputs:
          - {{updated_since: "{{since}}", updated_until: "{{until}}"}}
        records: issues
        id: key
        text: ["{{key}}: {{summary}}"]
        paging: {{style: Token, param: cursor, next: next, max_pages: 10}}
        child:
          operation: comments.list
          input: {{issue: "{{key}}"}}
          records: comments
          paging: {{style: Token, param: after, next: next_after, max_pages: 10}}
    policy: {{refresh_after_days: {refresh_after_days}, change: ContentHash, max_documents_per_run: {max_documents}, max_chars_per_document: 5000}}
serve: {{view_port: 18997}}
"#,
            ekr = ekr().display()
        ),
    )
    .unwrap();
    path
}

fn create(w: &World, name: &str, refresh_after_days: i64, max_documents: i64) {
    let path = spec(w, name, refresh_after_days, max_documents);
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
}

/// Creates instance `name` whose parent walk reads at most `parent_pages` pages and whose child
/// walks read at most `child_pages`.
fn create_capped(w: &World, name: &str, parent_pages: i64, child_pages: i64) {
    let path = spec(w, name, 3, 50);
    let text = std::fs::read_to_string(&path)
        .unwrap()
        .replace(
            "param: cursor, next: next, max_pages: 10",
            &format!("param: cursor, next: next, max_pages: {parent_pages}"),
        )
        .replace(
            "param: after, next: next_after, max_pages: 10",
            &format!("param: after, next: next_after, max_pages: {child_pages}"),
        );
    std::fs::write(&path, text).unwrap();
    let (code, created) = w.cortex(&["create", "--spec", path.to_str().unwrap(), "--no-units"]);
    assert_eq!(code, 0, "{created}");
}

fn last_log_line(w: &World, name: &str) -> Value {
    let text = std::fs::read_to_string(instance(w, name).join("cortex.log")).unwrap();
    serde_json::from_str(text.lines().last().expect("a log line")).unwrap()
}

fn run(w: &World, name: &str) -> Value {
    let (code, ran) = w.cortex(&["run", &format!("{name}/tracker")]);
    assert_eq!((code, ran["outcome"].as_str()), (0, Some("ran")), "{ran}");
    ran
}

fn instance(w: &World, name: &str) -> PathBuf {
    w.home.join("instances").join(name)
}

/// Every document text the runs of instance `name` showed the model.
fn prompted(w: &World, name: &str) -> Vec<String> {
    let mut docs = Vec::new();
    let Ok(runs) = std::fs::read_dir(instance(w, name).join("runs")) else {
        return docs;
    };
    for run in runs {
        for batch in std::fs::read_dir(run.unwrap().path()).unwrap() {
            let Ok(prompt) = std::fs::read_to_string(batch.unwrap().path().join("prompt.txt"))
            else {
                continue;
            };
            docs.extend(
                prompt
                    .split("\n=== Document — evidence id ")
                    .skip(1)
                    .map(str::to_string),
            );
        }
    }
    docs
}

/// The JSON value of `"<field>":"<value>"` in one logged invocation.
fn field(line: &str, name: &str) -> String {
    let marker = format!("\"{name}\":\"");
    let start = line
        .find(&marker)
        .unwrap_or_else(|| panic!("{name} in {line}"))
        + marker.len();
    let end = start + line[start..].find('"').unwrap();
    line[start..end].to_string()
}

fn state(w: &World, name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(state_path(w, name)).unwrap()).unwrap()
}

/// The overlap a run's `{since}` reaches back before the last successful run's start.
const OVERLAP_MS: i64 = 5 * 60 * 1000;

#[test]
fn three_token_pages_of_two_parents_with_two_paged_children_apply_six_documents() {
    let w = world();
    create(&w, "walks", 3, 50);
    let ran = run(&w, "walks");
    assert_eq!(ran["detail"]["documents_new"], 6, "{ran}");
    assert_eq!(ran["detail"]["documents_applied"], 6, "{ran}");

    // Three pages, then the walk stopped at the missing `next`: no fourth call.
    let pages = w.lines("issues.log");
    assert_eq!(pages.len(), 3, "{pages:#?}");
    assert!(!pages[0].contains("\"cursor\""), "{}", pages[0]);
    assert!(pages[1].contains("\"cursor\":\"t2\""), "{}", pages[1]);
    assert!(pages[2].contains("\"cursor\":\"t3\""), "{}", pages[2]);
    // Two child pages per parent.
    assert_eq!(w.lines("comments.log").len(), 12);

    let docs = prompted(&w, "walks");
    assert_eq!(docs.len(), 6, "{docs:#?}");
    for n in 1..=6 {
        let key = format!("I-{n}");
        let doc = docs
            .iter()
            .find(|d| d.contains(&format!("Source: record:tracker:issues.list:{key}\n")))
            .unwrap_or_else(|| panic!("no document for {key} in {docs:#?}"));
        assert!(doc.contains(&format!("{key}: Issue {key}")), "{doc}");
        assert!(doc.contains(&format!("first note on {key}\"")), "{doc}");
        assert!(doc.contains(&format!("second note on {key}\"")), "{doc}");
        assert_eq!(doc.matches("note on ").count(), 2, "{doc}");
    }
    let seen = state(&w, "walks");
    assert_eq!(seen["documents"].as_object().unwrap().len(), 6, "{seen}");
}

#[test]
fn the_window_starts_refresh_after_days_back_then_at_the_last_successful_run() {
    let w = world();
    create(&w, "window", 3, 50);
    let before = now_ms();
    run(&w, "window");
    let after = now_ms();

    let first = w.lines("issues.log")[0].clone();
    let started = state(&w, "window")["last_success_started_at"]
        .as_i64()
        .unwrap_or_else(|| panic!("no run start in {}", state(&w, "window")));
    assert!(
        (before..=after).contains(&started),
        "{before} <= {started} <= {after}"
    );
    assert_eq!(
        field(&first, "updated_since"),
        rfc3339(started - 3 * DAY_MS)
    );
    assert_eq!(field(&first, "updated_until"), rfc3339(started));

    std::fs::remove_file(w.root.join("issues.log")).unwrap();
    let ran = run(&w, "window");
    assert_eq!(ran["detail"]["documents_new"], 0, "{ran}");
    let second = w.lines("issues.log")[0].clone();
    assert_eq!(
        field(&second, "updated_since"),
        rfc3339(started - OVERLAP_MS)
    );
    let restarted = state(&w, "window")["last_success_started_at"]
        .as_i64()
        .unwrap();
    assert!(restarted > started, "{restarted} > {started}");
    assert_eq!(field(&second, "updated_until"), rfc3339(restarted));
}

#[test]
fn a_run_that_leaves_selected_documents_behind_does_not_move_the_window() {
    let w = world();
    create(&w, "capped", 3, 4);
    let ran = run(&w, "capped");
    assert_eq!(ran["detail"]["documents_applied"], 4, "{ran}");
    let seen = state(&w, "capped");
    assert!(seen.get("last_success_started_at").is_none(), "{seen}");
}

#[test]
fn an_instant_is_written_in_rfc_3339_to_the_second() {
    assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
    assert_eq!(rfc3339(1_759_622_400_999), "2025-10-05T00:00:00Z");
    assert_eq!(rfc3339(951_825_599_000), "2000-02-29T11:59:59Z");
    assert_eq!(rfc3339(-1_000), "1969-12-31T23:59:59Z");
}

#[test]
fn a_walk_cut_short_by_max_pages_says_so_and_does_not_move_the_window() {
    let w = world();
    create_capped(&w, "capped-pages", 2, 10);
    let ran = run(&w, "capped-pages");
    assert_eq!(w.lines("issues.log").len(), 2);
    assert_eq!(ran["detail"]["documents_applied"], 4, "{ran}");
    let stopped = ran["detail"]["stopped"].as_str().unwrap_or_default();
    assert!(stopped.contains("max_pages"), "{ran}");
    let logged = last_log_line(&w, "capped-pages");
    assert_eq!(logged["stopped"].as_str(), Some(stopped), "{logged}");
    let seen = state(&w, "capped-pages");
    assert!(seen.get("last_success_started_at").is_none(), "{seen}");

    // The next run's window still reaches back over the first run's.
    let until = field(&w.lines("issues.log")[0], "updated_until");
    std::fs::remove_file(w.root.join("issues.log")).unwrap();
    run(&w, "capped-pages");
    let since = field(&w.lines("issues.log")[0], "updated_since");
    assert!(since < until, "{since} < {until}");
}

#[test]
fn a_child_walk_cut_short_by_max_pages_does_not_move_the_window() {
    let w = world();
    create_capped(&w, "capped-children", 10, 1);
    let ran = run(&w, "capped-children");
    assert_eq!(w.lines("comments.log").len(), 6);
    assert_eq!(ran["detail"]["documents_applied"], 6, "{ran}");
    let stopped = ran["detail"]["stopped"].as_str().unwrap_or_default();
    assert!(stopped.contains("max_pages"), "{ran}");
    let seen = state(&w, "capped-children");
    assert!(seen.get("last_success_started_at").is_none(), "{seen}");
}

/// A record that changes within `refresh_after_days` of its last application is held back by the
/// refresh rule; the source remembers since when (`held_since`), so once the refresh time has
/// passed a run still asks for it and the change reaches the model.
#[test]
fn a_change_held_back_by_refresh_after_days_is_read_once_the_refresh_time_has_passed() {
    let w = world_with(FILTERING);
    let yesterday = rfc3339(now_ms() - DAY_MS);
    write_db(
        &w,
        &[
            (yesterday.clone(), "I-1", "v1"),
            (yesterday.clone(), "I-2", "v1"),
        ],
    );
    create_source(&w, "held", WINDOWED, 3, 5000, 50);
    let ran = run(&w, "held");
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
    assert!(state(&w, "held").get("held_since").is_none());

    // I-1 changes after the first run.
    sleep_past_a_second();
    write_db(
        &w,
        &[
            (rfc3339(now_ms()), "I-1", "v2"),
            (yesterday.clone(), "I-2", "v1"),
        ],
    );
    sleep_past_a_second();
    // The next scheduled run sees the change, inside its refresh time.
    run(&w, "held");
    assert!(
        state(&w, "held").get("held_since").is_some(),
        "{}",
        state(&w, "held")
    );
    // The refresh time passes; the next run is due.
    age(&w, "held", 4);
    run(&w, "held");

    let docs = prompted(&w, "held");
    assert!(
        docs.iter().any(|d| d.contains("I-1: v2")),
        "the change to I-1 never reached the model; state {}; asked {:#?}",
        state(&w, "held"),
        w.lines("issues.log")
    );
    // Applied: nothing is held any more.
    assert!(
        state(&w, "held").get("held_since").is_none(),
        "{}",
        state(&w, "held")
    );
}

/// The hash is taken of a document's whole text, children included, before the
/// `max_chars_per_document` cut: a new comment on an issue already past the cut changes it, and
/// the run counts the cut document as `truncated`.
#[test]
fn a_new_child_past_max_chars_per_document_changes_the_document() {
    let w = world_with(COMMENTS_FILE);
    let long = "the build on the release branch was slow again and somebody should look at it";
    std::fs::write(
        w.root.join("comments"),
        format!("{long} one\n{long} two\n{long} three\n{long} four\n"),
    )
    .unwrap();
    let rest = r#"inputs: [{}]
        records: issues
        id: key
        text: ["{key}: {summary}"]
        child: {operation: comments.list, input: {issue: "{key}"}, records: comments}"#;
    create_source(&w, "long", rest, 0, 300, 50);
    let ran = run(&w, "long");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    assert_eq!(last_log_line(&w, "long")["truncated"], 1);
    // The first run's text is already past the cut: its fourth comment never reached the model.
    assert!(!prompted(&w, "long")[0].contains("four\""));

    let mut comments = std::fs::read_to_string(w.root.join("comments")).unwrap();
    comments.push_str("NEWEST comment says the fix is merged\n");
    std::fs::write(w.root.join("comments"), comments).unwrap();
    let ran = run(&w, "long");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
}

/// A walk that stops at an empty page while the provider still names a next page has not read
/// everything, as a walk cut by `max_pages` has not: it says so in `stopped` and leaves the window
/// where it was, so the records past the empty page are asked for again.
#[test]
fn a_walk_stopped_by_an_empty_page_with_a_next_token_says_so_and_keeps_the_window() {
    let w = world_with(EMPTY_MIDDLE_PAGE);
    let rest = r#"inputs:
          - {updated_since: "{since}"}
        records: issues
        id: key
        text: ["{key}: {summary}"]
        paging: {style: Token, param: cursor, next: next, max_pages: 10}"#;
    create_source(&w, "gap", rest, 3, 5000, 50);
    let ran = run(&w, "gap");

    let stopped = ran["detail"]["stopped"].as_str().unwrap_or_default();
    assert!(stopped.contains("empty page"), "{ran}");
    let seen = state(&w, "gap");
    assert!(
        seen.get("last_success_started_at").is_none(),
        "I-5 and I-6 were not read and the window moved past them: {seen}; asked {:#?}",
        w.lines("issues.log")
    );
}

/// A `PageNumber` walk over a window whose result set changes between two page requests shifts a
/// record onto a page already read, and the walk skips it. The next run reads it again only when
/// its stamp lies inside the overlap its `{since}` reaches back before the last run's start: a
/// skipped record stamped earlier is never read again. That is a limit of offset paging over a
/// result set that changes during the walk, which no window can undo; `Token` or `Keyset` paging,
/// or a sort that writes do not reorder, avoids it.
#[test]
fn a_record_stamped_inside_the_overlap_and_skipped_by_a_page_number_walk_is_read_next_run() {
    let w = world_with(FILTERING);
    // A minute before the first run: inside the overlap of the next one.
    let recent = rfc3339(now_ms() - 60_000);
    let rows: Vec<(String, &str, &str)> = ["I-1", "I-2", "I-3", "I-4", "I-5", "I-6"]
        .into_iter()
        .map(|k| (recent.clone(), k, "issue"))
        .collect();
    write_db(&w, &rows);
    std::fs::write(w.root.join("pagesize"), "2").unwrap();
    std::fs::write(w.root.join("move-after-first-page"), "").unwrap();
    let rest =
        format!("{WINDOWED}\n        paging: {{style: PageNumber, param: page, max_pages: 10}}");
    create_source(&w, "shift", &rest, 3, 5000, 50);
    run(&w, "shift");
    assert!(
        !prompted(&w, "shift")
            .iter()
            .any(|d| d.contains("I-3: issue")),
        "the first run read I-3: the walk skipped nothing"
    );
    sleep_past_a_second();
    run(&w, "shift");

    let docs = prompted(&w, "shift");
    assert!(
        docs.iter().any(|d| d.contains("I-3: issue")),
        "I-3 never reached the model; asked {:#?}",
        w.lines("issues.log")
    );
}

/// A record whose stamp is before a run's `{until}` but that the provider shows only after that
/// run asked (a provider clock behind the host's, or an index that lags its writes) is read by the
/// next run, whose `{since}` reaches back an overlap before the last run's start.
#[test]
fn a_record_stamped_before_until_but_visible_after_the_run_is_read() {
    let w = world_with(FILTERING);
    let yesterday = rfc3339(now_ms() - DAY_MS);
    write_db(&w, &[(yesterday.clone(), "I-1", "issue")]);
    create_source(&w, "skew", WINDOWED, 3, 5000, 50);
    run(&w, "skew");
    let started = state(&w, "skew")["last_success_started_at"]
        .as_i64()
        .expect("the first run moved the window");

    // Written five seconds before the first run started, by the provider's clock.
    write_db(
        &w,
        &[
            (yesterday.clone(), "I-1", "issue"),
            (rfc3339(started - 5_000), "I-2", "late"),
        ],
    );
    sleep_past_a_second();
    run(&w, "skew");

    let docs = prompted(&w, "skew");
    assert!(
        docs.iter().any(|d| d.contains("I-2: late")),
        "I-2 never reached the model; asked {:#?}",
        w.lines("issues.log")
    );
}

/// A child walk whose tokens cycle through two values stops at the first token it has already
/// given: each child record lands in the parent's text once.
#[test]
fn a_two_token_cycle_in_a_child_walk_puts_each_child_record_in_the_parent_once() {
    let w = world_with(CHILD_CYCLE);
    let rest = r#"inputs: [{}]
        records: issues
        id: key
        text: ["{key}: {summary}"]
        child:
          operation: comments.list
          input: {issue: "{key}"}
          records: comments
          paging: {style: Token, param: after, next: next_after, max_pages: 10}"#;
    create_source(&w, "cycle", rest, 3, 5000, 50);
    run(&w, "cycle");

    let docs = prompted(&w, "cycle");
    assert_eq!(docs.len(), 1, "{docs:#?}");
    assert_eq!(
        docs[0].matches("second note").count(),
        1,
        "{}; {} child calls",
        docs[0],
        w.lines("comments.log").len()
    );
}

/// A child call that fails for one parent drops that parent from the run and says so in
/// `stopped`; the other parents apply, the window does not move, and the retry applies the dropped
/// parent once.
#[test]
fn a_child_call_failing_for_one_parent_drops_it_and_the_retry_applies_it_once() {
    let w = world_with(CHILD_FAILS);
    std::fs::write(w.root.join("fail-for"), "I-2").unwrap();
    let rest = r#"inputs: [{}]
        records: issues
        id: key
        text: ["{key}: {summary}"]
        child: {operation: comments.list, input: {issue: "{key}"}, records: comments}"#;
    create_source(&w, "childfail", rest, 3, 5000, 50);
    let ran = run(&w, "childfail");
    assert_eq!(ran["detail"]["documents_applied"], 2, "{ran}");
    assert!(
        ran["detail"]["stopped"]
            .as_str()
            .unwrap_or_default()
            .contains("comments.list: child call failed for tracker:issues.list:I-2"),
        "{ran}"
    );
    assert!(
        state(&w, "childfail")
            .get("last_success_started_at")
            .is_none(),
        "{}",
        state(&w, "childfail")
    );

    std::fs::remove_file(w.root.join("fail-for")).unwrap();
    let ran = run(&w, "childfail");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    let ran = run(&w, "childfail");
    assert_eq!(ran["detail"]["documents_applied"], 0, "{ran}");
}

/// A state file written before `last_success_started_at` existed reads as one with no successful
/// run, so the window reaches back `refresh_after_days`.
#[test]
fn an_old_state_file_without_the_run_start_reaches_back_refresh_after_days() {
    let w = world_with(FILTERING);
    write_db(&w, &[(rfc3339(now_ms() - DAY_MS), "I-1", "issue")]);
    create_source(&w, "old", WINDOWED, 3, 5000, 50);
    std::fs::create_dir_all(state_path(&w, "old").parent().unwrap()).unwrap();
    std::fs::write(
        state_path(&w, "old"),
        r#"{"format":"cortex.seen/1","documents":{"tracker:issues.list:I-9":{"hash":"00","applied_at":1}}}"#,
    )
    .unwrap();
    let ran = run(&w, "old");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    let started = state(&w, "old")["last_success_started_at"]
        .as_i64()
        .unwrap();
    let asked = &w.lines("issues.log")[0];
    assert!(
        asked.contains(&format!(
            "\"updated_since\":\"{}\"",
            rfc3339(started - 3 * DAY_MS)
        )),
        "{asked}"
    );
    assert!(
        state(&w, "old")["documents"]
            .get("tracker:issues.list:I-9")
            .is_some(),
        "{}",
        state(&w, "old")
    );
}

/// One day passes for instance `name`: every instant its seen state holds moves a day back.
fn pass_a_day(w: &World, name: &str) {
    let mut seen = state(w, name);
    for doc in seen["documents"].as_object_mut().unwrap().values_mut() {
        let at = doc["applied_at"].as_i64().unwrap();
        doc["applied_at"] = Value::from(at - DAY_MS);
    }
    for key in ["last_success_started_at", "held_since", "pending_since"] {
        if let Some(at) = seen.get(key).and_then(Value::as_i64) {
            seen[key] = Value::from(at - DAY_MS);
        }
    }
    std::fs::write(
        state_path(w, name),
        serde_json::to_vec_pretty(&seen).unwrap(),
    )
    .unwrap();
}

/// The provider's records: `(stamp ms, key, summary)`.
fn write_rows(w: &World, rows: &[(i64, &str, String)]) {
    let text: String = rows
        .iter()
        .map(|(stamp, key, summary)| format!("{}\t{key}\t{summary}\n", rfc3339(*stamp)))
        .collect();
    std::fs::write(w.root.join("db"), text).unwrap();
}

/// Two records of an active source, each edited every day, applied on different days (I-1 on day
/// 0, I-2 first seen on day 1), with `refresh_after_days: 3`. Every day one of them is held back.
/// Each held change was made after its record was last applied, which is under 3 days ago, so a
/// window reaching back 4 days covers every one of them.
#[test]
fn held_since_does_not_grow_without_bound_while_records_keep_changing() {
    let w = world_with(FILTERING);
    let mut rows = vec![(now_ms() - 3_600_000, "I-1", "v0".to_string())];
    write_rows(&w, &rows);
    create_source(&w, "busy", WINDOWED, 3, 5000, 50);
    sleep_past_a_second();
    run(&w, "busy");

    let mut asked = Vec::new();
    for day in 1..=9 {
        pass_a_day(&w, "busy");
        for row in rows.iter_mut() {
            row.0 -= DAY_MS;
        }
        // Both records are edited today.
        let now = now_ms();
        if day == 1 {
            rows.push((now, "I-2", "v1".to_string()));
        }
        for row in rows.iter_mut() {
            row.0 = now;
            row.2 = format!("v{day}");
        }
        write_rows(&w, &rows);
        sleep_past_a_second();
        let before = now_ms();
        run(&w, "busy");
        let line = w.lines("issues.log").last().unwrap().clone();
        asked.push((day, before, field(&line, "updated_since")));
    }
    let (day, before, since) = asked.last().unwrap();
    assert!(
        *since >= rfc3339(before - 4 * DAY_MS),
        "day {day}: since {since} reaches back past {} (4 days); state {}; every run asked {asked:#?}",
        rfc3339(before - 4 * DAY_MS),
        state(&w, "busy")
    );
}

/// A child call that fails on every attempt for one parent holds the window for its first two
/// runs; from the third the parent is skipped, the window moves on, and after 5 days it reaches
/// back no more than 4 days on a source whose `refresh_after_days` is 3.
#[test]
fn a_child_failing_on_every_run_does_not_widen_the_window_without_bound() {
    let w = world_with(CHILD_FAILS);
    let rest = r#"inputs:
          - {updated_since: "{since}", updated_until: "{until}"}
        records: issues
        id: key
        text: ["{key}: {summary}"]
        child: {operation: comments.list, input: {issue: "{key}"}, records: comments}"#;
    create_source(&w, "stuck", rest, 3, 5000, 50);
    run(&w, "stuck");
    assert!(state(&w, "stuck").get("last_success_started_at").is_some());

    std::fs::write(w.root.join("fail-for"), "I-2").unwrap();
    let mut asked = Vec::new();
    for day in 1..=5 {
        pass_a_day(&w, "stuck");
        let before = now_ms();
        let ran = run(&w, "stuck");
        let line = w.lines("issues.log").last().unwrap().clone();
        asked.push((
            day,
            field(&line, "updated_since"),
            ran["detail"]["stopped"].clone(),
            ran["detail"]["skipped"].clone(),
        ));
        if day == 5 {
            assert!(
                field(&line, "updated_since") >= rfc3339(before - 4 * DAY_MS),
                "the window never moved: {asked:#?}; {} child calls in all",
                w.lines("comments.log").len()
            );
        }
    }
    // Two runs hold the window; the third and later skip the parent without stopping.
    let failed = "comments.list: child call failed for tracker:issues.list:I-2";
    for (day, _, stopped, skipped) in &asked {
        let stopped = stopped.as_str().unwrap_or_default();
        let skipped = skipped.to_string();
        if *day <= 2 {
            assert!(stopped.contains(failed), "day {day}: {asked:#?}");
        } else {
            assert!(!stopped.contains(failed), "day {day}: {asked:#?}");
            assert!(
                skipped.contains(&format!("{failed} on {} runs; skipped", day)),
                "day {day}: {asked:#?}"
            );
        }
    }
}

/// A child call that succeeds again resets its failure count: two failures, a success, and two
/// more failures still hold the window.
#[test]
fn a_successful_child_call_resets_its_failure_count() {
    let w = world_with(CHILD_FAILS);
    let rest = r#"inputs: [{}]
        records: issues
        id: key
        text: ["{key}: {summary}"]
        child: {operation: comments.list, input: {issue: "{key}"}, records: comments}"#;
    create_source(&w, "reset", rest, 3, 5000, 50);
    let failed = "comments.list: child call failed for tracker:issues.list:I-2";
    std::fs::write(w.root.join("fail-for"), "I-2").unwrap();
    for _ in 0..2 {
        let ran = run(&w, "reset");
        assert!(
            ran["detail"]["stopped"]
                .as_str()
                .unwrap_or_default()
                .contains(failed),
            "{ran}"
        );
    }
    std::fs::remove_file(w.root.join("fail-for")).unwrap();
    let ran = run(&w, "reset");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
    assert!(
        state(&w, "reset").get("child_failures").is_none(),
        "{}",
        state(&w, "reset")
    );
    std::fs::write(w.root.join("fail-for"), "I-2").unwrap();
    for _ in 0..2 {
        let ran = run(&w, "reset");
        assert!(
            ran["detail"]["stopped"]
                .as_str()
                .unwrap_or_default()
                .contains(failed),
            "{ran}"
        );
        assert!(ran["detail"].get("skipped").is_none(), "{ran}");
    }
}

/// `spec-file.md`: a run that left documents beyond `max_documents_per_run` keeps its `{since}`
/// for the next run, also before the source's first successful run, so the window does not slide
/// with the clock.
#[test]
fn a_first_run_left_short_by_max_documents_keeps_its_since_for_the_next_run() {
    let w = world_with(FILTERING);
    let stamp = now_ms() - DAY_MS;
    let rows: Vec<(i64, &str, String)> = ["I-1", "I-2", "I-3", "I-4", "I-5", "I-6"]
        .into_iter()
        .map(|k| (stamp, k, "issue".to_string()))
        .collect();
    write_rows(&w, &rows);
    create_source(&w, "backlog", WINDOWED, 3, 5000, 4);
    let ran = run(&w, "backlog");
    assert_eq!(ran["detail"]["documents_applied"], 4, "{ran}");
    let first = field(&w.lines("issues.log")[0], "updated_since");

    sleep_past_a_second();
    run(&w, "backlog");
    let second = field(w.lines("issues.log").last().unwrap(), "updated_since");
    assert_eq!(
        second,
        first,
        "the second run's since moved after the first's although the first left 2 documents; state {}",
        state(&w, "backlog")
    );
}

/// A first run that fails keeps its `{since}` for the next run as a stopped one does: the window
/// does not slide with the clock before the source's first successful run.
#[test]
fn a_failed_first_run_keeps_its_since_for_the_next_run() {
    let w = world_with(FILTERING);
    write_rows(&w, &[(now_ms() - DAY_MS, "I-1", "issue".to_string())]);
    let failing = WINDOWED.replace("records: issues", "records: missing");
    create_source(&w, "failing", &failing, 3, 5000, 50);
    let (code, ran) = w.cortex(&["run", "failing/tracker"]);
    assert_eq!(
        (code != 0, ran["outcome"].as_str()),
        (true, Some("fetch-failed")),
        "{ran}"
    );
    let first = field(&w.lines("issues.log")[0], "updated_since");
    sleep_past_a_second();
    w.cortex(&["run", "failing/tracker"]);
    let second = field(w.lines("issues.log").last().unwrap(), "updated_since");
    assert_eq!(second, first, "state {}", state(&w, "failing"));
}

/// `truncated` counts the documents the run cut for the model: a run that applies nothing reports
/// none.
#[test]
fn a_run_that_applies_nothing_reports_nothing_truncated() {
    let w = world_with(COMMENTS_FILE);
    let long = "the build on the release branch was slow again and somebody should look at it";
    std::fs::write(
        w.root.join("comments"),
        format!("{long} one\n{long} two\n{long} three\n{long} four\n"),
    )
    .unwrap();
    let rest = r#"inputs: [{}]
        records: issues
        id: key
        text: ["{key}: {summary}"]
        child: {operation: comments.list, input: {issue: "{key}"}, records: comments}"#;
    create_source(&w, "long-again", rest, 0, 300, 50);
    let ran = run(&w, "long-again");
    assert_eq!(ran["detail"]["truncated"], 1, "{ran}");

    let ran = run(&w, "long-again");
    assert_eq!(ran["detail"]["documents_applied"], 0, "{ran}");
    assert!(
        ran["detail"].get("truncated").is_none(),
        "a run that applied nothing reports a cut document: {ran}; log {}",
        last_log_line(&w, "long-again")
    );
}

/// A windowed source with `refresh_after_days: 0` asks its first run for `{since}` = `{until}`, an
/// empty window, so history older than the overlap is not read (`spec-file.md` says so); with
/// `refresh_after_days: 1` the first run reads the last day.
#[test]
fn a_first_run_reads_refresh_after_days_of_history_and_none_with_zero() {
    let w = world_with(FILTERING);
    write_rows(&w, &[(now_ms() - DAY_MS / 2, "I-1", "issue".to_string())]);
    create_source(&w, "zero", WINDOWED, 0, 5000, 50);
    let ran = run(&w, "zero");
    let first = w.lines("issues.log")[0].clone();
    assert_eq!(
        field(&first, "updated_since"),
        field(&first, "updated_until")
    );
    assert_eq!(ran["detail"]["documents_applied"], 0, "{ran}");

    let w = world_with(FILTERING);
    write_rows(&w, &[(now_ms() - DAY_MS / 2, "I-1", "issue".to_string())]);
    create_source(&w, "one", WINDOWED, 1, 5000, 50);
    let ran = run(&w, "one");
    assert_eq!(ran["detail"]["documents_applied"], 1, "{ran}");
}
