---
format: aep.planning-md/3
id: story:unit-names-unique-across-homes
kind: story
status: draft
title: Two instances in two homes never take over each other's systemd unit
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
revision: 2
---
## Outcome

Two instances in two homes on one systemd user manager never take over each other's unit. Today
`Systemd::foreign` (`src/schedule.rs:393`) asks the other home's registry for the requesting
instance (`a-b`), finds none, and reads the live unit of `a/b-c` (same unit name
`cortex-a-b-c`) as orphaned.

## Acceptance

- With home 1 holding instance `a` with source `b-c` and its unit installed, `cortex create` of
  instance `a-b` with source `c` in home 2 exits non-zero naming the unit and the other home, and
  the unit file of home 1 is unchanged (new test).
- `cortex update` and `cortex adopt` refuse the same collision the same way (new test).
- A unit whose owning instance no longer exists in any home is still reported as orphaned, as
  today (existing tests green).

## Source

- `review-result:adversary-w30a-cli-pass-1`, finding 4 ("two homes can still take over each
  other's units", `src/schedule.rs:393`); outside `story:unit-names-unique`, whose acceptance says
  "in one home".
- Wave 20261030a coordinator review of the unit reports.
