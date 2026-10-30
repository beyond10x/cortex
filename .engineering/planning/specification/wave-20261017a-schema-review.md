---
format: aep.planning-md/3
id: specification:wave-20261017a-schema-review
kind: specification
status: implemented
title: 'Wave 20261017a: schema proposals on approval, edge type widening'
relations:
- specifies: story:schema-proposals-on-approval
- specifies: story:schema-widens-edge-type
revision: 4
transitions:
- {from: "draft", to: "in_review", at: "2026-10-17T02:22:09Z", actor: "human:timo", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-17T02:22:09Z", actor: "human:timo", revision: 3}
- {from: "approved", to: "implemented", at: "2026-10-30T09:39:33Z", actor: "human:timo", revision: 4}
---
## Wave 20261017a: schema proposals on approval, edge type widening

Opened 2026-10-17, `aep:implementing` wave mode. Based on 0.5.5 (`db39216a`). Integration branch
`wave/20261017a`, tree `cortex-w17a-int`.

## Units

| unit | story | tree |
|---|---|---|
| approve | `story:schema-proposals-on-approval` | `cortex-w17a-approve` |
| widen | `story:schema-widens-edge-type` | `cortex-w17a-widen` |

Width: 1, the two units run one after the other. Both change `src/schema.rs` and the schema
section of `spec/domains/instance.yaml`, and the widening is a new proposal kind of the review and
apply path the first unit adds (`depends_on`). The widen unit starts from the integration branch
once the approve unit is merged into it. Every other ready story waits on upstream work
(`story:repository-documents-source` on the Connectors raw-file reads) or follows this wave.
