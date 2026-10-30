---
format: aep.planning-md/3
id: story:connections-launch-deadline
kind: story
status: draft
title: A connections launch child is bounded by the run's deadline
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
revision: 1
---
## Outcome

A long-lived `connectors connections launch` child that stops answering cannot hold a run past
its deadline.

## Why

Unit tail of wave 20261030a bounded every Connectors call by a whole-call deadline (D35); the
long-lived `connections launch` children were out of the story's scope and have no bound.

## Work

1. Find where cortex starts `connections launch` children and how long they live.
2. Bound them by the run's deadline: kill the process group, report the source as timed out.

## Acceptance

- A test with a launch child that never answers ends the run within the deadline plus 1 s,
  with the source's outcome naming the timeout.
