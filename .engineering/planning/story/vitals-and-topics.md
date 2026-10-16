---
format: aep.planning-md/3
id: story:vitals-and-topics
kind: story
status: draft
title: Readers see an instance's live counts and its most asked topics
summary: GET /vitals and GET /topics?window=24h|7d as JSON from the ask service
relations:
- decomposes: epic:instance-answers-questions
- serves: vision:self-updating-instances
- depends_on: story:answer-records-and-review
- depends_on: story:question-entity-linking
revision: 1
---
## Outcome

`cortex ask serve` answers `GET /vitals` and `GET /topics?window=24h|7d` as JSON for any identity
with read scope. Vitals are live counts: asks and requests, model tokens and spend of the day, the
store's nodes and edges, and each source's freshness (its last run's time and outcome). Topics are
the most asked entities and terms over the window, counted from answer records. A user interface
only renders them.

No implementation starts until the operator accepts
`architecture-decision-record:answer-service-in-cortex`, through the two stories this one depends
on.

## Why

A downstream deployment wants these for its readers and has nothing yet. Verified in this tree:
each source's last run and counts by outcome are in `status/<source>.json` and, with
`monitoring.metrics`, `metrics.prom` (`src/status.rs`); the day's spend is in `spend.json`
(`src/spend.rs`, `cortex.instance.DailySpend`). Nothing counts asks (no ask service exists) or
reports node and edge counts (inferred: EKR can count them; not checked which verb). Topics need
answer records (`story:answer-records-and-review`) and the entities a question names
(`story:question-entity-linking`).

## Domain relations

No new entity: views over `cortex.ask.AnswerRecord`, `cortex.instance.Source` and the day's spend.
New value types `cortex.ask.Vitals` and `cortex.ask.Topics` and the views `cortex.ask.ShowVitals`
and `cortex.ask.ShowTopics` in `spec/domains/ask.yaml`.

## Work

- Spec first: the two value types and views; validate, conformance, `task generate`. Then the code.
- The two endpoints; topics counted from records in the window (entities from the linking story's
  matches, terms from the question text with a stop list), never returning a question's text.
- Docs: the service page.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- After three answers in a test service, `GET /vitals` reports `asks: 3` and the tokens of the
  three `done` events summed; each source's freshness equals its `status/<source>.json`.
- With records of one entity asked about twice in the last day and another once eight days ago,
  `GET /topics?window=24h` lists the first with count 2 and not the second; `window=7d` the same.
- `GET /topics?window=30d` answers 400 naming the allowed windows.
- No response of either endpoint contains a question's text verbatim.

## Not in this story

- A run's own report (`story:run-report`).
- Rendering: outside cortex.
