---
format: aep.planning-md/3
id: story:record-field-lookups
kind: story
status: draft
title: A file record's fields can be mapped through a lookup file
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:file-records
scope:
- confidence: inferred
  path: generated
- confidence: inferred
  path: spec/domains/instance.yaml
- confidence: inferred
  path: src/model_map.rs
- confidence: inferred
  path: src/sources.rs
- confidence: inferred
  path: tests/file_records.rs
revision: 4
---
## Outcome

A `records` spec can map a field through a JSON lookup file (an author id to a display name, inline `<@id>` mentions in text) and take the first non-empty of several text fields (for example a message's text, then its block or attachment text).

## Why

Found 2026-10-06 by the gap inventory for running Company Brain v3 entirely on cortex and EKR (cb3 `initiative:run-on-cortex`); no story covered it.
Chat exports carry user ids, not names; without a lookup the model sees `U1` instead of a person.

## Acceptance

`tests/file_records.rs`:
1. A JSON-lines record with author `U1` and text `hi <@U2>`, with a lookup `{U1: "Ana", U2: "Ben"}`, yields the document text `Ana: hi @Ben`.
2. A record whose `text` is empty and whose second listed text field holds `from blocks` yields `from blocks`; one with both empty yields no document and is named in `skipped`.

## Left out

`[file: …]` markers for attachments and a minimum message length are not ported; an existing filter can drop short records.

## Depends on

`story:file-records` (it owns `FileRecords` and `tests/file_records.rs`).

## Files (from the inventory, unverified)

`spec/domains/instance.yaml` (`FileRecords.lookups`), `generated/`, `src/model_map.rs`, `src/sources.rs`, `tests/file_records.rs`.
