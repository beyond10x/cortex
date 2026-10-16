---
format: aep.planning-md/3
id: story:gateway-dollar-limits
kind: story
status: draft
title: Behind a model gateway, runs are bounded in tokens and the reported dollar cost bounds nothing
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
- depends_on: story:token-budget
revision: 1
---
## Outcome

An instance whose `model.gateway` sends extraction to a self-hosted or otherwise unpriced model is
bounded in tokens only, and no run stops over a dollar spend that never happened:

1. With `model.gateway` given, `model.budget_usd` may be omitted (it stays required without a
   gateway, as now), and the cost `claude` reports for an answer is not counted against
   `budget_usd`, `daily_ceiling_usd` or a quality measurement's or schema proposal's dollar
   budget. `--max-budget-usd` is not passed to `claude`.
2. With `model.gateway` given, the spec check refuses (exit 2, naming the field, before anything is
   written, for `create`, `update` and `adopt` alike) a model with no `budget_tokens`: a gateway
   instance is never unbounded.
3. A gateway spec that still gives `budget_usd` or `daily_ceiling_usd` is accepted, and `create`,
   `update` and `adopt` print one line saying the dollar limits bound nothing behind a gateway.
4. The reported cost is still recorded in the run's report and `spend.json` as reported, marked as
   not counted, so an operator can read what `claude` claimed.

## Why

Reported by a downstream deployment that routes extraction through `model.gateway` to a
self-hosted model: `claude -p` prices an answer at its default model's rate (read from the binary's
price table, not observed), so `budget_usd` and `daily_ceiling_usd` can stop runs over spend that
never happened. The token limits of `story:token-budget` already bound such an instance; the dollar
limits only mislead it.

Verified in this tree:

- `cortex.instance.ModelSpec.budget_usd` is a required `Decimal`; `daily_ceiling_usd` optional.
- `src/run.rs` adds `answer.cost_usd` to the run's spend whatever the backend's route;
  `src/spend.rs` `Daily::add` adds it to the day.
- `src/extract.rs` passes `--max-budget-usd` to `claude` on every call.

## Domain relations

No new noun: `cortex.instance.ModelSpec.budget_usd` becomes `Optional<Decimal>`, with the rule
(required without a gateway, `budget_tokens` required with one) in its comment and in
`spec::check`.

## Work

- Spec first: `budget_usd` optional, the comment on `ModelSpec` and `ModelGateway` stating the
  rule; validate, conformance suite, `task generate`. Then the code.
- `src/spec.rs` check: the two refusals and the warning line.
- `src/run.rs`, `src/spend.rs`, the quality and schema paths: no dollar counting behind a gateway;
  the reported cost recorded as not counted.
- `src/extract.rs`: no `--max-budget-usd` behind a gateway.
- Docs: the spec-file page's `model` section, `website/docs/limits.md`, CHANGELOG.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- A gateway instance with `budget_tokens` and no `budget_usd` is created; a stand-in `claude`
  reporting `total_cost_usd` far above any dollar limit runs to the end of its batches, bounded by
  `budget_tokens` only, and the report shows the reported cost as not counted.
- The same with `budget_usd: 0.01` and `daily_ceiling_usd: 0.01` given: the run is not stopped by
  either, and `create` printed the warning line.
- A gateway instance with no `budget_tokens`: refused at `create`, naming `model.budget_tokens`.
- An instance without a gateway and without `budget_usd`: refused, naming `model.budget_usd`, as
  before; with `budget_usd`, dollar limits behave as in 0.5.1 (existing tests unchanged).
- The stand-in `claude` behind a gateway is never passed `--max-budget-usd`.

## Not in this story

- Pricing a gateway model's tokens in dollars (a price table per model).
- Any change to an instance without a gateway.
