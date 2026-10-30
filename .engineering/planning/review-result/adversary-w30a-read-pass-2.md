---
format: aep.planning-md/3
id: review-result:adversary-w30a-read-pass-2
kind: review-result
status: active
title: 'Adversary pass 2: unit read (wave 20261030a)'
relations:
- reviews: story:one-store-read-per-run
- reviews: story:idle-supersede-source-cheap
revision: 1
---
unit: read (story:one-store-read-per-run, story:idle-supersede-source-cheap), correction 2bf0426a, findings cover the integration head d63686d1; the case is in cb5c03fd on unit/w30a-p2
verdict: CONFIRMED
cases: executed 8→9, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 paths shared by the whole pass (part 6)
needs-coordinator: the finding comes from the merge of cost and read; decide which unit owns the fix

**1. Diff stat** (`git --no-pager diff --stat d63686d1 HEAD`): five test files, `tests/w30a_p2_{cli,cost,fail,read,redact}.rs`, 538 insertions, no other path. Commit cb5c03fd, author and committer `b10x-bot[bot]`; scan-text passed.

**2. Case** (`tests/w30a_p2_read.rs`; the real `ekr` 0.0.34, a stand-in `claude`, `batch_chars: 1000` for two batches)
- **`a_node_created_by_a_reapply_round_is_offered_to_the_next_batch_as_in_the_store`:** red, not ignored.
- **Setup:** batch 1 names the Service `widget` only in a `!Property` marked `replaces: true`. EKR rejects that fact for nothing to replace, before anything is resolved, so the first apply creates no node. The cost unit's re-apply round then stores it, and `widget` is a node of the store.
- **Before the assertion fails:** the case checks `facts_reapplied == 1` and that `widget` is in a fresh `ekr snapshot`. Both hold.
- **Red, run alone** with `cargo test --test w30a_p2_read`:
  `batch 2 is not offered widget, which the store holds:` / `(no known entities)` / `test result: FAILED. 0 passed; 1 failed`

**3. Suite run** (after the case existed; log `w30a/p2/suite.log`):
- `one_store_read` 2, `one_store_read_adv` 2, `idle_supersede` 2 and `idle_supersede_adv` 2 each ended `test result: ok … 0 failed; 0 ignored`. Pass-1's four cases run and pass.
- `w30a_p2_read`: `test result: FAILED. 0 passed; 1 failed`. The command ended `EXIT=101`.

**Pass-1 assertions:** the correction only removed four `#[ignore]` lines (`git show 2bf0426a -- tests/`). Nothing else changed.

**4. Findings**

| # | file:line | verdict | origin | severity |
|---|---|---|---|---|
| 1 | src/run.rs:1362 (`lookup.applied`), :1380 (re-apply rounds) | CONFIRMED | introduced | warning |

1. **A node created by a re-apply round is missing from the run's grown index.**
   - **The cause:** `lookup.applied(&doc, &applied)` runs once, for the first apply. The rounds of `extract::reapply_rounds` applied after it are never fed to the index. So the story's condition, that batch k+1 gets the store hits a fresh snapshot would give, fails for every node only a re-applied replacement named.
   - **Where it is documented:** `Index::applied`'s doc comment states this as a residual ("a `replaces` fact with nothing left to replace leaves out what only it named"). That held before the merge. Since cost re-applies those facts, cortex itself creates the node, and the run has the round's document and report in hand.
   - **What was measured:** the red case above, `tests/w30a_p2_read.rs:117`.
   - **What reaches it:** a batch whose only mention of a new entity is a changed value of it (`replaces: true`) on an empty property.
   - **Effect:** low, as in pass-1 finding 3. The next batch's model is not told the node exists, and EKR still resolves it by alias.
   - **Origin:** introduced by the merge of cost and read. Release 0.6.1 re-read the store per batch and never re-applied.
   - **Fix to name:** call `lookup.applied(&round.doc, &second)` after each committed round.

**5. Attacked and could not break**
- `ended_listing` is cleared and saved before the store handle, the stage and the before-apply copy. An undo restores state with no fingerprint (conservative). The idle early return writes nothing.
- A refused group is consistent with EKR. EKR names a rejection at the *first place* of a named thing (`RejectedExtraction.item` doc in EKR 0.0.34), so dropping the whole group matches what it did not create. A fact rejected without `transaction_id` is skipped from grouping, as EKR skips it.
- Exact-alias matching (`exact`) is byte for byte, and the canonical name is never compared, as `ekr resolve` does.

**6. Paths written outside the worktree** (shared by the pass, under `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/`): `p2/build0.log`, `p2/suite.log`, `p2/commit-msg.x1If5v`, and this report. Test tempdirs under `~/.cache/claude-tmp/` are removed by `tempfile`. I took no worktree session lease.

**7. Findings block**
```findings
- file: src/run.rs
  line: 1362
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the run grows its known-entity index from the first apply only, so a node a re-apply round creates is not offered to the next batch as in the store
```
