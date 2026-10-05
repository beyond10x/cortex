---
format: aep.planning-md/3
id: story:structured-source
kind: story
status: draft
title: A structured source imports records without a model call
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
- depends_on: story:spec-standalone-types
- depends_on: story:redaction-before-model
- depends_on: story:connectors-source-walks
scope:
- confidence: cited
  path: src/lib.rs
- confidence: cited
  path: src/run.rs
- confidence: cited
  path: src/sources.rs
- confidence: inferred
  path: src/structured.rs
- confidence: inferred
  path: tests/structured.rs
revision: 9
---
## Outcome

A `structured` source turns records from a Connectors operation into entities, properties and
relations by its `mapping`, with no model call and no model cost.

## Work

- `src/sources.rs`: fetch records as the `connectors` source does (`records` path).
- `src/structured.rs`: build an `ekr.extraction-document/1` from each record through
  `RecordMapping` (node type, id, name, aliases, property paths, relations to named targets); each
  fact cites the evidence id cortex issued for that record.
- `src/run.rs`: a structured batch goes to `apply-extraction` directly; redaction still applies to
  text fields.

## Acceptance

An e2e test whose stand-in `connectors` answers 3 records and whose stand-in `claude` fails if
called: the run applies 3 nodes with their mapped properties and 1 mapped relation, reports cost 0,
and the stand-in model log is empty.

## Depends on

`story:spec-standalone-types`, `story:redaction-before-model` (both edit `src/run.rs`).

## Scope

`src/structured.rs` (new), `src/sources.rs`, `src/run.rs`, `src/lib.rs`, `tests/structured.rs` (new).
