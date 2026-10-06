---
format: aep.planning-md/3
id: story:structured-children
kind: story
status: draft
title: A structured source walks child operations and maps them as linked, timed nodes
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
scope:
- confidence: inferred
  path: spec/domains/instance.yaml
- confidence: inferred
  path: src/sources.rs
- confidence: inferred
  path: src/structured.rs
- confidence: inferred
  path: tests/structured_children.rs
revision: 2
---
## Outcome

A `structured` source calls child operations per parent record (for example tags, releases, events, pipelines and deployments of a project) and maps each child record to its own node, linked to its parent and dated by a field. Optional: a derived link through a compare operation (a change to the first tag that contains it), and the old path kept as an alias when a parent is renamed.

## Why

Found 2026-10-06 by the gap inventory for running Company Brain v3 entirely on cortex and EKR (cb3 `initiative:run-on-cortex`); no story covered it.
Today a `structured` source maps flat records only (`src/structured.rs`), so a repository registry with its tags, events and deployments cannot move off the company brain's own importer.

## Acceptance

`tests/structured_children.rs`: a stand-in Connectors answering 2 projects with 3 tags and 4 events each imports 2 + 14 nodes; every child is linked to its parent; each event's valid time equals its record time; a renamed project keeps one node with the old path as an alias.

## Files (from the inventory, unverified)

`spec/domains/instance.yaml` (`StructuredSource.children`), `src/structured.rs`, `src/sources.rs`, `tests/structured_children.rs`.
