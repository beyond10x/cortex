---
format: aep.planning-md/3
id: story:postgres-run-undo
kind: story
status: draft
title: A run on a PostgreSQL store can be undone
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
scope:
- confidence: inferred
  path: src/run.rs
- confidence: inferred
  path: src/snapshot.rs
- confidence: inferred
  path: tests/store_backend.rs
revision: 2
---
## Outcome

A run that fails its gate on a `postgres` store is undone: the store's head and the sources' seen state are back where they were before the run.

## Why

Found 2026-10-06 by the gap inventory for running Company Brain v3 entirely on cortex and EKR (cb3 `initiative:run-on-cortex`); no story covered it.
Snapshots and the gate's undo are SQLite-only (`src/snapshot.rs`); a PostgreSQL-backed deployment, which the company brain uses, has no undo.

## Acceptance

In the docker PostgreSQL case of `tests/store_backend.rs`, a run that fails `facts_refused max 0` leaves `ekr head` equal to the head before the run and the seen state unchanged.

## Open

Whether EKR has, or needs, a revert or a provider-level restore point for this: unknown.

## Files (from the inventory, unverified)

`src/snapshot.rs`, `src/run.rs`, `tests/store_backend.rs`.
