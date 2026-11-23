---
format: aep.planning-md/3
id: story:review-change-detection
kind: story
status: active
title: A document is re-applied whenever what cortex stores from it would change
summary: Findings 4, 9 and 11 of the 0.7.2 review
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T01:53:59Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T01:53:59Z", actor: "human:timo", revision: 3}
---
## Outcome

A document is re-applied whenever what cortex stores from it would change, and fetch-only
forecasts the same documents the run will send to the model.

## Work

Findings 4, 9 and 11 of the external review of cortex 0.7.2 (2026-11-23):

- 4: adding a property to a structured source's mapping returns `updated`, but unchanged rows keep
  the old mapping; EKR gains no assertion until a row changes (`src/main.rs`, `src/run.rs`,
  `src/state.rs`, `src/structured.rs`). Include the mapping revision in change detection or
  schedule a remap.
- 9: a Web page whose title, description or publication date changed while URL and body stayed
  the same counts as unchanged, even with refresh interval 0 (`src/run.rs`, `src/state.rs`,
  `src/extract.rs`). Fingerprint every field cortex stores for a Web page.
- 11: after repeated extraction failures for an unchanged file, fetch-only classifies it as new
  while the run skips it (`src/run.rs`, `src/state.rs`). Apply the same skip state to the forecast.

## Acceptance

A test per finding reproduces the reviewed scenario, fails on 0.7.2's code and passes on this
change; `website/docs/operating.md` describes what counts as a change.
