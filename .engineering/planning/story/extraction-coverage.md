---
format: aep.planning-md/3
id: story:extraction-coverage
kind: story
status: active
title: One round leaves one active value per single-valued property and cites thread replies
summary: Defect 4 and uncited records of the runbook refresh probe
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T05:01:58Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T05:01:58Z", actor: "human:timo", revision: 3}
---
## Outcome

One refresh round leaves one active value for a single-valued property, and the claims inside
thread replies and long multi-item messages become facts.

## Work

Consumer: the runbook refresh (session `bf-runbook`), 2026-11-23, one round over 145 records:

- 16 node/property pairs hold 2 or 3 active values for a single-valued property after one round
  (a merge request's state both "merged" and "merged to main, not released").
- 69 of the 145 records are cited by no fact; the lost claims sit in thread replies and in long
  messages that list several items.

First find where each is decided, with evidence from that round's run folders: the prompt, the
batch size and `max_chars_per_document`, thread context, the extraction document, and how EKR
treats a second value of a property within one transaction or across batches. Then fix it in
cortex, or file the EKR side as an upstream blocker naming the EKR behaviour.

## Acceptance

Replaying the round's recorded input through a stand-in model holding the recorded answers leaves
one active value per single-valued property; the share of records cited by a fact is measured
before and after the fix and reported with both numbers.
