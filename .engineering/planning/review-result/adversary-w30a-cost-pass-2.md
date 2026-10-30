---
format: aep.planning-md/3
id: review-result:adversary-w30a-cost-pass-2
kind: review-result
status: active
title: 'Adversary pass 2: unit cost (wave 20261030a)'
relations:
- reviews: story:run-cost-counted-on-refusal
- reviews: story:replacement-applied-as-assertion
revision: 1
---
unit: cost (story:replacement-applied-as-assertion, story:run-cost-counted-on-refusal), correction b6750cf2, findings cover the integration head d63686d1; the cases are in cb5c03fd on unit/w30a-p2
verdict: NEEDS-CHANGE
cases: executed 6→9, red 2
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 paths shared by the whole pass (part 6)
needs-coordinator: whether finding 1 holds the unit (pass-1 finding 1 was the same class and was rated blocker)

**1. Diff stat** (`git --no-pager diff --stat d63686d1 HEAD`): five test files, `tests/w30a_p2_{cli,cost,fail,read,redact}.rs`, 538 insertions, no other path. Commit cb5c03fd, author and committer `b10x-bot[bot]`; scan-text passed on each file and the message.

**2. Cases** (`tests/w30a_p2_cost.rs`; the real `ekr` 0.0.34 and the stand-in `claude` of `tests/supersession.rs`). Run alone first with `cargo test --test w30a_p2_cost`: `test result: FAILED. 1 passed; 2 failed`.
- `an_older_replacement_beside_a_newer_plain_value_in_one_batch_leaves_one_value_active`: red, not ignored.
  - Input: an empty store and one batch. Document a (2026-01-02) says "owner is now Alice"; document b (2026-01-03) says "owner is Bob".
  - Red: `one active owner, Bob: [("Bob", "\"Active\""), ("Alice", "\"Active\"")]; run: {… "facts_reapplied":1, … "parts_rejected":0 …}`
- `a_newer_replacement_listed_before_an_older_plain_value_leaves_one_value_active`: red, not ignored.
  - Input: b (2026-01-03) says "owner is now Bob" and comes first; a (2026-01-02) says "owner is Alice".
  - Red: `one active owner, Bob: [("Alice", "\"Active\""), ("Bob", "\"Active\"")]`
- `three_replacements_out_of_order_in_one_batch_leave_the_newest_active`: green. The correction holds for its own shape, with replacements only, in any order.

**3. Suite run** (after the cases existed; one run with the other units' targets, log `w30a/p2/suite.log`):
- `supersession`: `test result: ok. 6 passed; 0 failed; 0 ignored`. Pass-1's two cases run and pass.
- `w30a_p2_cost`: `test result: FAILED. 1 passed; 2 failed`
- The whole command ended `EXIT=101`. The 6 is `supersession` alone, from the same run.

**Pass-1 assertions:** the correction only removed the two `#[ignore]` lines (`git show b6750cf2 -- tests/`). The unit test's `props` gained a `sort()`, because the rounds no longer keep document order; nothing was relaxed.

**4. Findings**

| # | file:line | verdict | origin | severity |
|---|---|---|---|---|
| 1 | src/extract.rs:1143 (`reapply_rounds`), :1196 | NEEDS-CHANGE | introduced | blocker |

1. **A plain value and a replacement of one property in one batch leave two active values.**
   - The cause: `reapply_rounds` orders only the facts EKR rejected for nothing to replace. A plain value of the same subject and property that the first apply stored is not in any group. Round 0 then strips `replaces` from the rejected fact, so it becomes a second plain assertion beside the stored one.
   - It fails whichever document is newer. When the replacement is the newer one (case 2), keeping `replaces` would have let EKR supersede the stored value.
   - **What was measured:** the two red cases above, `tests/w30a_p2_cost.rs:124` and `:144`, exit 101.
   - **What reaches it:** the first run of an instance, or the first value of a property, where one batch holds one document that states a value plainly and another that says it changed. The model is told to mark a changed value `replaces: true`. EKR applies the replacement before the plain fact in one document (case 2 shows that the order in the batch does not matter).
   - **Origin:** introduced. Release 0.6.1 dropped the rejected replacement, which left one active value (case 1's Bob; in case 2, Alice, the wrong one).
   - **Fix to name** (not applied): before stripping `replaces`, re-apply round 0 still replacing. Strip `replaces` only from a fact EKR rejects again for nothing to replace. Or put the stored plain facts of each key into the time ordering.

**5. Attacked and could not break**
- Three replacements out of date order in one batch: one active value, the newest (green case above).
- `observed_at` is the document time (`evidence::observed_at`), or the run start when there is none. With no times, the order falls back to the place in the document, so it is deterministic.
- A failed round marks the documents of that round and every later round as unstored. They are not counted in `documents_applied` and are extracted again. An unknown store outcome still goes through `unknown_write`.
- `facts_reapplied` and `parts_rejected` add up per round, and pseudonymised runs remove each round file.

**6. Paths written outside the worktree** (shared by the pass, under `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/`): `p2/build0.log`, `p2/suite.log`, `p2/commit-msg.x1If5v`, and this report. The tests' tempdirs under `~/.cache/claude-tmp/` are removed by `tempfile`. I took no worktree session lease.

**7. Findings block**
```findings
- file: src/extract.rs
  line: 1143
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: reapply_rounds orders only the rejected replacements, so a replacement re-applied without replaces beside a plain value of the same property stored in the same batch leaves two active values
```
