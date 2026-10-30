---
format: aep.planning-md/3
id: review-result:adversary-w30a-tail-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: unit tail (wave 20261030a)'
relations:
- reviews: story:connectors-call-deadline
- reviews: story:window-relative-placeholders
- reviews: story:runs-retention
- reviews: story:small-robustness
revision: 1
---
unit: `tail` of cortex wave 20261030a, branch `unit/w30a-tail`. Findings cover commit 3f6fa3ee (base effef82c, unit head 4f30cb2d plus my test commit).
verdict: CONFIRMED (1 warning). Also 1 INFEASIBLE warning, 1 INFEASIBLE note and 1 note from reading only.
cases: executed 381→384, red 3
origin: introduced 3 / pre-existing 1 / undecided 0
wrote-outside-worktree: 5 paths
needs-coordinator: The deadline case is labelled `introduced` because the bound is this unit's new contract. It would also be red at base, where there is no bound at all, so decide whether it stays in this wave.

**1. Diff stat**

`git --no-pager show --stat HEAD` (the bot made the commit; author and committer are both `b10x-bot[bot]`):
```
 tests/w30a_tail_adversary.rs | 207 +++++++++++++++++++++++++++++++++++++++++++
 1 file changed, 207 insertions(+)
```
The working tree has no tracked changes. The one untracked file, `tests/w30a_tail_security.rs`, belongs to the security reviewer. I changed only a test file.

**2. Cases added** (all in `tests/w30a_tail_adversary.rs`, all red now)

- **Deadline with a leftover child process.** `adv_the_deadline_holds_when_connectors_exits_and_a_descendant_keeps_its_output_open`. A stand-in `connectors` starts `sleep 15 &` and exits 1 straight away. The case asserts that with `CORTEX_CONNECTORS_TIMEOUT_S=1` the run ends within 3 s of the call starting.
  ```
  the run ended 15037 ms after the connectors call started, with CORTEX_CONNECTORS_TIMEOUT_S=1: the call was waited for until its descendant closed the pipe (exit 1, {"command":"run","detail":{"reason":"tavily.websearch.search: unavailable at execution"},"outcome":"fetch-failed"})
  ```
- **Timeout value too large.** `adv_an_unbounded_connectors_timeout_does_not_panic_the_run`. With `CORTEX_CONNECTORS_TIMEOUT_S=18446744073709551615`, the run must not panic and must answer `ran`.
  ```
  thread 'main' (1901446) panicked at src/terminate.rs:72:20:
  overflow when adding duration to instant
    left: Some(101)
   right: Some(101)
  ```
- **`{since_minutes_ago}` narrows the window.** `adv_since_minutes_ago_still_reaches_since_when_the_call_starts_after_the_run`. The source has two inputs, and the first takes 5 s to answer. For the second input, the case asserts that call time minus the rendered minutes is no later than `{since}`.
  ```
  the second input asked from 1793368332229 (invoked at 1793369232229, -15m), 2070 ms after the window's since 1793368330159: the window was narrowed
  ```

**3. Suite run** (after the cases existed)

`cargo test --lib --test connectors_call_deadline --test runs_retention --test small_robustness --test file_records --test structured_files --test connectors_walks --test w30a_tail_adversary --no-fail-fast` with `CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=4`:
```
lib 303 ok; connectors_call_deadline 1 ok; connectors_walks 39 ok; file_records 18 ok;
runs_retention 3 ok; small_robustness 4 ok; structured_files 13 ok;
w30a_tail_adversary: test result: FAILED. 0 passed; 3 failed
EXIT=101
```
"Before" is 381: the same run with my target left out.

**4. Findings**

- **`src/sources.rs:156`: CONFIRMED, introduced, warning.** `{since_minutes_ago}` is computed once from the run's start, but the tracker evaluates `-Nm` at the moment of each call.
  - What was measured: a call made after the run's start asks from later than `{since}`, by the call's delay minus the rounding slack (under 60 s). That contradicts "so the window is never narrowed" in `website/docs/spec-file.md:367` and `src/sources.rs:151`.
  - What reaches it: every connectors or structured run whose calls start after its start, through revalidation, earlier inputs, later pages or child calls.
  - When it loses records: when this run's delay exceeds the previous run's by more than the 5-minute overlap. Then changes in the gap are never read. That condition is reasoned, not run.
  - Fix: render the value per call as `ceil((now − since)/60 000)`, or change the docs.
- **`src/terminate.rs:139` / `:168`: INFEASIBLE, introduced, warning.** The deadline is checked only while `try_wait` answers `None`. Once the direct child has exited, `stdout.join()` waits for the pipe to close with no bound.
  - What reaches it: nothing found. The real `connectors` starts its owner process with null stdio, and the kubernetes credential plugin pipes stdout under its own deadline. Only a wrapper set as `CORTEX_CONNECTORS` that leaves a child behind reaches it.
  - Fix: bound the reader joins by the same deadline.
- **`src/terminate.rs:72` (value read at `src/connectors.rs:253`): INFEASIBLE, introduced, note.** `Instant::now() + limit` panics when the setting is near `u64::MAX`. The docs give no upper bound.
  - What reaches it: only an absurd operator value (over about 9.2e18 s).
  - Fix: `checked_add`, or cap the value.
- **`src/gate.rs:124`, `src/quality.rs:481`, `src/quality.rs:513`, `src/instance.rs:408`: CONFIRMED by reading only, pre-existing, note.** These `ekr` stderr strings are not masked. They are on the schema-apply gate, `quality` and `adopt`, not on a source run's paths. The CHANGELOG claims masking only for run paths, and the story's decision limited the scope to `ekr::run` and `Store::seeded`. No test was written.

**5. Attacked and not broken**

- **Child that stays alive past the deadline:** its whole process group is killed and reaped. A child that writes part of an answer and then hangs ends as `TimedOut`.
- **Timed-out calls and failure counts:** a timed-out invocation fails the run as `fetch-failed`. A timed-out child call counts in `child_failures` like any other child failure; only `Terminated` is left out. This matches the existing rules.
- **16 MiB cap:**
  - Exactly at the cap is read; one byte over is refused (`take(MAX+1)`). A file that grows while read is bounded.
  - Symlinks (including to `/dev/zero`), FIFOs and devices are skipped, because walkdir's `is_file` does not follow links.
  - Not tested: a `records.lookup` path naming a FIFO blocks in `File::open`. This is pre-existing and only reachable through a path the operator names.
- **`{since_minutes_ago}` edge cases:**
  - Replacement order cannot clash with `{since}`. A zero-length window renders `0`.
  - Arithmetic is `i64` with saturation. No load-time check reads the placeholders.
  - A negative value (`--Nm`) needs the clock to go back more than 5 minutes; not tested.
- **Retention:**
  - `keep_days < 1` is refused at every spec load (`parse` then `check`), so pruning never sees such a value. The current run's directory is named after `begun`, so it is never pruned.
  - Symlinks are not followed. Files, names cortex did not give, and future-dated directories are kept.
  - Schema runs live under `schema/`, not `runs/`. Nothing reads old `runs/` directories.
- **Small robustness items:**
  - `EKR_*` is stripped on both direct paths (`store_cmd`, `schema_cmd`) and on the launch path.
  - Masking covers `finished`, `head`, `Pipe` and `Session`.
  - Only an empty YAML tag panics; I probed other tag strings in scratch and they serialize.
  - Keep+Keep is accepted and Supersede on either side is refused. Prefix equality is identity equality.
- **Relaxed assertions:** none in pre-existing tests; the diff removes no line from them. The loosened checks (exit 1 or 2, and "somewhere under the instance") are in the unit's own new `tests/small_robustness.rs`.
- **Lease:** I did not acquire one, so there is none to release.

**6. Paths written outside the worktree** (all under `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/tail/`)

- `probe/`: a Rust probe crate with its own `probe/target/` build directory. I did not delete it.
- `adv-run1.log`
- `suite.log`
- `commit-msg.JhVW5H`

```findings
- file: src/sources.rs
  line: 156
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "{since_minutes_ago} is fixed at the run's start but the query evaluates it at call time, so a call made after the start asks from later than {since}, contradicting 'never narrowed' in spec-file.md:367."
- file: src/terminate.rs
  line: 139
  category: boundary
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: "Once the direct connectors child exits, the stdout/stderr reader joins wait with no deadline, so a leftover child process holding the pipe defeats CORTEX_CONNECTORS_TIMEOUT_S (15 s measured against a 1 s limit); nothing found that leaves one with the real binary."
- file: src/terminate.rs
  line: 72
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "CORTEX_CONNECTORS_TIMEOUT_S near u64::MAX makes Instant::now() + limit panic the run (exit 101); only an absurd operator value reaches it."
- file: src/gate.rs
  line: 124
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: "ekr stderr stays unmasked outside source runs (gate.rs:124 schema-apply gate, quality.rs:481 and :513, instance.rs:408 adopt); outside the story's decided scope, found by reading only."
```
