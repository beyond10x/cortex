---
format: aep.planning-md/3
id: review-result:adversary-schema-emergence-pass-1
kind: review-result
status: archived
title: 'Adversary pass 1: schema-emergence'
relations:
- reviews: story:schema-emergence
revision: 2
transitions:
- {from: "active", to: "archived", at: "2026-10-08T13:07:37Z", actor: "human:timo", revision: 2}
---
unit: story:schema-emergence (wave 20261006h, unit a): the uncommitted working tree at `~/.local/state/worktree/trees/b10x/cortex/cortex-w15-a` on base `53acc86`
verdict: NEEDS-CHANGE
cases: executed 456 (the implementer's `task check`) → 178 targeted (`--test schema` 19, of which 5 are mine; lib 159), red 3
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory, `~/.cache/cortex-wave-20261006h/a/adv/`
needs-coordinator: none

**1. Diff scope**

`git --no-pager diff --stat` reads `136 files changed, 3424 insertions(+), 372 deletions(-)`, the same as when I was handed the tree. My only edit is to `tests/schema.rs`, which is untracked, so it does not show in that stat. I appended 241 lines after its last test (it went from 730 to 971 lines). I edited no file outside `tests/`.

**2. Cases added** (in `tests/schema.rs`; each was run alone first, logs in `adv/red-*.log`)

| Test | What it asserts | Now |
|---|---|---|
| `adv_a_property_a_subtype_inherits_is_not_added_to_it_again` (:779) | `add_property Startup.country`, where `Startup` inherits `country` from `Maker`, is not applied | red |
| `adv_redeclaring_a_property_keeps_it_required` (:814) | redeclaring `Maker.country` from One to Many keeps `required: true` | red |
| `adv_a_masked_name_does_not_reach_the_next_round_through_a_property_name` (:843) | a property the model named `[Name-1]` does not bring the fixture's rare name to the next round's prompt in the clear | red |
| `adv_an_empty_store_asks_no_model_and_applies_nothing` (:901) | a store holding only its seed asks no model and commits nothing | green |
| `adv_a_type_a_run_adds_while_the_model_is_asked_is_not_defined_twice` (:932) | a `Maker` committed between the draw and the apply makes the proposal `invalid` | green |

Red output, verbatim:
```
panicked at tests/schema.rs:800:5:
Startup now declares a second `country` beside the one it inherits from Maker: ... "name": String("Startup"), "parents": [{"name": "Maker"}], "properties": [{"name": "country", "required": false, "value_type": {"value_kind": "Integer"}}]
```
```
panicked at tests/schema.rs:833:5:
the redeclaration also made `country` optional, which nobody proposed: {"cardinality":"One",...,"required":true,...} -> {"cardinality":"Many",...,"required":false,...}
  left: Bool(false)
 right: true
```
```
panicked at tests/schema.rs:893:5:
a name the run masks reaches the proposing model in the clear in the next round:
The ontology in force.
Node types:
- Person: role (String, One), <rare name> (String, One)
...
The engine is kept by [Name-1] today.
```
(I left the fixture's rare name out of the quote above. It is the same synthetic name acceptance test 5 uses.)

**3. Suite run** (after the cases existed)

- `cargo.sh test --test schema`: `test result: FAILED. 16 passed; 3 failed`, EXIT=101. The 3 failures are the cases above; all 14 of the unit's own tests pass.
- `cargo.sh test --lib`: `159 passed; 0 failed`, EXIT=0.
- `cargo.sh test --test stand_ins`: `2 passed`. My file passes the stand-in scan.

**4. Findings**

| # | file:line | Severity | Verdict / origin | What was measured | What reaches it |
|---|---|---|---|---|---|
| F1 | `src/schema.rs:540`, `:692` | blocker | NEEDS-CHANGE / introduced | `Names::of` reads only a type's own properties, so `add_property` on a subtype for a property its parent declares is applied. EKR 0.0.31 accepts it. On a probe store, a later `apply-extraction` of `Startup.country` text was then rejected with `extraction-value-mismatch`; without the shadowing property it committed. EKR has no operation that removes a property. | Subtypes come from `seed.schema`, `seed.ekr_seed` or a run's extraction document: `extract::merge` passes the model's `parents` through. Fix: include the properties of the ancestors in `Names`. |
| F2 | `src/ekr.rs:381`, `src/schema.rs:692` | warning | NEEDS-CHANGE / introduced | `redeclare_property` rewrites the whole declaration with `required: false` and `constraints: []`. A required property silently becomes optional. The model is never shown `required`, and `proposals.jsonl` does not record the change. | A required property in `seed.schema`, `seed.ekr_seed` or a run's extraction document. Fix: carry over the existing `required` and `constraints`. |
| F3 | `src/schema.rs:493`, `:348`/`:469` | blocker | NEEDS-CHANGE / introduced | The answer is restored everywhere, type and property names included. A `[Name-1]` property is committed under the clear name. `ontology_text` is not masked, so from the next round on every model is shown that name. Acceptance 5 fails. | The model has to put a placeholder in a name. Nothing in cortex stops it, and evidence the model reads could ask it to. Fix: drop or mark `invalid` any proposal whose type, property, owner or endpoint names hold a placeholder, or restore only `reason`. |

**5. Attacked and could not break**
- Narrowing over existing assertions (String to Integer, Many to One): EKR refuses it with named issues, and the unit's refusal test covers it.
- A run committing between the draw and the apply: `record` re-reads the ontology at the head (my green race case). This matters more than it looks. EKR 0.0.31 accepts a second node type with an existing name, and after that every `apply-extraction` fails ("two node types are named"). So this cortex check is the only guard. No test of the unit's covers it; a mutant planning against the drawn ontology would pass the unit's suite. I did not run that mutant.
- A type name reused with a different kind: `has_type` checks node and edge types.
- Edge endpoints that do not exist, or are edge types: `invalid`.
- `--dry-run`: no mint and no submit; acceptance test 4 holds.
- Empty store: no model call and nothing committed.
- Unexpected answer shapes: the generated schema's `kind`/`value` shape matches `change_of`, and anything else is dropped.
- Flag-like instance names: `is_name` refuses a leading `-`. `--sample` and `--dry-run` require a name.
- Failure halfway: `proposals.jsonl` is created before the first apply. An instance removed while the model is asked fails before anything is applied.

Not a failing case, a note only: EKR also accepts a type name that differs only in case (`maker` beside `Maker`), and cortex's name check is case-sensitive.

**6. Paths written outside the worktree**
- `~/.cache/cortex-wave-20261006h/a/adv/`: probe seeds, the extraction and transaction documents, `tx.sh`, the probe stores `m.sqlite`, `o.sqlite`, `p.sqlite`, `p2.sqlite`, `p3.sqlite`, and the `red-*.log` and `suite-*.log` files.
- The builds reused `~/.cache/b10x-target/cortex-w15-a`. The test worlds are temporary directories under it and remove themselves.

**7. Findings block**
```findings
[
  {"file": "src/schema.rs", "line": 692, "category": "boundary", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "add_property ignores properties a subtype inherits, so it adds a second same-named property that EKR accepts, permanently shadowing the parent's and making later extraction of that property fail with extraction-value-mismatch"},
  {"file": "src/ekr.rs", "line": 381, "category": "judgement", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "redeclare_property writes required false and empty constraints over the existing declaration, silently making a required property optional although the proposal only named kind and cardinality"},
  {"file": "src/schema.rs", "line": 493, "category": "acceptance", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "placeholders are restored into type and property names, so a masked rare name becomes an ontology name that ontology_text shows unmasked to the model in every later round, violating acceptance 5"}
]
```