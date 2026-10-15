---
format: aep.planning-md/3
id: story:eval-cases-from-review
kind: story
status: draft
title: A reviewed answer becomes an eval case, and a guidance change is kept only on evidence
summary: cortex eval case add --from-review, and cortex eval improve behind eval.gated_change/1 once published
relations:
- decomposes: epic:instance-answers-questions
- serves: vision:self-updating-instances
- depends_on: architecture-decision-record:answer-service-in-cortex
- depends_on: story:answer-records-and-review
- depends_on: story:eval-run
revision: 1
---
## Outcome

`cortex eval case add <instance> --set <questions.json> --from-review <review id>` turns a closed
review item into an eval case: the question of its answer record, and the expected facts and nodes
the reviewer wrote when closing it. Once a gated-change protocol (`eval.gated_change/1`) is
published upstream, `cortex eval improve <instance> --protocol eval.gated_change/1` proposes a
change to the instance's guidance, runs the held-out split, and keeps the change only when the
protocol admits it on evidence about that change; otherwise it records the proposal and keeps
nothing.

**No implementation starts until the operator accepts
`architecture-decision-record:answer-service-in-cortex`.** The `improve` half additionally waits for
the protocol to be published; until then this story delivers `case add` only.

## Why

A downstream deployment runs a bounded improvement loop in code of its own, and feeds reviewed
answers into its eval sets by hand. The two halves are one outcome: a reviewed failure becomes a
case, and a change is kept only on that evidence. Verified in this tree: cortex has no review items
(they arrive with `story:answer-records-and-review`) and no eval set (`story:eval-run`); the
instance's guidance today is `model.instructions` (`spec/domains/instance.yaml`,
`cortex.instance.ModelSpec`), used by extraction. Whether the ask service's guidance is the same
field or a field of `AskSpec` is open; inferred that it is `AskSpec.guidance`.

## Domain relations

- `cortex.ask.EvalCase` gains an optional `review_id`. Relation: `EvalCase` references
  `cortex.ask.ReviewItem` via `review_id`; `UNMAPPED: whether a review item may yield more than one
  case`.
- `cortex.ask.ReviewItem` gains `expect_facts` and `expect_nodes`, written at close.
- entity `cortex.ask.GuidanceChange` (identity: the stamp): the proposed text, the protocol's
  verdict, `kept: Boolean`. Relation: `cortex.instance.Instance` owns many via `instance_name`.

## Work

- Spec first: the fields and entity above, the commands `AddEvalCase` and `ImproveGuidance` with
  their outcomes; validate, conformance, `task generate`. Then the code.
- `case add`: read the review item, refuse one still open, append the case to the set file.
- `improve`: only behind the published protocol; the verb refuses with a reason naming the protocol
  until it is pinned.
- Docs: `commands.md`.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- `case add --from-review` on a closed item appends one case whose question equals the record's
  and whose `review_id` names the item; on an open item it refuses and the set file is unchanged.
- Without the protocol pinned, `eval improve` refuses naming `eval.gated_change/1` and changes no
  guidance.
- With the protocol pinned and a stand-in whose proposed change fails the held-out split, the
  guidance is unchanged and the `GuidanceChange` records `kept: false` with the protocol's verdict.

## Not in this story

- Running and scoring a set (`story:eval-run`).
- Writing or defining `eval.gated_change/1`: outside cortex, per the record.
