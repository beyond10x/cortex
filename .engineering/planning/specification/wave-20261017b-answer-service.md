---
format: aep.planning-md/3
id: specification:wave-20261017b-answer-service
kind: specification
status: approved
title: 'Wave 20261017b: the answer service'
relations:
- specifies: story:ask-service
- specifies: story:eval-run
- specifies: story:answer-records-and-review
- specifies: story:eval-cases-from-review
revision: 3
transitions:
- {from: "draft", to: "in_review", at: "2026-10-17T03:37:09Z", actor: "human:timo", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-17T03:37:09Z", actor: "human:timo", revision: 3}
---
## Wave 20261017b: the answer service

Opened 2026-10-17, `aep:implementing` wave mode, after the record
`architecture-decision-record:answer-service-in-cortex` was accepted. Based on wave/20261017a
(`84b36af`, release 0.5.6). Integration branch `wave/20261017b`, tree `cortex-w17b-int`.

## Units

| unit | story | tree |
|---|---|---|
| ask | `story:ask-service` | `cortex-w17b-ask` |
| eval | `story:eval-run` | `cortex-w17b-eval` |

Then, once both are merged: `story:answer-records-and-review` (depends on ask-service), then
`story:eval-cases-from-review` (depends on both).

Width: 2. These are the two stories whose dependencies hold; the other two depend on them. The
domain split is settled here: `story:ask-service` makes the service's own domain `cortex.ask`
(`spec/domains/ask.yaml`), and the eval types of `story:eval-run` go into `cortex.instance`, as
eval is a batch verb of the `cortex` binary. Their specification files are then disjoint except
for the optional `ask` field ask-service adds to `cortex.instance.InstanceSpec`. `src/main.rs`,
`src/lib.rs` and `generated/` are shared; the integration branch merges the first two additively
and regenerates the third. The tool loop over `ekr mcp` belongs to ask-service; eval-run's `mcp`
runner uses it once both are merged.
