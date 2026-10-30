---
format: aep.planning-md/3
id: story:runs-retention
kind: story
status: active
title: Run directories are pruned to a retention
summary: '`runs/<time>-<source>/` directories older than a configured retention are removed at the end of a run'
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
scope:
- confidence: cited
  path: generated/cortex-model
- confidence: cited
  path: generated/docs
- confidence: cited
  path: generated/schema
- confidence: cited
  path: generated/spec-types
- confidence: cited
  path: spec/domains/instance.yaml
- confidence: cited
  path: src/model_map.rs
- confidence: cited
  path: src/run.rs
- confidence: inferred
  path: src/spec.rs
- confidence: cited
  path: tests/conformance.rs
- confidence: inferred
  path: tests/runs_retention.rs
- confidence: cited
  path: website/docs/operating.md
- confidence: cited
  path: website/docs/reference/ess/cortex-instance.md
- confidence: inferred
  path: website/docs/spec-file.md
- confidence: cited
  path: website/static/schemas/instance-spec.schema.json
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T10:11:05Z", actor: "human:timo", revision: 12}
- {from: "proposed", to: "active", at: "2026-10-30T10:11:05Z", actor: "human:timo", revision: 13}
---
## Outcome

Each batch writes a directory under `runs/` and nothing prunes it (`src/run.rs:792`,
`:1118-1128`; `website/docs/operating.md:26`) (R2-06).

## Acceptance

With `runs.keep_days: 1`, a run removes run directories older than one day and keeps the current
one (new test); without the setting nothing is removed; the spec declares the setting.

## Findings it closes

R2-06.

## Scope

Derived 2026-10-30 by `story-scoper` against HEAD `b61a56c8` (`src`, `spec`, `website`, `tests`,
`generated` identical to `a870a628`). Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `src/run.rs`, the end of a run. Pruning goes in `process` (`:531-559`,
  after `settle(...)` at `:558`), or in a new helper it calls. That one call covers source runs
  (`run`, `:415`) and seed runs (`seed_documents`, `:469`). — inferred (placement); the run
  directory itself is cited
- **Files (cited by the story, confirmed):** `src/run.rs:792` (`run_dir = layout.runs().join("{started}-{label}")`);
  `src/run.rs:1120-1129` (batch writes: `extraction.yaml`, `prompt.txt`, `model.json`; the story's
  `:1118-1128` is off by one at the end); `website/docs/operating.md:26` (the `runs/<time>-<source>/batch-<n>/` row) — cited
- **Spec type:** `spec/domains/instance.yaml`. Add a new struct beside `cortex.instance.SnapshotPolicy`
  (`:628-631`) with one field, `keep_days: Integer`, and add `runs: Optional<that type>` to
  `cortex.instance.InstanceSpec` (`:707-722`). — cited (the story's "the spec declares the
  setting" and `runs.keep_days`). The type name is inferred.
- **Mapping:** `src/model_map.rs` `instance_spec` (`:61`, struct literal `:62`). It needs one arm
  like `snapshots` at `:134-139` (`int(&p.keep_days, "runs.keep_days")`). — cited (the literal
  is exhaustive, so the build forces the edit)
- **Test literal:** `tests/conformance.rs` `placeholder_spec` (`:49-82`) builds `m::InstanceSpec`
  by hand and needs `runs: None` — cited (exhaustive literal)
- **Generated (`task generate`, `Taskfile.yml:8-18`):** `generated/cortex-model/src/instance.rs`
  (`InstanceSpec` `:324`, plus the new struct), `generated/cortex-model/plan.json`,
  `generated/cortex-model/PLAN.md`, `generated/spec-types/types.rs` (`CortexInstanceInstanceSpec`
  `:633`), `generated/spec-types/source.schema.json`, `generated/spec-types/types-report.json`,
  `generated/schema/schema/types/cortex.instance.InstanceSpec.schema.json`, a new
  `generated/schema/schema/types/cortex.instance.<NewType>.schema.json`,
  `generated/schema/schema/commands/cortex.instance.{Create,Update,Adopt}Instance.schema.json`,
  `generated/docs/docs/domains/cortex-instance.md` — cited (every file that carries
  `SnapshotPolicy` today)
- **Generated docs (`task docs-generate`, `Taskfile.yml:57-60`):** `website/docs/reference/ess/cortex-instance.md`,
  `website/static/schemas/instance-spec.schema.json` — cited (both carry `SnapshotPolicy` today)
- **`spec/suite.json`:** probably unchanged, since `SnapshotPolicy` does not appear in it. It
  changes only if a conformance scenario is added. — inferred
- **Documents:** `website/docs/operating.md:26` (say the directories are pruned after `runs.keep_days`)
  — cited. Also `website/docs/spec-file.md`: a top-level table row beside `:26` and a `## runs`
  section after `## snapshots` (`:811-823`). — inferred (every other optional section has both)
- **Also likely:** `src/spec.rs` `check` (`:180-375`), to refuse `runs.keep_days` below 1, as
  `model.timeout_s` is refused at `:316-318`. The story does not say whether 0 means "keep
  nothing" or is refused. — inferred
- **Tests added:** a new `tests/runs_retention.rs`, one test file per story as the tree does. Case 1:
  `runs: {keep_days: 1}` with an old `<ms>-<source>` directory planted removes the old one and keeps
  the current one. Case 2: no `runs` section removes nothing. Model it on `tests/snapshots.rs:519`
  (`snapshots: {keep: 1}`). — inferred (the file name; the story says only "new test")
- **Tests that must keep passing unchanged:** they count `runs/` entries with no setting, which is
  the "nothing removed" default: `tests/snapshots.rs:461`, `tests/adv24.rs:41`,
  `tests/structured.rs:329,750,789`, `tests/connectors_walks.rs:319`, `tests/e2e.rs:238`. Also
  `tests/spec_compat.rs`: an absent optional field leaves its frozen-spec fixture unchanged. — cited (paths); the outcome is inferred
- **Confidence:** high for `src/run.rs`, `spec/domains/instance.yaml`, `src/model_map.rs`, the
  generated set and `operating.md`. They are cited by the story, or a struct literal or the
  generators force them. Medium for the prune point inside `run.rs` and for `src/spec.rs`.
- **Would collide with:** any unit that changes `cortex.instance.InstanceSpec` or adds a spec
  type, because the whole `generated/` set, `src/model_map.rs` `instance_spec` and
  `tests/conformance.rs` `placeholder_spec` all change. Also any unit editing `process`/`settle`
  in `src/run.rs`, or `website/docs/spec-file.md` / `operating.md` "Where things live".
- **Safety fact:** at runtime, nothing reads `runs/` except the run that writes it. `git grep`
  finds `layout.runs()` only at `src/run.rs:792`, defined at `src/instance.rs:78-80`. Undo and
  restore read `snapshots/` and `snapshots-state/`, and `schema.rs`'s `run_dir` (`:1192`) is
  `schema/<stamp>`, not `runs/`. Each directory name starts with the run's start in Unix ms
  (`started = now_ms()`, `src/run.rs:580`), so its age can be read from the name, the way
  `snapshot::taken_at` (`src/snapshot.rs:83`) does it. The current run is less than a day old.
  — step 2, unproven. Two things are not proven. First, that `cortex.lock` keeps another source's
  run of the same instance from writing a `runs/` directory while the prune runs. Second, what
  happens to a directory whose name does not parse; `snapshot::cortex_named` (`:68`) shows the
  safe rule: never remove a name cortex did not give.

## Decisions

Taken 2026-10-30 by the wave coordinator.

- Spec: `runs: Optional<RunsPolicy>` on `InstanceSpec`, `RunsPolicy { keep_days: Integer }`; `keep_days` below 1 is refused in `spec::check`.
- Pruning runs after `settle` in `process`, removes only directories whose name parses as `<ms>-<source>` and is older than `keep_days`, and never the current run.
- New test file `tests/runs_retention.rs`.
