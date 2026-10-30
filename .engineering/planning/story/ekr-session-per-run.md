---
format: aep.planning-md/3
id: story:ekr-session-per-run
kind: story
status: active
title: A run's store reads go through one `ekr session`
summary: ontology, snapshot, mint, propose, validate, commit, quality and head go through one session per run; readiness is checked once per run
tags:
- review-20261030
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:one-store-read-per-run
scope:
- confidence: cited
  path: src/connectors.rs
- confidence: cited
  path: src/ekr.rs
- confidence: cited
  path: src/gate.rs
- confidence: inferred
  path: src/instance.rs
- confidence: cited
  path: src/known.rs
- confidence: inferred
  path: src/main.rs
- confidence: cited
  path: src/run.rs
- confidence: inferred
  path: src/snapshot.rs
- confidence: cited
  path: src/structured.rs
- confidence: inferred
  path: tests/ekr_session.rs
- confidence: inferred
  path: tests/known_entities_adversary.rs
- confidence: inferred
  path: tests/parent_prefix.rs
- confidence: inferred
  path: tests/parent_prefix_adv.rs
- confidence: inferred
  path: tests/parent_prefix_adv2.rs
- confidence: cited
  path: tests/store_backend.rs
revision: 20
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T10:11:04Z", actor: "human:timo", revision: 19}
- {from: "proposed", to: "active", at: "2026-10-30T10:11:04Z", actor: "human:timo", revision: 20}
---
## Outcome

A run starts one `ekr session` (through one `connections launch` for a launched store, with
`EKR_STAGE` set when a stage is open) and sends its read and transact verbs through it;
`apply-extraction` and the `stage` verbs stay one-shot. Today every verb is its own process
(`src/ekr.rs:88-98`, `:317-322`) and, on a launched store, each is also a `connections launch`
and a `connections list` readiness check (`src/ekr.rs:225-245`, `src/connectors.rs:322-325`).
EKR measured `ekr resolve` at 0.77-1.47 s as a process and 20-34 ms inside a session (EKR
`CHANGELOG.md:1361-1363`). The gate's `ekr quality` (46-48 s measured downstream) runs through
the same session (`src/gate.rs:116-128`).

## Acceptance

1. With argv-logging stand-ins, a run's spawn count is at most one `ekr session`, plus one
   `apply-extraction` per batch and the stage verbs; `connections list` runs at most once per
   run (new test).
2. A session joined to an open stage reads the stage's state: a value applied by a one-shot
   `apply-extraction` in batch k is visible to the session's snapshot in batch k+1 (new test
   against the real `ekr` pinned by the tests; if the pinned EKR does not show it, the story
   records that and keeps the one-shot snapshot).
3. Snapshot output is parsed into typed structs from the pipe, not buffered whole into a
   `serde_json::Value` (P-11).

## Findings it closes

P-03, P-07, P-11.

## Open upstream questions

Whether a second `snapshot` inside one session is cheaper than a one-shot one, and whether a
session started with `EKR_STAGE` sees commits a separate process made, are EKR's to answer;
acceptance 2 measures the second.

## Scope

Derived 2026-10-30 by `story-scoper` against tree `b61a56c8`, whose `src/`, `tests/`, `spec/` and
`generated/` are identical to main `a870a628`. Every line is **cited** (read from the story or the
tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `src/ekr.rs`: a run-scoped `ekr session` handle and the store verbs routed
  through it — cited
- **Citation check (story body, on main a870a628):**
  - `src/ekr.rs:88-98` is `fn run`, one `Command::output()` per verb: confirmed — cited
  - `src/ekr.rs:317-322` is `Store::command`, which calls `Launch::ready` before every verb:
    confirmed — cited
  - `src/ekr.rs:225-245` is close but not exact. `Launch::command` (the `connections launch` argv)
    runs 220-245. The readiness check is not in that range: it is `Launch::ready` at 249-254,
    called from `Store::command` 318-320 and `Store::stage_command` 334-338 — cited
  - `src/connectors.rs:322-325` is inside `Connectors::revalidate_once` (317-); the
    `connections list` call is at 323-326. The path to it is `ensure_ready` 275-277, then
    `revalidate` 284-286, then `attempts` 292-315: confirmed, give or take one line — cited
  - `src/gate.rs:116-128` is `gate::quality`, a one-shot `store.command(["quality"])`: confirmed —
    cited
  - EKR `CHANGELOG.md:1361-1363` (EKR checkout `efe53653`, release 0.0.12 at line 1336): `ekr
    resolve` takes 1.47 s on file and 0.77 s on SQLite, and 33.9 ms and 19.8 ms inside `ekr
    session`. Confirmed — cited
- **Files and symbols edited:**
  - `src/ekr.rs:88-98` `run`, `:111-113` `json`, `:199-214` `struct Store` (it is `#[derive(Clone)]`
    and has 7 struct literals, listed below), `:247-254` `Launch::ready`, `:285-322`
    `Store::{cmd,store_cmd,command}`, `:445-451` `head`, `:488-499` `ontology` / `snapshot`,
    `:501-585` `transact` / `mint` / `submit` (the propose, validate and commit sequence). A new
    session type with spawn, request, answer and close also lands here — cited for the symbols,
    inferred for the new type
  - `src/gate.rs:115-128` `quality`, through the session — cited
  - `src/run.rs:805-825` `process_in`: the store is built at 805, `Staged::begin` at 823-825 (the
    session must start after `EKR_STAGE` is set). Ontology reads at 810, 902, 920 and 964;
    `for_batch` at 923 and 966; `apply` at 1131 stays one-shot — cited for the call sites,
    inferred that the session starts here
  - `src/run.rs:1644-1700` `held_to_gate` (the `gate::quality` call at 1660, then `Staged::publish`),
    `:1880-1894` `struct Records` (`store` field), `:2026-2091` `end_dropped` (snapshot and
    ontology at 2050-2051, `transact` at 2066) — cited for the call sites, inferred for the edits
  - `src/run.rs:1447-1626` `Staged`: `begin`, `recover`, `publish` and `abandon` call the stage verbs
    through `stage_command`, which runs `ready` each time. This decides whether `connections list`
    can run only once per run — inferred
- **P-11 (typed snapshot):**
  - `src/known.rs:44-101` `store_hits` and `:153-170` `for_batch` — cited
  - `src/structured.rs:1218-1418` `Source::ended`, plus the unit tests at 1887, 1933, 1952 and 1988
    that build snapshot `Value`s — cited
  - The `Store::snapshot` return type changes from `serde_json::Value` (`Cargo.toml:28` enables
    `arbitrary_precision`) — cited
- **Also likely:**
  - `src/connectors.rs`: the story cites it as the defect site. The once-per-run readiness can
    live entirely in `src/ekr.rs`, so this file may not be edited — inferred
  - `Store` struct literals to update if `Store` gains a field: `src/instance.rs:116`,
    `src/main.rs:753`, `src/run.rs:2430`, `src/snapshot.rs:763`,
    `tests/store_backend.rs:2191,2214` — cited for the literals, inferred that they change
  - `src/instance.rs:397-420` `evidence_identities` is a second raw snapshot parse. Its only
    caller, `main.rs:1285`, is not on the run path — inferred out of scope
  - `src/schema.rs:1047-1430` (the `cortex update` schema apply: mint, submit, head, quality) is not
    a run, so it stays one-shot — inferred out of scope
- **Tests changed:**
  - `tests/store_backend.rs:832-849`: `a_postgres_run_that_fails_its_gate_is_undone` expects
    `{stage}|apply-extraction` and `{stage}|ontology` lines in the stage log. The ontology line
    becomes `{stage}|session` — cited for the assertion, inferred for the change
  - `tests/store_backend.rs:619-638`: the launch log still contains `["head"]` and `["seed",`,
    which are not run verbs, so it likely survives — inferred
  - `tests/known_entities_adversary.rs:382`, `tests/parent_prefix.rs:301`,
    `tests/parent_prefix_adv.rs:87` and `tests/parent_prefix_adv2.rs:134,165,203` pass a `Value`
    snapshot to `store_hits` or `ended`, and break if those take a typed snapshot — cited for the
    call sites, inferred for the change
- **Tests added:** a spawn-count test with argv-logging stand-ins (acceptance 1), reusing
  `tests/store_backend.rs:424` `logging_ekr`, `:442` `launching_connectors` and `:702`
  `stage_logging_ekr`. A real-`ekr` stage-visibility test (acceptance 2) against the 0.0.34 pin in
  `tests/common/mod.rs:22,152`. File name not fixed; probably a new
  `tests/ekr_session.rs` — inferred
- **Spec / generated:**
  - `spec/domains/*.yaml`: no type change. `cortex.instance.PostgresStore`
    (`instance.yaml:554-561`) still says "every `ekr` of the store starts through `connectors
    connections launch`", which stays true of a session — cited
  - The UNMAPPED comment at `instance.yaml:1702-1704` belongs to `story:one-store-read-per-run` —
    cited
  - `generated/`: no impact — inferred, since no ESS type changes
- **Documents:** `AGENTS.md` may get a row on `ekr session` (as `:171-173` record EKR facts) —
  inferred
- **Upstream (EKR 0.0.34 `docs/cli.md`):**
  - `:1463-1540`: the session serves ontology, snapshot, mint, propose, validate, commit, quality
    and head, and refuses `apply-extraction` (`session-verb-refused`). Every request reads "the
    store as it stands when the request is read … one another process committed" — cited
  - `~:1845`: `ekr session` joins `EKR_STAGE` — cited
- **Confidence:** high for `src/ekr.rs`, `src/gate.rs` and the `src/run.rs` call sites, because
  the story names them and the tree confirms them. Medium for the P-11 spread and the test edits,
  which depend on the snapshot type the implementor picks.
- **Would collide with:**
  - any unit touching `src/ekr.rs` `Store` or its verbs
  - `src/run.rs` `process_in` (800-1000) and the store-holding helpers (`Records`, `end_dropped`,
    `held_to_gate`, `Staged`)
  - `src/known.rs` `for_batch` / `store_hits`; `story:one-store-read-per-run` edits the same lines
    and is this story's `depends_on`
  - `src/structured.rs` `Source::ended`
  - `tests/store_backend.rs` (stage log)
- **Safety fact:** acceptance 1's "`connections list` at most once per run" is safe only if the
  per-batch one-shot `apply-extraction` and the stage verbs can still launch after the
  connection's validation evidence lapses. Today each launch is preceded by `Launch::ready`
  because "a launch with lapsed evidence is refused" (`src/ekr.rs:247-248`), and Connectors keeps
  evidence for about 60 s on the shipped HTTP adapters (`src/connectors.rs:271-273`). Step 2,
  unproven.

## Decisions

Taken 2026-10-30 by the wave coordinator.

- The session is opened after `Staged::begin` and closed before `Staged::publish`/`abandon`.
- `apply-extraction` and the stage verbs stay one-shot, each still preceded by `Launch::ready`; only session verbs drop the per-call readiness check. Acceptance 1 counts `connections list` for session verbs only.
- The typed snapshot (P-11) is in scope only if it stays inside `src/ekr.rs`, `src/known.rs` and `src/structured.rs`; otherwise it is left out and named in the report.
- The new test file is `tests/ekr_session.rs`.
