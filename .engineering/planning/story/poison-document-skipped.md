---
format: aep.planning-md/3
id: story:poison-document-skipped
kind: story
status: implemented
title: A document that fails its batch the same way on consecutive runs no longer blocks later batches
summary: A per-document failure count in the source state; after N consecutive failures the document is split out, named in `skipped`, and later batches run
tags:
- review-20261030
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- informed_by: story:extraction-cache-by-content
scope:
- confidence: inferred
  path: generated/cortex-model
- confidence: inferred
  path: generated/docs
- confidence: inferred
  path: generated/schema
- confidence: inferred
  path: generated/spec-types
- confidence: cited
  path: spec/domains/instance.yaml
- confidence: inferred
  path: spec/suite.json
- confidence: inferred
  path: src/model_map.rs
- confidence: inferred
  path: src/ports.rs
- confidence: cited
  path: src/run.rs
- confidence: inferred
  path: src/sources.rs
- confidence: cited
  path: src/state.rs
- confidence: inferred
  path: tests/credential_mask.rs
- confidence: inferred
  path: tests/poison_document.rs
- confidence: inferred
  path: website/data/ess
- confidence: cited
  path: website/docs/operating.md
- confidence: inferred
  path: website/docs/reference/ess
- confidence: inferred
  path: website/docs/spec-file.md
- confidence: inferred
  path: website/static/schemas
revision: 25
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T10:11:03Z", actor: "human:timo", revision: 22}
- {from: "proposed", to: "active", at: "2026-10-30T10:11:03Z", actor: "human:timo", revision: 23}
- {from: "active", to: "implemented", at: "2026-10-30T15:30:11Z", actor: "human:timo", revision: 25, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Outcome

A document whose batch fails deterministically (a model refusal, a validation the model cannot
satisfy) stops blocking the source. Today a failing batch stops the run at that batch
(`src/run.rs:1098-1104`), nothing counts failures per document (only `CHILD_FAILURE_LIMIT` for
child calls, `src/sources.rs:431`), and the next run fails at the same batch: downstream, 35
documents went to the model four times in one day (cb3 report, C-05).

## Acceptance

1. With a stand-in model that always fails on document X, run N+1 (N the configured limit,
   default 3) applies every other document and lists X in the run report's `skipped` with the
   reason and its failure count (new test).
2. X is retried after its content hash changes.
3. The spec declares the per-document count and the skip outcome; `task drift` is clean after
   `task generate`.

## Findings it closes

C-05 (the poison-batch part). The lost model work of an abandoned gated run is
`story:extraction-cache-by-content`.

## Scope

Derived 2026-10-30 by `story-scoper` against branch `wave/20261030a` (b61a56c8). `src`, `spec`, `generated`, `website` and `tests` are identical to main a870a628, so the line numbers hold for both. Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `src/run.rs` `process_in` (567-1215), the extraction batch loop (940-1173) — cited
- **Story citations checked:**
  - `src/run.rs:1098-1104` is off by one. The two failure arms are at **1099-1105**: `documents_applied == 0` returns `Failure::Extract` (1099-1101), otherwise `report.stopped` is set and the loop breaks (1102-1105). The claim itself holds — cited
  - `src/sources.rs:431` `CHILD_FAILURE_LIMIT: u32 = 3` is confirmed, with its doc comment at 429-430 — cited
  - No per-document failure count exists anywhere. The only `BTreeMap<String, u32>` counter is `child_failures`, keyed by parent: `src/state.rs:53-56`, `src/run.rs:2165`/`2171` — cited
- **Files and symbols edited:**
  - `src/run.rs` `process_in`, the failure arms at 1099-1105 (count the failing batch's documents, split out or skip documents at the limit) — cited
  - `src/run.rs` `process_in`, the batching step `batches(issued, size)` at 912, where a document at the limit would be left out and named in `skipped` — inferred
  - `src/run.rs` `MAX_SPLITS` (238) and the split path at 1062-1068, if "split out" reuses the halving that `unfit` uses (1055-1081). That path already marks a lone unfit document as seen and skips it: the precedent — inferred
  - `src/run.rs` `finish` (2158-2182) persists `child_failures` today and would persist the new count. It returns early when `window` is `None` (2166-2168) — cited
  - `src/run.rs` `run` (415-451): its error path at 438-449 saves only `pending_since`, so a run whose first batch fails would lose the count — cited
  - `src/run.rs` `Report.skipped` (82-85): its doc comment names only parents and over-bound records and needs the new case. `log_line` 2229-2232 and `src/main.rs:2519-2521` already print `skipped` and need no change — cited
  - `src/state.rs` `SeenState` (33-63): a new field next to `child_failures` (53-56), with `#[serde(default, skip_serializing_if)]` so `cortex.seen/1` stays readable — inferred
  - `src/state.rs` `SeenState::load` (90-114): the credential-key `retain` at 96-100 and the empty default at 104-112 must cover the new map — cited
  - `src/state.rs` `select`/`wants` (165-194), only if skipping is done at selection; retry on a changed hash needs the count keyed by key and `doc_hash` (70-72) — inferred
  - `src/sources.rs:429-431`: a sibling default limit constant, if the limit is not read from the spec — inferred
- **Spec (`spec/domains/instance.yaml`):**
  - `RunSource` UNMAPPED block on the per-source state file (1677-1691): the per-document count and the skip go here as an UNMAPPED rule, because the block says ESS has no per-element record effect (beyond10x/ess#459) — inferred
  - `extraction-failed` outcome (1804-1823) and `ran` outcome (2008-2040): summaries — inferred
  - The configured limit "N" needs a new field. Candidates are `cortex.instance.FetchPolicy` (88-94) or the model settings (455-465). Not established which — inferred
  - Possibly a new count on `cortex.instance.SourceRan` (2818-2834) — inferred
  - The existing child-call limit is not declared in the spec at all. It is documented only in `website/docs/spec-file.md:369-371` — cited
- **Generated (via `task generate`, never by hand):**
  - `generated/cortex-model/src/instance.rs` (the `RunSource` contract text at 3579, plus `SourceRan` if changed) — inferred
  - `generated/spec-types/{types.rs,source.schema.json}` and `generated/schema/`, only if a spec-file field is added — inferred
  - `generated/docs/` and `spec/suite.json` — inferred
  - `website/docs/reference/ess/cortex-instance.md`, `website/data/ess/cortex-instance.domain-graph.json` and `website/static/schemas/` (`task docs-generate`) — inferred
- **Rust follow-on to a spec field:**
  - `src/model_map.rs:505-512` (`FetchPolicy` mapping) or `:105` (model fields) — inferred
  - `src/ports.rs:351` `source_ran`, if `SourceRan` gains a field — inferred
- **Tests:**
  - New integration test file, for example `tests/poison_document.rs`, with a stand-in `claude` that fails when a prompt holds document X. The pattern is the `WRAPPER` plus `common::World` in `tests/model_error.rs:12-24` and `tests/batch_fits_model_window.rs:20-40` — inferred
  - Unit tests in `src/state.rs` `mod tests` (278-611): load and default of the new field — inferred
  - `tests/credential_mask.rs:439-489` covers `child_failures` keys in the state file and should cover the new key-keyed map — inferred
  - `tests/conformance.rs` changes only through the regenerated `spec/suite.json` — inferred
- **Documents:**
  - `website/docs/operating.md:247-250` ("when the first batch fails, nothing was applied and the run fails") needs the skip rule — cited
  - `website/docs/spec-file.md:365-371` (the child-call skip, the parallel rule) and wherever the new limit field is documented — inferred
- **Confidence:** high for the primary surface (`run.rs` batch loop and `state.rs` `SeenState`): the story names them and each line was read. Medium for the spec shape and the field placement, which the story leaves open.
- **Would collide with:**
  - any unit touching `src/run.rs` `process_in`/`settle`/`Staged` (for example `story:extraction-cache-by-content`, which cites `run.rs:1268-1272`, `:1585-1606`)
  - any unit touching `src/state.rs` `SeenState` or the `cortex.seen/1` file
  - any unit that changes `spec/domains/instance.yaml` `RunSource` and so regenerates `generated/` and `spec/suite.json`
  - `website/docs/operating.md`
- **Safety fact:** the count only works if it survives the failing run, and today it would not:
  - A first-batch failure returns at `src/run.rs:1099-1101` before `finish` (1202-1205), and `run`'s error path (438-449) writes only `pending_since`.
  - A staged run that applied nothing keeps `state/` (`discard`, 1611-1615).
  - A gate-refused, killed or terminated gated run rewinds `state/` (`abandon` 1587-1589, `rewind` 1626-1636, `recover` 1506, sqlite undo via `snapshot::rewind_state` at 1741). There the count resets, and a poison document under a gate would never reach N.
  - Step 3 (walked the failure and rewind paths), unproven — cited

## Decisions

Taken 2026-10-30 by the wave coordinator.

- A failing batch of more than one document is halved (the `unfit` split path); a document is counted only when it fails alone.
- The limit is a constant of 3 beside `CHILD_FAILURE_LIMIT`, no spec field.
- The count lives outside `state/` so a rewind does not reset it: `failures/<source>.json` under the instance directory, written on the failure path too. Check the rewind code for the exact directory set first.
- A changed `doc_hash` resets the document's count.
- The skip is an UNMAPPED rule in `RunSource`; no new `SourceRan` field.
- New test file `tests/poison_document.rs`.
- Correction (2026-10-30, wave coordinator, D20): a failed batch of several documents is asked whole once more in the next run; only when it fails again are its documents asked alone. Only lone failures count toward the limit (`review-result:adversary-w30a-fail-pass-2`, finding 1).
