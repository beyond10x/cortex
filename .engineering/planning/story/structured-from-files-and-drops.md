---
format: aep.planning-md/3
id: story:structured-from-files-and-drops
kind: story
status: draft
title: Structured imports read files and supersede what a source dropped
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:structured-source
- depends_on: story:spec-standalone-types
- depends_on: story:connectors-source-walks
- depends_on: story:file-records
- depends_on: story:store-backend-per-instance
scope:
- confidence: cited
  path: src/ekr.rs
- confidence: cited
  path: src/sources.rs
- confidence: inferred
  path: src/structured.rs
- confidence: inferred
  path: tests/structured_files.rs
revision: 10
---
## Outcome

A `structured` source can import from files as well as from Connectors, and a value the source no
longer lists is superseded in the store instead of standing forever.

## Work

- `StructuredInput`: `connectors` (as in `story:structured-source`) or `files` (`paths`, `glob`;
  JSON documents whose `records` path holds the list).
- A `connectors` input with `paging` reuses the page walk `story:connectors-source-walks` builds in
  `src/sources.rs`; this story wires it into the structured path and adds no second walk.
- `dropped: Supersede`: after a run, an assertion this source made earlier for a record whose mapped
  value is now different or absent is superseded, citing the run's evidence. "Made earlier by this
  source" is read from the store, not from cortex state: `src/ekr.rs` gains a read of the active
  assertions whose evidence carries this source's identity (`ekr snapshot`), and
  `src/structured.rs` compares them with the run's mapped records. `Keep` (default) leaves them.

## Acceptance

`tests/structured_files.rs`: a registry file of 3 people with a `team` property imports 3 nodes;
after one person's team changes and another is removed, a second run leaves the changed person with
the new team as the active assertion and the old one superseded, and the removed person's
assertions superseded.

## Depends on

`story:spec-standalone-types` (the types), `story:structured-source` (both edit
`src/structured.rs`), `story:connectors-source-walks` (the page walk it reuses; both edit
`src/sources.rs`), `story:file-records` (both edit `src/sources.rs`),
`story:store-backend-per-instance` (both edit `src/ekr.rs`), and
`upstream-blocker:ekr-extraction-supersession`.

## Scope

`src/structured.rs`, `src/sources.rs`, `src/ekr.rs` (the read of this source's active assertions),
`tests/structured_files.rs` (new).
