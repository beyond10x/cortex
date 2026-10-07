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
- confidence: cited
  path: src/ekr.rs
- confidence: cited
  path: src/run.rs
- confidence: cited
  path: src/snapshot.rs
- confidence: cited
  path: tests/store_backend.rs
- confidence: cited
  path: website/docs/operating.md
- confidence: inferred
  path: website/docs/spec-file.md
revision: 16
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

## Scope

Derived 2026-10-07 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Blocked upstream:** `upstream-blocker:ekr-postgres-rewind`. EKR 0.0.32 has no command that puts a store's head back (`ekr --help`); `ekr migrate` refuses PostgreSQL sources; every `ekr operations` kind moves the head forward — cited
- **No spec change:** the run gate's undo is not in the spec (`RunSource`, `spec/domains/instance.yaml:1006-1083`, reports a failed gate as `apply-refused`, `:1029-1035`); `RestoreUnsupported` (`:680-685`) and `RestoreSnapshot`'s `backend-unsupported` (`:1176-1190`) change only if `cortex restore` itself gains `postgres`, which the acceptance does not ask — cited
- **Files:** `src/snapshot.rs:300-318` (`BeforeApply::new` copies nothing for `postgres`), `:544-555` (`rewind_state`), `:573-575` (`restore_held` refuses `postgres`) — cited
- **Files:** `src/run.rs:740-784` (`undo`; "not undone" for `postgres` at `:746-752`) — cited
- **Files:** `src/ekr.rs` (`Store` gains a head read and the EKR rewind call) — inferred
- **Files:** `tests/store_backend.rs:564` (the Docker case), `:525` (`postgres_head`) — cited
- **Documents:** `website/docs/operating.md:156-157`, `:211-212` (a `postgres` run cannot be undone) — cited; `website/docs/spec-file.md:424` — inferred
- **Seen state:** needs no EKR; cortex could copy `state/` on PostgreSQL and roll it back with `rewind_state` — inferred
- **Confidence:** medium: the cortex files are read, the change's shape depends on an EKR command that does not exist yet
