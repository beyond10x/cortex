---
format: aep.planning-md/3
id: review-result:adversary-events-carry-measurements-pass-1
kind: review-result
status: archived
title: 'Adversary pass 1: events-carry-measurements (wave 20261007b)'
relations:
- reviews: story:events-carry-measurements
revision: 2
transitions:
- {from: "active", to: "archived", at: "2026-10-08T13:07:36Z", actor: "human:timo", revision: 2}
---
```
unit: story:events-carry-measurements
verdict: green
cases: executed 36→39, red 0
origin: introduced 1, pre-existing 1, undecided 0
wrote-outside-worktree: none
needs-coordinator: yes
```

No case went red. I added three guard tests and all pass. There are two note-level findings, and F1 needs you because it is a planning-store edit, which I may not make.

**1. Diff of the worktree**
```
 tests/quality.rs | 92 ++++++++++++++++++++++++++++++++++++++++++++++++++++++++
 tests/schema.rs  | 34 +++++++++++++++++++++
 2 files changed, 126 insertions(+)
```
Only test files are in the diff. The findings cover `705a234` plus these uncommitted test additions in the unit's worktree `cortex-w18-b`.

**2. Cases added (each run alone first, before any suite run)**

| test | asserts | now |
|---|---|---|
| `tests/quality.rs` `adv_every_fact_judged_no_is_a_rate_of_zero_not_a_null_rate` | 0 of 2 facts passing prints `rate` as the number 0, not null, with the same text as `fact-quality.json`; `cost_usd` is `"0.0200"` | green |
| `tests/quality.rs` `adv_a_rate_of_one_third_is_printed_digit_for_digit_in_the_events_decimal_form` | `rate` prints as `0.3333333333333333`; `rate`, `lower` and `upper` match `fact-quality.json` exactly; all four decimals match the `pattern` in the generated `QualityMeasured` event schema | green |
| `tests/schema.rs` `adv_an_empty_store_costs_nothing_in_a_dry_run_and_when_applying` | with an empty store, both `proposed` and `applied` print `cost_usd` `"0.0000"` and no model is called. The existing adv test checks this only `if code == 0` | green |

Runs alone:
```
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 13 filtered out; finished in 0.74s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 22 filtered out; finished in 0.19s
```

**3. Suite runs** (`--list` first confirmed the test names: quality 15, schema 23, conformance 1)

Before, with my three cases excluded by `--exact --skip <name>`:
```
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 1.25s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 2.48s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
```
After, with everything included:
```
$ cargo test --locked -p cortex-cli --test quality
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.69s
quality EXIT=0
$ cargo test --locked -p cortex-cli --test schema
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.28s
schema EXIT=0
$ cargo test --locked -p cortex-cli --test conformance
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
conformance EXIT=0
```
Also: `ess specify validate --path spec --strict-requires` printed `cortex v1 — 3 file(s), valid` (exit 0), `task drift` exit 0, `cortex-docs generate --check` exit 0, clippy `-D warnings` on both test targets exit 0, fmt check exit 0.

**4. Findings**

| id | file:line | severity | verdict / origin | what | reach |
|---|---|---|---|---|---|
| F1 | `.engineering/planning/story/quality-judge.md:69`, `.engineering/planning/story/schema-emergence.md:67` | note | CONFIRMED / introduced | Both implemented stories still list "`rate` and `cost_usd` stay off the event" (ess#467) as Open. That was true at `69434ee`; this unit makes it false. | Anyone reading the planning store's open items. Only the coordinator may edit the store. |
| F2 | `tests/conformance.rs:602` | note | CONFIRMED / pre-existing | The shape check only tests that a key is present. The runner always renders a missing value as `null`, so the suite's `optional: true` and `kind: decimal` on `rate`/`cost_usd` are never checked, and the runner never fills the queue. The `measured`, `proposed` and `applied` scenarios therefore pass whatever these fields hold. The event schema says "absent" (not required, string type, no null). The binary-level tests do cover the values. | Same code at base for `SourceRan.cost_usd`. |

**5. Attacked and could not break**
- **Queue order:** the generated code reads `rate` and then `cost_usd`, matching the push order. A swap would turn the existing empty-store and 0.92 tests red.
- **Leftover values:** each process dispatches exactly one command (`src/main.rs:298`), and `generate_optional_decimal` is called only from the three spots in `behaviour.rs`. No refusal path fills the queue.
- **Printed form:** unchanged from before the unit. serde_json uses `arbitrary_precision`, so EKR's `rate` text passes through untouched, and `cost_decimal` uses the same `{c:.4}` as before.
- **Exponent form:** EKR 0.0.32 prints `rate`, `lower` and `upper` as plain decimals for every sample size from 1 to 1000 (smallest non-zero rate `0.001`, `lower` is clamped at `0.0`). I probed this with `probe-fq.sh`.
- **Cost rounding:** a cost such as 0.00005 rounds exactly as before the unit; no change.
- **Stale docs:** no `website/docs`, README, CHANGELOG or spec note still says the events cannot carry these fields.

**6. Paths written outside the worktree:** none. My scratch is inside the tree: `target/wave-scratch/adv/probe-fq.sh` and `target/wave-scratch/adv/tmp/{before,after}.log`. The lease I released was the worktree tool's own record.

**7. Findings block**
```findings
[
  {"file": ".engineering/planning/story/quality-judge.md", "line": 69, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "F1 quality-judge and schema-emergence (schema-emergence.md:67) still list rate and cost_usd staying off the event as Open, which this unit makes false; only the coordinator may edit the store."},
  {"file": "tests/conformance.rs", "line": 602, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "F2 the shape check only tests key presence on a payload the runner always renders with null, so the optional/decimal shape the suite now states for rate and cost_usd is never checked and the measured/proposed/applied scenarios pass whatever those fields hold."}
]
```
