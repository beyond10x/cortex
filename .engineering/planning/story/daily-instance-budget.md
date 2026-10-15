---
format: aep.planning-md/3
id: story:daily-instance-budget
kind: story
status: active
title: An instance stops model calls at a daily spend ceiling across its sources
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-15T03:29:15Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-15T03:29:15Z", actor: "human:timo", revision: 3}
---
## Outcome

An instance can carry a daily model-spend ceiling across all its sources: once the spend of the day
reaches it, no run of that instance makes another model call that day, and the run says so.

## Why

Verified in this tree:

- "The budget is per run. `budget_usd` limits one run of one source, not a day or an instance"
  (`website/docs/limits.md:121`).
- `budget_usd` is a field of `cortex.instance.ModelSpec` (`spec/domains/instance.yaml:282-288`,
  `:286`); a run reads it (`src/run.rs:573`), counts what costed answers spent (`:576`, `:642-644`)
  and stops before the next batch once it is spent (`:595-599`, `stopped: budget of <n> USD spent`).
  The run cost is written to `cortex.log` (`src/run.rs:1164`), and nothing sums it across runs.
- Quality measurement and schema proposals spend up to the same `budget_usd` per call
  (`spec/domains/instance.yaml:1272`, `:1344`).

## Domain relations

- `Instance -> Source`, one-to-many, the instance owns its sources: `spec/domains/instance.yaml`,
  `cortex.instance.Instance` relation `sources` (`:541-545`). The ceiling is the instance one and
  covers every source it owns.

## Work

- Spec first: add the ceiling as an optional field of the instance model settings, the record of the
  spend of the day per instance (a new noun or field, with its day boundary stated), and the outcome
  or report field of a run stopped by it; validate with `ess specify validate` and the conformance
  suite, then `task generate`. The ceiling field carries a number only, no credential.
- Before each model call (extraction, and the quality and schema calls), the remaining allowance is
  the lesser of the run budget and the ceiling minus the spend of the day; spend is recorded after
  each costed answer, under the home lock.
- Document the ceiling in the spec-file page and remove the limit at `website/docs/limits.md:121`.

## Acceptance

- With a ceiling whose spend of the day is already reached, `cortex run` of any source of that
  instance ends without a model call, and its report names the daily ceiling.
- A run that reaches the ceiling between batches stops before the next batch, with the batches
  before it applied, and its report names the daily ceiling.

## Not in this story

Choosing the ceiling of any deployment; alerting on an overspend beyond reporting it
(`story:run-failures-observable` shows the outcome); budgets across instances.
