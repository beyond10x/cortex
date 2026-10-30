---
format: aep.planning-md/3
id: story:extraction-cache-by-content
kind: story
status: draft
title: A document's extraction is reused when its content did not change
summary: Extractions are cached by content hash across runs, so an abandoned gated run does not pay for the same documents again
tags:
- review-20261030
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
revision: 1
---
## Outcome

A document's extraction is not paid for twice when its content did not change. Today an abandoned
gated run (gate refusal, SIGTERM at the deadline, an unknown write) rewinds all of `state/` from
the stage record (`src/snapshot.rs:600-617`, `:640-642`; `src/run.rs:1268-1272`, `:1585-1606`),
as the spec's all-or-nothing rule asks (`spec/domains/instance.yaml:1245-1250`), and every batch's
model work is thrown away: downstream, 35 documents went to the model four times in one day
(cb3 report 2026-10-30, C-05).

Decision taken (2026-10-30, coordinator, from the verifier's options): cache each document's
extraction by content hash across runs and mint fresh evidence when it is reused. The other
options were (a) gate and publish what a terminated run applied, which changes
`instance.yaml:1245-1250`, and (c) publish the stage per gated batch. Option (b) keeps the run's
meaning and only saves model work.

## Acceptance

1. A gated run abandoned after two batches, then run again on unchanged documents: the second run
   makes no model call for those documents and applies them with new evidence ids (new test).
2. A changed document misses the cache.
3. The cache is bounded and lives under the instance's `runs/` or `state/` (named in the spec).

## Not in wave 20261030a

Size L and it adds a store of its own; it follows the wave.
