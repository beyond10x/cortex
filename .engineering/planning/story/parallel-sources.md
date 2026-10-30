---
format: aep.planning-md/3
id: story:parallel-sources
kind: story
status: draft
title: Sources and instances of one home run in parallel
summary: The home lock, the per-instance stage record and stage recovery stop serialising every run
tags:
- review-20261030
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
revision: 2
---
## Outcome

Sources and instances of one home can run at the same time. Today `run` holds the home's
`cortex.lock` for the whole run, and every command except `restore` takes it (`src/home.rs:41-50`,
`src/main.rs:400-406`), so all instances and all sources run one after another; one `stage.json`
per instance holds every source's state (`src/snapshot.rs:600-604`); each run's start abandons
every begun stage of the store (`src/run.rs:1513-1524`); EKR refuses a publish when the store
moved (`src/ekr.rs:378-388`). Reported downstream as C-11, by the performance review as P-05 and
by the CLI review as R5-13.

The shape is an architecture decision: `decision-blocker:parallel-sources-shape`.

## Acceptance

Written once the shape is decided. Measured by the wall time of two concurrent runs (about the
maximum of the two, not their sum) and zero `stage-head-moved` refusals under concurrency.

## Decisions

Decided 2026-10-30 by `b10x-chef-orchestrator` (D270): option B. A lock per instance and a short registry lock, read commands take no lock, the stage record split per source; instances run in parallel, the sources of one instance stay serial. Option C waits until EKR says whether a stage can be rebased.
