---
format: aep.planning-md/3
id: story:eval-cases-from-review
kind: story
status: implemented
title: A reviewed answer becomes an eval case
summary: cortex eval case add --from-review
relations:
- decomposes: epic:instance-answers-questions
- serves: vision:self-updating-instances
- depends_on: architecture-decision-record:answer-service-in-cortex
- depends_on: story:answer-records-and-review
- depends_on: story:eval-run
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-17T10:06:01Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-17T10:06:02Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-17T16:10:57Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

`cortex eval case add <instance> --set <questions.json> --from-review <review id>` turns a closed
review item into an eval case: the question of its answer record, and the expected facts and nodes
the reviewer wrote when closing it.

The guidance change kept only on evidence (`cortex eval improve` under `eval.gated_change/1`) is
`story:eval-gated-improve`, split from this story once the protocol was published, because it needs
a Canon evaluation of its own.

## Why

A downstream deployment feeds reviewed answers into its eval sets by hand. Review items arrive with
`story:answer-records-and-review` and eval sets with `story:eval-run`; this story joins them.

## Domain relations

- `cortex.ask.EvalCase` gains an optional `review_id`. Relation: `EvalCase` references
  `cortex.ask.ReviewItem` via `review_id`; one case per review item.
- `cortex.ask.ReviewItem` gains `expect_facts` and `expect_nodes`, written at close.

## Work

- Spec first: the fields above and the command `AddEvalCase` with its outcomes; validate,
  conformance, `task generate`. Then the code.
- `case add`: read the review item, refuse one still open, append the case to the set file.
- Docs: `commands.md`.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- `case add --from-review` on a closed item appends one case whose question equals the record's
  and whose `review_id` names the item; on an open item it refuses and the set file is unchanged.

## Not in this story

- Running and scoring a set (`story:eval-run`).
- `cortex eval improve` (`story:eval-gated-improve`).
