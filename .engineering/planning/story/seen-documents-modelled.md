---
format: aep.planning-md/3
id: story:seen-documents-modelled
kind: story
status: draft
title: The spec says what cortex remembers between runs
relations:
- decomposes: epic:first-web-instance
- serves: vision:self-updating-instances
- depends_on: story:ci-runs-task-check
- depends_on: story:spec-standalone-types
scope:
- confidence: cited
  path: generated
- confidence: cited
  path: spec/domains/instance.yaml
- confidence: cited
  path: spec/suite.json
- confidence: cited
  path: src/home.rs
- confidence: cited
  path: src/state.rs
revision: 6
---
## Outcome

The spec says what cortex remembers between runs, so the conformance suite checks it instead of the
code alone.

## The three markers

| line | question |
|---|---|
| `spec/domains/instance.yaml:213` | which documents a source has already seen (document id, content hash, last fetch) — an entity owned by `Source`, or a field |
| `spec/domains/instance.yaml:328` | "the home already holds an instance with this name" reads the registry, not the entity store |
| `spec/domains/instance.yaml:382` | whether the seed changed compares the new spec with the frozen copy |

Each is settled in the spec (with `ess/20` constructs where they exist) or stays `UNMAPPED:` with the
ESS gap filed on `beyond10x/ess` and cited here.

## Acceptance

`ess specify validate` passes; `grep -c UNMAPPED spec/domains/instance.yaml` falls from 3 to the
number of filed ESS gaps; regenerated `generated/` and `spec/suite.json` committed; `task check` green.

## Scope

`spec/domains/instance.yaml`, `generated/`, `spec/suite.json`, and `src/state.rs` / `src/home.rs`
where the generated types change.
