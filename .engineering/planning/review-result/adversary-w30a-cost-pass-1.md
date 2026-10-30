---
format: aep.planning-md/3
id: review-result:adversary-w30a-cost-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: unit cost (wave 20261030a)'
relations:
- reviews: story:run-cost-counted-on-refusal
- reviews: story:replacement-applied-as-assertion
revision: 1
---
verdict: fail

```
unit: cost (story:run-cost-counted-on-refusal, story:replacement-applied-as-assertion), findings cover unit/w30a-cost at ec243210; my cases are commit 1f1a9077
verdict: NEEDS-CHANGE
cases: executed 4→6 (tests/supersession.rs, with --include-ignored), red 2
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 path
needs-coordinator: whether finding 1 holds the unit; I took no worktree session lease
```

**1. What I changed** (`git --no-pager diff --stat ec243210..HEAD`)
```
 tests/supersession.rs | 97 +++++++++++++++++++++++++++++++++++++++++++++++++++
 1 file changed, 97 insertions(+)
```
Only a test file. I committed it through `b10x-gates bot` as `1f1a9077 test: adversary cases for the re-applied replacement`, author and committer `b10x-bot[bot]`. scan-text was clean on the file and on the commit message.

**2. Cases added.** Both are red now and carry `#[ignore = "adversary: …"]`. Each was run alone first with `cargo test --test supersession <name>`.

- **`adversary_two_replacements_with_nothing_to_replace_in_one_batch_leave_the_newer_active`**
  - Setup: an empty store and one batch with two documents: a, dated 2026-01-02, says "owner is now Alice"; b, dated 2026-01-03, says "owner is now Bob".
  - It asserts that one owner is active, and that it is Bob.
  - Red output:
    `one active owner, Bob: [Owner { … value: "Alice", records: ["a"], from: Some(1767312000000), to: None, lifecycle: String("Active") }, Owner { … value: "Bob", records: ["b"], from: Some(1767398400000), to: None, lifecycle: String("Active") }]; run: {… "facts_reapplied":2, … "parts_rejected":0 …}`
- **`adversary_a_reapply_that_fails_once_does_not_lose_the_value`**
  - Setup: an `ekr` wrapper fails once on `reapply.yaml` with "the store is unavailable".
  - It asserts that Alice is stored after that run plus one more run.
  - Red output:
    `Alice is stored after the failed re-apply and a later run: []; first: {… "parts_rejected":1,"rejected":[{… "refusal":"replacement-without-active-assertion: facts[1]: …"}], … "stopped":"ekr apply-extraction failed: ekr: the store is unavailable: injected once"} …; second: {… "documents_new":0 …}`

**3. Suite run** (after the cases existed):
- `cargo test --test supersession`: `test result: ok. 4 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.68s`
- `cargo test --test supersession -- --include-ignored`: `test result: FAILED. 4 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.74s`

**4. Findings**

| # | file:line | verdict | origin | severity | what reaches it |
|---|---|---|---|---|---|
| 1 | src/extract.rs:1108 (`body.remove("replaces")`), called from src/run.rs:1204 | NEEDS-CHANGE | introduced | blocks merge | First run of any instance (or the first value of a property) where one batch holds two value changes. The model is told to mark changed values `replaces: true`. |
| 2 | src/run.rs:1231 (`Err(e) => stop_after = Some(e)`), then the seen record at :1248 | CONFIRMED | introduced | follow-up | A transient EKR or store failure between the first apply and the re-apply. I built this case with a wrapper and did not observe it in production. |

1. **Two active values for one property.** Every refused replacement is re-applied with `replaces` removed, so a batch with two changes leaves both values active. That contradicts website/docs/operating.md:187 ("one value stays current"). Before this change both values were lost; now the store contradicts itself, and a later replacement would supersede both. Suggested fix, not applied: per subject and property, drop `replaces` only from the earliest-dated re-applied fact and keep it on the later ones (or apply them in date order). The later ones then supersede the first through EKR.
2. **A failed re-apply loses the value for good.** The code comment justifies marking a rejected document seen because "EKR would reject the fact again". That does not hold when the re-apply failed for a cause of the store's. The document is marked seen, so no later run retries it. Suggested fix: do not record as seen the documents whose facts were in a re-apply that failed.

**5. Attacked and could not break:**
- Cut-off and refused cost reaches `spent` and `report.cost_usd`: removing either line breaks `a_cut_off_call_counts_against_the_budget_and_in_the_run_cost`.
- Acceptance 2 end to end: a fake `claude` answered at 0.4, sent SIGTERM to cortex and exited. The run ended exit 8 `terminated` and `spend.json` held `"0.4"`, 5 of 5 runs green. That probe cannot tell the two race paths apart, so I removed it.
- A plain value and its replacement in one batch: EKR supersedes inside one document and one value stays active.
- An entity counted twice through the re-apply: not reachable. EKR refuses a bad entity for the whole document (`extraction-type-undeclared`, `reference-without-identity`) and never lists it as a `rejected` part.
- PostgreSQL stage: `before.committed()` running twice only publishes the first time (src/run.rs:1341). No case written.
- `cortex create` without `facts_reapplied`: consistent with the story's Decisions, which name only the `ran` detail and the log line.

**6. Paths written outside the worktree:**
- `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/cost/commit.avy5tl` (the commit message). I deleted my other probe files from that directory; the rest there is the implementor's.

**7. Findings block**
```findings
- file: src/extract.rs
  line: 1108
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: two replacements of one property with nothing to replace in one batch are both re-applied without replaces and both stay active, against operating.md's one current value
- file: src/run.rs
  line: 1231
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a re-apply that fails for a cause of the store's leaves the document marked seen, so the replacing value is never stored by a later run
```
