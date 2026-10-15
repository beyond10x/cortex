---
format: aep.planning-md/3
id: story:connection-by-profile
kind: story
status: draft
title: A spec names a Connectors connection by adapter and profile, resolved at run time
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
revision: 1
---
## Outcome

A spec may name a Connectors connection by adapter and profile instead of by id, for example
`connection: {adapter: tavily, profile: tavily.api-key}`. cortex resolves it, each time it needs
the connection, to the one live connection of that adapter and profile that `connectors
connections list` returns. One match is used; none or several is refused with a reason that names
the adapter, the profile, the count and every matching id. The id form (`connection: conn_…`)
stays valid. This holds for a source's connection and for a `postgres` store's `connection` and
`schema_connection` (the `ekr` launch).

## Why

A downstream consumer runs cortex in pods whose Connectors state is ephemeral: each pod runs
`setup init` then `connections connect`, so each pod gets a new connection id. A stored spec that
names `conn_A` fails in a pod that connected `conn_B`. Verified in this tree:

- A source names its connection as a bare id: `connection: String` on `WebSource`,
  `ConnectorsSource` and `StructuredConnectors` (`spec/domains/instance.yaml:102`, `:129`, `:221`).
  The store's `cortex.instance.ConnectionRef` is `{adapter, connection}`, both strings
  (`:295-301`), used by `PostgresStore.connection` and `schema_connection` (`:303-311`).
- cortex matches the id exactly in `connections list --adapter <adapter>`:
  `Connectors::connection_state` (`src/connectors.rs:96-111`) and `revalidate`
  (`src/connectors.rs:121-133`, a miss is `no connection <id>`). `invoke` revalidates first
  (`src/connectors.rs:216`); the `ekr` launch passes `--connection <id>`
  (`src/ekr.rs:213-222`) after `ensure_ready` (`:231-237`), built from the spec in
  `src/instance.rs:200-211`.
- `create`, `update` and `adopt` check every named connection with `missing_connection`
  (`src/main.rs:464-485`, from `spec::connections` at `src/spec.rs:227-250` and the store's two
  refs) and answer `connection-missing` (`spec/domains/instance.yaml:866-872`, error
  `ConnectionMissing` with one `connection` string, `:629-634`). A connection is accepted in any
  state but `revoked`.
- Connectors already reports the profile: each entry of a `ConnectionListResult` carries
  `adapter`, `instance_id`, `connection`, `profile`, `revision` and `state` (Connectors
  `contracts/cli/v1alpha1/fixtures/values.json`, fixture `valid-ConnectionListResult-12`). The
  list is paged with opaque cursors (Connectors `docs/local-runtime-foundation.md:159`); cortex
  reads its first page only. The stand-ins in `tests/` list `adapter`, `connection`, `state` and
  `revision`, no `profile` (`tests/common/mod.rs:88`).

## Domain relations

No new noun. The reference is a value type of the instance spec: `cortex.instance.ConnectionRef`
(`spec/domains/instance.yaml:295-301`) gains the profile form, and the three source settings that
hold a bare `connection` string take the same choice. A connection is Connectors' noun; cortex
names it and never owns or reads it.

## Work

- Spec first: in `spec/domains/instance.yaml`, let a connection be named either by id or by
  `{adapter, profile}`: on `ConnectionRef`, and on the `connection` of `WebSource`,
  `ConnectorsSource` and `StructuredConnectors`. If ESS cannot type a field that is a string or an
  object, use a tagged form or an optional `profile` field beside an optional `connection`, and
  record that choice in the spec comment. A source's profile reference names the same adapter as
  the source, or the spec is refused. Widen the `connection-missing` outcome text and
  `ConnectionMissing` so the reason carries the adapter, the profile, the count and the matching
  ids. Run `ess specify validate` and the conformance suite green, then `task generate`; the
  JSON Schema of the spec file (`website/static/schemas/`) follows. Only then change the code.
- `src/connectors.rs`: a `resolve(adapter, reference)` that answers the id: the id form as
  given; the profile form by reading `connections list --adapter <adapter>` and keeping the entries
  whose `profile` equals it and whose `state` is not `revoked`. The profile form takes any state but
  `revoked`, as the id form does today, because cortex revalidates a `pending` connection itself.
  Requiring `ready` would refuse every connection whose evidence lapsed (60 s).
- Resolve once per command, before the fetch (`src/run.rs`, `src/sources.rs`), before the launch
  (`src/instance.rs:200`, `src/ekr.rs`), and in `missing_connection` (`src/main.rs:464`). The
  frozen spec keeps the reference, not the resolved id.
- Tests: a stand-in `connectors` whose list carries `profile`, in `tests/connectors_walks.rs` or
  a new test file, and a unit test of `resolve`.
- Document both forms on the spec-file page and in the operating page's container section.

## Acceptance

- `ess specify validate` passes with the profile form on `ConnectionRef` and the three source
  settings, and `task drift` is clean after `task generate`.
- A stand-in `connectors` that lists one `tavily` connection `conn_B` with `profile:
  tavily.api-key` serves a source whose spec names `{adapter: tavily, profile: tavily.api-key}`:
  `cortex create` succeeds and `cortex run` invokes the operation with `--connection conn_B`.
- With two such connections (`conn_B`, `conn_C`), `cortex create` answers `connection-missing` and
  `cortex run` answers `fetch-failed`, each with a reason naming `tavily`, `tavily.api-key`, the
  count 2 and both ids. With none, the reason names the count 0.
- A `postgres` store whose `connection` names `{adapter: <a>, profile: <p>}` starts `ekr` through
  `connections launch --connection <the one matching id>`.
- A spec that names `conn_test` by id runs as before.

## Not in this story

Selecting by `instance_id` or by external identity; reading past the first page of
`connections list`; choosing among several matches (the newest, the first); creating a connection
when none matches.
