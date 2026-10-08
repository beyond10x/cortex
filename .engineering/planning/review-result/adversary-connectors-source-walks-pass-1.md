---
format: aep.planning-md/3
id: review-result:adversary-connectors-source-walks-pass-1
kind: review-result
status: archived
title: 'Adversary pass 1: connectors-source-walks (wave 20261005d)'
relations:
- reviews: story:connectors-source-walks
revision: 2
transitions:
- {from: "active", to: "archived", at: "2026-10-08T13:07:36Z", actor: "human:timo", revision: 2}
---
unit: U1 story:connectors-source-walks, working tree ~/.local/state/worktree/trees/b10x/cortex/cortex-w3-walks (uncommitted on base 90ae12b)
verdict: NEEDS-CHANGE
cases: executed 159→167, red 6
origin: introduced 8 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory (~/.cache/cortex-wave-20261005d/walks/adversary-1/), plus test binaries in the assigned ~/.cache/b10x-target/cortex-w3-walks
needs-coordinator: choose fixes for A (refresh rule vs window) and C (stop at an empty page): the story's text sets the current behaviour for both

**1. Diff stat**
```
 src/run.rs     |  81 ++++++++++--
 src/sources.rs | 397 ++++++++++++++++++++++++++++++++++++++++++++++++++++++---
 src/state.rs   |  26 +++-
```
This matches the stat before I started, so I changed no implementation file. My only file is the new untracked test file `tests/connectors_walks_adversary.rs`, headed "ADVERSARY CASES".

**2. Cases (each run alone and red before the suite ran; logs are `adversary-1/alone-*.log`)**

| # | case (line) | asserts | now | red output |
|---|---|---|---|---|
| A | :224 | a change to I-1, held back by `refresh_after_days`, reaches the model once that time has passed | red | `the change to I-1 never reached the model`; state still holds the v1 hash `da60efdf…` (= sha256 "I-1: v1"); run 3 asked `updated_since 13:32:23` |
| B | :267 | a new comment on an issue already longer than `max_chars_per_document` reaches the model | red | `second run {"documents_applied":0,"documents_new":0,…}` |
| C | :302 | a walk that stops at an empty page that still names `next` either reads on, or leaves the window where it was | red | `I-5 and I-6 were not read and the window moved past them: {…"last_success_started_at":1791207149433}` |
| D | :329 | a PageNumber walk where I-1 changes between page 1 and page 2 still reads I-3 within two runs | red | `I-3 never reached the model`; pages 1–4, then run 2 with `updated_since 13:32:24` |
| E | :358 | a record stamped 5 s before run 1's `{until}` but visible only afterwards is read | red | `I-2 never reached the model` |
| F | :391 | a child walk whose tokens cycle c2→c3→c2 puts "second note" once in the parent's text | red | `left: 5 right: 1 … 10 child calls` |
| G | :419 | a child call failing for one parent fails the run, the window does not move, and the retry applies 3 then 0 | green | — |
| H | :449 | an old state file without the new field reaches back `refresh_after_days` and keeps its documents | green | — |

My first version of B was red for a different reason than its doc comment claimed. The first run's text was 296 chars, under the 300 cut, so the second run did re-select the document, and the cut fell inside the new comment. I added a fourth comment so the first run is already past the cut, and the case now fails for the stated reason. The first log is `alone-B-first-attempt.log`.

**3. Suite (run after the cases existed)**
- `task check`: `EXIT=201`. Spec, drift, fmt and clippy passed. It stops at `connectors_walks_adversary`, `test result: FAILED. 2 passed; 6 failed`.
- `cargo test --locked --workspace --no-fail-fast`: `EXIT=101`, 161 passed, 6 failed, 167 executed. Every other binary has the same counts as gate-2.log.

**4. Findings**

| # | file:line | verdict | what reaches it |
|---|---|---|---|
| A | src/run.rs:215 | NEEDS-CHANGE, blocker | A daily source with `refresh_after_days` > 1 (example.yaml uses 7 and 30; the story's own test uses 3) and a record edited again within that time. `wanted` leaves the held-back record out, so the window moves past its change, and no later run asks for it. Before this change every run read everything, so the change was picked up later. |
| B | src/sources.rs:480 | CONFIRMED, warning | Any issue whose comments push its text past `max_chars_per_document`. Children are added at the end and cut off without notice, and the hash is taken after the cut, so a new comment never reaches the model. |
| C | src/sources.rs:195 | INFEASIBLE, warning | It needs a provider that returns an empty page with a next token. The story says to stop at an empty page, but such a stop counts as a full read and moves the window. Nothing in this repo shows a provider doing this. |
| D | src/sources.rs:69 | INFEASIBLE, warning | It needs a PageNumber walk sorted by `updated` ascending while the result set changes during the walk. The skipped record is older than the next `{since}`, so the miss is permanent. A provider that does this is not shown in this repo. |
| E | src/sources.rs:69 | INFEASIBLE, warning | It needs the provider's clock to run behind the host's, or an index that shows a write late. The windows meet exactly with no overlap, and the seen-state hash would make an overlap cheap. Not shown in this repo. |
| F | src/sources.rs:171 | INFEASIBLE, note | It needs a provider with a token cycle. The guard catches only a token that repeats the one it was given; parents are deduplicated by key, children are not. |
| J1 | src/sources.rs:480 | CONFIRMED, note (judgement, no case) | `ChildCall` has no `text` template, so the whole child JSON goes into the parent text. Any field that changes on every call (durations, signed URLs) changes the hash, so the parent is extracted again after every refresh time. |
| J2 | src/sources.rs:476 | CONFIRMED, note (judgement, no case) | The child call's `?` makes one parent whose child call keeps failing (for example, no access to one issue) fail the whole source on every run (G shows this). Nothing is skipped or applied twice, but the source stops applying anything. |

All eight are `introduced`. Window, walk, child and `last_success` all count 0 in `git show 90ae12b:src/{sources,run,state}.rs`.

Fixes I would name, not applied:
- **A:** do not move the window while a changed record is held back by `refresh_after_days`, or start `{since}` no later than the oldest held-back change.
- **B:** hash the text before the cut, and report the cut or split the children into more than one document.
- **C:** treat an empty page that still names `next` like a `max_pages` cap.
- **D and E:** start `{since}` a margin before the last successful run's start.
- **F:** remember every token the walk has seen.

**5. Attacked, not broken**
- A record stamped exactly at `{since}` or `{until}`: both ends are cut to the second the same way, and the seen hash stops it being applied twice.
- A run that fails midway (fetch error, model error, apply error): the window stays where it was, and the retry applies nothing twice.
- The seed run: it passes `None` and records no window.
- A single repeated token, PageNumber with no `next`, and a dotted `param` that is missing (`set_at` creates the path).
- Duplicate parent keys: the first one is kept, and its child is called once.
- An old state file without the new field: H passes.
- Changing `refresh_after_days` between runs: it only affects the first window, and finding A covers the rest.

**6. Paths written outside the worktree**
- ~/.cache/cortex-wave-20261005d/walks/adversary-1/: `alone-*.log` (8), `alone-B-first-attempt.log`, `build.log`, `gate.log`, `suite.log`, `list.txt`, `tmp/`
- ~/.cache/b10x-target/cortex-w3-walks: the `connectors_walks_adversary-*` test binary, the assigned build directory

**7.**
```findings
[
 {
  "file": "src/run.rs",
  "line": 215,
  "category": "acceptance",
  "severity": "blocker",
  "verdict": "NEEDS-CHANGE",
  "origin": "introduced",
  "message": "a changed record held back by refresh_after_days is not counted as wanted, so the window moves past its change and no later run asks for it (case at tests/connectors_walks_adversary.rs:224)"
 },
 {
  "file": "src/sources.rs",
  "line": 480,
  "category": "boundary",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "children are added after the parent text and cut by max_chars_per_document after which the hash is taken, so a new child on a long parent is never read (case at :267)"
 },
 {
  "file": "src/sources.rs",
  "line": 195,
  "category": "boundary",
  "severity": "warning",
  "verdict": "INFEASIBLE",
  "origin": "introduced",
  "message": "a walk that stops at an empty page that still names next counts as a full read and moves the window past the unread pages (case at :302)"
 },
 {
  "file": "src/sources.rs",
  "line": 69,
  "category": "concurrency",
  "severity": "warning",
  "verdict": "INFEASIBLE",
  "origin": "introduced",
  "message": "a PageNumber walk whose result set changes between page requests skips a record, and with since equal to the last run start it is never asked for again (case at :329)"
 },
 {
  "file": "src/sources.rs",
  "line": 69,
  "category": "boundary",
  "severity": "warning",
  "verdict": "INFEASIBLE",
  "origin": "introduced",
  "message": "windows meet exactly with no overlap, so a record stamped before the previous until but visible only after that run is never read (case at :358)"
 },
 {
  "file": "src/sources.rs",
  "line": 171,
  "category": "boundary",
  "severity": "note",
  "verdict": "INFEASIBLE",
  "origin": "introduced",
  "message": "the paging loop guard catches only a token equal to the one given, so a two-token cycle repeats child records in the parent text until max_pages (case at :391)"
 },
 {
  "file": "src/sources.rs",
  "line": 480,
  "category": "judgement",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "the whole child JSON goes into the parent text, so a field that changes on every call makes the parent be extracted again after every refresh time"
 },
 {
  "file": "src/sources.rs",
  "line": 476,
  "category": "judgement",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "a child call that keeps failing for one parent fails the whole source on every run, so the source stops applying anything"
 }
]
```