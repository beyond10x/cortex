---
format: aep.planning-md/3
id: review-result:adversary-w30a-tail-pass-2
kind: review-result
status: active
title: 'Adversary pass 2: unit tail (wave 20261030a)'
relations:
- reviews: story:connectors-call-deadline
- reviews: story:window-relative-placeholders
- reviews: story:runs-retention
- reviews: story:small-robustness
revision: 1
---
unit: `tail` of cortex wave 20261030a, branch `unit/w30a-tail`. Findings cover commit f5998c1e: the correction 2861d8f5 plus my test commit.
verdict: CONFIRMED (1 warning, introduced). Also 2 INFEASIBLE notes with red cases, and 1 INFEASIBLE note from reading only.
cases: executed 394→397, red 3
origin: introduced 1 / pre-existing 3 / undecided 0
wrote-outside-worktree: 4 paths (part 6)
needs-coordinator: Decide whether the SIGTERM case and the 1 MiB hold case stay in this wave. Both are red at base by reading (base joined the readers with no bound and kept stderr raw), and both need a precondition nobody was shown to reach.

**1. Diff stat**

`git --no-pager diff --stat 2861d8f5 HEAD`. The bot made the commit; author and committer are both `b10x-bot[bot]`:
```
 tests/w30a_tail_adversary_pass2.rs | 345 +++++++++++++++++++++++++++++++++++++
 1 file changed, 345 insertions(+)
```
I changed only a test file and left no uncommitted changes. The gates scan of the file (`b10x-gates scan-text --policy …/policy.json`) exited 0.

**2. Cases added** (all in `tests/w30a_tail_adversary_pass2.rs`, all red now)

The output below is from the first run of the file alone (`cargo test -p cortex-cli --test w30a_tail_adversary_pass2`, EXIT=101).

- **The minutes are filled before a renewal, and the retry reuses them.** `adv2_since_minutes_ago_still_reaches_since_when_the_call_is_retried_after_a_renewal`.
  - Setup: the first `issues.list` call answers `not_granted` at admission, and `connections revalidate` takes 5 s.
  - Asserts: the retry's invocation time minus the rendered minutes is no later than `{since}`.
  ```
  the retry after the renewal asked from 1793369933534 (invoked at 1793370833534, -15m), 2084 ms after the window's since 1793369931450: the minutes were filled before the renewal and the window was narrowed
  ```
- **SIGTERM while a leftover process holds the pipe.** `adv2_sigterm_ends_a_connectors_call_whose_child_exited_and_left_its_output_held`.
  - Setup: `invoke` answers and exits, leaving `sleep 30 &` on its stdout. `CORTEX_CONNECTORS_TIMEOUT_S=60`. The run gets SIGTERM 0.5 s after the child exited.
  - Asserts: exit 8 within 10 s.
  ```
  the run ended 29.593694774s after SIGTERM with exit Some(8) (8, terminated, expected within 10 s): {"command":"run","detail":{"reason":"the run was sent SIGTERM and stopped before it used the documents it fetched"},"outcome":"terminated"}
  ```
- **The 1 MiB hold drops a key's BEGIN line and keeps its body.** `adv2_a_private_key_whose_begin_line_is_dropped_from_the_hold_is_not_kept_unmasked`.
  - Setup: `ekr session` writes one stderr line of 1 MiB of `x` that ends in ` -----BEGIN RSA PRIVATE KEY-----`, then three base64 key lines, and exits 1.
  - Asserts: no key line is printed or written under the home.
  ```
  the private key's lines are kept unmasked: 3 of 3 printed, the first written to [".../home/instances/m/status/news.json"]; the answer ends Some("…ekr session ended with exit status: 1: …Q1a/kJuT4dCnMxW7gFqP0Z+jItS3cBmLwV6fEpOzY9iHsR2bAlKvU5eDoNyX8hGr\\nbAlKvU5e…")
  ```

**3. Suite run** (after the cases existed)

`cargo test -p cortex-cli --lib --test connectors_call_deadline --test runs_retention --test small_robustness --test file_records --test structured_files --test connectors_walks --test w30a_tail_adversary --test w30a_tail_security --test w30a_tail_adversary_pass2 --no-fail-fast`, with `CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=4`:
```
lib 305 ok; connectors_call_deadline 1 ok; connectors_walks 39 ok; file_records 18 ok;
runs_retention 3 ok; small_robustness 5 ok; structured_files 13 ok;
w30a_tail_adversary 3 ok; w30a_tail_security 7 ok;
w30a_tail_adversary_pass2: test result: FAILED. 0 passed; 3 failed
EXIT=101
```
"Before" is 394, the same targets with my file left out. The pass-1 cases are green, and none of these targets regressed.

**4. Findings** (commit f5998c1e)

| # | file:line | verdict | origin | severity |
|---|---|---|---|---|
| 1 | src/connectors.rs:495 (filled at src/sources.rs:197) | CONFIRMED | introduced | warning |
| 2 | src/terminate.rs:171 | INFEASIBLE | pre-existing | note |
| 3 | src/ekr.rs:816 | INFEASIBLE | pre-existing | note |
| 4 | src/ask/records/postgres.rs:647 | INFEASIBLE | pre-existing | note |

1. **The minutes are fixed before the call's own retries.**
   - What was measured: `at_call` fills `{since_minutes_ago}` before `Connectors::invoke`. `invoke` serialises the input once (`:495`) and sends the same bytes to every `invoke_once` that follows a slow step:
     - the `revalidate` before it: up to 4 attempts at 5/10/20 s pauses, and `await_ready` polls;
     - a renewal after `not_granted` at admission;
     - the retry of a timeout at admission.
   - The retry asked from 2084 ms after `{since}` (case 1). This contradicts "to the moment each call is made … never narrowed" (`website/docs/spec-file.md:366`).
   - What reaches it: any connectors or structured source using `{since_minutes_ago}` whose connection lapses or is slow to revalidate. The docs say a lapse is routine, since evidence lives 60 s.
   - Records are lost only when the delay exceeds the 5-minute overlap. That is reasoned, not run.
   - Fix: fill the minutes inside `invoke`, per `invoke_once`. For example, pass the unfilled input and `since_ms` down, or a closure that renders the bytes at call time.
2. **The deadline loop ignores SIGTERM.**
   - What was measured: the post-exit loop D35 added (`until`) checks only the clock, never `requested()`. A run sent SIGTERM after the child exited waits until the leftover process lets go of the pipe or the deadline passes (120 s by default; 29.6 s in case 2). "Stopping a run" (`website/docs/operating.md:158`) promises it stops at once during the fetch, and the default 120 s exceeds a typical grace period.
   - Origin: by reading base, which joined the readers with no bound and no SIGTERM check. The same unbounded join remains for `wait` and `wait_finished` (deadline `None`, the `claude` call). I wrote no case for that path.
   - What reaches it: as in pass 1, nothing found that leaves such a process with the real `connectors`.
   - Fix: check `requested()` in `until`; on SIGTERM, kill the group and answer `Terminated`.
3. **The 1 MiB hold can strip a multi-line credential's marker.**
   - What was measured: `keep_read` drops whole lines from the front. A private key is marked only by its BEGIN line (`src/mask.rs` `KEY_LINES`). When that line is the one pushed past 1 MiB, the base64 lines stay and `window` masks nothing. They reached the run's answer and `status/news.json` (case 3).
   - A name at the end of a dropped line, with its value on the next line (`password:\n<value>`, which `SEPARATOR`'s `\s*` matches whole), fails the same way. I did not run that variant.
   - Origin: by reading base, which kept the last 16 KiB raw and unmasked.
   - What reaches it: nothing found. It needs an ekr stderr line over 1 MiB that ends in the marker.
   - Fix: when `keep_read` drops a line, also drop the lines that follow until a line that could not continue a multi-line shape. Or mask each dropped line together with the next lines before discarding it.
4. **The launch's stderr is masked line by line** (reading only, no case).
   - What was measured: the postgres launch's stderr is masked by `serve::log` one line at a time. A private key's base64 lines therefore reach the ask service's log unmasked.
   - Origin: base inherited that stderr raw, which leaked more.
   - What reaches it: nothing found that writes a key there.

**5. Attacked and not broken**

- **D37:**
  - A symlink to a FIFO, a FIFO swapped in after the stat, a directory and `/dev/zero` are refused by `metadata` or by the `fstat` after the non-blocking open.
  - The walkers do not follow links. All four readers go through `read_capped`.
- **D35, other paths:**
  - A child still running at the deadline is killed with its group.
  - A process outside the group that holds the pipe is bounded by `AFTER_KILL` and ends as `TimedOut`.
  - An oversized limit sets no deadline, and `sleep` saturates.
  - `kill_group` signals a reaped pid's group only while that group is alive.
- **D36, other paths:**
  - Every page is re-filled from the template, because `walk_until` keeps `{since_minutes_ago}` literal and `set_at` only changes the page parameter.
  - `{input.*}` in a record id reads `with_window`'s fill from the run's start.
- **D34:** the 16 KiB cut runs after masking, and a last line over 16 KiB is cut only after masking.
- **D38:** every `out.stderr` of an `ekr` goes through `said`, and the hook's stderr is masked before its 2000-character cut. `connectors.rs:273` parses stderr only as JSON. The MCP reader strips `EKR_*`.
- **D39:** only documentation changed.
- **Lease:** I acquired none, so there is none to release.

**6. Paths written outside the worktree** (all under `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/tail/`)

- `p2red.txt`: the run of the new cases alone.
- `p2suite.txt`: the suite run.
- `p2msg.Zcem4c`: the commit message.
- `adversary-report-pass-2.md`: this report.

The tests also made temporary homes under `~/.cache/claude-tmp/cortex-e2e*` and removed them when they ended.

```findings
- file: src/connectors.rs
  line: 495
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "{since_minutes_ago} is filled by at_call before Connectors::invoke, whose revalidation, lapse renewal and admission-timeout retry resend the same serialised bytes later, so the retried call asks from after {since} (2084 ms measured), contradicting spec-file.md:366."
- file: src/terminate.rs
  line: 171
  category: concurrency
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: "The post-exit loop D35 added waits on the clock only and never reads requested(), so a run sent SIGTERM while a leftover process holds the connectors pipe stops only when the pipe closes or the deadline passes (29.6 s measured), not at once as operating.md promises."
- file: src/ekr.rs
  line: 816
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: "keep_read drops a line past the 1 MiB hold whole, so a private key whose BEGIN line ends that line keeps its base64 lines unmarked and window() leaves them unmasked in the run's answer and status file."
- file: src/ask/records/postgres.rs
  line: 647
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: "The launch's stderr is masked one line at a time by serve::log, so the base64 lines of a multi-line private key are logged unmasked."
```
