---
format: aep.planning-md/3
id: story:replacement-applied-as-assertion
kind: story
status: implemented
title: A replacing fact with nothing active to replace is stored as a plain assertion
summary: A fact refused as `replacement-without-active-assertion` is re-applied without `replaces`, so the document's value is not lost
tags:
- review-20261030
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: src/extract.rs
- confidence: inferred
  path: src/main.rs
- confidence: cited
  path: src/run.rs
- confidence: cited
  path: tests/supersession.rs
- confidence: inferred
  path: website/docs/commands.md
- confidence: cited
  path: website/docs/operating.md
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T10:11:01Z", actor: "human:timo", revision: 11}
- {from: "proposed", to: "active", at: "2026-10-30T10:11:01Z", actor: "human:timo", revision: 12}
- {from: "active", to: "implemented", at: "2026-10-30T15:30:12Z", actor: "human:timo", revision: 13, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Outcome

A fact marked `replaces: true` for which EKR has no active assertion is stored as a plain
assertion. Today EKR refuses it with `replacement-without-active-assertion`, the document is
marked seen all the same (`src/run.rs:1148-1159`), and the value is not stored, as documented
(`website/docs/operating.md:184-189`); downstream 5 merge requests lost values this way, one with
no properties in the store (cb3 report, C-03).

Decision taken (2026-10-30, coordinator): cortex re-applies the refused facts without `replaces`
in the same batch. The alternative, EKR treating such a replacement as a plain assertion, is
asked of EKR; if EKR ships it, cortex's re-apply becomes dead code and is removed.

## Acceptance

1. A batch with a `replaces: true` fact and no active assertion: the value is in the store after
   the run, and the run report counts one re-applied fact (new test).
2. `tests/supersession.rs:293`, which asserts today's loss, asserts the new behaviour.
3. `website/docs/operating.md:184-189` describes the re-apply.

## Findings it closes

C-03.

## Scope

Derived 2026-10-30 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong). Line numbers are from the tree at `b61a56c8`, whose
`src`, `tests`, `website`, `spec` and `generated` are identical to `a870a628`.

- **Primary surface:** `src/run.rs`, `process_in` (567-1212), the batch apply step — cited
- **Files:** `src/run.rs:1148-1159` confirmed: comment 1148-1150, `extract::cited_by_rejected` 1151, the loop that pushes `Rejected` and calls `seen.record` 1152-1159 — cited
- **Files:** the re-apply goes between `store.apply(&path)` (1131-1139) / `before.committed()` (1140) and the `cited_by_rejected` loop (1151-1159), and adds a second `store.apply` of the refused facts with `replaces` removed — inferred
- **Files:** `src/run.rs` `Report` (struct 56-120, `Default` 165-196) gains a re-applied counter, and `log_line` (2189, field list 2190-2200) writes it — inferred
- **Files:** `tests/supersession.rs:293` confirmed: `#[test]` at 293, `a_replacement_with_nothing_to_replace_is_listed_and_the_run_goes_on` at 294-316; its doc comment (290-292) and the module doc (6-9) state today's loss — cited
- **Files:** `website/docs/operating.md:184-189` confirmed, inside step 5 (176-189); the loss is stated at 185-186 — cited
- **Symbols:** `process_in`, `extract::cited_by_rejected`, `extract::refusal`, `crate::ekr::applied`, `Report::parts_rejected` / `Report::rejected` — cited
- **Also likely:** `src/extract.rs`, a new helper beside `cited_by_rejected` (1096-1121) that returns the facts refused as `replacement-without-active-assertion` with `replaces` removed, and a unit test in `mod tests` (1140-) — inferred
- **Also likely:** `src/main.rs:2476-2485`, the `ran` detail JSON that carries `facts_refused` / `parts_rejected` from `report`, gains the new counter — inferred
- **Also likely:** `website/docs/commands.md:279-280`, the `ran` detail field list — inferred
- **Also likely:** `CHANGELOG.md` `## Unreleased` entry — inferred
- **Not touched:** `src/run.rs` `apply_records` (1908-) for structured sources: `src/structured.rs` never sets `replaces`, so that path cannot hit this refusal — inferred
- **Spec / generated:** none required. `parts_rejected` and `facts_refused` are not fields of `cortex.instance.SourceRan` (`spec/domains/instance.yaml:2818-2834`); they come from `Report`, not the spec. Only adding the counter to `SourceRan`, or to `REPORT_MEASURES` (`src/gate.rs:16-25`) and its spec comment (`spec/domains/instance.yaml:610`), would widen this, and then `task generate` rewrites `generated/` — inferred
- **Tests:** change `tests/supersession.rs:290-316`: owner `Alice` is stored and active, and the new counter is 1. Whether `parts_rejected` / `rejected` still count the first refusal is a design call. Add a new test for acceptance 1, likely in the same file using `world` (46), `run_with` (82), `owners` (137), `refusals` (181) — inferred
- **Documents:** `website/docs/operating.md:184-189` — cited
- **Confidence:** high — the story names the defect site, the test and the doc lines, and all three hold at line level in the tree
- **Would collide with:** any unit touching `src/run.rs` `process_in`'s apply-and-record step or `Report`, `src/extract.rs` around `cited_by_rejected`, `tests/supersession.rs`, step 5 of `website/docs/operating.md`, or the `ran` detail in `src/main.rs`
- **Safety fact:** a second `apply-extraction` in the same batch stays inside the run's one undo snapshot. `BeforeApply::take` copies only while `Stage::Due`, and `publish` names the copy only the first time (`src/snapshot.rs:333-350`, via `Undo::take` / `Undo::committed`, `src/run.rs:1236-1249`). On a `postgres` stage the same is assumed and not read — step 2, unproven

## Decisions

Taken 2026-10-30 by the wave coordinator.

- A fact refused as `replacement-without-active-assertion` and stored by the re-apply is not counted in `parts_rejected`/`rejected`.
- New counter `facts_reapplied` on `Report`, in the log line and the `ran` detail; not in `SourceRan` or `REPORT_MEASURES`.
- The re-apply document repeats the batch's `entities` and `evidence` that the facts cite.
