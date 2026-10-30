---
format: aep.planning-md/3
id: story:apply-failures-count-toward-skip
kind: story
status: draft
title: A document whose EKR apply fails on consecutive runs is skipped like a refused one
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
revision: 2
---
## Outcome

A document whose extraction is answered but whose EKR apply fails the same way on consecutive
runs is counted toward the poison-document skip, as an extraction refusal is. Today the
per-document count in `failures/<source>.json` is raised only on `model_failed_on_request`
(`src/run.rs:1291`, `:1311`), so a document EKR refuses on every apply blocks its batch every run.

## Acceptance

- With a stand-in EKR that refuses the apply of document X on every run, run N+1
  (N = `DOCUMENT_FAILURE_LIMIT`) applies every other document and lists X in `skipped` with the
  apply reason and its failure count (new test in `tests/poison_document.rs`).
- An apply failure whose outcome is unknown (store-outcome-unknown) counts nothing, as today
  (existing tests green).
- X is asked again after its content hash changes.

## Source

- Wave 20261030a coordinator review of the unit reports (unit fail).
- `story:poison-document-skipped` Decisions: only lone extraction failures count.
