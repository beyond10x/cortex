---
format: aep.planning-md/3
id: story:run-report
kind: story
status: draft
title: Every run writes a report of what it fetched, skipped, extracted, applied and rejected
summary: runs/<time>-<source>/run.json (cortex.run-report/1) and cortex report
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
- informed_by: story:run-failures-observable
- informed_by: story:run-cost-split
revision: 1
---
## Outcome

Every run of a source writes one run report, `runs/<time>-<source>/run.json` (`format`
`cortex.run-report/1`), saying what the run did end to end: documents fetched, unchanged, held,
skipped (with reasons), extracted, applied, facts refused, parts rejected (with EKR's codes), values
ended, masked, model calls, tokens, cost and seconds, and the outcome and reason. `cortex report
<instance> <time>-<source>` prints it, and `cortex report <instance> --last <source>` prints the
newest. A reader of the instance can then see per run and per source what was fetched, skipped,
extracted, merged and rejected, with cost and duration.

## Why

A downstream deployment wants ingestion transparency for its readers and has nothing yet. Most of
the numbers exist today but are spread out. Verified in this tree:

- A run's `cortex.log` line carries `documents_new`, `documents_applied`, `facts_refused`,
  `parts_rejected`, `masked`, `cost_usd`, `seconds`, `stopped`, the call and token counts, and,
  only when non-empty, `skipped`, `rejected`, `redacted`, `truncated`, `hashed` (`src/run.rs`,
  `log_line`). It holds no count of documents fetched or unchanged.
- Those counts (`fetched`, `new`, `changed`, `held`, `unchanged`, `unread`) are reported only by a
  fetch-only run (`src/main.rs`, the `fetched` outcome; `story:run-fetch-only`).
- `runs/<time>-<source>/batch-<n>/` keeps per model call the prompt, the answer, the extraction
  document and EKR's apply report (`src/run.rs`, `report.json`; `website/docs/operating.md`), but
  nothing at the run's level.
- `status/<source>.json` and `metrics.prom` hold the last outcome and counts by outcome
  (`src/status.rs`; `story:run-failures-observable`), not what a run read.

Related, not repeated: `story:run-cost-split` (the token split a report carries),
`story:run-failures-observable` (status, metric, hook). The HTTP form `GET /runs/{id}/report` the
deployment sketched would be served by `cortex ask serve` once `story:ask-service` lands; this
story makes the document it would serve.

## Domain relations

No new entity in this story: a run is still named by its directory. New value type
`cortex.instance.RunReport` (`cortex.run-report/1`), with the fields above, in
`spec/domains/instance.yaml`; the command `cortex.instance.ShowRunReport` with outcomes
`reported`, `no-such-run`, `no-such-instance`. Whether a run should become an entity owned by
`cortex.instance.Source` is left open (`UNMAPPED: Run as an entity`).

## Work

- Spec first: `RunReport` and `ShowRunReport`; validate, conformance, `task generate`. Then the
  code.
- `src/run.rs`: count fetched and unchanged on every run (the fetch-only path already counts them),
  write `run.json` beside the batches as the `cortex.log` line is written, on every outcome
  including a failed one.
- `src/main.rs`: `cortex report`.
- Docs: `commands.md` and `operating.md`'s instance directory table.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- A run of a files source with three records, one unchanged since the last run and one refused by
  the stand-in EKR, writes a `run.json` with `fetched: 3`, `unchanged: 1`, `applied: 1`, one
  `rejected` entry with EKR's code, and `cost_usd` and `seconds` equal to its `cortex.log` line.
- A run that fails at fetch writes `run.json` with `outcome: fetch-failed` and its reason.
- `cortex report <name> --last <source>` prints the newest report; an unknown run answers
  `no-such-run` with exit status 1.
- A pseudonymised run's `run.json` holds no placeholder's original text (the redaction tests'
  known names do not occur in it).

## Not in this story

- Serving reports over HTTP (`story:ask-service`, after the record is accepted).
- Live counts across the instance (`story:vitals-and-topics`).
