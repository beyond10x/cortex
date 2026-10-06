---
format: aep.planning-md/3
id: story:adopt-existing-store
kind: story
status: implemented
title: An existing EKR store becomes an instance without reseeding
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
- depends_on: story:run-snapshots
- depends_on: story:store-backend-per-instance
- depends_on: story:file-records
scope:
- confidence: cited
  path: generated
- confidence: cited
  path: spec/domains/instance.yaml
- confidence: cited
  path: spec/suite.json
- confidence: cited
  path: src/instance.rs
- confidence: cited
  path: src/main.rs
- confidence: cited
  path: src/state.rs
- confidence: inferred
  path: tests/adopt.rs
- confidence: cited
  path: tests/conformance.rs
revision: 18
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T23:46:20Z", actor: "agent:claude", revision: 15, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-05T23:46:20Z", actor: "agent:claude", revision: 16, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-10-06T00:23:50Z", actor: "agent:claude", revision: 18, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Outcome

An existing EKR store, with its full revision history, becomes a cortex instance without being
reseeded or re-extracted.

## Work

- Settle the `UNMAPPED:` `AdoptInstance` marker: outcomes adopted, name taken, store unreadable,
  backend mismatch; the instance enters `Active`. Each outcome becomes a synthesized conformance
  scenario.
- `cortex adopt --spec <file> --store <path or postgres config>`: reads the store's head and
  ontology, checks every seed type exists in the store, and registers the instance. No seed, no
  write to the store.
- Seen-documents state starts from `--seen <file>` (a `cortex.seen/1` document) or empty; with it
  empty, the first run fetches every document and applies the ones whose content hash it has not
  seen, which can re-extract documents the store already holds. That cost is stated in the
  command's help.

## Acceptance

`tests/adopt.rs` seeds a store with `ekr` directly and commits 2 revisions (head N), adopts it,
checks `ekr head` still answers revision N with the same root, then runs a `files` source of 2
documents and checks the head is N+1 and the run report says `documents_applied: 2`.

The outcomes other than adopted are held by the synthesized `AdoptInstance` scenarios in
`tests/conformance.rs`.

## Depends on

`story:run-snapshots` (both edit the spec and `src/main.rs`), `story:store-backend-per-instance`
(both edit `src/instance.rs`), `story:file-records` (both edit `src/state.rs`).

## Scope

Landed 2026-10-06 in `bcc2858` (wave 20261006a, merged `2e30e75`).

- **Files:** `spec/domains/instance.yaml`, `spec/components.yaml` (AdoptInstance, 39 -> 45 scenarios) and what they regenerate, `src/instance.rs`, `src/main.rs`, `src/state.rs`, `Cargo.toml`/`Cargo.lock` (rustix), `tests/adopt.rs` (new, 13 cases), `tests/conformance.rs`, `README.md`, `AGENTS.md`, `website/docs/commands.md`, `operating.md`
- **Beyond the story:** `--host` (ekr opens a store only under the tenant, agents and profile it was seeded with, measured), the `seed-types-missing` and `store-held` outcomes, `--seen <source>=<file>`, a free-space check, `seen_documents` and `seen_without_evidence` in the result
- **For the Company Brain v3 move:** adopting its store needs that brain's own host document through `--host`
- **Not tested:** adopt against a real PostgreSQL (stand-in `ekr` only)
- **Review:** `review-result:adversary-adopt-existing-store-pass-1` (7, one blocker), fixed or documented; coordinator check of the correction
