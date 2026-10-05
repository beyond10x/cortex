---
format: aep.planning-md/3
id: review-result:adversary-spec-standalone-types-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: spec-standalone-types (wave 20261005a)'
relations:
- reviews: story:spec-standalone-types
revision: 1
---
unit: story:spec-standalone-types (U1, wave 20261005a), uncommitted working tree on base fe85624
verdict: red
cases: executed 29→36, red 1
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: ~/.cache/cortex-wave-20261005a/spec/adversary-1/
needs-coordinator: decide whether the Acceptance's `store: {backend: sqlite}` is corrected to `{backend: sqlite, value: {}}` in the story and docs, or the type accepts the short form (F1)

## Cases added (tests/adversary_u1_spec_types.rs, 7 cases, untracked, headed "ADVERSARY CASES")

| case | asserts | now |
|---|---|---|
| adversary_the_acceptance_store_default_as_written_is_accepted | `create` accepts `store: {backend: sqlite}` exactly as the Acceptance writes it | red |
| adversary_two_costed_batches_report_the_sum_of_both_answers | 0.25 + 0.5 → `"0.7500"`, and 0.75 in the log | green |
| adversary_an_uncosted_first_answer_makes_a_two_batch_run_uncosted | null, then 0.25 → `null` | green |
| adversary_an_uncosted_last_answer_makes_a_two_batch_run_uncosted | 0.25, then null → `null` | green |
| adversary_a_zero_cost_answer_is_zero_not_null | 0 + 0 → `"0.0000"` | green |
| adversary_uncosted_answers_spend_no_budget_and_costed_ones_do | with budget 0.1: two uncosted answers both run; two costing 0.25 stop after one, with a budget reason | green |
| adversary_a_structured_source_is_registered_round_trips_and_does_not_run | the source is `Structured`; the registry reloads; the run gives fetch-failed naming story:structured-source; the web source still runs | green |

Red run, the file alone (`cargo test --locked --test adversary_u1_spec_types`, EXIT=101):

```
assertion `left == right` failed: stdout:  stderr: cortex: …/store.yaml: not a cortex.instance/1 spec: no structurally decodable union alternative
test result: FAILED. 6 passed; 1 failed
```

## Suite run, after the cases existed

| command | result |
|---|---|
| `task check` | EXIT=201: validate, drift, fmt and clippy passed; the test step stopped at the red case |
| `cargo test --workspace --no-fail-fast` | EXIT=101: lib 8, bin 1, adversary 6 of 7, conformance 1, e2e 8, spec_compat 3, cortex_docs 8 |
| `task docs-check` | EXIT=0 |

## Findings

| # | where | verdict / origin | what was measured |
|---|---|---|---|
| F1 | spec/domains/instance.yaml:268 | CONFIRMED / introduced | `create` refuses `store: {backend: sqlite}` (exit 2) because every union variant requires `value`. spec_compat.rs:55 writes `value: {}`, so nothing tests the Acceptance as it is written. |
| F2 | src/run.rs:229 | CONFIRMED / introduced | Mutant: `add_cost` replaced by the last answer's cost. All 29 of the unit's cases stay green; the sum and uncosted-first cases go red. Every unit cost test makes exactly one model call. |
| F3 | src/run.rs:228 | CONFIRMED / introduced | Mutant: `spent += …` dropped. The unit's suite stays green; the budget case goes red. Nothing in the suite reaches the budget stop or the rule that an uncosted answer spends no budget. |

## Attacked, not broken

- Fixture provenance: re-recorded independently from `git archive f2ce283`; byte-identical.
- Normalisation masks only the temp dir, port, `at` and `seconds`.
- Draft plus diff: byte-identical to the spec in the tree.
- `task drift` and `task docs-check`: clean.
- Variant mappings: all 8 match the generated serde renames.
- Cost rule: mixed, zero and no-call runs all correct, in the JSON and in the log.
- Structured kind: round-trips and does not run.
- The original 4 e2e tests: unchanged.

```findings
[
  {"file": "spec/domains/instance.yaml", "line": 268, "category": "acceptance", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "The Acceptance's default store `store: {backend: sqlite}` is refused by create (exit 2, no structurally decodable union alternative); spec_compat tests `value: {}` instead, so the Acceptance as written is never tested."},
  {"file": "src/run.rs", "line": 229, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "Replacing add_cost with the last answer's cost passes all 29 of the unit's cases because every cost test makes one model call; the adversary multi-batch cases catch it."},
  {"file": "src/run.rs", "line": 228, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "Dropping `spent += …` passes the unit's suite: nothing tests the budget stop or the new rule that uncosted answers spend no budget; the adversary budget case catches it."}
]
```
