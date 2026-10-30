---
format: aep.planning-md/3
id: story:down-vote-keeps-review-item
kind: story
status: draft
title: A committed down vote always leaves its voted-down review item or says it did not
tags:
- review-20261030
relations:
- decomposes: epic:instance-answers-questions
revision: 2
---
## Outcome

A down vote whose write committed always leaves a `voted-down` review item, or the client is told
the vote was kept. Today the vote write can commit and the following `open_review` time out in the
records queue (`src/ask/records.rs:1077`): the client gets 500 `store-failed`, the vote is kept,
and no `voted-down` review item exists.

## Acceptance

- With the records queue stalled after the vote write commits, a down vote either ends with a
  `voted-down` review item for the answer, or answers with an outcome that says the vote was
  recorded and the review item was not (new test).
- A down vote that commits nothing still answers `store-failed`, and no vote is kept.

## Source

- `review-result:adversary-w30a-askio-pass-2`, finding at `src/ask/records.rs:1077` (CONFIRMED,
  introduced; read, not run).
- Wave 20261030a coordinator review of the unit reports.
