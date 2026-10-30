---
format: aep.planning-md/3
id: review-result:adversary-w30a-fail-pass-2
kind: review-result
status: active
title: 'Adversary pass 2: unit fail (wave 20261030a)'
relations:
- reviews: story:poison-document-skipped
- reviews: story:window-advances-past-applied
revision: 1
---
unit: fail (story:poison-document-skipped, story:window-advances-past-applied), correction 88415357, findings cover the integration head d63686d1; the case is in cb5c03fd on unit/w30a-p2
verdict: CONFIRMED
cases: executed 36→37, red 1
origin: introduced 1 / pre-existing 1 / undecided 0
wrote-outside-worktree: 3 paths shared by the whole pass (part 6)
needs-coordinator: whether one model call per marked document is the intended price of the correction (its commit chose it), or the batch should be asked whole once more first

**1. Diff stat** (`git --no-pager diff --stat d63686d1 HEAD`): five test files, `tests/w30a_p2_{cli,cost,fail,read,redact}.rs`, 538 insertions, no other path. Commit cb5c03fd, author and committer `b10x-bot[bot]`; scan-text passed.

**2. Case** (`tests/w30a_p2_fail.rs`; a wrapped stand-in `claude` that answers one error costing 0.01 while a `flaky` file exists, then answers normally)
- **`one_failed_answer_on_a_batch_does_not_make_the_next_run_ask_each_document_alone`:** red, not ignored.
- **Setup:** six healthy documents in one batch. Run 1 errs once, which marks all six. On run 2 the model answers every call.
- **Red, run alone** with `cargo test --test w30a_p2_fail`:
  `run 2 asked the model 6 times for one batch of six healthy documents: {… "cost_usd":"0.0600","documents_applied":6, … "model_calls":6, …}` / `left: 6` / `right: 1` / `test result: FAILED. 0 passed; 1 failed`

**3. Suite run** (after the case existed; log `w30a/p2/suite.log`):
- `poison_document`: `test result: ok. 8 passed; 0 failed; 0 ignored`
- `connectors_walks`: `test result: ok. 28 passed; 0 failed; 0 ignored`
- `w30a_p2_fail`: `test result: FAILED. 0 passed; 1 failed`
- The command ended `EXIT=101`. Pass-1's four cases run and pass.

**Pass-1 assertions** (`git show 88415357 -- tests/`)
- The four adversary cases lost their `#[ignore]`.
- The unit's own tests were rewritten where they encoded the old design:
  - halving became lone asks;
  - the count reset on a text change became a kept count;
  - `a_split_batch_stopped_by_a_limit_before_any_part_fails_as_the_batch_did` was deleted with the in-run halving it guarded.
- No pass-1 adversary assertion was weakened.

**4. Findings**

| # | file:line | verdict | origin | severity |
|---|---|---|---|---|
| 1 | src/run.rs:1074 | CONFIRMED | introduced | warning |
| 2 | src/extract.rs (`Refused::model_failed_on_request`) | CONFIRMED | pre-existing | note |

1. **One answered failure of a batch makes the next run ask each of its documents alone.**
   - **The cause:** a batch holding a marked document is never asked whole again. Each marked document is asked alone until it applies. So one error answer on a batch of N healthy documents (one that cost something, as a model that erred once gives, e.g. `error_during_execution` or an answer with no structured output) marks all N. The next run then makes N calls, each carrying the batch's fixed prompt, where 0.6.1 and the unit before the correction made one.
   - **What was measured:** `tests/w30a_p2_fail.rs:84`. Run 2 made 6 calls and cost 0.06 against 0.01, exit 101.
   - **What reaches it:** any answered, non-deterministic batch failure. With `max_documents_per_run` 50 and one batch, one such answer means 50 calls, until the run's budget stops them.
   - **Origin:** introduced by 88415357. The removed comment read "A transient failure costs no split".
   - **Fix to name** (not applied): ask a batch that holds marked documents whole once, and go to lone asks only when it fails again. That keeps pass-1 finding 4 fixed, because the next run still asks each document alone after the second failure.
2. **A document that always makes the model time out still blocks its source for good.** Since the correction, a timeout or crash counts nothing (`answered: false`). That is the right reading of an outage (pass-1 finding 1), but a document that deterministically runs past `timeout_s` (very long, or one that makes the model loop) now stops every run at its batch and is never skipped.
   - **What was measured:** read only; no case.
   - **What reaches it:** a document near `max_chars_per_document` with a short `timeout_s`. The story's Outcome names refusals and validations, not timeouts.
   - **Origin:** pre-existing. 0.6.1 had no skip at all.

**5. Attacked and could not break**
- **The partition loop:** a split batch's parts are either unmarked or of length 1, so the queue cannot loop. The lone asks go in ascending failure count, after the unmarked rest.
- **Free error answers and SIGTERM:** they mark and count nothing; `model_failed_on_request` needs `answered`.
- **`pending_after`:** it is clamped to the run's `held_since`. A run that fails before `report.backlog` is set returns `window.since_ms`, which already folds the stored `held_since`.
- **`listed_only`:** it runs only on a full listing (`window.is_none()` and nothing unread), so a windowed connectors source never drops a count it cannot see. Pass-1's judgement on unbounded growth still holds for windowed sources; I am not reporting it again.
- **The merge with cost and cli on `Window::of_run`:** `pending_since` wins, `held_since` folds in, and the first-run branch saturates.

**6. Paths written outside the worktree** (shared by the pass, under `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/`): `p2/build0.log`, `p2/suite.log`, `p2/commit-msg.x1If5v`, and this report. Test tempdirs under `~/.cache/claude-tmp/` are removed by `tempfile`. I took no worktree session lease.

**7. Findings block**
```findings
- file: src/run.rs
  line: 1074
  category: property
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a batch holding marked documents is never asked whole again, so one answered failure of a batch of N healthy documents makes the next run make N model calls instead of one
- file: src/extract.rs
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: a timeout counts nothing since the correction, so a document that deterministically runs past timeout_s blocks its source on every run and is never skipped
```
