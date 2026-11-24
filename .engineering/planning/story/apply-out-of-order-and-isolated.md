---
format: aep.planning-md/3
id: story:apply-out-of-order-and-isolated
kind: story
status: implemented
title: An older record becomes history and one refused record costs only itself
summary: Defects 1 and 2 of the runbook refresh replay
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T08:14:54Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T08:14:55Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-24T00:20:38Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

A record older than the value it would replace becomes that value's history, and one record EKR
refuses costs that record, not the source's run.

## Work

Consumer: the runbook refresh of `timo/dev-operator` (session `bf-runbook`), 2026-11-23, a
three-round replay into one store (857 records, 3564 facts), synthetic wording:

- A `replaces` fact whose `valid_from` is older than the current value's is refused
  `invalid-supersession`, and the whole part it was applied in is lost: 16 parts in the replay.
  With sources read in any order, a record older than the current value is applied as history
  (closed at the current value's `valid_from`), not as a replacement, and not refused.
- One record whose extraction redeclared an edge type with other endpoints
  (`extraction-type-conflict`) undid the whole source run. A refusal EKR names for one record or
  part is isolated: that record is refused and kept for retry, the rest of the run applies, and
  the run report names the refused record and the reason.

## Acceptance

With a stand-in model, a record dated January applied after one dated February leaves February
current and January as its history, with zero refusals; a batch where one record's extraction
redeclares an edge type applies every other record, and reports that one as refused with
`extraction-type-conflict`.
