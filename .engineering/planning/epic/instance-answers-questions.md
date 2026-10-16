---
format: aep.planning-md/3
id: epic:instance-answers-questions
kind: epic
status: draft
title: An instance answers questions over its own store
summary: Ask service, answer records and review, eval, entity linking, vitals and research runs
relations:
- serves: vision:self-updating-instances
revision: 1
---
## Outcome

An instance answers questions over its own store, not only collects into it: a reader asks a
question and gets a streamed, cited answer within the limits the spec sets; every answer is kept
with its votes and reviewed when it went wrong; reviewed answers become eval cases an operator runs
end to end; and the readers see what is being asked and how the instance is doing.

## Why now

A downstream deployment runs this on top of a cortex instance in code of its own: an answer service
over the EKR MCP reader, answer records with votes and a review queue, an eval harness, and a
request log. None of it is specific to that deployment. Neither active epic covers it:
`epic:periodic-collection-reconcile` and `epic:organisation-scale-instance` are about writing the
store; this one is about reading it. cortex today serves no port and owns no tables
(`spec/domains/instance.yaml`, `cortex.instance.MonitoringSpec`), so the shape of the change is
decided first, in `architecture-decision-record:answer-service-in-cortex`.

## Stories

| story | adds |
|---|---|
| `ask-service` | `cortex ask serve` and its event stream contract |
| `ask-request-log` | one structured log line per request, with request and trace ids and no question text |
| `answer-records-and-review` | answer records, one changeable vote per user, a review queue, retention |
| `eval-run` | `cortex eval run` over a question set, scored deterministically |
| `eval-cases-from-review` | reviewed answers become eval cases; a gated improvement loop once the protocol exists |
| `question-entity-linking` | the store's entities a question names, before the answer |
| `vitals-and-topics` | live counts and the most asked entities and terms |
| `research-run` | a run that answers an open question by writing cited claims for review |

## Not in this epic

Token verification, the identity provider, the model backend, a chat front-end and any user
interface (the record's "What stays outside cortex").
