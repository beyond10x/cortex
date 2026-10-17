---
format: aep.planning-md/3
id: story:run-metrics
kind: story
status: active
title: A source's metrics say what each run did, and a terminated run is counted
summary: Prometheus series for run duration, documents read and applied, model tokens and failure reason, and a final write on SIGTERM; no credential, prompt or content in any series
relations:
- decomposes: epic:periodic-collection-reconcile
- depends_on: story:run-failures-observable
- serves: vision:self-updating-instances
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-17T15:56:26Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-17T15:56:26Z", actor: "human:timo", revision: 4}
---
## Outcome

With `monitoring.metrics`, `instances/<name>/metrics.prom` carries, per source, what each run did
as well as how it ended: the last run's duration, the documents it read and applied, the model
tokens it spent (input, output, cached), and its failure reason as the outcome label it already
uses. A run ended by SIGTERM (a Kubernetes Job's deadline, a `systemctl stop`) still writes its
status and its metrics before it exits, under a new outcome `terminated`. No series and no label
holds a credential, a prompt, a question, or document content: labels are the source id and fixed
outcome names only.

## Why

A downstream deployment runs cortex from a Kubernetes Job and alerts on Prometheus. Verified in
this tree: the metric file holds two series per source, `cortex_source_runs_total` by outcome and
`cortex_source_last_run_timestamp_seconds` (`src/status.rs:224-260`,
`website/docs/operating.md` "Failures"); duration, documents and tokens are in the run's JSON
line only, and cortex installs no SIGTERM handler, so a run its Job deadline kills records nothing
and the file counts it nowhere.

## Domain relations

The run report the spec already models (`cortex.instance.SourceRan`, `cortex.instance.RunFailed`)
carries the counts; the metric names, their types and the `terminated` outcome go into
`spec/domains/instance.yaml` (`cortex.instance.RunOutcome` gains `terminated`). The status file
(`instances/<name>/status/<source>.json`) keeps the last run's figures so the whole file can be
rewritten from it, as today.

## Work

- Spec first: the outcome, the figures the status keeps, validate, `task generate`, conformance.
- New series: `cortex_source_last_run_duration_seconds`, `cortex_source_last_run_documents{kind="read|applied"}`,
  `cortex_source_documents_total{kind}`, `cortex_source_model_tokens_total{kind="input|output|cached"}`.
- A SIGTERM handler for `cortex run`: stop the run at the next safe point (or abandon an open
  stage as the run's undo does), record `terminated`, write status and metrics, exit non-zero.
- The metric list and the SIGTERM behaviour on `operating.md`.

## Acceptance

- After one run of a stand-in source, the file holds the new series with the run's figures.
- A run sent SIGTERM mid-extraction exits within a bounded time, and the file then counts one
  `terminated` run for its source.
- A test over the file of a run whose documents hold a known secret and a known prompt finds
  neither in any line.
