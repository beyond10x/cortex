---
format: aep.planning-md/3
id: specification:wave-20261015c-gateway-tokens
kind: specification
status: implemented
title: 'Wave 20261015c: model through a gateway, token budget'
relations:
- specifies: story:model-through-gateway
- specifies: story:token-budget
revision: 4
transitions:
- {from: "draft", to: "in_review", at: "2026-10-15T15:50:11Z", actor: "human:timo", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-15T15:50:11Z", actor: "human:timo", revision: 3}
- {from: "approved", to: "implemented", at: "2026-10-30T09:39:33Z", actor: "human:timo", revision: 4}
---
## Wave 20261015c: model through a gateway, token budget

Opened 2026-10-15, `aep:implementing` wave mode, for a PostgreSQL-backed deployment that runs
extraction against an open model behind a gateway. Based on 0.4.0 (`df2c9f9`); released as 0.5.0.
Integration branch `wave/20261015c`, tree `cortex-w15c-int`.

## Units

| unit | story | tree |
|---|---|---|
| gateway | `story:model-through-gateway` | `cortex-w15c-gateway` |
| tokens | `story:token-budget` | `cortex-w15c-tokens` |

Width: the two ready stories; every other draft waits on upstream work or on 1.0. Both add fields
to `cortex.instance.ModelSpec`, so the specification change of both is made first on the
integration branch, regenerated, and both units start from that commit. Past it their scopes are
disjoint: gateway changes `src/extract.rs` and `src/schedule.rs`, tokens changes `src/run.rs` and
`src/spend.rs`; the documentation pages are shared and merged by the integration branch.
