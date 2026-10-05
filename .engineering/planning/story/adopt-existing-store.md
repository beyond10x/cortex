---
format: aep.planning-md/3
id: story:adopt-existing-store
kind: story
status: draft
title: An existing EKR store becomes an instance without reseeding
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
- depends_on: story:run-snapshots
- depends_on: story:store-backend-per-instance
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
revision: 13
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
(both edit `src/instance.rs`).

## Scope

`spec/domains/instance.yaml`, `generated/`, `spec/suite.json`, `src/instance.rs`, `src/main.rs`,
`src/state.rs`, `tests/adopt.rs` (new), `tests/conformance.rs`.
