---
format: aep.planning-md/3
id: story:durable-home-writes
kind: story
status: active
title: Home files survive a crash and a failed create leaves no half-registered instance
summary: '`write_atomic` syncs before rename; an unreadable status file is an error, not an empty one; `create` rolls back on a later failure; restore writes state before reporting'
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: generated/docs/docs/domains/cortex-instance.md
- confidence: inferred
  path: spec/domains/instance.yaml
- confidence: cited
  path: src/home.rs
- confidence: cited
  path: src/main.rs
- confidence: cited
  path: src/ports.rs
- confidence: inferred
  path: src/run.rs
- confidence: cited
  path: src/snapshot.rs
- confidence: cited
  path: src/status.rs
- confidence: inferred
  path: tests/snapshots.rs
- confidence: inferred
  path: tests/unit_homes.rs
revision: 16
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T10:11:03Z", actor: "human:timo", revision: 15}
- {from: "proposed", to: "active", at: "2026-10-30T10:11:03Z", actor: "human:timo", revision: 16}
---
## Outcome

- `write_atomic` writes then renames without `sync_all` (`src/home.rs:102-112`), so a crash can
  leave `registry.json`, status or state empty, after which every command exits 2 (R5-10);
- `status::load` returns the default on a read or parse error and the next run rewrites the file
  with its counts reset (`src/status.rs:141-146`, `:182-194`; R5-09);
- `create` returns `fail(e)` after the instance is registered and its meta saved, so a retry says
  the name is taken (`src/main.rs:1032`, `:1070-1083`, `:1413-1434`; R5-11);
- restore copies the store before writing `state/`, and a `write_state` failure leaves the two
  disagreeing (`src/snapshot.rs:719-725`; R2-10).

## Acceptance

1. `write_atomic` calls `sync_all` on the file before rename and on the directory after (unit test
   asserting the sequence through a seam, or code read in review if no seam exists — say which).
2. A corrupt status file makes `cortex sources` exit non-zero naming the file; it is not rewritten.
3. A `create` whose source install fails leaves no registry entry (new test).
4. A restore whose `write_state` fails reports the store as restored and the state as not.

## Findings it closes

R2-10, R5-09, R5-10, R5-11.

## Scope

Derived 2026-10-30 by `story-scoper` against `wave/20261030a` (b61a56c8). `git diff a870a628 HEAD -- src tests spec generated` is empty, so the body's line numbers still apply. Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** the `cortex` binary crate's home/status/snapshot layer, `src/` at the repo root — cited
- **Files and symbols (body citations checked):**
  - `src/home.rs:102-112` `write_atomic`: write, then rename, with no `sync_all`. Body is correct. The fix adds `File::sync_all` before the rename and a directory fsync after it, inside these lines only — cited
  - `src/status.rs:141-146` `status::load`: `.ok()…unwrap_or_default()` on both read and parse errors. Body is correct — cited
  - `src/status.rs:172-194` `record_run`: the load is at `:181` and the rewrite (`write_atomic`) at `:188-193`. Body's `:182-194` is off by one at the start — cited
  - `src/main.rs:1032` `create`: `app.create_instance` returns `Err → fail(e)`. Body is correct, but the fail-after-register sites are `:1067-1069` (`save_meta`) and `:1078-1080` (`add_source`), inside `fn create` `:922-1138` — cited
  - `src/main.rs:1413-1434` belongs to **`adopt`** (`fn adopt` `:1304`, the `Adopted` arm), not `create`. It is the same `save_meta`/`add_source → fail(e)` pattern at `:1421-1423` and `:1432-1434`. Body cites the lines correctly but names the wrong function — cited
  - `src/main.rs:441-445`: the registry is saved after `dispatch` whatever the command returned. A rollback has to remove the entry from `Shared.registry` (`src/ports.rs:68-90` `InstanceStorage`) and the instance dir before this save — cited
  - `src/snapshot.rs:719-725` `restore_held` (`:663-736`): store copied, then `write_state(...).map_err(RestoreError::Failed)?`. Body is correct — cited
  - `src/snapshot.rs:524-539` `RestoreError` / `Restored`: probably gains a "state failed" field or variant for acceptance 4 — inferred
  - `src/main.rs:1670-1758` `fn restore`: reports `state_restored` at `:1724`, and `RestoreError::Failed → fail(e)` at `:1688` — cited, edit inferred
- **Also likely:** `src/run.rs:1708-1752` `undo`. It calls `restore_held` (`:1722`) and treats `RestoreError::Failed` as "the store still holds the run". That message is false once the store has been copied, so it changes together with `restore_held`'s contract — inferred
- **Also likely:** `src/home.rs:68-86` `load_registry` and `src/status.rs:149-156` `overlay`, `:315-321` `metrics_text`. If `load` returns `Result`, every caller changes signature — inferred
- **Spec / generated:** probably no change to `spec/domains/instance.yaml` types. The corrupt-status error is a CLI `fail` (exit 2), not a declared outcome. If acceptance 4 is met inside the existing `restored` outcome, its summary at `spec/domains/instance.yaml:2177-2190` ("The store and the instance state were … put back") becomes inaccurate. Editing it regenerates `generated/docs/docs/domains/cortex-instance.md:1161`. `generated/cortex-model` is untouched unless a new outcome is added — inferred
- **Tests:**
  - unit tests in `src/home.rs` `mod tests` (`:350-540`) for the sync seam — inferred
  - `src/status.rs` `mod tests` (`:565-700`) for corrupt status → `Err` — inferred
  - `src/snapshot.rs` `mod tests` (`:738-1042`, `restore_held` tests at `:930-1011`) for a failing `write_state` — inferred
  - a new integration test for create rollback, likely beside `tests/unit_homes.rs` or `tests/adopt.rs` — inferred
  - `tests/snapshots.rs` if restore output changes — inferred
- **Documents:** `CHANGELOG.md` entry — inferred
- **Confidence:** high for `home.rs`, `status.rs`, `snapshot.rs` and `main.rs` create/adopt/restore, because the body names the sites and each was read. Medium for `run.rs` and the spec summary, which are reached by reading callers.
- **Would collide with:** any unit touching `src/main.rs` `create`/`adopt`/`restore` or the post-dispatch registry save; `src/status.rs` load/record/metrics; `src/snapshot.rs` restore; `src/run.rs` `undo`; `src/home.rs` `write_atomic`/`load_registry`
- **Safety fact:** `status::overlay` runs inside `Home::load_registry` (`src/home.rs:83`), which every locked command calls at `src/main.rs:411`, and the timer probe also calls it (`src/schedule.rs:357`). So "a corrupt status file is an error" makes **every** command exit 2, not only `cortex sources`, unless the error is raised only on the `sources`/`record_run` paths. Proof level 2 (file:line), unproven.

## Decisions

Taken 2026-10-30 by the wave coordinator.

- A corrupt status file is an error on `cortex sources` and `record_run` only; `load_registry`'s overlay keeps working and logs the error.
- Acceptance 3 is the `app.add_source`/`save_meta` failure path in `create` and `adopt`; a failed systemd install keeps today's behaviour.
- Acceptance 4 adds `state_restored: false` with the error to the existing `restored` detail and corrects the outcome summary; no new outcome.
