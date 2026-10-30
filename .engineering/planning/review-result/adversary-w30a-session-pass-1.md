---
format: aep.planning-md/3
id: review-result:adversary-w30a-session-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: unit session (wave 20261030a)'
relations:
- reviews: story:ekr-session-per-run
revision: 1
---
unit: `session` of cortex wave 20261030a. Findings cover unit commit 3ce84070; my cases are in commit 12371a83 on unit/w30a-session.
verdict: NEEDS-CHANGE
cases: executed 37→43, red 3
origin: introduced 2 / pre-existing 0 / undecided 1
wrote-outside-worktree: 9 paths
needs-coordinator: whether acceptance 1 ("`connections list` at most once per run") is binding as written. The unit's own test allows one check per one-shot launch.

**1. `git --no-pager diff --stat 3ce84070 HEAD`**
```
 tests/ekr_session.rs     |  26 +++++
 tests/ekr_session_adv.rs | 295 +++++++++++++++++++++++++++++++++++++++
 tests/store_backend.rs   |  51 ++++++++
 3 files changed, 372 insertions(+)
```
All three are test files; no implementation file was touched. The commit went through `b10x-gates bot`, author and committer `b10x-bot[bot]`. All touched files passed `scan-text`.

**2. Cases added. Each was run alone first; logs are in scratch as `case-<name>.log`.**

- `tests/ekr_session.rs:294` `adversary_a_launched_staged_run_checks_its_connections_readiness_once`: **red**. It asserts the story's acceptance 1, which says `connections list` runs at most once per run.
  ```
  7 `connections list` readiness checks in one run: [ ... list, launch ["stage","list"], list, launch ["stage","begin"], list, launch ["session"], list, launch ["apply-extraction",…batch-0…], list, …batch-1…, list, …batch-2…, list, launch ["stage","publish",…] ]
  ```
  - **Measured:** `Store::command` and `Store::stage_command` still call `launch.ready` before every one-shot apply and stage verb (`src/ekr.rs:335`, `:352`). The unit's own test only asserts `lists <= one_shot + 1`.
  - **Reached by:** every run on a store launched through Connectors.
  - **Fix:** check readiness once per run and keep the result in the `Store` or its `Launch`.
- `tests/ekr_session_adv.rs:208` `a_store_that_does_not_open_when_a_run_starts_is_a_store_failure_not_an_extraction_failure`: **red**.
  ```
  left: String("extraction-failed")  right: "apply-refused"
  {"detail":{"reason":"ekr schema failed: the run's ekr session ended (it closed its output): ekr session ended with exit status: 1: ekr: the store does not open: connection refused"},"outcome":"extraction-failed"}
  ```
  - **Measured:** a model run's first session request is now the extraction schema, mapped to `Failure::Extract` (`src/run.rs:985`). The spec reserves `ExtractionFailed` for "the model call failed or returned no valid extraction document", and here the model was never asked.
  - **Base:** `Binary::model_schema` never touched the store, so the first store read was `ontology` → `Failure::Apply`.
  - **Reached by:** any non-structured run whose PostgreSQL or SQLite store does not open after the readiness check passes, for example PostgreSQL down.
  - **Fix:** map a session-ended error at that call to `Failure::Apply`, or read the schema store-less as before.
- `tests/ekr_session_adv.rs:231` `a_run_blocked_on_a_session_that_never_answers_stops_on_sigterm`: **red**.
  ```
  panicked at tests/ekr_session_adv.rs:246:10: the run, blocked on its session's answer, did not end within 20 s of SIGTERM
  ```
  - **Measured:** `Session::request` blocks in `read_answer` (`src/ekr.rs:817`). It has no deadline and is not a SIGTERM safe point. The session process itself does not outlive a killed run.
  - **Reached by:** an `ekr session` that hangs while cortex alone receives SIGTERM (for example a Kubernetes Job deadline).
  - **Origin:** undecided. A hung one-shot `Command::output()` at base would block the same way, but I could not run base.
- `tests/ekr_session_adv.rs:255` `a_session_that_ends_part_way_fails_the_run_as_the_store_and_leaves_no_process`: green. The run ends `apply-refused`, the reason names `ekr session`, and no session process is left. The first version was not a finding: its stand-in hung because `head` buffers its output into a pipe. I fixed the stand-in and the case is now green.
- `tests/ekr_session_adv.rs:274` `a_three_batch_run_under_refuse_if_left_asks_its_session_for_one_snapshot`: green. It restores the one-snapshot-per-run check under `refuse_if_left`. `tests/one_store_read.rs` counts `snapshot` processes, of which a run now starts none, so it passes whatever the run does. The unit's `ekr_session.rs` covers only the plain policy.
- `tests/store_backend.rs:894` `adversary_a_postgres_runs_session_quality_reads_the_applies_of_its_stage`: green. It ran against a real PostgreSQL container in 17.9 s. The gate's quality, asked through the session in the run's stage, sees the run's applies, and `assertions.active max 0` refuses the run.

**3. Suite, run after the cases existed.** `cargo test --no-fail-fast --test ekr_session --test ekr_session_adv --test store_backend --test one_store_read` exited 101:
```
ekr_session      3 passed; 1 failed
ekr_session_adv  2 passed; 2 failed
one_store_read   2 passed
store_backend    33 passed (PostgreSQL lanes ran)
```
The baseline is the same command with `--skip adversary_` and without `ekr_session_adv`: 3 + 2 + 32 = 37 passed, exit 0. `cargo fmt --check` and `clippy -D warnings` pass on the touched test targets.

**4. Judgement findings.**
- Store-backend test edits (`store_backend.rs:843`, `:2331`): re-pinned, not relaxed in what they protect.
  - The descriptor-3 test still checks, for every logged store invocation including the session, that the connection is on descriptor 3. It no longer proves the gate asked `quality`. `ekr_session.rs` (quality counted as a request) and my PostgreSQL case cover that. No row.

**5. Attacked and not broken.**
- **Session closing:** the session closes before publish, abandon and restore: `held_to_gate` closes it before settling, and on every other exit it is dropped when `process_in` returns, before `settle`.
- **Refused kinds check after staging:** a refused kinds check now runs after `Staged::begin`. `settle` discards the stage when nothing was applied, so no stage is left. This is from reading the code, not from a run.
- **SQLite visibility:** a SQLite store outside a stage sees the applies: the existing `run_gate.rs` quality-gate case covers this. The SQLite snapshot copy and restore use SQLite's online backup, so an open session does not affect them.
- **Pipe protocol:** stderr is drained on its own thread. Answers split across reads or larger than the pipe buffer are parsed as a stream.

**6. Paths written outside the worktree.** All are under `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/session/`:
- `ekr.diff`
- `case-adversary_a_launched_staged_run_checks_its_connections_readiness_once.log`
- `case-adversary_a_postgres_runs_session_quality_reads_the_applies_of_its_stage.log`
- `case-a_run_blocked_on_a_session_that_never_answers_stops_on_sigterm.log`
- `case-a_session_that_ends_part_way_fails_the_run_as_the_store_and_leaves_no_process.log`
- `case-a_store_that_does_not_open_when_a_run_starts_is_a_store_failure_not_an_extraction_failure.log`
- `case-a_three_batch_run_under_refuse_if_left_asks_its_session_for_one_snapshot.log`
- `probe-dies.log`
- `suite.log` and `suite-before.log`

The commit message temp file was deleted. Other files in that directory are not mine. The tests' own temporary worlds live under `~/.cache/claude-tmp/cortex-e2e*` and are removed when each test ends. The builds went into the worktree's own `target/`. My lease `adversary-w30a-session-1` is released.

```findings
- file: src/ekr.rs
  line: 335
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a launched staged run runs `connections list` 7 times (before every apply-extraction and stage verb) where acceptance 1 says at most once per run; red in tests/ekr_session.rs:294
- file: src/run.rs
  line: 985
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a store that does not open at a model run's start now ends extraction-failed (the schema request through the dead session maps to Failure::Extract) instead of apply-refused; red in tests/ekr_session_adv.rs:208
- file: src/ekr.rs
  line: 817
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: a session request has no deadline and is no SIGTERM safe point, so a run blocked on a session that never answers ignores the first SIGTERM and never ends; red in tests/ekr_session_adv.rs:231
```
