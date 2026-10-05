---
format: aep.planning-md/3
id: story:structured-source
kind: story
status: implemented
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
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T14:13:37Z", actor: "agent:claude", revision: 10, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-05T14:13:37Z", actor: "agent:claude", revision: 11, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-10-05T14:48:54Z", actor: "agent:claude", revision: 13, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
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

Landed 2026-10-05 in `8a96919` (wave 20261005e).

- **Files:** `src/structured.rs` (new), `src/run.rs`, `src/sources.rs`, `src/lib.rs`, `tests/structured.rs` (new, 22 cases), `tests/e2e.rs` (its refusal message moved to the files story), `website/docs/spec-file.md`, `website/docs/operating.md`
- **Decided during the wave:** identity `<adapter>:<operation>:<id>` (a digest when the mask or a replacement rule would change the id); aliases `<name> (<identity>)`, the identity and the mapped aliases, so same-named records stay apart; a record with a part EKR rejects is not marked seen; values over 65,536 bytes are rejected by EKR 0.0.30
- **Documented limit:** a changed value stays beside the old one until EKR supersession is released
- **Review:** `review-result:adversary-structured-source-pass-1` (7), fixed or documented; coordinator check of the correction
