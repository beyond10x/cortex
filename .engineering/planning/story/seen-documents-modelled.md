---
format: aep.planning-md/3
id: story:seen-documents-modelled
kind: story
status: implemented
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
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T11:54:37Z", actor: "agent:claude", revision: 7, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-10-05T11:54:37Z", actor: "agent:claude", revision: 8, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-10-05T13:01:18Z", actor: "agent:claude", revision: 10, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
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

Landed 2026-10-05 in `ea5b45f` (wave 20261005c, merged `0a245fd`, with a coordinator resolution of `src/main.rs` against the store unit).

- **Files:** `spec/domains/instance.yaml`, `spec/suite.json` (35 → 34 scenarios), `generated/`, the website reference, schema and domain graph, `src/home.rs`, `src/state.rs`, `src/main.rs`, `tests/conformance.rs` (error fields compared; `excludes` handled), `tests/seed_change.rs` and `tests/seed_paths.rs` (new)
- **Beyond the scope:** `src/main.rs` (the generated inputs changed)
- **UNMAPPED, filed upstream:** the per-run upsert of seen documents, beyond10x/ess#459; the lost unchanged-subject check after a refused update, beyond10x/ess#461 (a Rust test covers it)
- **Moved out:** a symlinked seed directory or file inside one → `story:symlinked-seed-directory`
- **Review:** `review-result:adversary-seen-documents-modelled-pass-1-report` (8, by reading; the disk was full) and `-pass-2` (6), fixed; coordinator check of the final diff
