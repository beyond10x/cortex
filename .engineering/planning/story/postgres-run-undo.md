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
revision: 17
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

## Design

Planned 2026-10-07 against the design epistemic-knowledge-runtime chose for staged runs (not
released yet; the release tag is the clearing condition of `upstream-blocker:ekr-postgres-rewind`).
EKR does not rewind: deleting committed revisions would break its invariants, and its PostgreSQL
event log has no range delete.

**EKR's side, as designed:**
- A stage is its own event-log tenant in the same store, forked from the base at the head.
- Every one-shot verb run with `EKR_STAGE=<id>` or `--stage <id>` opens the stage: `ontology`,
  `snapshot`, `quality`, `propose`, `validate`, `commit`, `apply-extraction`. Reads inside the stage
  see its commits.
- `ekr stage publish <id> --expect-head <rev>` appends the stage's suffix into the base in one
  append group and removes the stage; abandoning removes it in one transaction. The verb that
  creates a stage is not fixed yet.
- Cost: the fork copies the base on each run, so a run's start time grows with the store's size.

**cortex's side:**
1. Before the first batch of a run on a `postgres` store with a gate: read `ekr head`, create a
   stage, and run every `ekr` of the run with `EKR_STAGE=<id>` (it crosses the Connectors launch,
   whose consumer passes `EKR_*`). `src/ekr.rs` `Store` carries the stage id into `Store::cmd`.
2. Gate passed: `ekr stage publish <id> --expect-head <pre-run head>`; a refusal (the head moved)
   fails the run with nothing published. Gate failed: abandon the stage; `state/` is restored from
   the copy taken before the run (`src/snapshot.rs`), as on SQLite.
3. A run without a gate, or on SQLite, is unchanged.
4. Spec: the run gate's undo is not in the specification (`RunSource` reports a failed gate as
   `apply-refused`), so no spec change is expected. If the stage needs a modelled noun or outcome,
   it goes into `spec/` first.

**Acceptance, added:** the acceptance run records how long creating the stage takes against the
store's size (`ekr snapshot` node and assertion counts), for the Docker case and for one larger
store, in the story's evidence.
