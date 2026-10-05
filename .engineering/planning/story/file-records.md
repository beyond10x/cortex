---
format: aep.planning-md/3
id: story:file-records
kind: story
status: draft
title: A files source yields one document per JSON line or markdown section
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:connectors-source-walks
- depends_on: story:spec-standalone-types
- depends_on: story:timer-runs-unattended
scope:
- confidence: cited
  path: src/sources.rs
- confidence: cited
  path: src/state.rs
- confidence: inferred
  path: tests/file_records.rs
revision: 8
---
## Outcome

A `files` source can treat each line of a JSON-lines export, or each section of a markdown file, as
its own document, so a chat export or a long curated file is cited record by record.

## Work

- `FileRecords.format`: `WholeFile` (today), `JsonLines` (one record per line), `MarkdownSections`
  (one record per `##` section, id = file path + heading).
- `id`, `time`, `author`, `text` templates as in `connectors` sources; `thread` names the field that
  groups records, and a record in a thread carries the earlier records of its thread as context
  marked `[context]`.
- `filters`: include or exclude records whose field equals one of the values (e.g. exclude a
  channel group, exclude direct messages).
- Seen state is per record id and content hash. A record whose content hash changed is delivered
  again on the next run, whatever the source's refresh window (`SeenState::select` re-delivers
  changed text only past the window today; records mode does not wait for it).

## Acceptance

`tests/file_records.rs`: a JSON-lines file of 5 records, 1 excluded by a filter, plus a markdown
file of 3 sections, produce exactly 7 documents with the ids their templates give.

Re-delivery is held by a second scenario in `tests/file_records.rs`: after editing one markdown
section, the next run (inside the refresh window) applies exactly 1 document, that section. Thread
context is held by a unit test in `src/sources.rs`: the second record of a thread carries the first
as `[context]`.

## Depends on

`story:spec-standalone-types` (the types), `story:connectors-source-walks` (both edit
`src/sources.rs` and `src/state.rs`), `story:timer-runs-unattended` (its fixes may touch
`src/state.rs`).

## Scope

`src/sources.rs`, `src/state.rs`, `tests/file_records.rs` (new).
