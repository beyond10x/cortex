---
format: aep.planning-md/3
id: review-result:adversary-structured-children-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: structured-children (wave 20261007c)'
relations:
- reviews: story:structured-children
revision: 1
---
```
unit: story:structured-children; commit e1447d0 plus the untracked tests/structured_children_adv.rs, tree cortex-w19-a
verdict: red
cases: executed 7→18, red 1
origin: introduced 1, pre-existing 0, undecided 0
wrote-outside-worktree: none
needs-coordinator: no
```

**1. Diff proof.** `git --no-pager diff --stat` is empty because nothing tracked changed. `git status --short` shows one line: `?? tests/structured_children_adv.rs`. That is a new test file of 745 lines, and nothing under `src/`, `spec/` or `generated/` was touched.

**2. Cases added** (all in `tests/structured_children_adv.rs`; each was run alone first)

| test | asserts | now |
|---|---|---|
| `adv_children_of_a_parent_whose_id_ends_in_a_credential_name_are_applied_once` | parent id `top-secret`, tags `v1.0.0` and `v2.0.0`: a second run with no changes has `documents_new == 0` | **red** |
| `adv_an_event_changed_at_its_fixed_time_holds_its_new_value_alone` | Supersede: event 101 changes its action at the same `created_at`, and one active `merged` value remains, valid from JAN_2 | green |
| `adv_a_parent_no_longer_listed_ends_every_child_and_its_edge` | parent dropped: 15 values retracted, and its children have no active assertions and no edges | green |
| `adv_a_failed_child_call_holds_its_records_until_it_succeeds_again` | forbidden: 0 retracted; the next successful run retracts 2 | green |
| `adv_a_failed_child_call_holds_no_parent_whose_id_its_id_begins` | parent `1` refused does not hold the records of parent `12` | green |
| `adv_a_child_call_failing_for_every_parent_holds_only_that_operation` | events held for both parents, and a dropped tag is still retracted | green |
| `adv_a_child_answer_without_records_or_not_json_is_skipped_not_fatal` | "no array" and "not JSON" failures are skipped and the run is not failed | green |
| `adv_an_event_time_the_reader_cannot_use_is_the_runs_and_a_usable_one_is_the_events` | missing, empty, epoch ms, unparseable and future times get the run's time; a numeric epoch-seconds value and a bare date get the event's time | green |
| `adv_parent_ids_with_colon_and_percent_keep_their_children_apart` | parents `a:b` and `a%3Ab` stay apart; a child id equal to its parent's id stays its own node | green |
| `adv_a_child_added_later_with_the_same_parent_name_is_linked` | a second child added through `cortex update` with the same `parent` name gets its edges and nothing is rejected | green |
| `adv_no_credential_of_a_child_or_its_parent_reaches_the_instance` | no token or assigned value appears in any file under the instance directory | green |

Red output from the solo run, verbatim:
```
thread 'adv_children_of_a_parent_whose_id_ends_in_a_credential_name_are_applied_once' (263884) panicked at tests/structured_children_adv.rs:427:5:
assertion `left == right` failed: nothing changed, so nothing is new: {"command":"run","detail":{"cost_usd":"0.0000","documents_applied":2,"documents_new":2,"facts_refused":0,"masked":0,"parts_rejected":0,"source_id":"m/projects","stopped":null},"outcome":"ran"}
  left: Number(2)
 right: 0
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 9 filtered out; finished in 4.03s
```
The first run of `adv_parent_ids_with_colon_and_percent_keep_their_children_apart` also went red, but the bug was in my own `assert_linked` helper: it did not escape the parent part. I fixed the helper and the test is green. It is not a finding.

**3. Suite runs** (both after the cases existed, with `TMPDIR` set under scratch and `CORTEX_TEST_EKR=<tree>/target/ekr-0.0.32/ekr`). `-- --list` confirmed that the 11 adversary tests exist in this tree.

- Before, with the adversary file excluded: `cargo test --locked -p cortex-cli --test structured_children` gave `test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.07s` (EXIT=0).
- After: `cargo test --locked -p cortex-cli --test structured_children --test structured_children_adv` gave:
  - `test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.14s`
  - `test result: FAILED. 10 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.26s` (EXIT=101)
  - the failing test was `adv_children_of_a_parent_whose_id_ends_in_a_credential_name_are_applied_once`: `left: Number(2) right: 0`

**4. Findings** (they cover e1447d0)

| id | file:line | severity | verdict / origin | what | reach |
|---|---|---|---|---|---|
| A1 | `src/structured.rs:330` | warning | CONFIRMED / introduced | A child identity is built as `<child prefix>:<parent part>:<id>`. When the parent part ends in a credential name, the `:` after it makes masking read the identity as an assignment. The digest fallback, `…:top-secret:<16 hex>`, is still an assignment. `SeenState::load` (`src/state.rs:84`) drops that key, so unchanged children are applied again on every run, with new evidence each time. This contradicts `website/docs/spec-file.md:399` ("a child's identity never holds an id masking … would change"). | A parent mapping `id` whose value ends, at a word boundary, in `secret`, `password`, `passwd`, `api_key`/`api-key`, `access_token` or `auth_token` (for example `id: "$.path"` with a project at `infra/secret`), together with a child id of 6 or more characters (`v1.0.0`, a 6-digit event id). A child operation whose name ends in one of those words reaches it too. Numeric parent ids, as in the unit's own spec, do not reach it. |

The fix for A1, which I did not apply: when the fallback identity is itself changed by `mask_key`, digest the parent part too. Or use a separator after the parent part that masking does not read as an assignment.

**5. Attacked and could not break** (each one is a green case listed above): linking after a parent is dropped; the failed child call is held, then succeeds again; a failure for every parent; failures other than `forbidden` (missing records array, result not JSON); the held prefix covering only its own parent (`1` against `12`); every time shape on the brief's list, read through EKR's valid time; escaping of `:` and `%`, and a child id equal to its parent's id; a shared `parent` edge type added later through `cortex update`; Supersede at an unchanged event time; no credential in `state/`, `cortex.log`, `skipped`, `runs/` or the store. I found no unit test that cannot fail.

**6. Paths written outside the worktree:** none.

```findings
[
  {"file": "src/structured.rs", "line": 330, "category": "boundary", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "A1: a parent id ending in a credential name (top-secret) makes each child identity <prefix>/<op>:top-secret:<digest> still credential-shaped, so SeenState::load drops it and unchanged children are applied again every run, contrary to spec-file.md:399"}
]
```
