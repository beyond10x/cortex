---
format: aep.planning-md/3
id: story:child-text-and-filter
kind: story
status: active
title: Child records are rendered by a template, and a filter limits child calls
summary: '`child.text` renders each child record; `child.when` skips the child call for records that do not match'
tags:
- review-20261030
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:connectors-invoke-0-60
scope:
- confidence: cited
  path: generated
- confidence: cited
  path: spec/domains/instance.yaml
- confidence: cited
  path: spec/suite.json
- confidence: cited
  path: src/model_map.rs
- confidence: cited
  path: src/sources.rs
- confidence: inferred
  path: tests/conformance.rs
- confidence: cited
  path: tests/connectors_walks.rs
- confidence: cited
  path: website/docs/reference/ess/cortex-instance.md
- confidence: cited
  path: website/docs/spec-file.md
- confidence: cited
  path: website/static/schemas/instance-spec.schema.json
revision: 16
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T10:11:04Z", actor: "human:timo", revision: 15}
- {from: "proposed", to: "active", at: "2026-10-30T10:11:04Z", actor: "human:timo", revision: 16}
---
## Outcome

A child call can be limited to the records that need it, and its records are rendered as text.
Today one child call is made per parent record (`src/sources.rs:1101-1108`; spec `ChildCall` at
`instance.yaml:137-145` has no filter), so chat thread replies are never fetched cheaply (C-09),
and child records are appended as raw JSON after the parent text (`src/sources.rs:1131-1134`), so
the head cut at `max_chars_per_document` (`src/run.rs:269-276`) can drop the newest (C-08).

## Acceptance

1. With `child.when: reply_count > 0`, a parent with zero replies makes no child call; one with
   replies makes one (call count in the stand-in, new test).
2. With `child.text: "{author.displayName} {created}: {body}"`, the document text holds the
   rendered lines and no raw JSON object (new test).
3. A child without `text` or `when` behaves as today.

## Findings it closes

C-08, C-09.

## Scope

Derived 2026-10-30 by `story-scoper` against main a870a628. The tree's `src/`, `spec/`, `tests/` and `generated/` do not differ from a870a628 (`git diff --quiet`). Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `src/sources.rs` `fetch_records` (fn at :1048), the `child` block at :1101-1135. Cited.
- **Citation check:** `src/sources.rs:1101-1108` is confirmed. `if let Some(child)` is at :1101, and the `walk(...)` call that runs once per parent record spans :1102-1109. Cited.
- **Citation check:** `src/sources.rs:1131-1134` is off by one. The loop that appends raw JSON (`for child in &children` / `text.push_str(&child.to_string())`) is at :1132-1135. Cited.
- **Citation check:** `instance.yaml:137-145` is confirmed. `cortex.instance.ChildCall` has its fields operation, input, records and paging at :137-143 and its invariant at :144-145. There is no filter and no text field. Cited.
- **Citation check:** `src/run.rs:269-276` is confirmed, give or take a line. `truncate` (doc comment :269, fn :270-277) cuts the text and keeps its head. `cut_text` (:280-283) is applied with `max_chars_per_document` at :694 and :717. Context only; `src/run.rs` is not edited. Cited.
- **Files: `src/sources.rs:1101-1109`.** A `child.when` test goes in front of the `walk` call, so a parent that does not match makes no child call. Cited for the site, inferred for the shape.
- **Files: `src/sources.rs:1132-1135`.** When `child.text` is set, `child.to_string()` is replaced by `render(t, child)`. Cited for the site, inferred for the shape.
- **Files: `src/sources.rs:1009-1030`.** `render` is reused unchanged for `child.text`. It already fills `{a.b}` dotted paths, which is what the story's `{author.displayName}` needs. Cited.
- **Files: `src/sources.rs` (new function).** `when: reply_count > 0` needs a comparison. The only record filter in the tree is `passes` / `m::RecordFilter` (`src/sources.rs:1461-1465`, spec `instance.yaml:180-185`). It tests whether a field equals or does not equal a list of values and cannot express `>`. No expression evaluator exists in `src/`. A small predicate parser and evaluator is new code, probably next to `passes`/`scalar` (:1461-1476). Inferred.
- **Files: `src/model_map.rs:538-550`.** This builds `m::ChildCall { operation, input, records, paging }` field by field with no `..`, so it stops compiling once the generated struct gains fields. The new fields are mapped here, and a `when` that does not parse is refused at load here, like the other refusals in this file. Cited for the site, inferred for the refusal.
- **Spec: `spec/domains/instance.yaml:137-145` `cortex.instance.ChildCall`.** Add `text: Optional<String>` (or `Optional<List<String>>` to match `ConnectorsSource.text` at :165) and `when: Optional<String>`, plus an `# UNMAPPED:` comment that states the `when` grammar, following the pattern at :155-160. Cited for the type, inferred for the field types.
- **Generated (regenerate with `task generate`, never by hand, AGENTS.md:41,54-55).** Every file that `git grep ChildCall` hits changes. Cited:
  - `generated/cortex-model/src/instance.rs:19-32` (struct `ChildCall`)
  - `generated/cortex-model/plan.json`
  - `generated/cortex-model/PLAN.md`
  - `generated/spec-types/types.rs`
  - `generated/spec-types/source.schema.json`
  - `generated/spec-types/types-report.json`
  - `generated/schema/schema/types/cortex.instance.{ChildCall,ConnectorsSource,InstanceSpec,SourceSettings,SourceSpec}.schema.json`
  - `generated/schema/schema/commands/cortex.instance.{Adopt,Create,Update}Instance.schema.json`
  - `generated/docs/docs/domains/cortex-instance.md`
  - `spec/suite.json`
  - `website/docs/reference/ess/cortex-instance.md`
  - `website/static/schemas/instance-spec.schema.json` (generated per AGENTS.md:74-76)
- **Documents: `website/docs/spec-file.md`.** Hand-written. Line :332 (the `child` row) and :419-428 say "Each child record is added to the parent's text as one paragraph of JSON". `text`, `when` and the `when` grammar need to be added there. Cited.
- **Tests: `tests/connectors_walks.rs`.** Gets new tests for acceptance 1 and 2. Cited from the story.
  - The stand-in `CONNECTORS` (:21-48) returns issues as `{key, summary}` and comments as `{body}` only. It has no `reply_count`, `author` or `created`, so acceptance 1 and 2 need either a new stand-in (the file already builds per-test stand-ins from `HEAD` at :58-) or extended fields. Cited.
  - Call counting can use the existing `comments.log` (:39) or the "{} child calls" count (:686). Cited.
  - Acceptance 3 ("behaves as today") is already covered by the existing child tests at :358, :469, :540, :667 and :696. Cited.
- **Also likely: `tests/conformance.rs`.** ESS conformance synthesis "witnesses every Optional as present" (`instance.yaml:168-169`), so a synthesized `when` string must get past any load-time refusal, or the witness has to be shaped for it. Inferred.
- **Would collide with:**
  - `src/sources.rs` `fetch_records` (the connectors walk, :1048-1161)
  - `src/model_map.rs` (the connectors mapping, :520-552)
  - `spec/domains/instance.yaml` (`ChildCall`/`ConnectorsSource`)
  - the shared generated outputs: `spec/suite.json`, `generated/spec-types/types.rs`, `generated/cortex-model/plan.json`. Any unit that changes any spec domain regenerates these.
  - `tests/connectors_walks.rs`. The dependency `story:connectors-invoke-0-60` lists it in its scope, and its stand-in still matches the removed `--operation` form (:29, :37).
  - `website/docs/spec-file.md` (the `kind: connectors` section)

  Inferred, from the surfaces above.
- **Safety fact:** both new fields are Optional. When they are absent, the block at `src/sources.rs:1101-1135` takes today's path, so existing instances keep the same document text and change hash and are not extracted again. Adopting `child.text` changes every parent's text, so each parent is extracted once more. Step 2 (`src/sources.rs:1101-1135`, `website/docs/spec-file.md:421-424`), unproven. Cited.
- **Confidence:** high for the code sites and the spec type, because the story names them and the tree confirms them with a one-line offset. Medium for the generated set and the `when` grammar, because the field types and the expression language are not fixed by the story.

## Decisions

Taken 2026-10-30 by the wave coordinator.

- `ChildCall.text` has the type of `ConnectorsSource.text` and is rendered by `render`.
- The filter is `ChildCall.follow: Optional<cortex.instance.FollowRule>`, the type `story:structured-children-follow-rule` declares (`equals {field, value}`, `within_days {field, days}`, `at_least {field, number}`), so connectors and structured children share one rule. C-08's `reply_count > 0` is `at_least {field: reply_count, number: 1}`. No expression grammar.
- A parent the rule does not match makes no child call; its text is the parent alone.
- The two stories are one unit; `structured-children-follow-rule` lands first and declares `FollowRule`.
