---
format: aep.planning-md/3
id: story:redaction-before-model
kind: story
status: draft
title: Personal data is replaced before the model sees a document
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
- depends_on: story:spec-standalone-types
- depends_on: story:release-pipeline
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: src/lib.rs
- confidence: inferred
  path: src/redact.rs
- confidence: cited
  path: src/run.rs
- confidence: inferred
  path: tests/redaction.rs
revision: 9
---
## Outcome

Personal data in a fetched document is replaced before the model sees it, by the classes and rules
the instance's `redaction` policy names.

## Work

- `src/redact.rs`: built-in classes `Email`, `Phone`, `IpAddress`, `PaymentCard` and the spec's
  regex rules, each replaced by its `replacement` (default `[REDACTED:<class>]`).
- `src/run.rs`: apply redaction after credential masking (`mask::mask`) and before the batch is
  built, so the stored evidence payload and the prompt carry the redacted text, never the original.
- A run's report counts redactions per class.
- AGENTS.md's "no PII redaction" hard rule changes to name the policy and its limits (pattern-based;
  names of people are not detected).

## Acceptance

A unit test per class and for a custom rule, and an e2e test whose stand-in model receives a prompt
in which a planted email address, phone number and card number are absent and the run report counts
3 redactions.

## Depends on

`story:spec-standalone-types`.

## Scope

`src/redact.rs` (new), `src/lib.rs`, `src/run.rs`, `tests/redaction.rs` (new), `AGENTS.md`.
