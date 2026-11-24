---
format: aep.planning-md/3
id: story:update-no-wait
kind: story
status: implemented
title: cortex update and other lock-taking commands can answer busy at once
summary: Defect 5 of the runbook refresh replay
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T08:47:14Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T08:47:14Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-24T00:21:12Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

An orchestrator that changes an instance's spec while a run may be in flight gets an answer at
once instead of a wait of minutes.

## Work

Consumer: the runbook refresh of `timo/dev-operator` (session `bf-runbook`), 2026-11-23: `cortex
update` blocked for over 2 minutes while a run held the home lock. `cortex run --no-wait`
(finding 18 of the 0.7.2 review) answers `busy` with exit 9 at once.

- `cortex update <name> --spec <file> --no-wait` answers `busy`, exit 9, at once while another
  command holds the home lock, and changes nothing; without the flag it waits as before.
- The same flag on the other commands that take the home lock and that an orchestrator calls
  between runs (`create`, `adopt`, `remove`, `source enable`), where the spec states them.

## Acceptance

With the home lock held, `update --no-wait` returns within 1 s with exit 9 and the frozen spec
unchanged; a default `update` waits and then applies.
