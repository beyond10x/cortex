---
format: aep.planning-md/3
id: story:store-outcome-unknown-not-counted
kind: story
status: active
title: An unknown store outcome fails a run without counting against the source
summary: An EKR stage verb whose outcome is unknown fails the run as store-outcome-unknown, exit 4, and is not recorded as a source failure
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T11:50:40Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-16T11:50:41Z", actor: "human:timo", revision: 4}
---
## Outcome

A source run that ends because EKR cannot say whether a stage verb took effect (`ekr stage begin`
or `ekr stage publish` answering "publication outcome is unknown") fails with its own outcome,
`store-outcome-unknown`, and its own exit status, and is not counted as a failure of the source:
`cortex run --record-failure` sends no `RecordFailure`, so `consecutive_failures` does not move and
the source is never disabled by it. The source's status, metric and alert hook still report it.
The next run settles what the stage left, as it settles the stage of a run that was killed.

## Why

A run on a PostgreSQL store held to a gate begins an EKR stage after its fetch and before any
apply. On a downstream deployment the first run's `ekr stage begin` answered "publication outcome
is unknown; resolve the exact retained occurrence", while EKR had recorded the stage Begun. cortex
counted that as `apply-refused` and recorded a failure (`src/ports.rs`, `RecordFailure`: the second
failure in a row disables the source). Nothing of the source was applied and nothing about the
source was wrong; a second such store fault would switch off a healthy source until an operator
runs `cortex source enable`.

Verified in this tree:

- `Staged::begin` maps any `begin_stage` error to `Failure::Apply` (`src/run.rs`), which
  `run_source` records as a failure when `--record-failure` is given (`src/main.rs`).
- `Staged::publish` on any publish error rewinds `state/` and abandons the stage; with an unknown
  outcome the publish may have landed, and the abandon is then refused `stage-already-published`.
- Every run starts with `Staged::recover`, which completes a publish recorded as in progress
  (`publishing`) at the same head, and abandons every stage of the store still `Begun`.

## Domain relations

No new noun: a `store-outcome-unknown` outcome of `cortex.instance.RunSource` with its error
`cortex.instance.StoreOutcomeUnknown { reason }`, and the value in `cortex.instance.RunOutcome`.

## Work

- Spec first: the outcome, its error and the `RunOutcome` value in `spec/domains/instance.yaml`,
  its summary saying the scheduler sends no `RecordFailure`; validate, conformance suite,
  `task generate`. Then the code.
- `src/ekr.rs`: tell an unknown outcome from a refusal by what EKR prints for it (the message
  "publication outcome is unknown", or a stable code or exit status if EKR has one).
- `src/run.rs`: an unknown outcome at `stage begin` fails the run as the new outcome; at
  `stage publish` it keeps `stage.json` with `publishing` set, neither rewinding `state/` nor
  abandoning, so the next run's `recover` completes or settles it. An unknown outcome while
  `recover` settles a stage is the same outcome.
- `src/main.rs`: the outcome's exit status (4, distinct from a failed run's 1 and a disabled
  source's 3), no `RecordFailure`; status, metric and hook report it.
- Docs: the operating page's exit statuses and failures.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- A stand-in `ekr` whose `stage begin` fails with "publication outcome is unknown": two runs with
  `--record-failure` each exit 4 with outcome `store-outcome-unknown`; the source stays enabled
  with `consecutive_failures` 0, and its status file names the outcome.
- A stand-in whose `stage publish` answers the unknown outcome after committing: the run exits 4,
  `stage.json` stays with `publishing` set and `state/` is not rewound; the next run publishes it
  at the same head and runs.
- Every other `apply-refused` is still counted, and the second disables the source
  (existing tests unchanged and green).

## Not in this story

- Resolving the retained occurrence inside EKR.
- Counting fetch or extraction failures differently.
