---
format: aep.planning-md/3
id: story:structured-compare-links
kind: story
status: active
title: A structured source links a merged change to the first tag that ships it
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:structured-children
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
  path: tests/structured_compare.rs
- confidence: cited
  path: website/docs/reference/ess/cortex-instance.md
- confidence: cited
  path: website/docs/spec-file.md
- confidence: cited
  path: website/static/schemas/instance-spec.schema.json
revision: 22
transitions:
- {from: "draft", to: "proposed", at: "2026-10-07T09:14:18Z", actor: "human:timo", revision: 21}
- {from: "proposed", to: "active", at: "2026-10-07T09:14:18Z", actor: "human:timo", revision: 22}
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

## Scope

Derived 2026-10-07 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `src/structured.rs` (the edge) and `fetch_structured` in `src/sources.rs` (the compare calls) — cited
- **Spec:** a new struct after `StructuredChild` (`spec/domains/instance.yaml:193-202`) and `links: Optional<List<…>>` after `children` (`:212`) on `StructuredConnectors` — cited, validated on a scratch copy with ESS 0.55.0 (all generators exit 0, obligations stay 2, scenarios stay 55). The existing `RelationMapping` cannot carry it: another node type's target is named by its bare text (`src/structured.rs:802-804`), which would merge same-named tags across projects
- **Files:** `generated`, `spec/suite.json` (digests and 12 scenario type lists), `src/model_map.rs:256-282` — cited
- **Files:** `src/sources.rs:474-531` (`fetch_structured`: the compare walks after the children loop, keeping the tag records `:512` drops) — cited
- **Files:** `src/structured.rs` (`Source::new`, `document`, `ontology`, `listed`, `lead`, `failed_child`) — cited
- **Files:** `tests/structured_compare.rs` (new) — cited
- **Also likely:** `src/spec.rs:165-185` (refuse `tags`/`changes` naming no child operation; ESS 0.55.0 refused the invariant form, ESS-TYPE-002) and `src/run.rs` (a failed compare's held prefix) — inferred
- **Documents:** `website/docs/spec-file.md:382-423` (`kind: structured`, Children); derived pages regenerate — cited
- **Matching:** a change is found as `identity_in(child_scope(changes, identity(parent)).prefix, id)`, the identity `prepare` gives the change record, so project 1's `c1` never matches project 2's — cited
- **Safety:** the derived link must enter the change document's hashed content, or a change merged before its tag never gets the edge in a later run (`src/state.rs:140-147`, `src/run.rs:435-437`) — inferred
- **Collides with:** `story:docs-for-1-0` on `website/docs/spec-file.md` — cited
- **Confidence:** medium; the open points are decided on the wave 20261007d page
