---
format: aep.planning-md/3
id: story:structured-children-follow-rule
kind: story
status: implemented
title: A structured source calls a child operation only for the parents its rule matches
relations:
- serves: vision:self-updating-instances
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T15:30:02Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-10-30T15:30:02Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-10-30T15:30:14Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Outcome

A structured source's `children` entry may carry a `follow` rule, and the run calls that child
operation only for the parent records the rule matches: a field equals a value, a date field lies
within the last N days, or a number field is at least N. A parent the rule does not match is still
read and applied as a node; only its children of that operation are not asked for, and none of its
earlier children is ended, as for a child call that fails (`dropped: Supersede`). The run report
counts the parents each child operation skipped by its rule.

## Why

A downstream deployment reads every repository of its code host through a structured source and,
for each one, its tags and merge requests as children. Most repositories are archived or idle, so
every run spends one child call per repository and parent on records that never change. With a
rule such as `archived` equals `false` or `last_activity_at` within 30 days, a run reads the
children of the active repositories only. Today every run calls each child once for every parent
it reads (`website/docs/spec-file.md`, "Children").

## Domain relations

- `cortex.instance.StructuredChild` gains an optional `follow: cortex.instance.FollowRule`.
- `cortex.instance.FollowRule` is one of: `equals {field, value}`, `within_days {field, days}`,
  `at_least {field, number}`. `field` is a dotted path into the parent record, as `{a.b}`
  placeholders are. A parent without the field, or with a value of another kind, does not match.

## Work

- Spec first: the two types, their invariants (`days` and `number` positive where it applies) and
  the paragraph of `RunSource` on children; validate, conformance, `task generate`.
- Code: `src/structured.rs` evaluates the rule on each parent before its child calls; `within_days`
  reads RFC 3339 and date-only values and measures from the run's start.
- Docs: `website/docs/spec-file.md` ("Children").

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- With `follow: {equals: {field: archived, value: false}}` and three parents, one archived, the
  stand-in `connectors` receives the child call for the two others only; all three parents are
  applied; the archived parent's children from an earlier run still hold their values.
- `within_days` with N = 30 follows a parent active 3 days ago and skips one active 90 days ago and
  one without the field; `at_least` follows a parent whose number equals N.
- A child entry without `follow` behaves as today (the existing structured tests unchanged).

## Not in this story

- Rules combined with and/or, or a rule on the input's own records (a parent filter at the source
  is the operation's input).

## Decisions

- Correction (2026-10-30, wave coordinator): the spec form of a rule is `{rule: equals, value: {field, value}}`, ESS adjacent tagging (likewise `within_days {field, days}` and `at_least {field, number}`), not the shape this story's examples use. The rule is checked in `src/sources.rs` `follows()`, not in `src/structured.rs` as Work says.
