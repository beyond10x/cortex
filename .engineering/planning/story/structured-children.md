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
revision: 4
---
## Outcome

A `structured` source calls child operations per parent record through the existing walk (`ChildCall`, from `story:connectors-source-walks`; no second walk) and maps each child record to its own node, linked to its parent and dated by a field of the record.

## Why

Found 2026-10-06 by the gap inventory for running Company Brain v3 entirely on cortex and EKR (cb3 `initiative:run-on-cortex`); no story covered it.
Today a `structured` source maps flat records only (`src/structured.rs`), so a repository registry with its tags, events and deployments cannot move off the company brain's own importer.

## Acceptance

1. `tests/structured_children.rs`: a stand-in Connectors answering 2 projects with 3 tags and 4 events each imports 2 + 14 nodes, and every child node has one edge to its parent.
2. Each event node's valid time equals the record field the mapping names as its time.
3. Run 1 imports project `alpha` at path `group/alpha`; run 2 answers the same project id at path `group/alpha-renamed`; after run 2 the store holds one node for that id, with `group/alpha` among its aliases.

## Left out

A derived link through a compare operation (a change to the first tag that contains it) and dropping refused records are not part of this story; dropping is `story:structured-from-files-and-drops`.

## Depends on

`story:structured-source`, `story:connectors-source-walks`, `story:document-time-as-valid-time` (acceptance 2 goes through the evidence time path), and `story:structured-from-files-and-drops` (both edit `src/structured.rs` and `fetch_structured`; this one lands after it).

## Files (from the inventory, unverified)

`spec/domains/instance.yaml` (`StructuredSource.children`), `generated/`, `spec/suite.json`, `src/model_map.rs`, `src/structured.rs`, `src/sources.rs`, `tests/structured_children.rs`.
