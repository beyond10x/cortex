---
format: aep.planning-md/3
id: story:update-refuses-rekey
kind: story
status: implemented
title: '`cortex update` refuses a spec that re-keys a source with seen documents'
summary: A change to a source's `id`, `adapter` or `operation` while its seen state is not empty is refused with its own outcome; the frozen spec is unchanged
tags:
- review-20261030
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
scope:
- confidence: inferred
  path: generated/cortex-model
- confidence: inferred
  path: generated/docs
- confidence: inferred
  path: generated/schema
- confidence: cited
  path: spec/domains/instance.yaml
- confidence: inferred
  path: spec/suite.json
- confidence: cited
  path: src/main.rs
- confidence: cited
  path: tests/conformance.rs
- confidence: cited
  path: tests/seed_change.rs
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T10:11:02Z", actor: "human:timo", revision: 9}
- {from: "proposed", to: "active", at: "2026-10-30T10:11:02Z", actor: "human:timo", revision: 10}
- {from: "active", to: "implemented", at: "2026-10-30T15:30:15Z", actor: "human:timo", revision: 11, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Outcome

`cortex update` refuses a spec that changes how a source keys its documents while the source has
seen documents. Today `update` checks the store, the seed and structured kinds but never a
source's `id`, `adapter` or `operation` (`src/main.rs:1489-1532`; spec `instance.yaml:1539-1590`
has no such outcome); downstream a change of the `id` path re-ingested 38 merge requests
(cb3 report, C-04).

Decision taken (2026-10-30, coordinator): a plain refusal. A `--rekey` mapping is not built; the
operator removes the source's state to re-key on purpose.

## Acceptance

1. `cortex update` with a spec whose source `id` path changed, on an instance whose seen state for
   that source holds keys, exits 1 with the new outcome naming the source and the field, and the
   frozen spec and state are unchanged (new test).
2. The same change on a source with empty seen state is accepted.

## Findings it closes

C-04.

## Scope

Derived 2026-10-30 by `story-scoper` on `wave/20261030a` at b61a56c8. Nothing under `src/`, `spec/`, `tests/` or `generated/` differs from a870a628, so the main citations still hold. Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `src/main.rs`, function `update` (lines 1489-1625) — cited
- **Citation check, `src/main.rs:1489-1532`:** confirmed, with one correction. `fn update` opens at 1489, the store and seed-path refusal is at 1516-1527, and the structured-kind refusal runs 1529-1535, so the refusal chain ends at 1535, not 1532. No check reads a source's `id`, `adapter` or `operation` — cited
- **Citation check, `spec/domains/instance.yaml:1539-1590`:** confirmed. `cortex.instance.UpdateInstance` runs 1539-1591 and has four outcomes: `seed-change-refused` (1550), `not-active`, `updated` and `no-such-instance` (1585). None of them covers a re-key — cited
- **Files and ranges edited:**
  - `src/main.rs:1516-1535`: add the re-key check to the refusal chain, before the frozen-seed read at 1540-1550. It compares the old spec (`layout.load_spec()`, used the same way at 1520) with the new one, source by source, matched by `name` — inferred
  - `src/main.rs:1552-1558`, the `m::UpdateInstance { .. }` literal: gains the new input field if ESS selects the outcome from the input — inferred
  - `src/main.rs:1559-1578`, the outcome `match`: one new arm with `ok: false` whose detail names the source and the field. The match has no `_` arm (1561-1578), so the new generated variant forces this arm — cited
  - `spec/domains/instance.yaml:1539-1591`: a new outcome in `cortex.instance.UpdateInstance`, ahead of `updated` — inferred
  - `spec/domains/instance.yaml:1543-1548`: possibly a new input field. The outcomes here select on the Instance subject (`when_subject`), and the Instance subject holds no seen state for its sources. Input-selected outcomes already exist elsewhere in the file (`when:` at 1735 and 2488) — inferred
  - `spec/domains/instance.yaml:1113-1118`: a new error type next to `cortex.instance.SeedChangeRefused`, with fields `name`, `source` and `field` — inferred
- **Symbols:** `update` (`src/main.rs:1489`); `m::UpdateInstanceOutcome` (`generated/cortex-model/src/instance.rs:2706`); `m::UpdateInstance` (`generated/cortex-model/src/instance.rs:2687`); `Layout::seen`, which is `state/<source name>.json` (`src/instance.rs:72-74`); `SeenState::load` and `SeenState.documents` (`src/state.rs:90`, `src/state.rs:36`); `ConnectorsSource` with `adapter`, `operation` and `id` (`spec/domains/instance.yaml:147-166`) — cited
- **Generated impact:** `task generate` (`Taskfile.yml:9-17`) wipes and regenerates `generated/cortex-model`, `generated/schema` and `generated/docs`, and rewrites `spec/suite.json`. The drift check (`Taskfile.yml:21-35`) fails on any stale file — cited
  - Files expected to change: `generated/cortex-model/src/instance.rs`, `src/behaviour.rs` (898-921), `src/ports/cortex.rs` (386-394), `PLAN.md` and `plan.json`; a new `generated/schema/schema/errors/cortex.instance.<NewError>.schema.json`; `generated/docs/docs/domains/cortex-instance.md`; `spec/suite.json` — inferred
  - `generated/spec-types` should not change: its roots are `InstanceSpec` and `AskEvent` (`Taskfile.yml:14`), and this story changes no type reachable from them — inferred
- **Tests:**
  - `tests/seed_change.rs`: add the two acceptance tests, (1) a re-key with seen keys is refused with exit 1 and leaves the frozen spec and state unchanged, (2) the same change with empty state is accepted — cited (story scope)
  - Helpers in `tests/seed_change.rs` that can be reused: `update` (62), `observed` (96), `outcome` (113) — cited
  - The world spec these tests use has only a `kind: web` source (`tests/common/mod.rs:155-159`). The new tests need a `kind: connectors` source spec plus a seeded `state/<source>.json`, following `tests/connectors_walks.rs` or `tests/connection_profile.rs` — inferred
  - `tests/conformance.rs:623-649`, the `UpdateInstance` arm: needs a new outcome arm, and the new input field if one is added, or it will not compile. The match is exhaustive (633-648) — cited
- **Documents:** only `generated/docs`. `git grep seed-change-refused` finds no hand-written page; it matches only `generated/`, `spec/`, `src/main.rs` and `tests/` — cited
- **Confidence:** high for the `src/main.rs` and spec edit sites, which the story cites and the tree confirms; medium for the exact ESS shape and the set of generated files, because both depend on whether the outcome is selected by input or by subject.
- **Would collide with:** any unit editing `update` in `src/main.rs` (1489-1625), or `cortex.instance.UpdateInstance` and its errors in `spec/domains/instance.yaml`. It also collides with every unit that touches any `spec/domains/*.yaml`, because both regenerate `generated/cortex-model`, `generated/docs`, `generated/schema` and `spec/suite.json` from scratch (`Taskfile.yml:11`) — cited. It also collides with any unit that adds an arm to `tests/conformance.rs` — inferred
- **Safety fact:** a refused update writes nothing — step 2, unproven:
  - The frozen spec is written only in the `Updated` arm (`home::write_atomic(&layout.spec(), ..)` at `src/main.rs:1580`).
  - `update` never writes seen state, so a refusal raised before `app.update_instance` (1559) leaves both the frozen spec and the state unchanged.
  - Seen state is stored per source *name* (`src/instance.rs:72-74`). A renamed source gets a new, empty state file, so a rename is not a re-key.
- **Not established:** (each line inferred)
  - Which source kinds count as re-keyed. Only `ConnectorsSource` has all three fields (`id`, `adapter`, `operation`). `StructuredConnectors` (306-) has `adapter` and `operation` but no `id`, `WebSource` has an optional `adapter` only (101), and `files` has none. I did not read how structured and web keys are derived.
  - Whether a change to `child.operation` (`ChildCall`, 139-145) or to `paging` also changes the keys.
  - How ESS should select the new outcome (input field with `when:`, or a subject field). This is a design call the story does not make.
  - That `SeenState::load` returns an empty state for a missing file. This is read from `src/state.rs:99-105`, not traced through the code.
  - The downstream evidence (C-04, "38 merge requests"). Nothing in this repository can confirm it.

## Decisions

Taken 2026-10-30 by the wave coordinator.

- Re-key fields: `ConnectorsSource` `adapter`, `operation`, `id` (and `key` once `story:record-key-template` lands); `StructuredConnectors` `adapter`, `operation`.
- The refusal is a spec outcome `rekey-refused` on `cortex.instance.UpdateInstance`, modelled on `seed-change-refused`.
- A source with no `state/<source>.json` or an empty one is not refused.
