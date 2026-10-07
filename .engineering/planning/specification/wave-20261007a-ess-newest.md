---
format: aep.planning-md/3
id: specification:wave-20261007a-ess-newest
kind: specification
status: approved
title: 'Wave 20261007a: the newest ESS (0.55.0)'
relations:
- specifies: story:ess-pin-newest
revision: 3
transitions:
- {from: "draft", to: "in_review", at: "2026-10-07T03:19:41Z", actor: "human:timo", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-07T03:19:41Z", actor: "human:timo", revision: 3}
---
## Wave 20261007a: the newest ESS (0.55.0)

Opened 2026-10-07 by the coordinating session, `aep:implementing` 0.20.1 wave mode. N is one: the
pin move regenerates `generated/` and may touch `src/model_map.rs`, which every spec-changing story
also touches, so no other story can share the wave. `story:codex-model-backend` is held by its
decision blocker.

**Approval:** the operator approved every wave up front on 2026-10-05 ("I approve all waves upfront
and now. do not ask for permission, you orchestrate this").

## Units

| unit | story | branch | worktree | build dir | scratch |
|---|---|---|---|---|---|
| U1 | `story:ess-pin-newest` | `unit/ess-pin-newest` | `cortex-w17-a` | `cortex-w17-a/target` | `cortex-w17-a/target/wave-scratch` |

Integration branch `wave/20261007a`, tree `cortex-w17-int`.

## Rules

- One implementor (`aep:implementor`); the unit is a toolchain move, so no adversary: the full
  gate on the integration branch and the pull request's CI decide.
- Every command runs with `TMPDIR` under the unit's scratch directory.

## Commits approval authorises

The unit commit through `b10x-gates bot`; its merge into `wave/20261007a`; coordinator commits for
the store and the changelog; the closing planning-store commit; the pull request into `main` and
its merge.
