---
format: aep.planning-md/3
id: story:unit-names-unique
kind: story
status: implemented
title: Two instances never write one systemd unit, and `update` leaves a disabled source's timer off
summary: Unit names cannot collide across instance and source names; reinstalling units keeps a disabled source disabled
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
scope:
- confidence: inferred
  path: generated/cortex-model
- confidence: inferred
  path: spec/domains/instance.yaml
- confidence: cited
  path: src/main.rs
- confidence: cited
  path: src/schedule.rs
- confidence: cited
  path: src/spec.rs
- confidence: inferred
  path: tests/unattended.rs
- confidence: cited
  path: tests/unit_homes_adversary.rs
- confidence: inferred
  path: website/docs/operating.md
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T10:11:03Z", actor: "human:timo", revision: 12}
- {from: "proposed", to: "active", at: "2026-10-30T10:11:03Z", actor: "human:timo", revision: 13}
- {from: "active", to: "implemented", at: "2026-10-30T15:30:15Z", actor: "human:timo", revision: 14, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Outcome

Unit names are unique per home and `update` respects a disabled source:
- `cortex-{instance}-{source}` collides when names contain `-` (instance `a` source `b-c` and
  instance `a-b` source `c`), and a source named `view` collides with the viewer unit
  (`src/schedule.rs:85-91`, `:357-358`, `:441-447`, `:698-728`; names admitted by
  `src/spec.rs:171-178`, `:192`); the second install overwrites the first and `remove` can delete
  the other instance's unit (R5-01);
- `cortex update` re-enables every source's timer with `enable --now`, including one disabled
  after repeated failures (`src/main.rs:900-920`, `:1613-1616`, `src/schedule.rs:558-559`; R5-03).

## Acceptance

1. Creating the two colliding pairs above in one home is refused, or yields two distinct unit
   names (new test in `tests/unit_homes_adversary.rs`); a source named `view` cannot overwrite the
   viewer unit.
2. After a source is disabled, `cortex update` leaves its timer inactive and `cortex sources`
   still shows it disabled (new test).

## Findings it closes

R5-01, R5-03.

## Scope

Derived 2026-10-30 by `story-scoper` from the branch head (b61a56c8; `src/`, `tests/`, `spec/` and
`generated/` are identical to a870a628). Every line is **cited** (read from the story or the tree)
or **inferred** (a reading that could be wrong).

- **Primary surface:** the cortex binary crate's systemd and CLI layer (`src/schedule.rs`, `src/main.rs`) and name admission (`src/spec.rs`) — cited
- **Files and ranges confirmed against the tree:**
  - `src/schedule.rs:85-91` `source_unit` / `view_unit` build `cortex-{instance}-{source}` and `cortex-{instance}-view` — cited
  - `src/schedule.rs:357-358`, inside `Systemd::foreign` (`:347-369`): the owner check looks up the *requesting* instance in the other home's registry, so a colliding unit of instance `a-b` reads as orphaned to instance `a`. Inside one home, `:353-354` returns `None`, so there is no check at all — cited
  - `src/schedule.rs:441-447`, inside `Systemd::write` (`:429-451`), overwrites the file without a check — cited
  - `src/schedule.rs:698-728` `Systemd::remove_instance` deletes the units by computed name — cited
  - `src/schedule.rs:558-559` `install_source` (`:502-560`) ends with `daemon-reload` and `enable --now {unit}.timer` — cited
  - `src/spec.rs:171-178` `is_name` admits `-` inside a name; `src/spec.rs:192` applies it to source names inside `check` (`:180-200+`). Nothing rejects `view` — cited
  - `src/main.rs:900-920` `install_sources` installs every `spec.sources` entry and does not read source state — cited
  - `src/main.rs:1613-1616`, in `update` (`:1489-1626`), calls `install_sources` — cited
- **Symbols the change edits:**
  - `spec::is_name` / `spec::check`: refuse a source named `view` — inferred
  - `install_sources` (`src/main.rs:900-920`): skip, or install without `enable --now`, a source whose `registry.sources["{name}/{source}"].state == m::SourceState::Disabled`. The key format is at `src/main.rs:1585-1586`; the state enum is read at `src/ports.rs:402` — inferred
  - `Systemd::install_source` (`src/schedule.rs:502-560`) probably gains an `enabled: bool`, or a sibling that writes units without enabling them, around `:558-559` — inferred
  - A new same-home collision check next to `foreign_units` (`src/main.rs:868-882`), called before units are written in `create` (`:939`), `adopt` (`:1332`) and `update` (`:1509`) — inferred
- **Also likely:**
  - `src/main.rs:1126-1127` (`create`) and `src/main.rs:1449-1450` (`adopt`) also call `install_sources`. `adopt` may carry a disabled source in and re-enable it the same way — inferred, not traced through `adopt_into` (`:1205`)
  - `website/docs/operating.md:40-43` documents the unit names; it changes only if the fix renames units instead of refusing — inferred
- **spec/domains and generated/:** no change expected if collisions are refused in the CLI with `fail(reason)`, as `foreign_units` does. `cortex.instance.InstanceName` (`spec/domains/instance.yaml:14-16`) is a plain `String` newtype with no pattern. Systemd units are not modelled in ESS. The `UpdateInstance` command (`:1539`) does not touch source state. A new typed outcome for the refusal on `CreateInstance`, `UpdateInstance` or adopt would touch `spec/domains/instance.yaml`, `spec/suite.json` and `generated/cortex-model` — inferred
- **Tests:**
  - `tests/unit_homes_adversary.rs` (518 lines, helpers `create` `:56`, `units` `:76`, `at` `:52`): new tests for the two colliding pairs and for `view` (acceptance 1) — cited
  - A new test that `update` leaves a disabled source's timer inactive and that `cortex sources` still shows it disabled (acceptance 2). The story names no file; `tests/unit_homes_adversary.rs` or `tests/unattended.rs` are the nearest homes, using the systemctl stand-in call log (`calls_since`, `tests/unit_homes.rs:62`) — inferred
  - Unit tests in `src/schedule.rs` (`:749`) or `src/spec.rs`, if `is_name` gains the `view` rule — inferred
- **Documents:** none required by the acceptance — inferred
- **Confidence:** high for the defect sites and the edited functions, which the story cites and which were read in the tree. Medium for the test file of acceptance 2 and for the spec and generated impact, which depend on refusing rather than renaming.
- **Would collide with:** any unit touching `src/schedule.rs`'s `Systemd` unit writing or naming, the `create`, `adopt` and `update` flows or `install_sources` in `src/main.rs`, or name validation in `src/spec.rs::check`; also any unit adding tests to `tests/unit_homes_adversary.rs` — cited
- **Safety fact:** refusing colliding names at admission keeps the `cortex-{instance}-{source}` and `cortex-{instance}-view` formats unchanged. Unit files already installed, the old-unit attribution in `home_of` (`src/schedule.rs:97-118`), and about 18 test files plus `website/docs/operating.md` that hard-code unit names (`git grep '"cortex-\|cortex-{'`) all stay valid. Renaming would break all of them and would need a migration of installed units. Level 2, unproven — cited
- **Safety fact (second):** the viewer collision is not only the source `view`. Instance `a` with source `b-view` writes `cortex-a-b-view.service`, which is the viewer of instance `a-b` (`src/schedule.rs:85-91`). Refusing `view` alone does not close acceptance 1's viewer clause; the same-home check must include viewer names. Level 3 by hand, unproven — inferred

## Decisions

Taken 2026-10-30 by the wave coordinator.

- Collisions are refused at admission; unit names do not change.
- The check covers source units and viewer units of every instance in the home, and runs in `create`, `adopt` and `update`.
- `adopt` keeps a disabled source disabled.
