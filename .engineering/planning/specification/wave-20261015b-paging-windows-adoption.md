---
format: aep.planning-md/3
id: specification:wave-20261015b-paging-windows-adoption
kind: specification
status: implemented
title: 'Wave 20261015b: paging, windows, fetch-only, cost split, adoption by key, connection by profile'
relations:
- specifies: story:next-link-paging
- specifies: story:run-fetch-only
- specifies: story:run-cost-split
- specifies: story:adopt-seen-by-key
- specifies: story:epoch-window-placeholders
- specifies: story:connection-by-profile
revision: 4
transitions:
- {from: "draft", to: "in_review", at: "2026-10-15T03:31:24Z", actor: "human:timo", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-15T03:31:24Z", actor: "human:timo", revision: 3}
- {from: "approved", to: "implemented", at: "2026-10-30T09:39:33Z", actor: "human:timo", revision: 4}
---
## Wave 20261015b: paging, windows, fetch-only, cost split, adoption by key, connection by profile

Opened 2026-10-15, `aep:implementing` wave mode, for a PostgreSQL-backed deployment that runs
cortex at a 10-minute cadence. Based on the 0.3.0 head (`61c1a37`); none of it goes into 0.3.0.
Integration branch `wave/20261015b`, tree `cortex-w15b-int`.

## Units

| unit | story | tree |
|---|---|---|
| paging | `story:next-link-paging` | `cortex-w15b-paging` |
| fetch | `story:run-fetch-only` | `cortex-w15b-fetch` |
| cost | `story:run-cost-split` | `cortex-w15b-cost` |
| adopt | `story:adopt-seen-by-key` | `cortex-w15b-adopt` |
| epoch | `story:epoch-window-placeholders` | `cortex-w15b-epoch` |
| profile | `story:connection-by-profile` | `cortex-w15b-profile` |

Width: all six drafted stories of the deployment's list, one tree each. Every unit changes
`spec/domains/instance.yaml` and the generated model; `src/run.rs` (fetch, cost, adopt, profile)
and `src/sources.rs` (paging, epoch, profile) are shared too. The integration branch merges them
one at a time and regenerates from the merged specification.
