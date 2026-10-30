---
format: aep.planning-md/3
id: story:window-relative-placeholders
kind: story
status: active
title: An input can name the window start as minutes before now
summary: '`{since_minutes_ago}` renders the window start as whole minutes before the run start, for query languages that take relative dates'
tags:
- review-20261030
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: cited
  path: spec/domains/instance.yaml
- confidence: cited
  path: src/sources.rs
- confidence: inferred
  path: tests/connectors_walks.rs
- confidence: inferred
  path: website/docs/spec-file.md
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T10:11:04Z", actor: "human:timo", revision: 9}
- {from: "proposed", to: "active", at: "2026-10-30T10:11:05Z", actor: "human:timo", revision: 10}
---
## Outcome

An input template can use `{since_minutes_ago}`. Today only `{since}`, `{until}`,
`{since_epoch}` and `{until_epoch}` exist (`src/sources.rs:130-156`, spec
`instance.yaml:155-160`); RFC 3339 is not a date the issue tracker's query language accepts, so
the downstream spec uses a fixed `updated >= -3d` and re-reads 191 or more issues per run plus one
comment call each (cb3 report, C-07).

## Acceptance

1. `updated >= -{since_minutes_ago}m` renders to the ceiling of (run start - window start) in
   minutes (unit test beside the existing placeholder test near `src/sources.rs:1710`).
2. The spec declares the placeholder; `task drift` is clean.

## Findings it closes

C-07.

## Scope

Derived 2026-10-30 by `story-scoper` against `wave/20261030a` (b61a56c8; it changes no `src`, `spec` or `generated` file since a870a628). Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `src/sources.rs`, the window placeholder renderer — cited
- **Files:** `src/sources.rs:130-156`, the story's citation, confirmed: doc comment 130-132, `fn with_window` 133-156, with the `.replace` chain at 136-145 where the new placeholder is added — cited
- **Files:** `spec/domains/instance.yaml:155-160`, the story's citation, confirmed: the `# UNMAPPED` comment above `cortex.instance.ConnectorsSource.inputs` (field at 161) — cited
- **Files:** `spec/domains/instance.yaml:314-316`, the `# UNMAPPED` comment on `cortex.instance.StructuredConnectors.inputs`. It lists the same four placeholders and goes stale unless it is edited too. The story does not cite it — cited (read in tree)
- **Files:** `src/sources.rs:1705-1742`, `#[test] fn the_window_fills_since_and_until_in_every_string_of_an_input`. Acceptance 1 says "near 1710". The test starts at 1706, and 1710 is its first `with_window` call. The new unit test or assertion goes here — cited
- **Symbols:** `with_window` (sources.rs:133), `Window { since_ms, until_ms }` (sources.rs:76-79; `Window::of_run` sets `until_ms: start_ms` at 102, so "run start" = `until_ms`), and `rfc3339` (sources.rs:107-128, only as the reference for "cut to the second") — cited
- **Callers unchanged:** `with_window` is called at `src/sources.rs:648` (Connectors source) and `src/sources.rs:1074` (structured source). Both get the new placeholder without being edited — cited
- **Spec types / generated:** no ESS type changes. Placeholders live only in `# UNMAPPED` YAML comments, not in a declared type or invariant. No `generated/` file contains `since_epoch` or `UNMAPPED` (`git grep` empty). The precedent change `0d33933f` ("sources: {since_epoch} and {until_epoch} window placeholders") touched `spec/domains/instance.yaml`, `src/sources.rs`, `tests/connectors_walks.rs` and `website/docs/spec-file.md`, and nothing under `generated/` — cited (git history)
- **Generated drift:** `task drift` (Taskfile.yml:20-31) stays clean after a comment-only spec edit. This assumes the ESS model digest in `generated/cortex-model/Cargo.toml:2` is computed over the parsed model and not raw bytes — inferred, supported by the precedent above, not run
- **Also likely:** `tests/connectors_walks.rs`, a new integration test next to `the_window_fills_since_epoch_and_until_epoch_as_whole_epoch_seconds` (1043-1080), as the precedent added. The acceptance asks only for the unit test — inferred
- **Documents:** `website/docs/spec-file.md:326` (the `inputs` row lists the placeholders) and `:337-350` (the "The window" paragraph and the `{since_epoch}` example). The precedent edited both, and `task check` runs `docs-check` — inferred
- **Documents:** `CHANGELOG.md:6`, an entry under `## Unreleased` — inferred, repository habit
- **Tests added/changed:** change or extend `src/sources.rs` `the_window_fills_since_and_until_in_every_string_of_an_input` (cited, acceptance 1); maybe a new test in `tests/connectors_walks.rs` (inferred)
- **Confidence:** high. The story cites the code and spec lines, both match the tree at a870a628, and the precedent commit shows the full set of files a change like this touches
- **Would collide with:** any unit editing `src/sources.rs` `with_window`/`Window` (lines 68-156) or its test module around 1700-1745; any unit editing the `inputs` comments of `ConnectorsSource`/`StructuredConnectors` in `spec/domains/instance.yaml`; the "The window" section of `website/docs/spec-file.md`; and, trivially, every unit that adds to `CHANGELOG.md` Unreleased
- **Safety fact:** adding `.replace("{since_minutes_ago}", …)` changes no existing rendering. `"{since}"` includes its closing brace, so it is not a substring of `"{since_minutes_ago}"`, and the replacement order at sources.rs:136-145 cannot partially consume it. The documented contract "another `{...}` … reaches Connectors unchanged" (instance.yaml:158-159; test at sources.rs:1732-1735) narrows by exactly this one token. Level 3 (walked the replace chain), unproven

## Decisions

Taken 2026-10-30 by the wave coordinator.

- `{since_minutes_ago}` is the ceiling of (run start − since) in whole minutes from the raw milliseconds; it renders as the number only, e.g. `-{since_minutes_ago}m` in the spec.
- Both UNMAPPED comments (`instance.yaml:155-160`, `:314-316`) and `website/docs/spec-file.md` list it.
