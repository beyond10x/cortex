---
format: aep.planning-md/3
id: story:postgres-credential-from-connectors
kind: story
status: implemented
title: A PostgreSQL store's credential comes from a Connectors connection
relations:
- decomposes: epic:organisation-scale-instance
- depends_on: story:store-backend-per-instance
- serves: vision:self-updating-instances
scope:
- confidence: cited
  path: spec/domains/instance.yaml
- confidence: cited
  path: src/connectors.rs
- confidence: cited
  path: src/ekr.rs
- confidence: cited
  path: src/gate.rs
- confidence: cited
  path: src/instance.rs
- confidence: cited
  path: src/main.rs
- confidence: cited
  path: src/model_map.rs
- confidence: cited
  path: src/quality.rs
- confidence: cited
  path: src/run.rs
- confidence: cited
  path: src/schedule.rs
- confidence: cited
  path: src/snapshot.rs
- confidence: cited
  path: tests/spec_forms.rs
- confidence: cited
  path: tests/store_backend.rs
- confidence: cited
  path: website/docs/operating.md
- confidence: cited
  path: website/docs/spec-file.md
revision: 32
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T07:36:01Z", actor: "agent:claude", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-06T07:36:01Z", actor: "agent:claude", revision: 6}
- {from: "active", to: "implemented", at: "2026-10-07T02:56:41Z", actor: "human:timo", revision: 32, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Outcome

A PostgreSQL-backed instance starts every `ekr` through `connectors connections launch`,
so neither cortex nor any file cortex writes holds the database password.

## Why

Operator decision 2026-10-05: database credentials for a PostgreSQL store come from a Connectors connection held in Connectors' secret backends, and cortex never sees them. Design B of the cortex design note (chosen 2026-10-06, operator: "do it"): Connectors launches an operator-pinned `ekr` with the connection's password document on fd 3; EKR reads a `password_file`; cortex starts every `ekr` through the launch. Order: EKR `postgres-password-file` and the Connectors launch story can run in parallel. The
cortex story depends on both: its test needs a real `ekr` with `password_file`, and its stand-in
mimics only the Connectors verb.

## Work

1. Spec: `cortex.instance.PostgresStore` gains `connection: {adapter: String, connection: String}`, and optionally `schema_connection` for provisioning. Regenerate the spec types.
2. `ekr::Store` gains the connection reference. `Store::cmd()` builds `connectors connections launch --adapter A --connection C --consumer ekr --args '<JSON array of the ekr arguments>'`, after `Connectors::ensure_ready(A, C)`. The operator's `[consumers]` entry for `ekr` pins the `ekr` binary and lists `pass_env = ["EKR_"]`, so the `EKR_*` environment cortex sets reaches it; nothing else does. `provision()` launches through `schema_connection` when it is given. (Interface settled in Connectors 2026-10-06: `--args` and `pass_env` replaced the `-- <args>` passthrough.)
3. `install_view` writes `environment()` and the `CORTEX_CONNECTORS` path into the viewer unit, and starts `ekr view` through the launch.
4. `cortex instance check` refuses a postgres store whose `ekr.postgres/1` has no `password_file`, or whose connection is not listed. (Check: does `connections describe` expose `role@database`, so it can be compared with the DSN's user and database?)
5. Record the design decision (Design B) in the story.

## Acceptance
- The docker case in `tests/store_backend.rs` (`a_postgres_instance_is_provisioned_seeded_run_and_served_from_postgres`) runs with the application password held only in the stand-in `connectors`' own state dir. The stand-in implements `connections launch` as `exec 3<"$STATE/password.json"; exec "$@"`, and the `ekr.postgres/1` names `"/proc/self/fd/3"`.
- The test still asserts the sentinel password is absent under the instance home, in every unit file written, and in the argv of every process cortex started (logged by the stand-in).
- A new unit test in `src/ekr.rs` asserts that the postgres `Store::cmd()` program is the `connectors` binary with `connections launch … -- ` before the `ekr` path.

## Files (from the design, unverified) `spec/domains/instance.yaml`, `generated/spec-types/types.rs` (regenerated), `src/model_map.rs`, `src/instance.rs`, `src/ekr.rs`, `src/schedule.rs`, `src/main.rs`, `tests/store_backend.rs`, `tests/common/mod.rs`.

## Design

Design B, chosen 2026-10-06 by the operator ("do it"), as built in `8514d95` (its commit message is the source of each line):

- **Who holds the password:** Connectors, in its own secret backend. cortex never reads it, and no file cortex writes (spec copy, registry, unit files, logs) holds it.
- **How `ekr` gets it:** every `ekr` that opens a PostgreSQL store with `connection` set (runs, the run gate, `quality`, `schema`, `adopt`, the printed MCP line, the viewer unit) starts as `connectors connections launch --adapter A --connection C --consumer ekr --args '<JSON array>'`, after `ensure_ready`. Connectors hands the connection's `{"password": …}` document on descriptor 3; the `ekr.postgres/1` names `"password_file": "/proc/self/fd/3"` (EKR 0.0.32).
- **Which `ekr` runs:** the operator pins the binary as the Connectors consumer `ekr`, with `pass_env = ["EKR_"]`, so only the `EKR_*` environment cortex sets reaches it.
- **Refusals:** `create`, `update` and `adopt` refuse a connection whose `ekr.postgres/1` does not name that `password_file`, and a `schema_connection` without `connection`; the reasons name fields, never values.
- **Provisioning:** `schema_connection` launches `ekr postgres-schema` the same way; without it, provisioning runs directly with the operator's own owner configuration (`website/docs/operating.md`).
- **Without `connection`:** `ekr` starts directly and reads the password where its configuration says, as before.

Rejected alternatives are not recorded here: the design note that compared them was in the unit report of wave 20261006i, which was not kept.

## Scope

Where the unit landed, from `git diff --name-only 30fd665 8514d95` and the security fix `d63e8f5`, generated files left out (cited):

- `spec/domains/instance.yaml` (`ConnectionRef`, `PostgresStore.connection`, `schema_connection`), regenerated
- `src/ekr.rs` (`Launch`, `Store::cmd`, inherited `EKR_*` removal), `src/instance.rs` (launch refusals), `src/main.rs` (create, update, adopt, viewer reinstall), `src/model_map.rs`, `src/schedule.rs` (viewer unit), `src/connectors.rs`
- every other `ekr` caller that opens the store: `src/gate.rs`, `src/quality.rs`, `src/run.rs`, `src/snapshot.rs`
- `tests/store_backend.rs` (the Docker case and the security cases), `tests/spec_forms.rs`
- `website/docs/spec-file.md`, `website/docs/operating.md`

Corrections to the design's list: `tests/common/mod.rs` was not touched; `src/connectors.rs`, `src/gate.rs`, `src/quality.rs`, `src/run.rs`, `src/snapshot.rs`, `tests/spec_forms.rs` and the two documentation pages were not in it.
