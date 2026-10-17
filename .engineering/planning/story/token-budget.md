---
format: aep.planning-md/3
id: story:token-budget
kind: story
status: implemented
title: A run budget and a daily ceiling in tokens, for a model whose cost is unknown
summary: budget_tokens and daily_ceiling_tokens bound runs by the token counts a run reports
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-15T15:50:11Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-15T15:50:11Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-17T03:36:58Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

An instance whose model's cost `claude` cannot report still has a run budget and a daily ceiling:
`model.budget_tokens` bounds the input and output tokens of one run, and
`model.daily_ceiling_tokens` the tokens of an instance per UTC day, counted from the token counts a
run reports since `story:run-cost-split`. A run that reaches either stops before its next batch,
keeps what it applied, and says which limit stopped it, as the dollar limits do.

## Why

With a model behind a gateway, `claude`'s `total_cost_usd` is not the gateway's real cost: it is a
figure from `claude`'s own price table or no figure at all (unverified for an unpriced model id).
Verified in this tree:

- An answer with no `total_cost_usd` is an unknown cost (`src/extract.rs`, `cost_usd` read as a
  number). The run's budget adds it as `0` (`src/run.rs`, `spent += answer.cost_usd.unwrap_or(0.0)`),
  so `budget_usd` never stops such a run.
- `Daily::add(None)` adds nothing (`src/spend.rs`, held by its unit test), so
  `model.daily_ceiling_usd` is never reached.
- `--max-budget-usd` is passed to `claude`, which enforces it against its own figure only.
- Since `story:run-cost-split`, an answer's `usage` is read and a run reports `input_tokens`,
  `cache_read_input_tokens`, `cache_creation_input_tokens` and `output_tokens`; a count is `null`
  when an answer did not report it.

## Domain relations

No new noun: optional Integer `budget_tokens` and `daily_ceiling_tokens` on
`cortex.instance.Model`; the day's tokens beside `spent_usd` in `spend.json`.

## Work

- Spec first: add both fields with a comment stating what counts (input, cache reads, cache writes
  and output tokens) and that an answer with no token count stops the run when a token limit is set;
  validate, conformance suite, `task generate`. Then the code.
- `src/run.rs`: before each batch, check the run's tokens against `budget_tokens`; `src/spend.rs`:
  record the day's tokens under the home's lock as the spend is recorded, and check
  `daily_ceiling_tokens` in `Daily::allow`. An answer with no token count, under a token limit,
  stops the run before its next batch with a reason saying the tokens could not be counted.
- `quality` and `schema` count against the daily token ceiling as they do against the dollar one.
- Docs: the spec-file and operating pages; `limits.md` on what `claude` reports for an unpriced model.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- `budget_tokens: 3000` and a stand-in `claude` answering 2000 tokens per call and no
  `total_cost_usd`: a run with three batches makes two calls and stops with a reason naming the
  token budget; its report carries the tokens of both calls.
- `daily_ceiling_tokens: 5000`: two runs of one instance on one UTC day, 2000 tokens per call, stop
  after the third call in all, the second run's reason naming the daily token ceiling; the next UTC
  day starts from 0.
- Under a token limit, an answer with no `usage` stops the run before its next batch, the reason
  saying the tokens could not be counted.
- Without a token limit, runs behave as before (`tests/cost.rs` unchanged and green).

## Not in this story

- Pricing tokens in dollars per model.
- The gateway itself (`story:model-through-gateway`).
