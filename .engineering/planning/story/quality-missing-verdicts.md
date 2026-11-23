---
format: aep.planning-md/3
id: story:quality-missing-verdicts
kind: story
status: active
title: cortex quality retries missing verdicts and never counts them as failures
summary: Defect 2 of the runbook refresh probe
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T05:01:56Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T05:01:57Z", actor: "human:timo", revision: 3}
---
## Outcome

`cortex quality` reports a fact the judge gave no verdict for as unjudged, retries it, and keeps
what it sent and got back.

## Work

Consumer: the runbook refresh (session `bf-runbook`), 2026-11-23: one `cortex quality --sample 60`
got no verdict for 39 of 60 facts and counted them as failures (28 % supported); the repeat gave
90 % with 1 of 40 missing. Nothing was retried, and the batch folders were empty.

- A fact without a verdict is retried once in a later judge call; still without one, it is
  reported as `unjudged` and left out of the supported rate, which names its denominator.
- The judge's prompt and raw answer are kept in the quality run's folder.

## Acceptance

A stand-in judge that omits verdicts for some facts on the first call and answers them on the
second gives a rate over all facts; one that never answers them reports them `unjudged`, outside
the rate; the folder holds the prompt and the answer.
