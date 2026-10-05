---
format: aep.planning-md/3
id: story:run-snapshots
kind: story
status: draft
title: A bad run can be undone from the snapshot taken before it
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
- depends_on: story:seen-documents-modelled
- depends_on: story:structured-source
- depends_on: story:store-backend-per-instance
- depends_on: story:timer-runs-unattended
- depends_on: story:release-pipeline
- depends_on: story:redaction-names-and-gate
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: generated
- confidence: cited
  path: spec/domains/instance.yaml
- confidence: cited
  path: spec/suite.json
- confidence: cited
  path: src/home.rs
- confidence: cited
  path: src/lib.rs
- confidence: cited
  path: src/main.rs
- confidence: cited
  path: src/run.rs
- confidence: cited
  path: src/schedule.rs
- confidence: inferred
  path: src/snapshot.rs
- confidence: cited
  path: tests/conformance.rs
- confidence: inferred
  path: tests/snapshots.rs
revision: 21
---
## Outcome

Before a run applies anything, the instance's store is snapshotted, and a bad run can be undone with
one command.

## Work

- Settle the `UNMAPPED:` `RestoreSnapshot` marker in `spec/domains/instance.yaml`: outcomes restored,
  no such snapshot, instance busy, and backend unsupported for `postgres` (where the snapshot is the
  operator's database backup). Each outcome becomes a synthesized conformance scenario.
- `src/snapshot.rs`: for `sqlite`, SQLite's online backup API through the `rusqlite` crate (a new
  dependency in `Cargo.toml`; cortex has no SQLite crate today); keep `snapshots.keep` (default 3)
  per instance under the instance directory. `src/run.rs` calls it once, before `apply-extraction`.
- `cortex restore <instance> <snapshot>`: refuses while the instance lock is held or a reader holds
  the store; stops the viewer, restores, restarts it. The stop and start are a new
  `Systemd::restart_view` in `src/schedule.rs`.

- `snapshot::restore_held(instance)`: the same restore for a caller that already holds the instance
  lock (used by `story:run-gate` inside a run); `cortex restore` takes the lock and calls it.

## Acceptance

`tests/snapshots.rs` runs a source twice, restores the snapshot taken before the second run, and
`ekr head` then answers the revision the first run ended at.

The keep count and the refusals are held by their own checks: a unit test in `src/snapshot.rs`
(`keep: 2` leaves 2 after 3 snapshots) and the synthesized `RestoreSnapshot` scenarios in
`tests/conformance.rs`.

## Depends on

`story:seen-documents-modelled` (both edit the spec), `story:structured-source` and
`story:redaction-names-and-gate` (all edit `src/run.rs`), `story:store-backend-per-instance` (both
edit `src/main.rs`), `story:timer-runs-unattended` (both edit `src/schedule.rs`),
`story:release-pipeline` (both edit `Cargo.toml`).

## Scope

`spec/domains/instance.yaml`, `generated/`, `spec/suite.json`, `src/snapshot.rs` (new), `src/run.rs`, `src/schedule.rs`, `Cargo.toml`, `Cargo.lock`,
`src/lib.rs`, `src/main.rs`, `tests/snapshots.rs` (new), `tests/conformance.rs`.
