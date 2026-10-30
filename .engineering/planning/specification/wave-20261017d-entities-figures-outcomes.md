---
format: aep.planning-md/3
id: specification:wave-20261017d-entities-figures-outcomes
kind: specification
status: implemented
title: 'Wave 20261017d: store entities, figure renderer, run outcomes'
relations:
- specifies: story:store-entities-known-to-model
- specifies: story:figure-svg-renderer
- specifies: story:run-metrics
- specifies: story:model-unavailable-outcome
revision: 4
transitions:
- {from: "draft", to: "in_review", at: "2026-10-17T15:56:17Z", actor: "human:timo", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-17T15:56:17Z", actor: "human:timo", revision: 3}
- {from: "approved", to: "implemented", at: "2026-10-30T09:39:33Z", actor: "human:timo", revision: 4}
---
## Wave 20261017d: store entities, figure renderer, run outcomes

Opened 2026-10-17, `aep:implementing` wave mode, while 0.6.0's gate runs. Based on
wave/20261017b (`21bc1de8`, release 0.6.0). Integration branch `wave/20261017d`, tree
`cortex-w17d-int`.

## Units

| unit | story | tree |
|---|---|---|
| entities | `story:store-entities-known-to-model` | `cortex-w17d-entities` |
| figsvg | `story:figure-svg-renderer` | `cortex-w17d-figsvg` |
| runout | `story:run-metrics`, then `story:model-unavailable-outcome` | `cortex-w17d-runout` |

Then, once figsvg is merged: `story:ask-figure-tool`, then `story:ask-figure-event`, then
`story:ask-canvas-blocks`.

Width: 3. run-metrics and model-unavailable-outcome both add a run outcome, its exit status and
its `metrics.prom` series in `src/run.rs` and the run domain, so they are one unit, in that
order. The figure tool and event depend on the renderer. entities touches `src/run.rs` (known
names) and `src/structured.rs` (alias templates) and the `RecordMapping` type; runout touches the
run outcome; their spec edits are in different types, and `generated/` is regenerated on the
integration branch after each merge.
