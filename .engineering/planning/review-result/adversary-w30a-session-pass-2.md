---
format: aep.planning-md/3
id: review-result:adversary-w30a-session-pass-2
kind: review-result
status: active
title: 'Adversary pass 2: unit session (wave 20261030a)'
relations:
- reviews: story:ekr-session-per-run
revision: 1
---
unit: `session` of cortex wave 20261030a. The findings cover the correction 0a4312a9. My cases are in commit c1822eec on unit/w30a-session.
verdict: NEEDS-CHANGE
cases: executed 49→51, red 1
origin: introduced 0 / pre-existing 0 / undecided 1
wrote-outside-worktree: 3 paths
needs-coordinator: none

**1. `git --no-pager diff --stat 0a4312a9 HEAD`**
```
 tests/ekr_session_unit_stop_adv.rs | 248 +++++++++++++++++++++++++++++++++++++
 1 file changed, 248 insertions(+)
```
I changed one file, a test file. No implementation, spec or `.engineering/` file was touched.
- The commit went through `b10x-gates bot`; author and committer are `b10x-bot[bot]`.
- `scan-text` was clean, rustfmt passes, and clippy `-D warnings` passes on the target.

**2. Cases added, in `tests/ekr_session_unit_stop_adv.rs`. Each was run alone first.**

- **`a_gated_run_whose_unit_is_stopped_during_a_session_read_ends_terminated_and_undone`: red, 3 out of 3 runs.**
  - **Setup:** a gated SQLite run applies batch 0. On the session's next request (`ontology`), the stand-in sends the run SIGTERM and then SIGTERM to the session's own process group. This is what `systemctl stop` does: it signals every process in the unit.
  - **Red output:**
  ```
  assertion `left == right` failed: a gated run its unit's stop ended while it waited on `ontology` ends terminated with the store at its head from before the run (0): {"command":"run","detail":{"reason":"ekr ontology failed: the run's ekr session ended (it closed its output): ekr session ended with signal: 15 (SIGTERM): "},"outcome":"apply-refused"}
    left: (Some(1), Some("apply-refused"), false)
   right: (Some(8), Some("terminated"), true)
  ```
  - **Measured:** the run ends `apply-refused` instead of `terminated`, and batch 0 stays in the store, so a gated run is not undone.
    - `Answers::read` returns `Ok(0)` on `Disconnected` (`src/ekr.rs:855`) without checking `terminate::requested()`.
    - The request then fails as "session ended". `store_failure` (`src/run.rs`) matches only the `SESSION_TERMINATED` text, so it gives `Failure::Apply`. `schema_failure` does the same for the schema read.
    - On a staged PostgreSQL run, `settle` publishes the stage after a part-way `Failure::Apply`. So a gated run would publish without being judged by its gate. I did not run that path.
  - **What reaches it:**
    - The run unit cortex installs is `Type=oneshot` with no `KillMode` (`src/schedule.rs:592`), so the default `control-group` signals the session too.
    - The `RunTerminated` summary changed in this correction (`spec/domains/instance.yaml:1296`). It names `systemctl stop` and says a read of the store in flight is ended and a gated run undone.
    - `src/terminate.rs` already treats a child that ended after the run's SIGTERM as ended by it.
  - **Fix:** on end of file or `Unread::Ended`, when `terminate::requested()` is set, fail as `SESSION_TERMINATED`. Or have `store_failure`/`schema_failure` check `requested()`.
  - **Origin:** undecided. At base a one-shot `ekr ontology` killed by a unit stop would most likely also have mapped to `Failure::Apply`. The session verbs do not exist at base, so I could not run the case there.

- **`a_gated_run_sent_sigterm_while_its_session_hangs_on_a_read_is_undone_and_leaves_no_session`: green.**
  - SIGTERM goes to the run alone, as from a Kubernetes deadline, while the session hangs on a read after an apply.
  - The run exits 8 `terminated`, the store head is back to where it was before the run, and no stand-in session process is left. Run alone: `1 passed`.

**3. Suite, run after both cases existed.** The command was `cargo test --no-fail-fast --test ekr_session --test ekr_session_adv --test ekr_session_unit_stop_adv --test one_store_read --test store_backend --test run_metrics`, and it exited 101:
```
ekr_session                5 passed
ekr_session_adv            4 passed
ekr_session_unit_stop_adv  1 passed; 1 failed (panicked at tests/ekr_session_unit_stop_adv.rs:199:5)
one_store_read             2 passed
run_metrics                5 passed
store_backend             33 passed
```
- The 49 before my file is the same run with `ekr_session_unit_stop_adv` left out: 5+4+2+5+33.
- All three pass-1 red cases are now green: readiness checked once, a store that does not open ends `apply-refused`, and a hung session stops on SIGTERM.

**4. Judgement findings.** None beyond the case above.

**5. What I attacked and could not break.**
- **Lapse re-check loop:** `launched_output` retries exactly once and has no loop. A second `unavailable`/`readiness` refusal is returned as the failure. I checked this by reading the code, not by a run.
- **Other refusals retried:** `lapsed` needs both `code=unavailable` and `stage=readiness`. I found no other refusal or consumer stderr that matches.
- **Session launch refused for lapse:** this launch is not retried. Its readiness check comes milliseconds earlier (`stage list`/`stage begin`), and I could not show anything reaching that window. No case.
- **A write that never answers:** after SIGTERM it is waited for with no deadline. That is the stated design, and a second SIGTERM exits 143. A one-shot `apply-extraction` at base waits the same way. Not a finding.
- **Process group:** the Connectors consumer stays in the launcher's group with `PR_SET_PDEATHSIG` (connectors `launch.rs:441`), so the group kill reaches the launched `ekr`. The green case shows that the kill and then the undo leave no session and no lock held.
- **`ready` leaking across runs:** `store_handle` builds a fresh `Readiness` on every call, and only clones share it. `ask serve` builds a reader command and never goes through `Store::command`.
- **Pass-1 assertions dropped:** none. `git show 0a4312a9 -- tests/` tightens the unit's readiness assertion to `== 1`, adds a lapse case, and adds only `ready:` struct fields elsewhere. My pass-1 files are unchanged.

**6. Paths written outside the worktree.** All are under `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/session/`:
- `pass2-case-unit-stop.log`
- `pass2-case-unit-stop-x3.log`
- `pass2-case-gated-hang.log`

`pass2-suite.log` is in the same directory as well. The commit message file was deleted. Builds went into the worktree's own `target/`. My lease `adversary-w30a-session-2` is released.

```findings
- file: src/ekr.rs
  line: 855
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: undecided
  message: a session that dies of the unit's SIGTERM while the run waits on a read is read as end of file without checking the run's SIGTERM, so a gated run under systemctl stop ends apply-refused and keeps its applied batch instead of terminated and undone as RunTerminated now says; red in tests/ekr_session_unit_stop_adv.rs:199
```
