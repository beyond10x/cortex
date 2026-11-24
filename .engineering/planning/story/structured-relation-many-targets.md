---
format: aep.planning-md/3
id: story:structured-relation-many-targets
kind: story
status: proposed
title: One relation name may have several target types in a structured mapping
summary: A structured mapping that names one relation with two target types declares one edge type listing both, instead of two edge types of one name that EKR refuses as extraction-name-duplicate
relations:
- serves: vision:self-updating-instances
revision: 2
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T12:47:42Z", actor: "human:timo", revision: 2}
---
## Outcome

A structured mapping may name one relation with several target types: the instance declares one
edge type of that name whose `target_types` lists every target type, and its batches commit.

## Work

Consumer defect in cortex 0.8.0 and 0.9.0. In `src/structured.rs`, the ontology a structured batch
proposes declares the top mapping's relations with `edge_types.push(edge_type(..))` (about line
1732), one edge type per relation entry, while child mappings use `join`, which merges entries of
one name into one edge type. A mapping whose `relations` lists the same relation name twice, with
two target types, therefore proposes two edge types of one name, and EKR refuses the extraction
with `extraction-name-duplicate`. A consumer works around it today with one relation name per
target type, which splits one relation into several edge types in the graph.

Red first: a test with a synthetic mapping of one node type whose `relations` holds one relation
name with two target types (for example `LINKS_TO` to `Page` and to `File`), run through the
structured path against the pinned EKR. On main it is refused with `extraction-name-duplicate`;
after the change both edges commit under one edge type whose `target_types` names both types.

Then use `join` for the top mapping's relations as for the children's, and check the other place
that declares edge types (`edge_types.push` near line 506) for the same case. Document in the
structured-source docs that a relation name may have several target types.

## Acceptance

The red-first test fails on main with `extraction-name-duplicate` and passes after the change; the
committed store holds one edge type of that name with both target types and both edges; existing
structured tests unchanged.
