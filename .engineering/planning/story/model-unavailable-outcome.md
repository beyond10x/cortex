---
format: aep.planning-md/3
id: story:model-unavailable-outcome
kind: story
status: active
title: A run that cannot reach its model ends model-unavailable and is not counted against the source
summary: RunOutcome model-unavailable, exit 7, not counted by --record-failure
relations:
- serves: vision:self-updating-instances
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-17T15:56:20Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-17T15:56:21Z", actor: "human:timo", revision: 3}
---
## Outcome

A run whose model cannot be reached (connection refused, the model not loaded, the gateway down)
ends with the outcome `model-unavailable` and exit status 7. `cortex run --record-failure` does not
count it against the source, so repeated model outages never disable a healthy source. A wrapper
tells it apart from a source failure by the exit status alone.

## Why

Today such a run ends `extraction-failed` (exit 1), and `--record-failure` counts it against the
source: a model outage of an hour suspends every source that ran in it. Verified in this tree:
`cortex.instance.RunOutcome` (`spec/domains/instance.yaml:613`) has no variant for it, and the
`RunSource` exit statuses use 0 to 6 (`spec/domains/instance.yaml:1552`).

## Domain relations

- `cortex.instance.RunOutcome` gains `model-unavailable`; an error `cortex.instance.ModelUnavailable`
  carries the cause (`refused | not-loaded | gateway-down`), never a prompt or a credential.
- `RecordFailure` leaves the failure counter of the source unchanged on `model-unavailable`, as
  for `store-outcome-unknown`.

## Work

- Spec first: the variant, the error and the `RunSource` outcome with its exit status; validate,
  conformance, `task generate`. Then the code: classify the model call's failure in
  `src/extract.rs` (spawn refused, gateway HTTP 502/503/504 or connection refused through
  `model.gateway`, the gateway's model-not-loaded answer) before any batch is applied.
- `metrics.prom` counts it under its own outcome; the hook is called with it.
- Docs: `operating.md` exit statuses, `limits.md`.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- With a stand-in gateway that refuses connections, `cortex run --record-failure` exits 7, prints
  `model-unavailable`, and the source's consecutive failures are unchanged after three such runs.
- With a stand-in model that answers invalid output, the run still ends `extraction-failed`, exit
  1, and is counted.
- A batch applied before the model became unavailable stays applied; the outcome is still
  `model-unavailable`.

## Not in this story

- Retrying the model call inside a run.
- Run metrics beyond the outcome count (`story:run-metrics`).
