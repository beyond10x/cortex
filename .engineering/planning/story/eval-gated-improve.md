---
format: aep.planning-md/3
id: story:eval-gated-improve
kind: story
status: draft
title: A guidance change is kept only when the gated-change protocol admits it on evidence
summary: cortex eval improve under eval.gated_change/1, split from eval-cases-from-review
relations:
- decomposes: epic:instance-answers-questions
- serves: vision:self-updating-instances
- depends_on: story:eval-cases-from-review
- depends_on: story:eval-run
revision: 1
---
## Outcome

`cortex eval improve <instance> --protocol eval.gated_change/1` proposes one change to the ask
service's guidance, runs the tuning and held-out splits of the instance's eval set, and keeps the
change only when the protocol admits `change.keep` on evidence about that candidate revision;
otherwise it records the proposal and keeps nothing.

## Why

Split from `story:eval-cases-from-review`, whose `case add` half ships first. The protocol is
published (engineering-protocols 0.9.0, `protocols/eval-gated-change/1.yaml`): artifacts
`eval_design` and `candidate`, evidence kinds `design_review`, `guard_check`,
`tuning_comparison`, `heldout_comparison`, each keep claim demanding a producer independent on
`different_agent_run`; evaluation belongs to Canon (`b10x-canon-engineering` crate). cortex has
no Canon dependency today, so this is its own unit of design: how the candidate revision is
hashed, which runs produce which evidence, and how independence is shown.

## Domain relations

- entity `cortex.ask.GuidanceChange` (identity: the stamp): the proposed text, the candidate
  revision, the protocol's outcome (`kept | rejected | proposed | inconclusive`), `kept:
  Boolean`. Relation: `cortex.instance.Instance` owns many via `instance_name`.

## Work

- Spec first: the entity and the command `ImproveGuidance` with its outcomes; validate,
  conformance, `task generate`. Then the code, evaluating through the published crate at its tag.
- Docs: `commands.md`.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- With a stand-in model whose proposed change regresses the held-out split, the guidance is
  unchanged and the `GuidanceChange` records `kept: false` with outcome `rejected`.
- Evidence produced by the run that wrote the candidate leaves the outcome undecided, and the
  guidance is unchanged.
- A change that improves tuning and holds on held-out, with independent evidence, is kept and the
  guidance file holds it.

## Not in this story

- `cortex eval case add --from-review` (`story:eval-cases-from-review`).
- Defining `eval.gated_change/1`: outside cortex.
