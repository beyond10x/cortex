---
format: aep.planning-md/3
id: story:supersession-refusals-after-history
kind: story
status: implemented
title: Older facts apply as history instead of invalid-supersession
summary: 'Documents with facts older than the stored values are applied as history; the remaining invalid-supersession refusal path after #101 is found and closed (runbook 0.8.0 finding 2)'
relations:
- serves: vision:self-updating-instances
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T16:55:33Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T16:55:33Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-24T00:21:11Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

A document whose facts are older than the stored values is applied as history and not refused with
`invalid-supersession`.

## Work

0.8.0 (#101) keeps an older value as history instead of refusing it. The runbook consumer's run of
0.8.0 still had two chat-message documents of one batch refused with `invalid-supersession`.

- Reproduce the refusal with a synthetic store and document that has the same shape as the
  refused ones (read the consumer's batch files read-only, copy them to the scratch directory, and
  put only an invented example in the repository).
- Find which path still refuses: a fact type #101 does not cover (relation, alias, a property
  with several values), a second round after the history step, or EKR refusing the history write
  itself.
- Fix it in cortex; if EKR refuses a valid history write, file the EKR defect with the synthetic
  example and keep the document out of the refusals with a named reason.

## Acceptance

The synthetic reproduction, red on main, applies with every fact either current or kept as
history, and the run reports no `invalid-supersession` refusal for it.
