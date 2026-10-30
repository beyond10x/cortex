---
format: aep.planning-md/3
id: story:timeout-document-skipped
kind: story
status: draft
title: A document that always times out the model is skipped instead of blocking its source
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
revision: 2
---
## Outcome

A document that makes the model run past `timeout_s` on every run is eventually skipped instead of
blocking its source on every run, while a model outage still marks and skips no healthy document.
Today a timeout or crash counts nothing (`answered: false`), so such a document stops every run at
its batch and is never skipped (pre-existing in 0.6.1).

## Acceptance

- With a stand-in model that times out only on document X, and answers every other document, X is
  listed in `skipped` after a bounded number of runs and the other documents apply (new test).
- With a stand-in model that times out on every document over the same number of runs, no
  document is skipped (`adversary_a_model_timing_out_on_consecutive_runs_skips_no_healthy_document`
  stays green).

## Source

- `review-result:adversary-w30a-fail-pass-2`, finding 2 (`src/extract.rs`,
  `Refused::model_failed_on_request`), origin pre-existing.
- Wave 20261030a coordinator review of the unit reports.
