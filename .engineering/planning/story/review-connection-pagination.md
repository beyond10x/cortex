---
format: aep.planning-md/3
id: story:review-connection-pagination
kind: story
status: active
title: An explicit connection id is found on any inventory page
summary: Finding 8 of the 0.7.2 review
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T01:54:00Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T01:54:00Z", actor: "human:timo", revision: 3}
---
## Outcome

An explicit Connectors connection id is found on any page of the connection inventory.

## Work

Finding 8 of the external review of cortex 0.7.2 (2026-11-23): with the id on the second page of
a paginated inventory, the CLI reports a missing connection and never requests the next cursor;
two timer-style retries then mark the source Disabled (`src/connectors.rs`). Follow the inventory
cursor for id lookup and for revalidation.

## Acceptance

A test with a stand-in `connectors` whose inventory holds the id on page two passes for lookup
and for revalidation, and fails on 0.7.2's code.
