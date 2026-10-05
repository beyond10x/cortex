---
format: aep.planning-md/3
id: story:extraction-links-facts
kind: story
status: draft
title: Extraction produces edges between the nodes it finds
relations:
- decomposes: epic:first-web-instance
- serves: vision:self-updating-instances
- depends_on: story:ci-runs-task-check
revision: 2
---
## Outcome

Extraction states the relations its documents carry between the things they name, not mainly
properties of them.

## Starting point (corrected 2026-10-05)

The first draft said the smoke store holds 0 edges. That was a misreading: EKR 0.0.30 records an
extracted relation as an assertion with a `Relation` predicate, and the snapshot's separate `edges`
section stays empty for extraction (`ekr snapshot` on the smoke store: 14 nodes, 22 assertions =
18 `Property` + 4 `Relation`, `edges` 0, revision 6).

Relations do reach the store; there are few of them. The model's saved answers
(`runs/*/batch-0/model.json` in the smoke instance) hold 1 relation for the 3 search documents and
3 for the 5 crawl documents, all `DEVELOPS`, the only edge type in `examples/seed/schema.yaml`.
The system prompt in `src/extract.rs` asks for no relations at all.

## Work

- Benchmark: the two saved prompts (`runs/*/batch-0/prompt.txt`, 8 documents) replayed without
  re-fetching.
- Change the prompt so it asks for every relation a document states between named things, and give
  the seed schema more than one edge type.
- Whether the empty `edges` section matters to any reader (`ekr view`, `ekr mcp` `expand`) is
  checked on the smoke store and recorded; if it does, that is an EKR story, not a cortex one.

## Acceptance

On the same 8 documents the store gains at least 8 `Relation` assertions (1 per document on
average; baseline 4), with model cost under $0.15 for the replay. Before and after counts from
`ekr snapshot` recorded as `test_result` evidence.

## Scope

`src/extract.rs`, `examples/seed/schema.yaml`, a fixture directory under `tests/`.
