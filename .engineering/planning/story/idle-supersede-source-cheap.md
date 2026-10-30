---
format: aep.planning-md/3
id: story:idle-supersede-source-cheap
kind: story
status: active
title: 'An unchanged `dropped: Supersede` source begins no stage and takes no snapshot'
summary: 'A structured source with `dropped: Supersede` and nothing selected computes its end operations before opening a stage, and skips `end_dropped` when its listing is unchanged'
tags:
- review-20261030
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:one-store-read-per-run
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: cited
  path: src/run.rs
- confidence: cited
  path: src/state.rs
- confidence: cited
  path: src/structured.rs
- confidence: cited
  path: tests/parent_prefix.rs
- confidence: cited
  path: tests/parent_prefix_adv.rs
- confidence: cited
  path: tests/parent_prefix_adv2.rs
- confidence: inferred
  path: tests/store_backend.rs
- confidence: cited
  path: tests/structured_compare.rs
- confidence: cited
  path: tests/structured_compare_adv.rs
- confidence: cited
  path: website/docs/spec-file.md
revision: 16
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T10:11:01Z", actor: "human:timo", revision: 15}
- {from: "proposed", to: "active", at: "2026-10-30T10:11:01Z", actor: "human:timo", revision: 16}
---
## Outcome

A structured source with `dropped: Supersede` whose listing did not change since its last
successful end begins no stage and reads no snapshot. Today such a source always begins a stage,
which copies the whole store (EKR `docs/cli.md:2070-2081`), and takes a full snapshot even when
nothing was selected (`src/run.rs:735-737`, `:794`, `:823-826`, `end_dropped` at `:2049-2057`).
This is the only zero-document path with a full read, and plausibly the 5 min 15 s zero-document
run measured downstream (P-04; which source it was is not known).

`Source::ended` is quadratic in this source's assertions and builds an O(store) set even when
nothing is retracted (`src/structured.rs:1318-1326`, `:1342-1356`, `:1391-1399`, P-10).

## Acceptance

1. A run of a `dropped: Supersede` source with an unchanged listing and zero selected documents
   makes zero `stage begin` and zero `snapshot` calls (argv-logging stand-in, new test).
2. A changed listing still ends the dropped values exactly as today (`tests/structured_compare*.rs`
   green).
3. `ended` returns early when nothing is retracted, and indexes kept assertions instead of
   scanning them per ended one.

## Findings it closes

P-04, P-10.

## Scope

Derived 2026-10-30 by `story-scoper` against main a870a628 (`src/`, `tests/`, `spec/` and `generated/`
are identical on wave/20261030a). Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `src/run.rs`, `process_in` (`:567` on), the zero-document branch — cited
- **Citations checked:** `src/run.rs:735-737` (`listed`) is right — cited
- **Citations checked:** `src/run.rs:794` should be `:795` (`if selected.is_empty() && listed.is_none()`, the early return at `:795-803`) — cited
- **Citations checked:** `src/run.rs:823-826` is right (`Staged::begin` at `:824-826`, PostgreSQL only, `:1459-1461`) — cited
- **Citations checked:** `end_dropped` `:2049-2057` should be `:2050-2056` (`store.snapshot()` `:2050`, `ontology()` `:2051`, `ended` `:2052-2058`); the function is `:2026-2087`, called at `:851-853` — cited
- **Citations checked:** `src/structured.rs:1318-1326` should be `:1321-1336` (`earlier` closure `:1321-1330`, `kept` `:1331-1336`) — cited
- **Citations checked:** `src/structured.rs:1342-1356` (scan of `mine` per ended assertion, `:1349`) and `:1391-1399` (`staying`, built even when `retracted` is empty) are right — cited
- **Citations checked:** EKR `docs/cli.md:2070-2081` (`ekr stage begin` copies the store) matches the local EKR checkout at efe5365 — cited
- **Edits, run.rs:** move the early return at `:795-803` so it also covers a `Supersede` source with an unchanged listing, before `store_handle` `:805`, the `ontology()` read `:809-813`, `BeforeApply::new` `:817-821` and `Staged::begin` `:824-826` — inferred
- **Edits, run.rs:** record the listing fingerprint after a successful `end_dropped` (`:851-853` or `:2026-2087`), and persist it through `finish` `:2158-2182` — inferred
- **Edits, state.rs:** one new optional, serde-defaulted field on `SeenState` (`src/state.rs:34-63`) for the fingerprint of the last successfully ended listing; `load` `:90` and `read_document` `:120` should tolerate its absence — inferred
- **Edits, state.rs:** the fingerprint is probably keys plus `doc_hash` (`src/state.rs:70`) — inferred
- **Edits, structured.rs:** `Source::ended` `:1218-1418`: return early when nothing is retracted (before `gone` `:1384`), and index kept assertions by `(records, subject, predicate)` instead of the per-ended scan at `:1342-1356`; signature unchanged — inferred
- **Symbols:** `process_in`, `end_dropped`, `finish`, `SeenState`, `structured::Source::ended`, `structured::Source::listed` (`:1097`) — cited
- **spec/domains / generated/:** none. `SeenState`'s run-level fields are not modelled in `spec/domains/instance.yaml` (`git grep last_success_started_at -- spec generated` is empty), and `DropPolicy` (`:342-344`) does not change — inferred
- **Tests added:** a new argv-logging test, with zero `snapshot` on SQLite and zero `stage begin` on PostgreSQL with a gate. Pattern: `ekr-staged` stand-in at `tests/store_backend.rs:700-707`; probably in `tests/structured_compare.rs` or `tests/store_backend.rs` — inferred
- **Tests added:** unit tests for `ended`'s early return and index in the `structured.rs` test module (existing `ended` tests at `:1933`, `:1952`, `:1988`) — inferred
- **Tests kept green:** `tests/structured_compare.rs`, `tests/structured_compare_adv.rs` (acceptance 2) — cited
- **Tests kept green:** `ended` callers `tests/parent_prefix.rs:301`, `tests/parent_prefix_adv.rs:87`, `tests/parent_prefix_adv2.rs:134,165,203`, unaffected if the signature holds — cited
- **Documents:** `website/docs/spec-file.md:629-630` ("after each run cortex reads the store (`ekr snapshot`)") becomes false and needs a line — cited
- **Documents:** a `CHANGELOG.md` `## Unreleased` entry — inferred
- **Confidence:** high for the edit sites — the story cites them and each was re-read in the tree
- **Confidence:** medium for the state field and the test placement — inferred from the design, not named in the story
- **Would collide with:** any unit editing `process_in` in `src/run.rs`, notably `story:one-store-read-per-run` (its `depends_on`, which edits `src/run.rs:919-967`) — inferred
- **Would collide with:** any unit touching `SeenState` in `src/state.rs`, or `Source::ended` in `src/structured.rs` — inferred
- **Safety fact:** skipping `end_dropped` is only safe if, with the listing, the `unsettled` set (`src/run.rs:2040-2049`, from `seen` hashes) and `held` unchanged since the last successful end, `ended` would return no operations. That fails if another writer changed this source's assertions, or a held child failure cleared since. Level 2 (`src/run.rs:2040-2058`), unproven — inferred

## Decisions

Taken 2026-10-30 by the wave coordinator.

- The fingerprint covers the listing keys and hashes, the `unsettled` set and `held`; any change runs `end_dropped` as today.
- The fingerprint lives on `SeenState` as an optional serde-defaulted field.
- Acceptance 1 is tested on SQLite (zero `snapshot`); the PostgreSQL `stage begin` count is tested only where `tests/store_backend.rs` already runs PostgreSQL.
- `website/docs/spec-file.md:629-630` is corrected.
