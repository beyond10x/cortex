---
format: aep.planning-md/3
id: review-result:adversary-spec-standalone-types-pass-2
kind: review-result
status: archived
title: 'Adversary pass 2: spec-standalone-types (wave 20261005a)'
relations:
- reviews: story:spec-standalone-types
revision: 2
transitions:
- {from: "active", to: "archived", at: "2026-10-08T13:07:37Z", actor: "human:timo", revision: 2}
---
unit: U1 story:spec-standalone-types, pass 2, uncommitted working tree on fe85624 in cortex-w1-spec
verdict: CONFIRMED
cases: executed 36→44, red 2
origin: introduced 4 / pre-existing 0 / undecided 2
wrote-outside-worktree: ~/.cache/cortex-wave-20261005a/spec/adversary-2/; two build subdirectories, made and deleted
needs-coordinator: none

The diff stat is unchanged at 93 files, +8086/-440. The adversary added one untracked file, tests/adversary_u1_pass2.rs, with 8 cases.

## Cases

| case | now |
|---|---|
| every_sqlite_form_the_published_schema_accepts_parses_to_sqlite | green |
| every_store_form_the_published_schema_refuses_is_refused (11 forms) | green |
| a_gate_bound_the_published_schema_refuses_as_a_decimal_is_refused | red: ``the gate check `min: "abc"` was accepted`` |
| a_refused_store_names_the_store_field | red: ``does not name `store`: … no structurally decodable union alternative`` |
| both_sqlite_forms_freeze_update_into_each_other_and_run | green |
| a_budget_spent_exactly_stops_the_next_call | green, catches the `<=`→`<` mutant |
| an_uncosted_answer_does_not_lift_the_budget_for_later_costed_ones | green, catches the remaining-from-reported-cost mutant |
| each_call_is_given_the_budget_that_remains | green, catches 3 mutants |

## Suite run

| command | result |
|---|---|
| `task check` | EXIT=201, stopped at adversary_u1_pass2 (6 passed, 2 failed) |
| `cargo test --workspace --no-fail-fast` | EXIT=101 |

## Mutants of src/run.rs that survive the unit suite (tests/cost.rs included)

| line | mutant |
|---|---|
| 212 | `<=`→`<` |
| 211 | remaining computed from the reported cost |
| 228 | `spent =` instead of `+=` |
| 228 | `spent` double-counted |
| 220 | the full budget passed to every call |

Both add_cost null-order mutants are caught.

## Findings

| # | where | verdict / origin | measured |
|---|---|---|---|
| G1 | src/model_map.rs:119 | CONFIRMED / introduced, warning | `create` accepts and freezes `gate.checks[].min: "abc"`, which the schema's decimal pattern refuses |
| G2 | src/spec.rs:37 | CONFIRMED / introduced, note | Store refusals no longer name `store`; the base binary said "unknown field `store`" |
| M1 | src/run.rs:211 | CONFIRMED / introduced, warning | The remaining-from-reported-cost mutant survives |
| M2 | src/run.rs:228 | CONFIRMED / introduced, warning | The assign and double-count mutants survive |
| M3 | src/run.rs:212 | CONFIRMED / undecided, note | `<=`→`<` survives; the line is unchanged from base |
| M4 | src/run.rs:220 | CONFIRMED / undecided, note | The full-budget-per-call mutant survives; no test reads `--max-budget-usd` |

## Attacked, not broken

- F1 is fixed. 11 store forms agree with the schema.
- None and Some are read only in model_map.rs:95-101.
- The frozen spec and the registry entry round-trip each form.
- The schema and `create` agree on 52 of 55 forms. Of the other 3, `keep: 3.0` and `rare_limit: 2.0` come from the pre-existing `int()` rule and are not raised; the third is G1.

The adversary returned its findings block as YAML. The coordinator converted it to JSON with no change in content, because the YAML has unquoted ": " in its messages.

```findings
[
{"file":"src/model_map.rs","line":119,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"create accepts and freezes gate.checks[].min/max values such as \"abc\" that the published schema's decimal pattern refuses."},
{"file":"src/spec.rs","line":37,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"every refused store form reports \"no structurally decodable union alternative\" without naming store, where the base named the field for the same input."},
{"file":"src/run.rs","line":211,"category":"mutant","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"computing the remaining budget from the reported cost instead of spent passes the whole unit suite although it stops enforcing the budget after one uncosted answer."},
{"file":"src/run.rs","line":228,"category":"mutant","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"assigning or double-counting spent instead of accumulating it passes the whole unit suite because no unit test makes a third model call."},
{"file":"src/run.rs","line":212,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"undecided","message":"flipping the budget stop from <= to < passes the unit suite because no case spends exactly the budget."},
{"file":"src/run.rs","line":220,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"undecided","message":"passing the full budget instead of the remaining one to each model call passes the unit suite because no test reads --max-budget-usd."}
]
```
