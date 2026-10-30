---
format: aep.planning-md/3
id: story:connectors-invoke-0-60
kind: story
status: active
title: A Connectors source runs against Connectors 0.60.0 through `connectors invoke`
summary: cortex calls `connectors invoke <connector>/<instance> <op> --input-stdin`, not the removed `operations invoke`; one spawn per call besides readiness
tags:
- review-20261030
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: cited
  path: src/connectors.rs
- confidence: inferred
  path: src/sources.rs
- confidence: cited
  path: tests/adv24.rs
- confidence: cited
  path: tests/agent_tooling.rs
- confidence: inferred
  path: tests/answer_records_postgres.rs
- confidence: cited
  path: tests/common/mod.rs
- confidence: cited
  path: tests/connection_profile.rs
- confidence: cited
  path: tests/connection_profile_adv.rs
- confidence: cited
  path: tests/connections_revalidate_retry.rs
- confidence: cited
  path: tests/connections_revalidate_retry_adv.rs
- confidence: cited
  path: tests/connectors_admission_retry.rs
- confidence: cited
  path: tests/connectors_admission_retry_adv.rs
- confidence: cited
  path: tests/connectors_walks.rs
- confidence: cited
  path: tests/credential_mask.rs
- confidence: cited
  path: tests/e2e.rs
- confidence: cited
  path: tests/parent_prefix.rs
- confidence: cited
  path: tests/run_gate.rs
- confidence: cited
  path: tests/runout_adversary.rs
- confidence: inferred
  path: tests/store_backend.rs
- confidence: cited
  path: tests/structured.rs
- confidence: cited
  path: tests/structured_children.rs
- confidence: cited
  path: tests/structured_children_adv.rs
- confidence: cited
  path: tests/structured_compare.rs
- confidence: cited
  path: tests/structured_compare_adv.rs
- confidence: cited
  path: tests/structured_files.rs
revision: 31
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T10:11:00Z", actor: "human:timo", revision: 30}
- {from: "proposed", to: "active", at: "2026-10-30T10:11:01Z", actor: "human:timo", revision: 31}
---
## Outcome

A source of kind `connectors` runs `ran` against Connectors 0.60.0 and later. Connectors 0.60.0
removed `operations invoke`, `invoke --endpoint`, `describe --endpoint` and `serve`, each now a
parse refusal with exit 2; the replacement is `connectors invoke <connector>[/<instance>] <op>`
with `--input-json`, `--input-file` or `--input-stdin`, where `--schema` and `--revision` default
to the current description (Connectors `CHANGELOG.md`, 0.60.0, read with `git show v0.60.0:`).

cortex still calls the removed form at `src/connectors.rs:483-500`, and the test stand-ins accept
it (`tests/common/mod.rs:94-95` and 20 more files), so the suite is green while every Connectors
source fails against a current `connectors`. Tracked by the orchestrator as D263.

Decision taken (2026-10-30, coordinator): the spec keeps `adapter`; cortex resolves
`<connector>/<instance>` from the `connections list` entry, which carries `connector` and
`instance` beside `adapter` since 0.60.0 (Connectors `semantics.md:450-452`).

## Acceptance

1. With a stand-in `connectors` that refuses `operations invoke` with exit 2 and accepts
   `invoke <c>/<i> <op> --input-stdin`, a connectors source run ends `ran` (new test).
2. The input reaches the child on standard input; no record field or cursor is in the child's
   argv (closes R3-11; the stand-in asserts argv).
3. One `invoke` spawn per call: no `operations describe` before each call (closes P-06's
   describe half); `tests/connectors_admission_retry.rs:55` asserts the new count.
4. Every stand-in in `tests/` uses the 0.60.0 form; `grep -rn 'operations invoke' src tests`
   prints nothing.
5. A non-object `result` is reported with its JSON type, not as "no array at …" (R3-10).

## Findings it closes

R3-01, R3-10, R3-11, P-06 (describe per call), D263. Review record of 2026-10-30 (five Haiku
reviewers, one Opus performance reviewer, one Opus verifier).

## Out of scope

The `--adapter` deprecation on `connections list`, `revalidate`, `status` and `launch` (still
accepted in 0.60.0); a call deadline (`story:connectors-call-deadline`).

## Scope

Derived 2026-10-30 by `story-scoper` against `a870a628` (the worktree head `b61a56c8` changes nothing under `src`, `tests`, `spec` or `generated`). Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `src/connectors.rs`, the `Connectors` invoke path — cited
- **Files:** `src/connectors.rs:455-501` `Connectors::invoke_once`: the per-call `operations describe` is at 462-480 and the removed `operations invoke` argv is at 482-500 (the story's 483-500 is the argv array; the `self.json(` call starts at 482) — cited
- **Files:** `src/connectors.rs:408-442` `Connectors::invoke`: the result unwrap at 435-441 sends a non-object `result` back as the whole envelope (`_ => Ok(answer)`), which is where R3-10 starts. The doc comment at 402-404 still describes the envelope as `{adapter, operation, revision, result}`, and a 0.60.0 invoke result has no `adapter` member (Connectors `CHANGELOG.md` at `v0.60.0`) — cited
- **Files:** `src/connectors.rs:206-220` `Connectors::answer` (and possibly `json` at 191-193): it spawns with `.output()`, so the child gets no stdin. `--input-stdin` needs a piped stdin, either here or in a new sibling function — inferred
- **Files:** `src/connectors.rs:275-286, 317-366` `ensure_ready`, `revalidate`, `revalidate_once`: `revalidate_once` already reads the `connections list` entry (`found`, 327-336) before each call. That entry is where `<connector>/<instance>` comes from under the 2026-10-30 decision, so these signatures probably change to hand the entry back — inferred
- **Files:** `src/sources.rs:257-280` `walk_until`: the error "no array at {records:?}" at 280 is the R3-10 message. The fix may go here instead of `connectors.rs:439-440`. `sources.rs:587` reads files and is not in scope — inferred
- **Files:** `src/connectors.rs:512-684` `mod tests`: a unit test for the JSON-type message of a non-object `result` — inferred
- **Symbols:** `Connectors::invoke_once`, `Connectors::invoke`, `Connectors::answer` — cited
- **Symbols:** `Connectors::revalidate_once`, `Connectors::ensure_ready` — inferred, see above
- **Tests changed:** `tests/common/mod.rs:94-95` (describe and invoke arms of the shared stand-in). Its `connections list` entry at 89 has no `connector` or `instance` field yet — cited
- **Tests changed:** `tests/connectors_admission_retry.rs:55` (`describes.log` count 2) — cited. Line 54 (`invocations[0] == invocations[1]`, "the same input both times") compares argv lines, and the input will no longer be in argv — cited
- **Tests changed:** `tests/agent_tooling.rs:78-89` `inputs()`: it parses `--input-json` out of the argv log — cited
- **Tests changed:** `tests/e2e.rs:41-45`: it asserts `--adapter tavily`, `--operation websearch.search` and `"content":"full"` in the argv log — cited
- **Tests changed:** stand-ins with `operations describe`/`invoke` arms: `tests/adv24.rs:245-246`, `connection_profile.rs:26-27`, `connection_profile_adv.rs:23-24`, `connections_revalidate_retry.rs:56-57`, `connections_revalidate_retry_adv.rs:54-55`, `connectors_admission_retry.rs:22-23`, `connectors_admission_retry_adv.rs:21-22`, `connectors_walks.rs:28-37, 63` plus the `*"--operation …"*` arm constants at 75-136, 1038 and 1087, `credential_mask.rs:364-367`, `parent_prefix.rs:23-24`, `run_gate.rs:390-391`, `runout_adversary.rs:203`, `structured.rs:20-21, 384-386`, `structured_children.rs:26-29`, `structured_children_adv.rs:30-33`, `structured_compare.rs:29-33`, `structured_compare_adv.rs:32-36`, `structured_files.rs:679`. That is 20 files besides `e2e.rs`, matching the store's 22 scope entries — cited
- **Tests changed:** the `*"--operation <op>"*` case patterns in those stand-ins become `*"<connector>/<instance> <op>"*` or similar — inferred
- **Tests possibly changed:** `tests/answer_records_postgres.rs:493`, `tests/store_backend.rs:449`: these stand-ins list connections but never invoke. They change only if the `connector`/`instance` lookup moves into `resolve` or `ensure_ready` and refuses an entry without those fields — inferred
- **Tests added:** one new file for acceptance 1-2 (the stand-in refuses `operations invoke` with exit 2, asserts argv, reads stdin). Its name is not fixed — inferred
- **Spec / generated:** none. The decision keeps `adapter`, and `git grep` finds `operations invoke|describe`, `input-json` and `no array at` nowhere in `spec/` or `generated/`. `instance.yaml:1788` (one retry on timeout at admission) stays true — inferred
- **Documents:** `CHANGELOG.md` `## Unreleased` gets a fix entry — inferred
- **Confidence:** high for `src/connectors.rs` and the 22 test files (the story names them and the tree confirms each line). Medium for the helper signatures and where the R3-10 fix lands, which are design choices
- **Would collide with:** any unit touching `Connectors::answer`, `invoke`, `invoke_once` or the revalidation helpers in `src/connectors.rs`, which includes a per-call deadline on the spawn. Also any unit editing the shared stand-in in `tests/common/mod.rs` or the `connectors` stand-in scripts in the other 20 test files
- **Safety fact:** the `connections list` read that `revalidate_once` makes before every call (`src/connectors.rs:323-336`) already holds the entry that carries `connector` and `instance` in 0.60.0 (Connectors `semantics.md:450-452`, per the story). So resolving `<connector>/<instance>` adds no spawn and the call stays one `invoke` per call. Step 2, pointed at `file:line`, unproven

## Decisions

Taken 2026-10-30 by the wave coordinator from the scoper's open questions.

- Read the invoke result shape from Connectors `v0.60.0` (`docs/semantics.md`, `CHANGELOG.md` in the `connectors` checkout) before writing the stand-in; the stand-in answers in that shape.
- A `connections list` entry without `connector` or `instance` is refused with a message naming the entry and "Connectors 0.60.0 or newer"; no fallback to the old form.
- The `<connector>` comes from the list entry, not from the spec's `adapter`.
- A non-object `result` is refused at `Connectors::invoke` with its JSON type named (R3-10); `walk_until` keeps its message for a missing array.
- The new test file is `tests/connectors_invoke_0_60.rs`.
