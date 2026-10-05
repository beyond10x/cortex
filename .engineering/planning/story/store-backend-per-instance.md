---
format: aep.planning-md/3
id: story:store-backend-per-instance
kind: story
status: implemented
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
revision: 17
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T11:54:37Z", actor: "agent:claude", revision: 14, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "proposed", to: "active", at: "2026-10-05T11:54:37Z", actor: "agent:claude", revision: 15, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "active", to: "implemented", at: "2026-10-05T13:01:18Z", actor: "agent:claude", revision: 17, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
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
- Credentials come from Connectors (decided by the operator 2026-10-05): the database credential is
  a Connectors connection held in Connectors' secret backends; the instance spec names the
  connection, never a secret. cortex never reads, prints or stores it. How EKR's PostgreSQL provider
  receives it is designed first in this story with the Connectors and EKR owners: Connectors hands it
  to the `ekr` child it launches over a private channel, or EKR's provider reads it through
  Connectors. The design is recorded here before code starts.

## Acceptance

`tests/store_backend.rs` starts a disposable PostgreSQL container, creates a `postgres` instance,
runs a `files` source, and reads the head revision back through `ekr head` with
`EKR_BACKEND=postgres`. The test runs, not skips, in the `Check` workflow on GitHub Actions (whose
`ubuntu-latest` runner has Docker); the run URL is the evidence. Locally it skips with a named
reason when no container runtime exists.

## Depends on

`story:spec-standalone-types`.

## Scope

Landed 2026-10-05 in `6bc3f00` (wave 20261005c, merged `694d9b3`).

- **Files:** `src/ekr.rs`, `src/instance.rs`, `src/main.rs`, `src/schedule.rs`, `tests/store_backend.rs` (new, 15 cases, a real PostgreSQL in docker), `tests/spec_forms.rs` (one case: a sqlite `value.path` update is now refused)
- **Inferred line, not needed:** `.github/workflows/check.yml`; the test runs `docker` itself and fails, not skips, when `GITHUB_ACTIONS` is set and no runtime answers
- **Beyond the story:** `cortex create --postgres-schema-config`, because the application role may not own DDL (measured: `SQLSTATE 42501`); a sqlite `value.path` is refused as not supported yet
- **Moved out:** the Connectors credential channel → `story:postgres-credential-from-connectors`; a conditional seed that closes the two-home race → EKR `story:seed-if-absent`
- **Review:** `review-result:adversary-store-backend-per-instance-pass-1` (6) and `-pass-2` (2), both fixed; coordinator check of the final diff
