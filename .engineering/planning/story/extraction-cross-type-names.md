---
format: aep.planning-md/3
id: story:extraction-cross-type-names
kind: story
status: draft
title: An extracted entity attaches to a store node of another type with its name
tags:
- downstream-request
relations:
- decomposes: epic:standalone-1-0
revision: 1
---
## Outcome

A model extraction that names an entity whose canonical name or alias already belongs to a store
node of another type attaches to that node instead of creating a second node of a new type.

## Why

Reported by a downstream instance on 2026-10-30, read against v0.6.1: a chat record keyed by its
timestamp names component `x`; the store holds `Service "x"`; the run writes a new `Component "x"`
beside it. On one run 9 of 153 new nodes had the exact canonical name of an older node of another
type. `src/known.rs` builds lookup terms from the document key and record id only, so a timestamp
key shows the model no store hits; `src/extract.rs` passes entities to `ekr apply-extraction` as
typed, and EKR resolves within one type. Distinct from `story:structured-lookup-across-types`,
which covers structured imports.

## Work

1. Build lookup terms from the document text's candidate names as well, bounded.
2. Before apply, resolve an extracted entity whose name matches a node of another type: attach to
   it, or report the conflict in the run report.

## Acceptance

- A run over a record naming `x` with `Service "x"` in the store attaches to `Service "x"`.
- Two entities of one name and two types in one batch become one node.
- A test with a record keyed by a timestamp shows the model the store hit for a name in its text.
