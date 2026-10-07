---
format: aep.planning-md/3
id: story:structured-children
kind: story
status: active
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
- confidence: cited
  path: generated
- confidence: cited
  path: spec/domains/instance.yaml
- confidence: cited
  path: spec/suite.json
- confidence: cited
  path: src/model_map.rs
- confidence: inferred
  path: src/run.rs
- confidence: cited
  path: src/sources.rs
- confidence: inferred
  path: src/spec.rs
- confidence: cited
  path: src/structured.rs
- confidence: cited
  path: tests/structured_children.rs
- confidence: cited
  path: website/docs/reference/ess/cortex-instance.md
- confidence: cited
  path: website/docs/spec-file.md
- confidence: cited
  path: website/static/schemas/instance-spec.schema.json
revision: 27
transitions:
- {from: "draft", to: "proposed", at: "2026-10-07T07:43:13Z", actor: "human:timo", revision: 26, decided_on: {"recorded":{"review_outcome":8}}}
- {from: "proposed", to: "active", at: "2026-10-07T07:43:13Z", actor: "human:timo", revision: 27, decided_on: {"recorded":{"review_outcome":8}}}
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

## Scope

Derived 2026-10-07 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `src/structured.rs` and the structured branch of `src/sources.rs` — cited
- **Files:** `spec/domains/instance.yaml:183-225` (new `StructuredChild`; `children: Optional<List<…>>`) — cited, validated on a scratch copy with ESS 0.55.0 (all five generators exit 0, obligations stay 2)
- **Files:** `generated`, `spec/suite.json` (digests and 12 scenario type lists; steps unchanged) — cited, scratch regeneration
- **Files:** `src/model_map.rs:215-267` (`structured()`) — cited
- **Files:** `src/sources.rs:324-464` (`fetch`, `structured_records`, `fetch_structured`), reusing `walk`/`render_json` at :145-258 — cited
- **Files:** `src/structured.rs` (`Source::{prepare,document,index,listed,ended,owner,target}`, `ontology`) — cited
- **Files:** `tests/structured_children.rs` (new) — cited
- **Also likely:** `src/run.rs:325-479, 824-960` (one `Source` per mapping; dropped-record ending at :474, :938-952) and `src/spec.rs:139-163` (the shared-prefix refusal covering child operations) — inferred
- **Documents:** `website/docs/spec-file.md:285` (`kind: structured`) — cited; derived pages regenerate
- **Not touched:** `tests/conformance.rs` — cited: it reads only `scenario_initial_state` (:730) and `steps` (:738), and neither changes
- **Today:** `ChildCall` is only a field of `ConnectorsSource` (`spec/domains/instance.yaml:137`) and only `fetch_records` uses it (`src/sources.rs:688-723`); `fetch_structured` (`:437-464`) walks the parent operation alone — cited
- **EKR 0.0.32, probed on a scratch store:** an id-based alias keeps one node across a rename, and the old path stays an alias; `apply-extraction` never adds new aliases to a matched node (an `AddAlias` transaction could); things of one type sharing any alias are one node — cited
- **Safety:** a child's identity must include its parent's id, or same-named children of two parents merge; acceptance 4 must keep a failed parent's earlier children from being retracted as dropped (`src/structured.rs:474-476, 503-512`, `src/run.rs:474`) — inferred
- **Confidence:** medium: the primary surfaces are cited and the spec shape was generated on a scratch copy; whether `src/run.rs` and `src/spec.rs` change depends on design choices
