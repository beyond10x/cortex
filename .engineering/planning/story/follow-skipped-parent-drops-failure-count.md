---
format: aep.planning-md/3
id: story:follow-skipped-parent-drops-failure-count
kind: story
status: draft
title: A connectors parent the follow rule skips keeps no earlier child-failure count
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
revision: 2
---
## Outcome

A connectors parent that the child follow rule skips does not keep a child-failure count from
earlier runs. Today it keeps it, so a parent that matches the rule again starts from a stale count
toward `CHILD_FAILURE_LIMIT`.

## Acceptance

- A parent whose child call failed once, and which the follow rule skips on the next run, has no
  entry in the source state's `child_failures` after that run (new test in
  `tests/connectors_walks.rs`).
- A parent the rule follows keeps counting failed child calls as today (existing tests green).

## Source

- Wave 20261030a coordinator review of the unit reports.
- `story:child-text-and-filter` (the connectors child follow rule).
