---
format: aep.planning-md/3
id: story:record-field-lookups
kind: story
status: implemented
title: A file record's fields can be mapped through a lookup file
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:file-records
scope:
- confidence: cited
  path: generated
- confidence: cited
  path: spec/domains/instance.yaml
- confidence: inferred
  path: spec/suite.json
- confidence: cited
  path: src/model_map.rs
- confidence: cited
  path: src/sources.rs
- confidence: cited
  path: tests/file_records.rs
- confidence: cited
  path: website/docs/reference/ess/cortex-instance.md
- confidence: cited
  path: website/docs/spec-file.md
- confidence: cited
  path: website/static/schemas/instance-spec.schema.json
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T02:11:55Z", actor: "agent:claude", revision: 9, decided_on: {"recorded":{"review_outcome":5}}}
- {from: "proposed", to: "active", at: "2026-10-06T02:15:00Z", actor: "agent:claude", revision: 10, decided_on: {"recorded":{"review_outcome":5}}}
- {from: "active", to: "implemented", at: "2026-10-06T02:33:11Z", actor: "agent:claude", revision: 12, decided_on: {"recorded":{"test_result":1,"review_outcome":6}}}
---
## Outcome

A `records` spec can map the author field and inline `<@id>` mentions through a JSON lookup file (an author id to a display name), and can name fallback text templates that are used when its `text` templates render empty.

## Why

Found 2026-10-06 by a gap inventory for running a downstream company deployment entirely on cortex and EKR; no story covered it.
Chat exports carry user ids, not names; without a lookup the model sees `U1` instead of a person.

## Design (decided 2026-10-06)

- `FileRecords.lookup: Optional<String>`: the path of a JSON object file (id to name), resolved against the source's base the way `paths` is. It applies to the `author` value and to every `<@id>` in the rendered text. An id missing from the file stays as it is.
- A missing or unreadable lookup file, or one that is not a JSON object of strings, fails the source's fetch with an error naming the file.
- `FileRecords.fallback_text: List<String>`: templates tried in order only when `text` renders empty; the first non-empty one is the document text. `text` keeps its meaning (every non-empty template, joined by a blank line).
- A record whose `text` and every fallback render empty is skipped without a document, as an empty `text` is today (`src/sources.rs:842`).

## Acceptance

`tests/file_records.rs`:
1. A JSON-lines record with author `U1` and text `hi <@U2>`, with a lookup `{U1: "Ana", U2: "Ben"}`, yields the document text `Ana: hi @Ben`.
2. A record whose `text` renders empty and whose `fallback_text` holds `from blocks` yields `from blocks`; one with both empty yields no document.
3. A spec naming a lookup file that does not exist fails the fetch, and the error names that path.

## Left out

`[file: …]` markers for attachments and a minimum message length are not ported; an existing filter can drop short records.

## Depends on

`story:file-records` (it owns `FileRecords` and `tests/file_records.rs`).

## Shared files

This story lands before `story:structured-children` and `story:structured-from-files-and-drops` (their edges record it): all three edit `src/sources.rs`, and `structured-children` also edits `spec/domains/instance.yaml`, `generated/`, `spec/suite.json` and `src/model_map.rs`. `story:postgres-credential-from-connectors` edits `spec/domains/instance.yaml` and `src/model_map.rs` in another type; whichever lands second regenerates before its merge.

## Scope

Landed 2026-10-06 in `fd774c1` (wave 20261006c).

- **Files:** `spec/domains/instance.yaml` (`FileRecords.lookup`, `FileRecords.fallback_text`), `src/model_map.rs`, `src/sources.rs` (`read_lookup`, `mentions`, the fallback in `file_records`), `tests/file_records.rs`, `tests/record_lookups_adversary.rs` (new), `website/docs/spec-file.md`; regenerated `generated/`, `spec/suite.json`, `website/docs/reference/ess/cortex-instance.md`, `website/static/schemas/instance-spec.schema.json`
- **Decided in the unit:** `fallback_text` is optional so existing specs parse; an unknown or blank-named id stays as written; `<@id|label>` is looked up by `id`; a byte-order mark in the lookup is stripped
- **Review:** one adversary pass (`review-result:adversary-record-field-lookups-pass-1`), 4 findings, all fixed
