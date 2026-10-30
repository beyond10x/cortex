---
format: aep.planning-md/3
id: decision-blocker:parallel-sources-shape
kind: decision-blocker
status: cleared
title: 'Which parallelism shape: stage per source, lock per instance, or extract outside the lock'
relations:
- blocks: story:parallel-sources
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-30T10:11:38Z", actor: "human:timo", revision: 3}
---
## The question

How do sources and instances of one home run in parallel? See `story:parallel-sources` for the
code that serialises them today.

## Options

| option | does | costs |
|---|---|---|
| A | keep the home lock; split the stage record per source (`stage-<source>.json`) and abandon only unowned stages | no parallelism yet; removes the cross-source rewind |
| B | A, plus a lock per instance and a short registry lock; read commands take no lock | instances run in parallel; sources of one instance stay serial |
| C | B, plus extract outside the lock and serialise only apply, gate and publish, rebasing on a moved head | full parallelism; changes the stage model, because EKR never rebases a stage (EKR `docs/cli.md:2096-2101`) |

Recommended: B now, C after EKR answers whether a stage can be rebased.

## What would clear it

A choice of A, B or C by the operator, written into `story:parallel-sources`.

## Decision

Decided 2026-10-30 by `b10x-chef-orchestrator` (D270): option B. A lock per instance and a short registry lock, read commands take no lock, the stage record split per source; instances run in parallel, the sources of one instance stay serial. Option C waits until EKR says whether a stage can be rebased.
