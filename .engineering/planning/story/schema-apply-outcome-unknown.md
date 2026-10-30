---
format: aep.planning-md/3
id: story:schema-apply-outcome-unknown
kind: story
status: implemented
title: '`cortex schema` stops on an unknown store write and reports what was published'
summary: An `outcome unknown` from a proposal's apply stops the loop as `run.rs` does; a failure after publish is not reported as "still approved"
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
scope:
- confidence: inferred
  path: generated/cortex-model
- confidence: inferred
  path: generated/docs
- confidence: cited
  path: spec/domains/instance.yaml
- confidence: inferred
  path: spec/suite.json
- confidence: inferred
  path: src/main.rs
- confidence: cited
  path: src/schema.rs
- confidence: inferred
  path: tests/conformance.rs
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T10:11:02Z", actor: "human:timo", revision: 7}
- {from: "proposed", to: "active", at: "2026-10-30T10:11:02Z", actor: "human:timo", revision: 8}
- {from: "active", to: "implemented", at: "2026-10-30T15:30:13Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Outcome

Schema proposals handle an unknown write outcome and post-publish failures truthfully:
- an EKR "publication outcome is unknown" from `apply()` is recorded as a `Refused` proposal and
  the loop continues (`src/schema.rs:1110-1114`, `:1162-1164`), while `run.rs` stops on the same
  error (R4-02);
- after `held.publish()` succeeded, a `store.head()` or `write_run` failure returns
  `ApplyFailed` with "every approved proposal is still approved" (`src/schema.rs:1430-1431`;
  R4-10);
- a `proposals.jsonl` append failure returns `Failure::Propose` after earlier proposals were
  applied (`src/schema.rs:1124-1130`; R4-11).

## Acceptance

1. A stand-in `ekr` answering outcome-unknown on proposal 2 of 3: the run stops, proposal 3 is not
   attempted, and the outcome names the unknown write (new test).
2. A failure after publish reports that the store was published and names the revision when known.
3. The spec declares the outcome; `task drift` clean.

## Findings it closes

R4-02, R4-10, R4-11.

## Scope

Derived 2026-10-30 by `story-scoper` against `wave/20261030a` (b61a56c8). Since a870a628 that branch has changed nothing under `src/`, `spec/`, `generated/` or `tests/`, so the story's line numbers still apply. Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `src/schema.rs` (`record`, `apply`, `apply_run`, `Failure`, `Refusal`). **Cited.**
- **Citations checked:**
  - `src/schema.rs:1110-1114`: confirmed. `Mode::Apply` turns any `Err` from `apply()` into `Refused` and the loop continues. **Cited.**
  - `src/schema.rs:1162-1164`: confirmed. The `?` on `t.store.submit(...)` passes up the EKR error string, the outcome-unknown one included. That string can come from `ekr commit` inside `Store::submit` (`src/ekr.rs:541-575`). **Cited.**
  - `src/schema.rs:1430-1431`: confirmed. After `held.publish()` (1423-1428), `store.head()` and `write_run` errors become `Refusal::ApplyFailed`. Its `SchemaApplyFailed` summary says "nothing was published; every approved proposal is still approved" (`spec/domains/instance.yaml:1367-1373`). The literal text "every approved proposal is still approved" is built at 1409, 1417 and 1425, not at 1430-1431. The 1430-1431 errors carry only the bare error, so the false claim reaches the user through the outcome's declared summary. **Cited.**
  - `src/schema.rs:1124-1130`: confirmed. The append to `proposals.jsonl` returns `Failure::Propose` after earlier proposals may have been `Applied`. **Cited.**
  - `run.rs` stops on the same error at `src/run.rs:1133`, `:1967` and `:2068`, through `unknown_write` (`src/run.rs:1309-1327`), classified by `crate::ekr::outcome_unknown` (`src/ekr.rs:103-108`). **Cited.**
- **Symbols and ranges edited in `src/schema.rs`:**
  - `pub enum Failure` (58-65): needs an outcome-unknown variant, or one that carries partial progress. **Inferred.**
  - `pub fn record` (1061-1146): the classification at 1109-1115 and the append error at 1124-1130. **Cited.**
  - `fn apply` (1148-1188): its doc says an error means "nothing is recorded"; this is probably a doc-only edit. **Inferred.**
  - `pub fn apply_run` (1355-1446): the post-publish path at 1422-1431. **Cited.**
  - `pub enum Refusal` (1240-1255): a variant for "published, not recorded" if the spec gets a new outcome. **Inferred.**
  - `apply_run` 1406-1411: an outcome-unknown from `apply()` inside the held closure is reported today as `ApplyFailed`, not `StoreOutcomeUnknown`. The story does not name this, but the same fix likely belongs here. **Inferred.**
- **Also, `src/main.rs`:**
  - `fn propose_schema` (2034-2172): the `schema::Failure` match at 2077-2103 and the exhaustive `ProposeSchemaChangesOutcome` match at about 2132-2170. **Inferred** (needed only if `Failure` or the spec outcomes change, which acceptance 3 implies).
  - `fn apply_schema_run` (2273-2347): outcome mapping at 2292-2302, `Done` match at 2306-2345. **Inferred.**
  - `fn refusal_reason` (2349-2358). **Inferred.**
- **Spec:** `spec/domains/instance.yaml`. **Cited** (the story's scope and acceptance 3).
  - `cortex.instance.ProposeSchemaChanges` (2440-~2560): add an outcome such as `store-outcome-unknown`, and fix the `propose-failed` summary "Nothing was applied and no `proposals.jsonl` was written" (2479-2486), which R4-11 contradicts. **Inferred.**
  - `cortex.instance.ApplySchemaProposals` (2679-~2760): the `apply-failed` summary (2726-2732), or a new post-publish outcome. **Inferred.**
  - Error types `cortex.instance.StoreOutcomeUnknown` (1215-1222; its summary names only run verbs) and `cortex.instance.SchemaApplyFailed` (1367-1373). **Inferred.**
- **Generated (regenerated, `task drift`):** **inferred** (each file below already carries `ProposeSchemaChanges`, so an outcome change regenerates it):
  - `generated/cortex-model/src/instance.rs`, `behaviour.rs`, `ports/cortex.rs`, `actor.rs`, `system.rs`, `plan.json`, `PLAN.md`
  - `generated/docs/docs/domains/cortex-instance.md`
  - `spec/suite.json` (new `.../outcome/store-outcome-unknown` scenarios, as at `spec/suite.json:15990` for `ApplySchemaProposals`)
  - Possibly `generated/schema/schema/commands/cortex.instance.ProposeSchemaChanges.schema.json`
- **Tests changed:** `tests/conformance.rs`. **Inferred** (its exhaustive matches on `ProposeSchemaChangesOutcome` at 919-948 and `ApplySchemaProposalsOutcome` at 1003-1040 break when a variant is added; the injected external at about 706 is the pattern to follow).
- **Tests added:** a new file such as `tests/schema_outcome_unknown.rs`. **Inferred** (acceptance 1 says "new test"; the file name is a guess).
  - It needs a stand-in `ekr` that answers outcome-unknown on `ekr commit` for proposal 2. The existing stand-in `unknown_ekr` (`tests/store_outcome_unknown.rs:38-68`) intercepts only `stage begin`, `stage publish` and `apply-extraction`, and is private to that file. The proposer and answer helpers (`tests/schema.rs:45-110`) are private too. **Cited** (that the helpers are private) / **inferred** (that they must be copied or moved to `tests/common/mod.rs`).
  - Acceptance 2 probably adds a case next to `tests/schema_review.rs` (approve, then apply). **Inferred.**
- **Documents:** none beyond the generated docs. **Inferred.**
- **Would collide with:**
  - any unit touching `src/schema.rs` around proposal record or apply;
  - `src/main.rs` around `propose_schema` and `apply_schema_run`;
  - the `ProposeSchemaChanges` and `ApplySchemaProposals` blocks of `spec/domains/instance.yaml`;
  - any unit that changes the spec, because all of them regenerate `generated/cortex-model/*` and `spec/suite.json`;
  - `tests/conformance.rs`.

  **Inferred.**
- **Safety fact:** the outcome-unknown error reaches `record` as a plain `String` through the `?` at `src/schema.rs:1164`, and `crate::ekr::outcome_unknown` (`src/ekr.rs:107`) identifies it by substring with no other state. So stopping the loop only needs a check at `src/schema.rs:1111`. The wire-visible effect is a new `outcome` value in the `cortex schema` JSON line, which has no caller beyond `src/main.rs` and `tests/conformance.rs` (`git grep ProposeSchemaChangesOutcome`). Level 2, unproven. **Inferred.**
- **Confidence:** high for the `src/schema.rs` sites, because every citation was confirmed in the tree. Medium for the spec, `generated/` and `main.rs` lines, because the shape of the spec change (new outcome or changed summaries) is not decided in the story.
- **Not established:**
  - The source of findings R4-02, R4-10 and R4-11: no review report in the tree carries them, only `.engineering/drafts/stories.txt:334-350`. **Cited.**
  - Whether acceptance 3 means a new `ProposeSchemaChanges` outcome, a new `ApplySchemaProposals` outcome, or only changed summaries. **Inferred.**
  - Whether the exit code for a schema outcome-unknown should be 4, as `EXIT_STORE_OUTCOME_UNKNOWN` is for runs (`src/main.rs:2389`). I did not trace how `print("schema", ...)` picks an exit code. **Inferred.**
  - Which "revision when known" acceptance 2 wants after a `store.head()` failure: the stage head passed to `publish`, or none. **Inferred.**
  - Whether `generated/schema/.../ProposeSchemaChanges.schema.json` changes: it may hold only the input, which does not change. **Inferred.**

## Decisions

Taken 2026-10-30 by the wave coordinator.

- New outcome `store-outcome-unknown` on `cortex.instance.ProposeSchemaChanges` and `ApplySchemaProposals`, exit 4 as runs use.
- `propose-failed` and `apply-failed` summaries are corrected so they do not claim nothing was applied.
- The post-publish `head`/`write_run` failure reports the stage head passed to `publish` as the revision.
