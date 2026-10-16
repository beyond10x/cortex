---
format: aep.planning-md/3
id: story:question-entity-linking
kind: story
status: draft
title: A question's matching entities are shown before its answer
summary: POST /ask/entities and an early entities event on the ask stream
relations:
- decomposes: epic:instance-answers-questions
- serves: vision:self-updating-instances
- depends_on: story:ask-service
revision: 1
---
## Outcome

`POST /ask/entities {question}` answers the store's entities the question names, as
`[{node_id, type, name, span, score}]`, ordered by score; the same list is the first event of an
ask stream, `entities`, before any `delta`. A reader sees the matching entities before the answer.

No implementation starts until the operator accepts
`architecture-decision-record:answer-service-in-cortex`, through `story:ask-service`.

## Why

A downstream deployment wants the store's matching entities shown first, and has nothing yet.
Verified in this tree: cortex keeps the entity names earlier runs found in `state/entities.json`
and offers them to the model during extraction (`src/instance.rs`, `ENTITIES` and `entities()`;
`website/docs/operating.md`, "entity names from earlier runs"). That file is a list of names, not
node ids; a match needs the node, which is in the store and reachable through the EKR MCP reader
(inferred: by its search tool; the tool's exact name and ranking are not checked here).

## Domain relations

No new entity. New value type `cortex.ask.EntityMatch` (`node_id`, `type`, `name`, `span` as start
and end offsets, `score`), the variant `entities` of `cortex.ask.AskEvent`, and the view
`cortex.ask.LinkEntities` in `spec/domains/ask.yaml`. `node_id` refers to an EKR node, not a cortex
entity.

## Work

- Spec first: `EntityMatch`, the event variant and the view; validate, conformance, `task
  generate`. Then the code.
- Candidate spans from the question matched against the store through the MCP reader, without a
  model call; the stream emits `entities` before the tool loop starts.
- Docs: the service page's event contract.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- Against a stand-in MCP reader holding a node `Acme Widget` of type `Product`, the question
  "who owns Acme Widget?" answers one match with that node id, `span` covering exactly
  `Acme Widget`, and no model call is made.
- On `POST /ask` with the same question, the first event after `status` is `entities` with the
  same list, and it precedes every `delta`.
- A question naming nothing in the store answers `[]` and an `entities` event with an empty list.

## Not in this story

- The service (`story:ask-service`).
- Topics counted from matches (`story:vitals-and-topics`).
