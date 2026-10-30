---
format: aep.planning-md/3
id: story:window-advances-past-applied
kind: story
status: active
title: A source's window advances past applied documents under a backlog
summary: For inputs ordered by record time, `pending_since` moves to the earliest document not applied, not the window start
tags:
- review-20261030
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:poison-document-skipped
scope:
- confidence: cited
  path: spec/domains/instance.yaml
- confidence: cited
  path: src/run.rs
- confidence: cited
  path: src/sources.rs
- confidence: inferred
  path: src/state.rs
- confidence: inferred
  path: tests/connectors_walks.rs
- confidence: cited
  path: website/docs/operating.md
- confidence: cited
  path: website/docs/spec-file.md
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T10:11:04Z", actor: "human:timo", revision: 11}
- {from: "proposed", to: "active", at: "2026-10-30T10:11:04Z", actor: "human:timo", revision: 12}
---
## Outcome

Under a backlog the window advances. Today `finish` advances only when every wanted document
applied, else sets `pending_since` to the window minimum (`src/run.rs:2153-2182`, doc at
`:512-520`; `Window::of_run` at `src/sources.rs:82-102`), so one document that keeps failing pins
the window for good; downstream two sources have `pending_since` 2026-10-01 and 2026-10-03 and
`last_success_started_at: null` (cb3 report, C-06).

## Acceptance

1. With a backlog where one document fails, `pending_since` becomes that document's record time
   (minus the configured overlap), and the next run's `{since}` moves forward (new test).
2. A document skipped by `story:poison-document-skipped` does not hold the window.
3. A run where everything applied advances as today (existing window tests green).

## Findings it closes

C-06.

## Scope

Derived 2026-10-30 by `story-scoper`, read on `wave/20261030a` at b61a56c8. `git diff a870a628 HEAD`
is empty for `src/ spec/ generated/ website/ tests/`, so the story's line numbers still apply. Every
line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `src/run.rs`, `finish` and the code that calls it — cited
- **Citation check, `src/run.rs:2153-2182`:** correct apart from the first line. `finish` runs
  2154-2182 (doc comment 2154-2157, fn 2158). The branch that pins the window is 2172-2180, and its
  `pending_since.map_or(window.since_ms, |p| p.min(..))` at 2176-2179 also blocks any forward move — cited
- **Citation check, `:512-520`:** off by one. The window part of `process`'s doc comment is 513-520,
  the whole comment is 513-529, and `fn process` is at 531 — cited
- **Citation check, `src/sources.rs:82-102`:** close. `impl Window` is 81-105 and `Window::of_run`
  is 82-104 (doc 82-86, fn 87). `of_run` folds `pending_since` in with `min` at 98-101 — cited
- **Missed by the story, `src/run.rs:438-450`:** the failure path in `run`. Whenever `process`
  returns `Err`, this resets `pending_since` to `min(p, window.since_ms)`. `Err` comes from three
  places: a first batch that fails at 1099-1100, which never reaches `finish`; `BatchTooLarge` and
  `ModelUnavailable` at 1207-1209, both after `finish` at 1203, so this path overwrites any advance.
  The edit has to cover this path too — cited
- **Symbols:** `finish` (`src/run.rs:2158`); its three callers at `src/run.rs:798-800`, `:859-861`
  and `:1203-1205`; `wanted` at `src/run.rs:750-753` and `held_since` at `:756-761` in `process_in`;
  `run` (`src/run.rs:438-450`); `SeenState::select` and `wants` (`src/state.rs:165-191`);
  `OVERLAP_MS` (`src/sources.rs:71`) — cited
- **Record time:** a document's record time is `Document.published` (`src/sources.rs:34`), turned
  into milliseconds by `evidence::document_time_ms` (`src/evidence.rs:212-222`). Where `published`
  is set from the configured `time`: connectors sources at `src/sources.rs:1142-1147`, file records
  at `:1426`, structured sources at `src/structured.rs:991-995` — cited
- **Likely shape:** `finish` takes the earliest `document_time_ms` among the documents that are
  still wanted after applying, minus `OVERLAP_MS`. That set is `wants` evaluated again once the
  applies are written, and it must include documents left past `max_documents_per_run`, because
  `select` at `src/run.rs:785` consumes `docs`. With unread records, or a wanted document whose time
  cannot be read, it falls back to the window start. It sets that value instead of taking the `min`
  with the old `pending_since` — inferred
- **Also likely:** `src/state.rs` gets a helper for the earliest unapplied time — inferred.
  `src/sources.rs` changes only if `of_run` changes, and the story needs nothing there (the
  `pending_since` fold already does the job). Its doc comment at 82-86 may need rewording — inferred
- **Spec:** `spec/domains/instance.yaml` does not model the window rule. Its only mention is the
  comment at 1724, and `ConnectorsSource.time` is at 164. If the change is an UNMAPPED comment near
  `ConnectorsSource` (147-167), `generated/` is unchanged — inferred. If the rule needs a new field
  declaring the input's order, that field changes `generated/spec-types/types.rs`, `generated/schema`,
  `generated/docs` and `generated/cortex-model` through `task generate` (`Taskfile.yml:8-21`) — inferred
- **Documents:** `website/docs/spec-file.md:359-367` is the success and `{since}` rule that changes.
  The story's scope does not list this file — cited. `website/docs/operating.md:171-172` ("Which
  documents a run keeps, and its window, still go by the run's own times") needs a matching sentence
  — cited
- **Tests added:** a new end-to-end test in `tests/connectors_walks.rs`, which already has a
  `time: updated` setup (1233) and window assertions. Acceptance 1 needs a stand-in model that fails
  on one document X. `tests/common/mod.rs` has none today, and its `fail` switch at :98 is the
  `connectors` stand-in's — inferred
- **Tests that must stay green:** `src/sources.rs:1706-1742`
  (`the_window_fills_since_and_until_in_every_string_of_an_input`) — cited;
  `tests/connectors_walks.rs:1125`
  (`a_next_link_walk_stops_at_the_first_record_older_than_since_and_moves_the_window`) — cited;
  `tests/connectors_walks.rs:1165-1186`
  (`a_next_link_walk_cut_by_max_pages_before_an_older_record_holds_the_window`), which asserts that
  `pending_since` equals the run's since when records were left unread, so the unread case must keep
  the window start — cited; and `tests/run_gate.rs:335`,
  `tests/batch_fits_model_window_adv.rs:297` and `tests/e2e.rs:326`, which only check that
  `pending_since` is or is not an integer — cited
- **Confidence:** high for the edit sites, which the story cites and the tree confirms, plus the
  missed failure path. Medium for the spec and `generated/` impact, which depends on an
  ordering-declaration decision the story does not make
- **Would collide with:** any unit touching `src/run.rs` in `process_in` (the batch loop 1080-1105,
  the `finish` callers), `run`'s failure path or `finish`; any unit touching `src/state.rs`
  `SeenState`; the window section of `website/docs/spec-file.md`; `spec/domains/instance.yaml`
  `ConnectorsSource`. Its `depends_on`, `story:poison-document-skipped`, touches the same `run.rs`
  batch loop, `src/state.rs`, `instance.yaml` and `operating.md`
- **Safety fact:** moving `{since}` to the earliest unapplied document's record time minus
  `OVERLAP_MS` loses nothing only when two things hold: the operation's `{since}` filter is on the
  same field the source's `time` maps, and every record in the window was read. Otherwise the run
  must keep the window start. Level 1, stated, unproven. A second fact: `pending_since` is read only
  by `Window::of_run`, through `Planned::of` (`src/run.rs:337-344`), so its meaning reaches nothing
  else. Level 2 (`git grep pending_since`), unproven

## Decisions

Taken 2026-10-30 by the wave coordinator.

- The overlap is the fixed `OVERLAP_MS`; no setting.
- No order field: the window moves to the earliest wanted-but-unapplied record time minus `OVERLAP_MS` only when every record was read and every wanted document's time is readable; otherwise it stays at the window start.
- The failure path in `run` (`src/run.rs:438-450`) follows the same rule.
- A document skipped by `story:poison-document-skipped` counts as applied for the window.
