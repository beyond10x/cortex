---
format: aep.planning-md/3
id: story:postgres-run-undo
kind: story
status: draft
title: A run on a PostgreSQL store can be undone
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:run-snapshots
- depends_on: story:run-gate
- depends_on: story:store-backend-per-instance
- depends_on: story:postgres-credential-from-connectors
scope:
- confidence: inferred
  path: src/ekr.rs
- confidence: inferred
  path: src/run.rs
- confidence: inferred
  path: src/snapshot.rs
- confidence: inferred
  path: tests/store_backend.rs
revision: 5
---
## Outcome

A run that fails its gate on a `postgres` store is undone: the store's head and the sources' seen state are back where they were before the run.

## Why

Found 2026-10-06 by the gap inventory for running Company Brain v3 entirely on cortex and EKR (cb3 `initiative:run-on-cortex`); no story covered it.
Snapshots and the gate's undo are SQLite-only (`src/snapshot.rs`); a PostgreSQL-backed deployment, which the company brain uses, has no undo.

## Acceptance

In the docker PostgreSQL case of `tests/store_backend.rs`, a run that fails `facts_refused max 0` leaves `ekr head` equal to the head before the run and the seen state unchanged.

## Depends on

`story:run-snapshots`, `story:run-gate`, `story:store-backend-per-instance`, and `story:postgres-credential-from-connectors` (it rewrites the same docker case and `Store::cmd()`; this one lands after it).

## Open

Whether EKR has, or needs, a revert or a provider-level restore point for this: unknown. If it needs one, an EKR story comes first.

## Files (from the inventory, unverified)

`src/snapshot.rs`, `src/run.rs`, `src/ekr.rs`, `tests/store_backend.rs`.

## Shared files

`story:structured-from-files-and-drops` also edits `src/ekr.rs`: it adds a read of a source's active assertions, and this story adds the PostgreSQL restore point around a run. Different functions; the overlap is accepted (2026-10-06), and the second to land rebases.
