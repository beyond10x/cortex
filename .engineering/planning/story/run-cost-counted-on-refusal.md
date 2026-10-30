---
format: aep.planning-md/3
id: story:run-cost-counted-on-refusal
kind: story
status: implemented
title: A refused or cut-off model call's cost counts against the run budget and the gate's cost
summary: The cost of a split, refused or failed call is added to `spent` and `report.cost_usd`, not only the daily ledger
tags:
- review-20261030
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
scope:
- confidence: inferred
  path: generated/cortex-model/src/instance.rs
- confidence: inferred
  path: generated/docs/docs/domains/cortex-instance.md
- confidence: inferred
  path: spec/domains/instance.yaml
- confidence: cited
  path: src/extract.rs
- confidence: cited
  path: src/run.rs
- confidence: inferred
  path: tests/batch_fits_model_window.rs
- confidence: inferred
  path: tests/common/mod.rs
- confidence: cited
  path: tests/cost.rs
- confidence: inferred
  path: tests/run_metrics.rs
- confidence: inferred
  path: tests/runout_adversary.rs
revision: 16
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T10:11:01Z", actor: "human:timo", revision: 14}
- {from: "proposed", to: "active", at: "2026-10-30T10:11:01Z", actor: "human:timo", revision: 15}
- {from: "active", to: "implemented", at: "2026-10-30T15:30:13Z", actor: "human:timo", revision: 16, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Outcome

Every costed answer counts against the run's own budget and the run report's cost. Today a
failed or cut-off call's cost reaches the daily ledger (`src/run.rs:1028-1035`) but `spent` and
`report.cost_usd` are updated only on the `Ok` arm (`src/run.rs:1107-1109`), although
`Refused.cost_usd` carries it (`src/extract.rs:882-916`). Peers add refused cost
(`src/quality.rs:649`, `src/schema.rs:575`, `src/eval.rs:585`).

A call that finished just as SIGTERM arrived keeps its answer's cost instead of being dropped as
terminated (`src/extract.rs:633-636`).

## Acceptance

1. A run whose first call is cut off at the output limit and split: `budget_usd` minus the
   reported `cost_usd` equals the remaining budget the next call is given, and the report's
   `cost_usd` includes the cut-off call (new case in `tests/cost.rs`).
2. A child that exited before SIGTERM was observed has its `total_cost_usd` recorded.

## Findings it closes

R2-02, R2-03 (the finished-before-SIGTERM sub-case).

## Scope

Derived 2026-10-30 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong). Checked against the worktree head b61a56c8, whose
`src/`, `tests/`, `spec/` and `generated/` are unchanged from main a870a628 (`git diff --stat` is empty).

- **Primary surface:** `src/run.rs`, the batch loop of `process_in` — cited
- **Files:** `src/run.rs:1028-1035` daily-ledger add: confirmed, though the call actually runs 1030-1036 (1028-1029 is its comment) — cited
- **Files:** `src/run.rs:1107-1109` `spent +=` and `report.cost_usd = add_cost(..)` only after the `Ok` arm: confirmed exactly — cited
- **Files:** `src/extract.rs:882-916` `cost_usd` read at 882, and `Refused { cost_usd, .. }` built at 905-914 in the `is_error` branch: confirmed — cited
- **Files:** `src/extract.rs:633-636` `Ok(Some(_)) if crate::terminate::requested()` returns `Waited::Terminated` and drops the finished child's output: confirmed (the arm's comment starts at 631) — cited
- **Files:** peers `src/quality.rs:649`, `src/schema.rs:575`, `src/eval.rs:585` each run `add_cost(cost_usd, refused.cost_usd)`: confirmed, read only, not edited — cited
- **Symbols / ranges edited (run.rs):** `process_in` (`src/run.rs:567-1212`). Add `spent` and `report.cost_usd` for both `Ok` and `Err` beside `daily.add_answer` (1023-1039), before `let answer = match asked` (1040). Remove the two lines at 1107 and 1109; 1108 (`truncated`) stays — inferred
- **Symbols / ranges edited (extract.rs):** `wait_or_terminate` (`src/extract.rs:603-659`, the arm at 631-636); `enum Waited` (587-594), which needs a variant or flag for "finished, but SIGTERM was seen"; and `ask_claude_with` (793-935) at 860-865, so that the parsed answer's `total_cost_usd` reaches a `Refused { terminated: true, cost_usd: Some(..) }` — inferred
- **Not edited:** `add_cost` (`src/run.rs:228-230`), `Refused::terminated`/`uncosted` (`src/extract.rs:223-244`), `src/gate.rs` (reads `report.cost_usd` at 103 and gains the fix as a consumer) — inferred
- **Tests:** `tests/cost.rs` (532 lines, helpers `costs_per_call` at 71, `budgets_given` at 180) gets a new case: a cut-off first call that is split, with remaining budget equal to `budget_usd` minus the reported `cost_usd` — cited
- **Tests, also likely:** the cut-off answer fake is local to `tests/batch_fits_model_window.rs:58-130`. Copy it into `tests/cost.rs` or move it to `tests/common/mod.rs` — inferred
- **Tests, also likely:** the acceptance-2 case (a child that exited before SIGTERM was observed) fits beside `terminated_run` in `tests/run_metrics.rs:193` or in `tests/runout_adversary.rs`. The story's scope lists neither — inferred
- **spec/domains:** no type change. `cost_usd` stays `Optional<Decimal>` (`spec/domains/instance.yaml:1058`, `:2823`). The `SourceRan` prose (`instance.yaml:2805-2813`) may gain "a failed or cut-off call included" — inferred
- **generated/:** none if the spec prose is left alone. If it is edited, it regenerates into `generated/cortex-model/src/instance.rs` and `generated/docs/docs/domains/cortex-instance.md`, which carry that prose — inferred
- **Documents:** none required — inferred
- **Confidence:** high for `src/run.rs` and `tests/cost.rs`, because the story cites the defect sites and every citation checks out. Medium for the `src/extract.rs` shape of the SIGTERM sub-case, because the story names the site but not the design
- **Would collide with:** any unit touching the extraction batch loop's budget and spend bookkeeping in `src/run.rs` `process_in` (lines 940-1110); any unit touching model-call process handling or SIGTERM in `src/extract.rs` (`Waited`, `wait_or_terminate`, `ask_claude_with`); any unit adding cases to `tests/cost.rs`
- **Safety fact:** `add_cost` returns `None` once any answer is uncosted (`src/run.rs:228-230`). Today a refused call with `cost_usd: None` reaches `report.stopped` at `run.rs:1102-1105`; that covers `Refused::uncosted`, e.g. "claude exited without a JSON answer". After this change such a run reports `cost_usd` null, and a gate `cost_usd` check then fails with "no answer of the run carried a cost" (`src/gate.rs:186`, test at `gate.rs:315-323`). The peers already behave this way. `spent` still uses `unwrap_or(0.0)`, so the budget is not lifted. Level 2 (pointed at file:line), unproven — inferred

## Decisions

Taken 2026-10-30 by the wave coordinator.

- `spent` and `report.cost_usd` add the cost of every answer, refused or not, beside `daily.add_answer`.
- A child that finished before SIGTERM was seen returns `Refused { terminated: true, cost_usd }` with the parsed cost; the run still stops.
- A run with an uncosted refusal reports `cost_usd: null`, as the peers do.
- If the SIGTERM race cannot be tested deterministically, cover it with a unit test of `wait_or_terminate` and say so.
