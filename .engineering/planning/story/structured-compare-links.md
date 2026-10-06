---
format: aep.planning-md/3
id: story:structured-compare-links
kind: story
status: draft
title: A structured source links a merged change to the first tag that ships it
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:structured-children
scope:
- confidence: inferred
  path: generated
- confidence: inferred
  path: spec/domains/instance.yaml
- confidence: inferred
  path: src/model_map.rs
- confidence: inferred
  path: src/structured.rs
- confidence: inferred
  path: tests/structured_compare.rs
revision: 2
---
## Outcome

A `structured` source can derive a link from a compare operation: each merged change is linked to the first tag whose comparison contains it, so an agent can answer which release shipped a change.

## Why

Found 2026-10-06 by a gap inventory for running a downstream company deployment entirely on cortex and EKR; no story covered it.
The downstream deployment derives this link today; `story:structured-children` leaves it out.

## Acceptance

`tests/structured_compare.rs`: with a stand-in answering tags `v1` and `v2` and a compare that puts change `c1` in `v1` and `c2` in `v2`, `c1` has one `shipped_in` edge to `v1` and `c2` one to `v2`; a change in no tag has none.

## Depends on

`story:structured-children` (it maps tags and changes as nodes; this story links them).

## Files (inferred)

`spec/domains/instance.yaml`, `generated/`, `src/structured.rs`, `src/model_map.rs`, `tests/structured_compare.rs`.
