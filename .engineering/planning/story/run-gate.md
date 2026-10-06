---
format: aep.planning-md/3
id: story:run-gate
kind: story
status: implemented
title: A run that fails its checks is undone
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:run-snapshots
- depends_on: story:spec-standalone-types
- depends_on: story:redaction-names-and-gate
scope:
- confidence: inferred
  path: src/gate.rs
- confidence: cited
  path: src/lib.rs
- confidence: cited
  path: src/run.rs
- confidence: inferred
  path: tests/run_gate.rs
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T23:46:20Z", actor: "agent:claude", revision: 8, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "proposed", to: "active", at: "2026-10-05T23:46:20Z", actor: "agent:claude", revision: 9, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "active", to: "implemented", at: "2026-10-06T00:23:50Z", actor: "agent:claude", revision: 11, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
## Outcome

A run that leaves the store worse than its own checks allow is undone automatically.

## Work

- `RunGate.checks`: each names a measure from the run report that `cortex run` prints
  (`facts_refused`, `documents_applied`, `documents_new`; `src/run.rs:36,250`) or from
  `ekr quality`, and a `min`/`max`.
- After apply, cortex evaluates the checks; a failing check restores the snapshot taken before the
  run through `snapshot::restore_held`, the in-process restore `story:run-snapshots` provides for a
  caller that already holds the instance lock, records the failure with the measure and value, and
  counts it as a failed run for the source.

## Acceptance

`tests/run_gate.rs`: with `checks: [{measure: facts_refused, max: 0}]` and a stand-in model whose
answer has 1 refused fact, the run reports the gate failure naming `facts_refused = 1`, and
`ekr head` answers the revision from before the run.

## Depends on

`story:spec-standalone-types` (the types), `story:run-snapshots` (`snapshot::restore_held`; both
edit `src/run.rs`), `story:redaction-names-and-gate` (its pre-model refusal and this post-apply
restore share the run path in `src/run.rs`).

## Scope

Landed 2026-10-06 in `b5a2364` (wave 20261006a, merged `a5c9894`; spec wording `fc45e02`).

- **Files:** `src/gate.rs` (new), `src/run.rs`, `src/lib.rs`, `src/snapshot.rs`, `tests/run_gate.rs` (new, 9 cases), `tests/snapshots.rs` (the busy case), `spec/domains/instance.yaml` (busy wording) and what it regenerates, `website/docs/spec-file.md`, `operating.md`
- **Decided during the wave:** a failed gate is the `apply-refused` outcome and counts toward `record-failure`; a restore writes into the live store through the online backup and checkpoints the `-wal`, busy only while a writer holds the lock; a refused undo still rewinds `state/`; bounds compared at 4 decimals
- **Documented limit:** a reader holding one read transaction open across a restore sees `store-replaced` until it closes it; cortex warns
- **Review:** `review-result:adversary-run-gate-pass-1` (4, one blocker), fixed; coordinator check of the correction
