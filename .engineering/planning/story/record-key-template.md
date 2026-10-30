---
format: aep.planning-md/3
id: story:record-key-template
kind: story
status: implemented
title: Two records with equal ids in two inputs are both applied
summary: A source's record key can be a template over record fields; a null id is named in `skipped`, not keyed `null`
tags:
- review-20261030
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:connectors-invoke-0-60
- depends_on: story:update-refuses-rekey
scope:
- confidence: inferred
  path: CHANGELOG.md
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
  path: src/main.rs
- confidence: inferred
  path: src/model_map.rs
- confidence: cited
  path: src/sources.rs
- confidence: inferred
  path: tests/conformance.rs
- confidence: cited
  path: tests/connectors_walks.rs
- confidence: cited
  path: website/docs/spec-file.md
revision: 19
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T10:11:04Z", actor: "human:timo", revision: 16}
- {from: "proposed", to: "active", at: "2026-10-30T10:11:04Z", actor: "human:timo", revision: 17}
- {from: "active", to: "implemented", at: "2026-10-30T15:30:12Z", actor: "human:timo", revision: 19, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Outcome

A source's key can include another field of the record, so two records with the same id in two
inputs stay two documents. Today the key is `adapter:op:id` with `id` one path
(`src/sources.rs:1080-1100`, spec `instance.yaml:163`); equal message timestamps in two chat
channels collide and the second is dropped by `keys.insert` (cb3 report, C-10). A JSON null id
becomes the string `null`, so every null-id record shares one key and all but one are dropped
silently (`src/sources.rs:1081-1084`; R3-02), contradicting `website/docs/spec-file.md:334`.

## Acceptance

1. A source whose key template names the input field `channel` and the record's `ts`: two records
   with equal `ts` in two channels are both applied (new test).
2. A record whose id is null is listed in `skipped` with the reason; no key `…:null` is written.
3. A plain `id` path keys exactly as today (existing tests green).

## Findings it closes

C-10, R3-02.

## Scope

Derived 2026-10-30 by `story-scoper` against main a870a628. The checkout's HEAD b61a56c8 differs from a870a628 only in the planning store: `git diff --quiet a870a628 HEAD -- src spec tests website generated crates` showed no difference. Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `src/sources.rs` `fetch_records` (lines 1048-1160) — cited
- **Body citations checked:**
  - `src/sources.rs:1080-1100` is correct. 1080 is `for record in &records`, 1081 is `let Some(id) = at(record, &c.id)`, 1097 is `let key = format!("{}:{}:{id}", c.adapter, c.operation)` and 1098 is `if !keys.insert(key.clone()) { continue; }` — cited
  - `src/sources.rs:1081-1084` is correct. `Value::Null` falls to `other => other.to_string()` at 1083, which gives `"null"`. Only a missing path (`None` from `at`, line 1000) is skipped, and that skip is silent: `continue` at 1085, nothing goes to `skipped` — cited
  - `spec/domains/instance.yaml:163` is correct: `{name: id, type: String}` in `cortex.instance.ConnectorsSource` (lines 147-178) — cited
  - `website/docs/spec-file.md:334` is correct: "A record without an id, or whose text is empty, is skipped. A document's key is `<adapter>:<operation>:<id>`" (334-335) — cited
- **Edits in `src/sources.rs`:**
  - `fetch_records` 1081-1098: build the key from the id path or the key template, and send a null id to `skipped` through `note` (fn at 364) — cited
  - `let mut keys` at 1059 stays as it is — cited
  - `render` (1009-1029) fills `{a.b}` from a single `record: &Value`. A template naming the input field `channel` needs the input too, so `render` gets a combined context or the key gets its own renderer — inferred
  - `at` (1000-1006) is read only — cited
  - `Fetched.skipped` (391) is unchanged — cited
- **Spec type:**
  - `spec/domains/instance.yaml` 147-178 `cortex.instance.ConnectorsSource`. Either a new field (for example `key: Optional<String>`), or `id` read as a template when it holds `{…}`, recorded as an UNMAPPED comment beside line 163. The story does not choose — inferred
  - If a field is added, its invariants at 171-175 are unaffected — inferred
- **`generated/` impact:**
  - Only if a new field is added. The regenerated files would be:
    - `generated/spec-types/types.rs` (`CortexInstanceConnectorsSource`, 460) and `generated/spec-types/source.schema.json` (503-559)
    - `generated/cortex-model/src/instance.rs` (`ConnectorsSource`, 77) and `plan.json`
    - `generated/schema/schema/types/cortex.instance.{ConnectorsSource,InstanceSpec,SourceSettings}.schema.json`, plus `commands/cortex.instance.{Create,Adopt,Update}Instance.schema.json`
    - `generated/docs/docs/domains/cortex-instance.md:57-59` ("record of 11 fields")
  - If `id` becomes a template, `generated/` is unchanged — inferred
- **Also likely:**
  - `src/model_map.rs` 516-551 `settings()`, the `m::ConnectorsSource { … id: c.value.id.clone() … }` literal at 522-529. Edited only if a new field is added — inferred
  - `src/main.rs` 1489-1532, the rekey check that `story:update-refuses-rekey` adds (a `depends_on` of this story). It must treat a change of the key template as a re-key — inferred
- **Documents:**
  - `website/docs/spec-file.md` 328-335: the `id` row of the table and the sentence about key and skip — cited
  - `CHANGELOG.md` `## Unreleased` (line 6) — inferred
- **Tests:**
  - `tests/connectors_walks.rs`: new tests for acceptance 1 (equal `ts` in two `channel` inputs, both applied) and 2 (a null id is named in `skipped`, no `…:null` key). The existing spec helpers that write `id: key` are at 146-160 and 235-260, with more at 550 and 576 — cited
  - Acceptance 3 is the existing tests in that file and `tests/structured_files.rs:604-610`, which asserts the `<adapter>:<operation>:<id>` key — cited
  - Possibly unit tests in the `src/sources.rs` `mod tests` (1604) for the key renderer — inferred
  - A conformance scenario in `tests/conformance.rs` if the spec type changes — inferred
- **Confidence:** high for `src/sources.rs`, the docs and the test file, because the story names them and each citation checks out. Medium for spec, `generated/` and `model_map.rs`, because whether a field is added or `id` is reinterpreted is undecided, and that choice decides the `generated/` churn.
- **Would collide with:**
  - any unit touching `src/sources.rs` around `fetch_records` or `render`
  - any unit touching the `cortex.instance.ConnectorsSource` type in `spec/domains/instance.yaml` and its `generated/` output
  - any unit touching `tests/connectors_walks.rs`. `story:connectors-invoke-0-60` also lists it, as a dependency.
  - any unit touching the `cortex update` rekey check in `src/main.rs`
- **Safety facts:**
  - A plain `id` path still yields exactly `<adapter>:<operation>:<id>`. So the seen-state keys of existing instances are not re-keyed, nor is the masked child-failure map: `crate::mask::mask_key(&key)` at 1109/1112. Proof level 2 (`src/sources.rs:1097`, `:1109`). Unproven.
  - Downstream readers parse the key by position: `src/known.rs:34` (`key.splitn(3, ':').nth(2)`) and the shared citation `record:<adapter>:<operation>:<id>` (`src/spec.rs:218`, `tests/structured_files.rs:605`). A templated key keeps its first two segments, so `known.rs` reads the whole rendered template as the id. A templated connectors key no longer matches a structured source's raw-id identity on the same operation, so the supersede refusal at `src/spec.rs:218` does not reach it. Proof level 2. Unproven.
  - Existing instances whose seen state holds a `…:null` key stop refreshing that document once null ids are skipped. Proof level 1. Unproven.

## Decisions

Taken 2026-10-30 by the wave coordinator.

- New optional field `key: Optional<String>` on `cortex.instance.ConnectorsSource`; `id` keeps its meaning.
- The template is rendered by `render` with `{a.b}` for record fields and `{input.a}` for the input's fields; the key is `<adapter>:<operation>:<rendered>`.
- A null or missing id (or a template field that resolves to nothing) is named in `skipped`; no `…:null` key.
- `update-refuses-rekey` treats a change of `key` as a re-key.
- Correction (2026-10-30, wave coordinator): the key renders with `structured::render_template`, not `render`; a template field missing from the record sends the record to `skipped`.
