---
format: aep.planning-md/3
id: story:connectors-source-walks
kind: story
status: implemented
title: A connectors source pages, calls a child per record and reads only what changed
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:spec-standalone-types
- depends_on: story:seen-documents-modelled
- depends_on: story:redaction-before-model
scope:
- confidence: cited
  path: src/connectors.rs
- confidence: cited
  path: src/run.rs
- confidence: cited
  path: src/sources.rs
- confidence: cited
  path: src/state.rs
- confidence: inferred
  path: tests/connectors_walks.rs
revision: 16
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T13:10:33Z", actor: "agent:claude", revision: 13, decided_on: {"recorded":{"review_outcome":7}}}
- {from: "proposed", to: "active", at: "2026-10-05T13:10:33Z", actor: "agent:claude", revision: 14, decided_on: {"recorded":{"review_outcome":7}}}
- {from: "active", to: "implemented", at: "2026-10-05T14:10:03Z", actor: "agent:claude", revision: 16, decided_on: {"recorded":{"test_result":1,"review_outcome":7}}}
---
## Outcome

A `connectors` source reads every page of a paged operation, calls a child operation once per
record, and asks only for what changed since its last successful run.

## Work

- `Paging` (`PageNumber`, `Token`, `Keyset`): `param` is the input field to advance, `next` the
  answer path of the next token or key; stop at an empty page, a missing `next`, or `max_pages`.
- `ChildCall`: for each record, invoke `operation` with `input` (templated from the record, e.g.
  `{"issue": "{key}"}`) and attach the child's records (comments per issue, jobs per pipeline) to
  the parent document's text. When the `ChildCall` has `paging`, every page of the child answer is
  walked the same way.
- `{since}` / `{until}` in `inputs`: RFC 3339 instants. `SeenState` (`src/state.rs`) gains the
  start instant of the source's last successful run, written when a run succeeds;
  `{since}` = that instant (first run: now minus the source's `policy.refresh_after_days`),
  `{until}` = this run's start. `src/run.rs` passes the window to `sources::fetch`
  (`src/sources.rs:67`), the one call site at `src/run.rs:70`.

## Acceptance

`tests/connectors_walks.rs`: a stand-in `connectors` that serves 3 pages by token, each with 2
parents and 2 child records per parent, yields a run that applies exactly 6 parent documents, each
carrying its 2 children in its text.

The window is held by its own check in `tests/connectors_walks.rs`: on a fresh instance the stand-in
receives `{since}` = run start minus `refresh_after_days`; the run records its start instant in the
source's state; a second run's stand-in receives exactly that instant as `{since}`.

## Depends on

`story:spec-standalone-types` (the types), `story:seen-documents-modelled` (both edit
`src/state.rs`), `story:redaction-before-model` (both edit `src/run.rs` at the fetch call site).
`story:structured-source` and `story:timer-runs-unattended` come after this story (they edit
`src/sources.rs` and `src/state.rs`; their edges record it).

## Scope

Landed 2026-10-05 in `c3fa691` (wave 20261005d, merged `3079840`).

- **Files:** `src/sources.rs`, `src/run.rs`, `src/state.rs`, `src/main.rs` (`truncated` and `skipped` in the run result), `website/docs/spec-file.md`, `tests/connectors_walks.rs` (new, 21 cases)
- **Not needed:** `src/connectors.rs`
- **Beyond the story, decided during the wave:** a 5-minute window overlap; `held_since` (changes held back by `refresh_after_days`), `pending_since` (runs that stopped or failed) and per-key child-failure counts in the source state, all omitted when empty so old state loads; a child call failing on 3 runs in a row is skipped; the hash is taken before the `max_chars_per_document` cut and cut documents count as `truncated`
- **Documented limits:** `PageNumber` paging over a changing result set can skip records stamped before the overlap; text past the cut does not reach the model; `refresh_after_days: 0` gives the first run an empty window
- **Review:** `review-result:adversary-connectors-source-walks-pass-1` (8) and `-pass-2` (6), fixed or documented; coordinator check of the final diff
