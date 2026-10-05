---
format: aep.planning-md/3
id: story:run-snapshots
kind: story
status: implemented
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
revision: 25
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T16:13:32Z", actor: "agent:claude", revision: 22, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "proposed", to: "active", at: "2026-10-05T16:13:32Z", actor: "agent:claude", revision: 23, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "active", to: "implemented", at: "2026-10-05T18:48:43Z", actor: "agent:claude", revision: 25, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
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

Landed 2026-10-05 in `d795671` (wave 20261005h, merged `a6f536b`).

- **Files:** `src/snapshot.rs` (new), `src/run.rs`, `src/main.rs`, `src/home.rs`, `src/schedule.rs`, `spec/domains/instance.yaml` (RestoreSnapshot; 34 -> 39 scenarios) and what it regenerates, `Cargo.toml`/`Cargo.lock` (rusqlite 0.40.2), `tests/snapshots.rs` (new, 13 cases), `AGENTS.md`, `website/docs/operating.md`, `spec-file.md`, `commands.md`
- **Decided during the wave:** snapshots carry `state/` (in `snapshots-state/`) and a restore rewinds it; published only after the first apply commits; named by the run's start; only cortex-named snapshots rotate; a restore first snapshots the current state; the viewer restarts only if it was running
- **Review:** `review-result:adversary-run-snapshots-pass-1` (10), fixed or documented; coordinator check of the correction
