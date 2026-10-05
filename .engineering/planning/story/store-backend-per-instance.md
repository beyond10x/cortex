---
format: aep.planning-md/3
id: story:store-backend-per-instance
kind: story
status: draft
title: An instance's store lives on the backend its spec names
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
- depends_on: story:spec-standalone-types
scope:
- confidence: inferred
  path: .github/workflows/check.yml
- confidence: cited
  path: src/ekr.rs
- confidence: cited
  path: src/instance.rs
- confidence: cited
  path: src/main.rs
- confidence: cited
  path: src/schedule.rs
- confidence: inferred
  path: tests/store_backend.rs
revision: 12
---
## Outcome

An instance's store lives on the backend its spec names: one SQLite file (default) or EKR's
PostgreSQL provider.

## Work

- `src/ekr.rs` `Store::cmd`: `EKR_BACKEND=postgres` and `EKR_STORE=<store.config>` for
  `postgres` (ekr 0.0.30 `--help`: "configuration file for `postgres`"); unchanged for `sqlite`.
- `src/ekr.rs` gains `Store::provision`, which runs `ekr postgres-schema --config <file>`; the one
  call site is `seed_instance` (`src/main.rs:327-363`), before seeding. A scheduled run never
  provisions schema.
- Every other place that names the store follows the backend: the viewer unit
  (`src/schedule.rs:127-146`, `install_view`, which writes `EKR_BACKEND=sqlite` and the
  `store.sqlite` path today) and the `claude mcp add` line (`src/main.rs:815`).
- cortex holds no credential: the `ekr.postgres/1` file and the connection file it names are the
  operator's. Where those credentials come from (an operator-written file, or Connectors' secret
  backends) is an open question recorded here, not decided by this story.

## Acceptance

`tests/store_backend.rs` starts a disposable PostgreSQL container, creates a `postgres` instance,
runs a `files` source, and reads the head revision back through `ekr head` with
`EKR_BACKEND=postgres`. The test runs, not skips, in the `Check` workflow on GitHub Actions (whose
`ubuntu-latest` runner has Docker); the run URL is the evidence. Locally it skips with a named
reason when no container runtime exists.

## Depends on

`story:spec-standalone-types`.

## Scope

`src/ekr.rs`, `src/instance.rs`, `src/main.rs` (`seed_instance` and the MCP line), `src/schedule.rs` (`install_view`), `tests/store_backend.rs` (new),
`.github/workflows/check.yml` (only if the container needs a service entry).
