---
format: aep.planning-md/3
id: story:review-ekr-pin-and-busy
kind: story
status: implemented
title: The EKR pin is the created store's engine, and a busy collection can be skipped
summary: Findings 17 and 18 of the 0.7.2 review
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T01:54:00Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T01:54:00Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-24T00:21:04Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

An instance records the EKR version of the binary that created its store, and an external
scheduler can start a run that returns at once when the collection is busy.

## Work

Findings 17 and 18 of the external review of cortex 0.7.2 (2026-11-23):

- 17: a spec declaring EKR 0.0.34 with an explicit binary that reports 0.0.32 creates a store and
  lists the engine as 0.0.34; a 0.0.32 binary in the 0.0.34 cache passes setup (`src/ekr.rs`,
  `src/main.rs`). Check the executable's version before accepting a cache hit or recording a pin.
- 18: under a held home lock a normal run waits; the container collection plan
  (`epic:periodic-collection-reconcile`, `story:scheduler-one-shot-run`) needs a distinct
  lock-held result. Add a non-waiting run mode with its own exit status.

## Acceptance

A test per finding reproduces the reviewed scenario and passes on this change;
`website/docs/commands.md` names the new run mode and its exit status.
