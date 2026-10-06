---
format: aep.planning-md/3
id: story:record-field-lookups
kind: story
status: draft
title: A file record's fields can be mapped through a lookup file
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
scope:
- confidence: inferred
  path: spec/domains/instance.yaml
- confidence: inferred
  path: src/sources.rs
- confidence: inferred
  path: tests/file_records.rs
revision: 2
---
## Outcome

A `records` spec can map a field through a JSON lookup file (an author id to a display name, inline `<@id>` mentions in text) and pick the first non-empty of several text fields.

## Why

Found 2026-10-06 by the gap inventory for running Company Brain v3 entirely on cortex and EKR (cb3 `initiative:run-on-cortex`); no story covered it.
Chat exports carry user ids, not names; without a lookup the model sees `U1` instead of a person.

## Acceptance

`tests/file_records.rs`: a JSON-lines record with author `U1` and text `hi <@U2>`, with a lookup `{U1: "Ana", U2: "Ben"}`, yields the document text `Ana: hi @Ben`.

## Files (from the inventory, unverified)

`spec/domains/instance.yaml` (`FileRecords.lookups`), `src/sources.rs`, `tests/file_records.rs`.
