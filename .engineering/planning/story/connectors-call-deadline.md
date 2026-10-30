---
format: aep.planning-md/3
id: story:connectors-call-deadline
kind: story
status: implemented
title: A `connectors` call that hangs ends within a deadline, and a structured file has a size cap
summary: Every `connectors` child is waited for with a deadline and killed past it; structured and markdown files over a cap are skipped with a note
tags:
- review-20261030
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:connectors-invoke-0-60
- informed_by: story:fetch-stop-point
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: spec/domains/instance.yaml
- confidence: cited
  path: src/connectors.rs
- confidence: cited
  path: src/sources.rs
- confidence: inferred
  path: tests/connectors_call_deadline.rs
- confidence: inferred
  path: tests/file_records.rs
- confidence: cited
  path: tests/structured_files.rs
- confidence: inferred
  path: website/docs/operating.md
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T10:11:04Z", actor: "human:timo", revision: 12}
- {from: "proposed", to: "active", at: "2026-10-30T10:11:04Z", actor: "human:timo", revision: 13}
- {from: "active", to: "implemented", at: "2026-10-30T15:30:09Z", actor: "human:timo", revision: 14, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
## Outcome

`Command::output()` waits for `connectors` without a deadline and buffers all its output
(`src/connectors.rs:209-213`), so a hung call blocks the run until an external SIGTERM; structured
and markdown files are read whole with no cap (`src/sources.rs:579-580`, `:1221`) (R3-08).
Unreadable or non-UTF-8 files are skipped without a `skipped` entry (`src/sources.rs:1221-1223`,
`:1244-1246`; R3-07). Extends `story:fetch-stop-point`, which covers SIGTERM only.

## Acceptance

1. A stand-in `connectors` that sleeps past the deadline: the call fails as `timeout` within the
   deadline plus one second and the child is gone (`kill -0` in the test).
2. A structured file over the cap, and an unreadable one, are each listed in `skipped` with the
   reason.

## Findings it closes

R3-07, R3-08.

## Decisions

Taken 2026-10-30 by the wave coordinator.

- The deadline is the env setting `CORTEX_CONNECTORS_TIMEOUT_S`, default 120, read through `setting()` like `CORTEX_REVALIDATE_*`; past it the child is killed and reaped and the call fails as `timeout` (a message on the existing failure, no new outcome).
- The size cap is a constant of 16 MiB per file.
- `files` sources: an unreadable, over-cap or non-UTF-8 file is named in `skipped`.
- `structured` files: an unreadable or over-cap file keeps failing the fetch, because skipping it would let `dropped: Supersede` retract its records; the story records this.
- Builds on the `answer`/`invoke_once` that `story:connectors-invoke-0-60` leaves.

## Scope

Derived 2026-10-30 by `story-scoper`, against main `a870a628`. Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `src/connectors.rs`, `Connectors::answer` — cited (story Outcome; read at `src/connectors.rs:205-220`)
- **Line check, connectors:** the story cites `src/connectors.rs:209-213`. The `Command::new(&self.bin)…output()` chain is actually at `:207-211` and the stdout/stderr choice is at `:212-217`. Use `:206-220` (`fn answer`) — cited
- **Line check, structured files:** `src/sources.rs:579-580` holds, inside `fetch_structured_files` (`:554-602`). But an unreadable structured file does not go unlisted, as the story implies: today it fails the whole fetch (`map_err(FetchError::Failed)`). This is tested by `tests/structured_files.rs:287-315` `a_registry_file_that_is_not_json_fails_the_run_and_ends_nothing` — cited
- **Line check, files source:** `src/sources.rs:1221` holds. `let Ok(bytes) = std::fs::read(..) else { continue; }` at `:1221-1223` in `fetch_files` (`:1195-1241`) skips without a `skipped` entry — cited
- **Line check, non-UTF-8:** `src/sources.rs:1244-1246` holds. `whole_file` drops non-UTF-8 at `:1245` with no `skipped` parameter. It is reached from `fetch_files` `:1238` and from `RecordFormat::WholeFile` at `:1345-1349`. `MarkdownSections` (`:1350-1354`) and `json_lines` (`:1479-1500`) already note non-UTF-8 — cited
- **Symbols, connectors.rs:** `Connectors::answer` (`:206-220`), which every `connectors` call goes through: `json` `:191-193`, `json_or_unavailable` `:196-203`, `revalidate_once` `:342`. Possibly also a new `InvokeError` variant or message for `timeout` (`:109-132`). If it is a variant, `impl From<InvokeError> for FetchError` at `src/sources.rs:50-57` changes too — cited for the sites, inferred that a variant is added
- **Deadline mechanism:** spawn, then a `try_wait` loop, then kill. The model is `src/extract.rs:603` `wait_or_terminate`, which this story reads but does not have to edit. The deadline would be an env setting read through `setting()` at `src/connectors.rs:176-181`, following the `CORTEX_REVALIDATE_*` precedent — inferred
- **Symbols, sources.rs:** `fetch_structured_files` (`:554-602`) changes from returning `Vec<Document>` to also returning `skipped`, so the `fetch` match arm at `:489-494` changes with it. Also `fetch_files` (`:1195-1241`), `whole_file` (`:1244-1258`) and a new size-cap constant or setting — cited for the sites, inferred for the signature change
- **Spec:** `spec/domains/instance.yaml`. Likely an `UNMAPPED` comment beside `fetch-failed` (`:1787-1797`) naming the deadline setting, as `:1875-1882` does for `CORTEX_REVALIDATE_ATTEMPTS`. No type change if the cap is a constant or env setting — inferred
- **Generated:** none if the spec change is comment-only. `CORTEX_REVALIDATE_ATTEMPTS` appears nowhere under `generated/`. If the cap instead becomes a field of `FilesSource` (`:200-205`), `StructuredFiles` (`:329`) or `FetchPolicy` (`:89-95`), then `generated/cortex-model`, `generated/schema`, `generated/docs` and `generated/spec-types` all regenerate — inferred
- **Tests added:** `tests/connectors_call_deadline.rs`, for acceptance 1: a sleeping stand-in, the run ends within the deadline plus 1 s, then `kill -0`. Following the repo's one-file-per-story pattern there may also be a `_adv` file — inferred
- **Tests changed:** `tests/structured_files.rs`, where the over-cap and unreadable structured file cases now go to `skipped`. The not-JSON test at `:287-315` stays a fetch failure unless the story widens to that — cited for the file, inferred for the edit. Also `tests/file_records.rs` for the files source cap and the unreadable file — inferred
- **Documents:** `website/docs/operating.md` near `:305`, where the env settings are documented, and `CHANGELOG.md` — inferred
- **Not in scope:** `src/ekr.rs:230` and `src/ask/records/postgres.rs:452` also spawn `connectors`, but as long-lived `connections launch` children. The story's "every `connectors` child" reads as the `Connectors` calls only — inferred
- **Confidence:** medium. The story names the defect sites and they check out. The deadline's configuration, the shape of the `timeout` error and where the cap lives are not stated
- **Would collide with:** `src/connectors.rs` `Connectors::answer`/`invoke_once`, which `story:connectors-invoke-0-60` rewrites first (piped stdin) and `story:fetch-stop-point` also targets (SIGTERM wait on the fetch child). Also the files and structured-files fetch in `src/sources.rs` (`fetch`, `fetch_structured_files`, `fetch_files`, `whole_file`) and `tests/structured_files.rs`
- **Safety fact:** with `dropped: Supersede`, a structured file that is skipped instead of failing the fetch drops its records from `docs`. Then `listed` (`src/run.rs:735-737`) no longer holds them, and `end_dropped` (`src/run.rs:852`, `:2026`) retracts every value that file held. So a skipped structured file must either keep the fetch failing or keep its keys from being ended. Proof level 3 (traced through the code by reading, not run), unproven
