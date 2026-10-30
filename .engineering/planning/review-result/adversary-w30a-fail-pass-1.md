---
format: aep.planning-md/3
id: review-result:adversary-w30a-fail-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: unit fail (wave 20261030a)'
relations:
- reviews: story:poison-document-skipped
- reviews: story:window-advances-past-applied
revision: 1
---
verdict: fail

```
unit: fail (story:poison-document-skipped, story:window-advances-past-applied), findings cover c541c047 on unit/w30a-fail (implementation at 569c0bb2)
verdict: NEEDS-CHANGE
cases: executed 32→36, red 4
origin: introduced 2 / pre-existing 0 / undecided 2
wrote-outside-worktree: 8 paths (listed in part 6)
needs-coordinator: decide whether findings 3 and 4 hold this unit or become a follow-up story
```

Four cases are red. Two of them should block the merge: a healthy document gets skipped for good after a model outage, and a change held back by `refresh_after_days` is lost after a failed run. The other two are cases where a poison document still blocks its source forever.

**1. Diff stat** (`git --no-pager diff --stat 569c0bb2 HEAD`). Only test files changed. Commit c541c047 was made by the bot as author and committer.
```
 tests/connectors_walks.rs |  57 ++++++++++++++++++++
 tests/poison_document.rs  | 130 ++++++++++++++++++++++++++++++++++++++++++++++
```

**2. Cases.** Each is `#[ignore = "adversary: …"]`. I ran each one alone with `env CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=4 nice -n 19 cargo test --test <target> -- --ignored --exact <name>`. All four failed on their first run.

| # | severity | test | verbatim result |
|---|---|---|---|
| 1 | **blocks merge** | `poison_document::adversary_a_model_timing_out_on_consecutive_runs_skips_no_healthy_document` | `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 7 filtered out; finished in 11.74s` with `left: ["…/d1", "…/d2", "…/d3"]` `right: ["…/d0", "…/d1", "…/d2", "…/d3"]`, and skipped `"…/d0: extraction failed with the document alone on 3 consecutive runs (claude exited exit status: 124 without a JSON answer: ); skipped until its text changes"` |
| 2 | **blocks merge** | `connectors_walks::adversary_a_failed_run_does_not_move_the_window_past_a_change_it_held_back` | `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 27 filtered out; finished in 3.88s` with `the change to I-1 never reached the model`. The third run asked from `"updated_since":"2026-10-30T10:14:45Z"`, but I-1's change is stamped 2026-10-29T15:19Z. |
| 3 | follow-up | `poison_document::adversary_a_poison_document_whose_text_changes_every_run_blocks_no_other_document` | `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.79s` with `left: []` `right: ["https://example.org/d1", "https://example.org/d2"]` after 6 runs |
| 4 | follow-up | `poison_document::adversary_a_failing_batch_that_spends_the_run_budget_does_not_block_the_source_for_good` | `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.63s` with `left: []` `right: ["https://example.org/d0"]`, `after 6 runs, 6 calls` |

**What each finding is, and what reaches it:**

1. **A healthy document is skipped after a timeout.** `src/run.rs:1194-1237`. A call ended by `timeout` returns `Refused::uncosted`: no cost, `error_answer: false`. So `free_error_answer()` is false, and the call is marked, split and counted like a model refusal.
   - Reached by: any `model.timeout_s` overrun, a slow model for a few hours, or a `claude` crash that leaves no JSON.
   - After 3 consecutive outage runs, the first document of the batch is skipped until its text changes. From then on, about one more healthy document is lost every two runs.
   - Each outage run also makes log2(batch)+1 calls instead of 1.
   - Fix to name: count only answers where the model actually answered (`error_answer`, or `no structured output`). Treat a timeout or a missing JSON answer like `unavailable`.
   - Origin: introduced (the base had no skip).

2. **A held-back change is lost after a failed run.** `src/run.rs:485-497`. The failure path now sets `pending_since` by `pending_after` but never records `held_since`; only `finish` does. A run that fails while holding back a change moves the window past that change, and nothing records it.
   - Reached by: `refresh_after_days > 0`, any `time:`-mapped connectors source, and any extraction failure before the first apply.
   - The base kept `min(p, window.since_ms)` here, as `git show 7119552e:src/run.rs` lines 481-493 show. Origin: introduced.
   - Fix to name: have the failure path also save this run's `held_since`, for example by carrying it on `Report` next to `backlog`, or clamp `pending_since` to it.

3. **A poison document whose text changes every run blocks the source forever.** `src/state.rs:543-583`. `marked()` and `failed()` match on the hash, so each new text starts at runs=1. The halving puts the document first in its batch and stops the run there.
   - Reached by: a page or thread whose text changes each fetch (a counter, a new reply).
   - It follows the story's decision that a changed hash resets the count, but it misses the Outcome.
   - Origin: undecided. I did not run it against the base, which had no skip at all.

4. **A failing batch that uses up the run's budget is never split far enough.** `src/run.rs:1199-1209` and `src/run.rs:1031-1035`. When the failing call costs the whole `budget_usd`, every later run splits the batch and then stops at the budget before asking either half.
   - Reached by: an `error_max_budget_usd` answer, since each call is capped at the remaining `--max-budget-usd`.
   - The unit's own `a_split_batch_stopped_by_a_limit_before_any_part_fails_as_the_batch_did` treats this state as accepted after one run, and does not check what happens on the runs after it.
   - Origin: undecided.

**3. Suite runs** (after the cases existed):
- `cargo test --test poison_document --test connectors_walks` (adversary cases ignored): `test result: ok. 27 passed; 0 failed; 1 ignored` and `test result: ok. 5 passed; 0 failed; 3 ignored`, EXIT=0.
- With `-- --include-ignored`: `test result: FAILED. 27 passed; 1 failed; 0 ignored` (connectors_walks, EXIT=101) and `test result: FAILED. 5 passed; 3 failed; 0 ignored` (poison_document, EXIT=101).

**4. Judgement findings** (no case written):
- **Failures file never pruned** (`src/state.rs:507-588`, warning). Entries for documents that are no longer fetched are never removed, so the file grows. Origin: introduced.
- **Unreadable failures file** (`src/run.rs:1110`, note). If `failures/<source>.json` cannot be read or parsed, every run of the source fails as a fetch failure. Origin: introduced.

**5. Attacked and could not break:**
- Two runs racing on the failures file: every run holds the home lock (`src/home.rs:42`).
- How the failures file is written: through `write_atomic`, keys are masked before they are counted, and the reason is masked.
- A gate refusal does not reset the count, and is not counted as a failure itself.
- SIGTERM and a free error answer are never counted.
- A skipped document is asked again once its text changes.
- The skip shows in `skipped` on every run and in the log line.
- Record times in the future: clamped to the run start, nothing lost.
- Record times before the window, and the overlap subtracted near the epoch: both clamp to the window start (`saturating_sub`); `clamp` cannot panic.
- A source whose records carry no time: the window stays at its start and never moves back.
- The cost unit's re-apply rounds: documents a failed round left unstored keep their mark, are not counted, and hold the window.

**6. Paths written outside the worktree** (all under `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/fail/`): `build.log`, `adversary_a_model_timing_out_on_consecutive_runs_skips_no_healthy_document.log`, `adversary_a_poison_document_whose_text_changes_every_run_blocks_no_other_document.log`, `adversary_a_failing_batch_that_spends_the_run_budget_does_not_block_the_source_for_good.log`, `held.log`, `adv-suite-default.log`, `adv-suite-all.log`, `adv-suite-poison-all.log`. The commit message files `adv-msg-draft.txt` and `adv-msg.qFdrEJ` are there too. The worktree's own `target/` grew from the test builds.

**7. Findings block**

```findings
- file: src/run.rs
  line: 1209
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: an uncosted timeout (no JSON answer) is not a free error answer, so a model outage over three runs marks, splits and counts a healthy document until it is skipped for good
- file: src/run.rs
  line: 492
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the failed-run path advances pending_since past a change it held back by refresh_after_days without recording held_since, so the change is never fetched again
- file: src/state.rs
  line: 579
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: a poison document whose text changes every run is never marked with its current hash nor counted past one, and each split stops the run at it, blocking the source forever
- file: src/run.rs
  line: 1199
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: a failing batch whose call spends the run's budget is split on every run and stopped before either half is asked, so the poison document is never counted and the source stays blocked
- file: src/state.rs
  line: 507
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: failures entries for documents no longer fetched are never pruned, so the file grows without bound
- file: src/run.rs
  line: 1110
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: an unreadable failures file fails every run of the source as a fetch failure
```
