---
format: aep.planning-md/3
id: story:partial-run-counts
kind: story
status: draft
title: Documents applied and tokens spent count for every run outcome
summary: metrics.prom adds a partial run's documents and tokens, not only ran
relations:
- serves: vision:self-updating-instances
revision: 1
---
## Outcome

`metrics.prom` counts the documents a run applied and the model tokens it spent whatever outcome
the run ended with: a run that applied batches and then ended `model-unavailable`,
`batch-too-large`, `terminated` or `extraction-failed` adds them to
`cortex_source_documents_total{kind}` and `cortex_source_model_tokens_total{kind}`, and its
`cortex_source_last_run_documents` reports them.

## Why

`story:run-metrics` promises the tokens a run spent, but the counts are added only for a run that
ends `ran` (`src/main.rs`, `src/status.rs` `Status::add`); the spec says so. A run that applied a
batch and then lost its model leaves both series under the truth, so a dashboard of spend or of
documents applied misses exactly the runs an operator is looking at. Verified by
`tests/runout_adversary.rs::a_partial_model_unavailable_run_counts_the_tokens_it_spent_and_documents_it_applied`
(ignored: `documents_total{kind="applied"}` stayed 0 after one applied document).

## Domain relations

- No new noun. The metric paragraph of `RunSource` counts documents and tokens for every outcome
  that names a source, not only `ran`.

## Work

- Spec first: the paragraph; validate, `task generate`, `task drift`.
- Code: each failure that ends a run after a batch carries the run's report (documents read and
  applied, tokens) to the status write; `Status::add` adds them for every outcome.
- Docs: `website/docs/operating.md` metric table.

## Acceptance

- `ess specify validate` passes and `task drift` is clean.
- The ignored case above runs and passes.
- A run that fails before any batch adds 0 to every series, as today.
- `metrics.prom` stays valid Prometheus text (the adversary's mixed-outcome case unchanged).

## Not in this story

- Counting tokens of a model call ended by SIGTERM, whose usage `claude` never reported.
