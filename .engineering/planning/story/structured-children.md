---
format: aep.planning-md/3
id: story:structured-children
kind: story
status: draft
title: A structured source walks child operations and maps them as linked, timed nodes
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:structured-source
- depends_on: story:connectors-source-walks
- depends_on: story:document-time-as-valid-time
- depends_on: story:structured-from-files-and-drops
- depends_on: story:record-field-lookups
scope:
- confidence: inferred
  path: generated
- confidence: inferred
  path: spec/domains/instance.yaml
- confidence: inferred
  path: spec/suite.json
- confidence: inferred
  path: src/model_map.rs
- confidence: inferred
  path: src/sources.rs
- confidence: inferred
  path: src/structured.rs
- confidence: inferred
  path: tests/structured_children.rs
revision: 5
---
## Outcome

A `structured` source calls child operations per parent record through the existing walk (`ChildCall`, from `story:connectors-source-walks`; no second walk) and maps each child record to its own node, linked to its parent and dated by a field of the record. A parent whose path changes between runs keeps its one node, with the old path as an alias, so its children stay linked to it.

## Why

Found 2026-10-06 by a gap inventory for running a downstream company deployment entirely on cortex and EKR; no story covered it.
Today a `structured` source maps flat records only (`src/structured.rs`), so a repository registry with its tags, events and deployments cannot move off the downstream deployment's own importer.

## Acceptance

1. `tests/structured_children.rs`: a stand-in Connectors answering 2 projects with 3 tags and 4 events each imports 2 + 14 nodes, and every child node has one edge to its parent.
2. Each event node's valid time equals the record field the mapping names as its time.
3. Run 1 imports project `alpha` at path `group/alpha`; run 2 answers the same project id at path `group/alpha-renamed`; after run 2 the store holds one node for that id, with `group/alpha` among its aliases.
4. When the stand-in answers `forbidden` to one project's events, that project is imported with its tags and without events, and the run's `skipped` list names it with the reason.

## Left out

The derived link through a compare operation (a change to the first tag that contains it) is `story:structured-compare-links`.

## Depends on

`story:structured-source`, `story:connectors-source-walks`, `story:document-time-as-valid-time` (acceptance 2 goes through the evidence time path), `story:structured-from-files-and-drops` (both edit `src/structured.rs` and `fetch_structured`; this one lands after it, and so waits for `upstream-blocker:ekr-extraction-supersession` through it: accepted 2026-10-06 rather than splitting `fetch_structured`), and `story:record-field-lookups` (both add a field to `spec/domains/instance.yaml` and regenerate `generated/`, `spec/suite.json` and `src/model_map.rs`; that one lands first).

`story:postgres-credential-from-connectors` also edits `spec/domains/instance.yaml` and `src/model_map.rs`, in a different type (`StoreBackend`). The overlap is accepted: whichever lands second regenerates before its merge.

## Files (from the inventory, unverified)

`spec/domains/instance.yaml` (`StructuredSource.children`), `generated/`, `spec/suite.json`, `src/model_map.rs`, `src/structured.rs`, `src/sources.rs`, `tests/structured_children.rs`.
