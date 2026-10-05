---
format: aep.planning-md/3
id: specification:wave-20261006a-cortex-wave-8
kind: specification
status: approved
title: 'Wave 20261006a: cortex 1.0 wave 8'
revision: 3
transitions:
- {from: "draft", to: "in_review", at: "2026-10-05T23:46:20Z", actor: "agent:claude", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-05T23:46:21Z", actor: "agent:claude", revision: 3}
---
## Wave 20261006a: cortex 1.0 wave 8

Opened 2026-10-06 by the coordinating session, `aep:implementing` 0.19.2 wave mode.

**Approval:** the operator approved every wave up front on 2026-10-05: "I approve all waves upfront and now. do not ask for permission, you orchestrate this".

## Units

| unit | story | branch | worktree | build dir | scratch |
|---|---|---|---|---|---|
| U1 | `story:run-gate` | `unit/run-gate` | `cortex-w8-gate` | `~/.cache/b10x-target/cortex-w8-gate` | `~/.cache/cortex-wave-20261006a/gate` |
| U2 | `story:adopt-existing-store` | `unit/adopt-existing-store` | `cortex-w8-adopt` | `~/.cache/b10x-target/cortex-w8-adopt` | `~/.cache/cortex-wave-20261006a/adopt` |

## Rules

- At most two adversary passes per unit; a small defect fix gets a coordinator review only.
- Every command runs with `TMPDIR` under the unit's scratch directory.
- Units whose scopes share a file are merged by the coordinator, who resolves the overlap and gates the result.

## Commits approval authorises

One commit per unit through `b10x-gates bot`; the merges into `wave/20261006a`; coordinator commits for docs and the changelog; the closing planning-store commit; the pull request into `main` and its merge.
