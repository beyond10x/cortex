---
format: aep.planning-md/3
id: story:extraction-links-facts
kind: story
status: draft
title: Extraction produces edges between the nodes it finds
relations:
- decomposes: epic:first-web-instance
- serves: vision:self-updating-instances
- depends_on: story:ci-runs-task-check
revision: 1
---
## Outcome

Extraction produces edges between the nodes it finds, not only properties on them.

## Starting point

The smoke store holds 14 nodes, 22 assertions and 0 edges from 8 web documents. The prompt and the
JSON schema the model answers in are built in `src/extract.rs`; the schema comes from
`ekr schema ekr.extraction-document/1` minus evidence and `$schema`.

## Work

- Find why no edge is produced: the schema admits edge facts, the prompt does not ask for them, or
  the seed schema declares no edge types (`examples/seed/schema.yaml`). Record which, with the
  model's raw answer for one document.
- Fix that cause. A fixed set of 8 documents (saved from the smoke run, not re-fetched) is the
  benchmark.

## Acceptance

On the same 8 documents, the extraction applies at least 1 edge per document on average, the
per-run model cost stays under $0.15, and `ekr view` shows the edges. Before/after counts recorded
as `test_result` evidence.

## Scope

`src/extract.rs`, `examples/seed/schema.yaml`, a fixture directory under `tests/`.
