---
format: aep.planning-md/3
id: story:feed-source
kind: story
status: draft
title: A source names a connection and cortex reads it through the feed family
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
scope:
- confidence: inferred
  path: generated
- confidence: inferred
  path: spec/domains/instance.yaml
- confidence: inferred
  path: src/connectors.rs
- confidence: inferred
  path: src/run.rs
- confidence: inferred
  path: src/sources.rs
- confidence: inferred
  path: tests/feed_source.rs
revision: 2
---
## Outcome

An instance spec names a Connectors connection as a `feed` source, with optional container selection and reading and indexing options, and cortex reads it through `datasource.feed/v1alpha1` with no code for that provider. A connector added or upgraded in Connectors is usable by an installed cortex without rebuilding cortex.

## Why

Operator requirement 2026-10-06: cortex supports any connector whose provider implements a datasource contract; the instance config names the connection reference and options. Today a `connectors` source names an operation and maps its records with templates (`cortex.instance.ConnectorsSource`), which works without a rebuild but needs per-operation mapping in every spec.

## Work

- Spec: a `feed` source kind: `connection` (adapter and connection reference), optional `containers` (include and exclude by id or name pattern), optional `since` (how far a first read reaches), and indexing options (`max_items_per_run`, thread context on or off, `lookups` for author names, `dropped` for deleted items).
- At run time cortex asks Connectors which operations of the connection bind the family (`story:feed-bindings-discoverable` there), lists the containers, and reads each since its watermark.
- Where cortex stands: the watermark per (source, container) and the `(container, id, revision)` of each applied item live in the source's seen state, so nothing is processed twice and a changed item is processed again.
- An item reported `deleted` is ended through the drop machinery of `story:structured-from-files-and-drops` when the source sets `dropped: Supersede`.

## Acceptance

`tests/feed_source.rs` with a stand-in `connectors` that binds the family for a provider cortex has never seen:
1. A first run applies every item of two containers; a second run with no change applies nothing.
2. After one item changes (new revision) and one is added, a third run applies exactly those two.
3. A container excluded by the spec is never read.
4. The stand-in is replaced by one answering a second, different profile; the same cortex binary reads it.

## Depends on

Outside this store: Connectors `story:feed-contract` and `story:feed-bindings-discoverable`, recorded as `upstream-blocker:connectors-feed-family`.
