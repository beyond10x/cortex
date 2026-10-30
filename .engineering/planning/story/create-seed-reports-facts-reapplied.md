---
format: aep.planning-md/3
id: story:create-seed-reports-facts-reapplied
kind: story
status: draft
title: The seed detail of cortex create reports facts_reapplied
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
revision: 2
---
## Outcome

The seed detail `cortex create` prints carries `facts_reapplied`, as the `ran` detail
(`src/main.rs:2758`) and the run's log line (`src/run.rs:2661`) do.

## Acceptance

- A `cortex create` whose seed run re-applies a replacing fact prints a detail with
  `facts_reapplied` equal to the count in that run's log line (new test).
- A seed run that re-applies nothing prints `facts_reapplied: 0` or omits it the same way the `ran`
  detail does.

## Source

- `review-result:adversary-w30a-cost-pass-1` § 5 ("`cortex create` without `facts_reapplied`:
  consistent with the story's Decisions, which name only the `ran` detail and the log line").
- Wave 20261030a coordinator review of the unit reports.
