---
format: aep.planning-md/3
id: story:document-time-as-valid-time
kind: story
status: draft
title: A fact is dated by when its source said it
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:file-records
- depends_on: story:spec-standalone-types
- depends_on: story:codex-model-backend
scope:
- confidence: cited
  path: src/evidence.rs
- confidence: cited
  path: src/extract.rs
- confidence: inferred
  path: tests/document_time.rs
revision: 5
---
## Outcome

A fact is dated by when its source said it: the document's time is the evidence's observed time and
the fact's valid-from time.

## Work

- `src/evidence.rs`: the evidence entry for a document carries the document's `published` /
  record `time` as its observed time.
- `src/extract.rs`: the extraction document asks EKR to take valid time from evidence time, once
  `upstream-blocker:ekr-extraction-valid-time` clears.

## Acceptance

`tests/document_time.rs`: a `files` source with two JSON-lines records dated 2026-01-02 and
2026-03-04 yields facts whose `valid_time.from` in `ekr snapshot` equal those instants.

## Depends on

`story:spec-standalone-types` (the types), `upstream-blocker:ekr-extraction-valid-time`,
`story:file-records` (record times come from it), `story:codex-model-backend` (both edit
`src/extract.rs`).

## Scope

`src/evidence.rs`, `src/extract.rs`, `tests/document_time.rs` (new).
