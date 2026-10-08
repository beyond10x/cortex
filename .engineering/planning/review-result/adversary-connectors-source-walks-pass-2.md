---
format: aep.planning-md/3
id: review-result:adversary-connectors-source-walks-pass-2
kind: review-result
status: archived
title: 'Adversary pass 2: connectors-source-walks (wave 20261005d)'
relations:
- reviews: story:connectors-source-walks
revision: 2
transitions:
- {from: "active", to: "archived", at: "2026-10-08T13:07:36Z", actor: "human:timo", revision: 2}
---
unit: U1 story:connectors-source-walks, uncommitted working tree ~/.local/state/worktree/trees/b10x/cortex/cortex-w3-walks on base 90ae12b (coordinator's main.rs and docs patches applied)
verdict: NEEDS-CHANGE
cases: executed 167→172, red 5
origin: introduced 5 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory (~/.cache/cortex-wave-20261005d/walks/adversary-2/), plus the adversary test binary in the assigned ~/.cache/b10x-target/cortex-w3-walks
needs-coordinator: (1) choose a bound for `held_since` (finding P). (2) Decide whether the decided "a failing child call stops the window" may hold the window with no limit (finding Q).

**1. Diff stat**
```
 src/main.rs               |   4 +
 src/run.rs                | 132 ++++++++++++--
 src/sources.rs            | 454 ++++++++++++++++++++++++++++++++++++++++++++--
 src/state.rs              |  48 ++++-
 website/docs/spec-file.md |  51 +++++-
```
This is the stat I was handed, so I changed no implementation or docs file. My only file is the new untracked `tests/connectors_walks_adversary.rs`, headed "ADVERSARY CASES". I ran `rustfmt` on that file alone and the stat did not change.

**2. Cases (each run alone and red before the suite ran; logs are `adversary-2/alone-<name>.log`)**

| # | case (line) | asserts | now | red output (verbatim, trimmed) |
|---|---|---|---|---|
| P | :191 | Two records are edited daily and were applied on different days, with refresh 3. By day 9, `{since}` reaches back no more than 4 days. | red | `day 9: since 2026-09-26T13:51:44Z reaches back past 2026-10-01T13:56:55Z (4 days)`. `held_since` stays at day 0 while `last_success_started_at` moves every run. `since` steps back one day per run across days 1 to 9. |
| Q | :235 | After a child call has failed for I-2 on every run for 5 days, `{since}` reaches back no more than 4 days. | red | `the window never moved`: since steps back a day per run (10-04 to 09-30), `18 child calls in all` |
| S | :275 | A first run left 2 of 6 documents unapplied by `max_documents_per_run`, so the next run keeps its `{since}` (the doc promises this). | red | `left: "2026-10-02T13:56:57Z" right: "2026-10-02T13:56:56Z"`: the window slid by 1 s with the clock |
| T | :303 | A run that applies nothing does not report `truncated`. | red | `{"documents_applied":0,…,"truncated":1}`, in the result and in `cortex.log` |
| R | :333 | A record stamped a day before the first run of a windowed source with `refresh_after_days: 0` is read within two runs. | red | first run asked `updated_since == updated_until == 2026-10-05T13:56:58Z`; second run asked `13:51:58Z`; 0 applied in both |

**3. Suite (run after the cases existed)**
- Command: `cargo test --locked --workspace --no-fail-fast`. It ended with `EXIT=101`: 167 passed, 5 failed, 172 executed.
- Only `connectors_walks_adversary` failed: `test result: FAILED. 0 passed; 5 failed`.
- Every other binary has the same count as in fix-1/gate-2.log (`connectors_walks` 14, lib 71, store_backend 15, …).

**4. Findings**

| # | file:line | verdict, severity | what reaches it |
|---|---|---|---|
| P | src/run.rs:425 | NEEDS-CHANGE, blocker | Any active source with `refresh_after_days` > 0 and at least 2 records applied on different days that keep changing, for example a child field that changes on every call (the docs name durations and signed URLs). `held_since` only ever takes the minimum, and is cleared only when nothing at all is held, so it stays at the first hold for ever. The window grows by a day per run, with a child call per parent, until `max_pages` stops every run. The doc line at spec-file.md:148, "until the change is applied", is not what the code does. Fix (not applied): each run, recompute `held_since` from the documents held in that run, from when each was last applied, minus the overlap. Do not take the minimum with the old value. |
| Q | src/sources.rs:522 | CONFIRMED, warning | Any parent whose child call fails on every attempt, for example one that always times out (nothing in this repo shows one). The coordinator decided the window stops, and the code does that. The cost is that the window never moves again and its width grows with no limit. |
| S | src/sources.rs:83 | CONFIRMED, warning | A first run with a backlog larger than `max_documents_per_run` (example.yaml uses 20 with refresh 30), a budget stop or `max_pages`. With no successful run yet, `{since}` = now − refresh, so it slides with the clock. Under a newest-first sort, unapplied records at the old edge drop out and are never read. This contradicts spec-file.md:141. Fix: when a run stops, store the window's `since` and use it as the next run's lower bound. |
| T | src/run.rs:199 | CONFIRMED, note | The count covers every fetched document that was cut, including ones not selected. So an unchanged long document is reported as `truncated` on every run that re-reads it. |
| R | src/sources.rs:83 | CONFIRMED, note | A windowed source with `refresh_after_days: 0` gets an empty window on its first run and never reads any history older than 5 minutes before that run. The story's rule says this, but nothing warns the user. |
| J1 | src/state.rs:43 | CONFIRMED, note (judgement, no case) | The hash is now taken before the cut. A state file written at the base holds the hash of the cut text, so after the upgrade every document longer than `max_chars_per_document` is extracted once more without having changed (files and web sources too). The docs do not mention this. |

**5. Attacked, not broken**
- Pass-1 fixes A, B, C, E, F, G, H: all hold, and the merged cases in `connectors_walks.rs` pass.
- `held_since` with one record changing every run: it is cleared on the run that applies that record, so the window stays within the refresh time.
- `held_since` is also cleared when the held change is reverted or the record leaves the query.
- Overlap: nothing was applied twice. The hash and `wants` stop it, including with refresh 0, records past `{until}` and records stamped exactly on the boundary.
- `documents_new` and `documents_applied` are not counted twice.
- State file: `write_atomic` writes a temp file and renames it, so a process crash cannot corrupt it. It never calls fsync, which predates this unit. The home-wide `cortex.lock` stops two runs saving at once.
- A crash between batches: the window is kept, and documents already applied are not wanted again.
- The docs patch matches the code on the paging rules, the stop conditions, the key dedupe, the `truncated` presence, the overlap value and `{until}` = run start.

**6. Paths written outside the worktree**
- In ~/.cache/cortex-wave-20261005d/walks/adversary-2/: `build.log`, `list.txt`, `suite.log`, five `alone-adv_*.log` files and `tmp/`.
- The `connectors_walks_adversary-*` test binary in ~/.cache/b10x-target/cortex-w3-walks.
- I took no worktree lease: the `worktree` CLI shows no lease verb.

**7.**
```findings
[
 {"file": "src/run.rs", "line": 425, "category": "property", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "held_since only ever takes the minimum and is cleared only when nothing is held, so with two staggered changing records it stays at the first hold for ever and the window grows a day per run (case at tests/connectors_walks_adversary.rs:191)"},
 {"file": "src/sources.rs", "line": 522, "category": "boundary", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "a child call failing on every run holds last_success for ever, so the window never moves and every run re-lists and re-calls children for every parent since the freeze (case at :235)"},
 {"file": "src/sources.rs", "line": 83, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "before the first successful run a stopped run does not keep its since as spec-file.md promises; the window slides with the clock and unapplied records at the old edge can fall out (case at :275)"},
 {"file": "src/run.rs", "line": 199, "category": "boundary", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "truncated counts every fetched long document, not the ones the run cut for the model, so a run applying nothing reports truncated 1 (case at :303)"},
 {"file": "src/sources.rs", "line": 83, "category": "boundary", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "refresh_after_days 0 gives the first run an empty window, so a windowed source never reads records stamped before its first run (case at :333)"},
 {"file": "src/state.rs", "line": 43, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "hashing before the cut makes every document longer than max_chars_per_document in a pre-upgrade state file look changed, so each is extracted once more after the upgrade, undocumented"}
]
```