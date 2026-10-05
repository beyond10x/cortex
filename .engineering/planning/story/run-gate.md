---
format: aep.planning-md/3
id: story:run-gate
kind: story
status: draft
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
revision: 7
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

`src/gate.rs` (new), `src/run.rs`, `src/lib.rs`, `tests/run_gate.rs` (new).
