---
format: aep.planning-md/3
id: specification:wave-20261005h-cortex-wave-7
kind: specification
status: approved
title: 'Wave 20261005h: cortex 1.0 wave 7'
revision: 3
transitions:
- {from: "draft", to: "in_review", at: "2026-10-05T16:13:32Z", actor: "agent:claude", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-05T16:13:32Z", actor: "agent:claude", revision: 3}
---
## Wave 20261005h: cortex 1.0 wave 7

Opened 2026-10-05 by the coordinating session, `aep:implementing` 0.19.2 wave mode.

**Approval:** the operator approved every wave up front on 2026-10-05: "I approve all waves upfront and now. do not ask for permission, you orchestrate this".

## Units

| unit | story | branch | worktree | build dir | scratch |
|---|---|---|---|---|---|
| U1 | `story:file-records` | `unit/file-records` | `cortex-w7-records` | `~/.cache/b10x-target/cortex-w7-records` | `~/.cache/cortex-wave-20261005h/records` |
| U2 | `story:first-web-instance` | `unit/first-web-instance` | `cortex-w7-web` | `~/.cache/b10x-target/cortex-w7-web` | `~/.cache/cortex-wave-20261005h/web` |
| U3 | `story:run-snapshots` | `unit/run-snapshots` | `cortex-w7-snap` | `~/.cache/b10x-target/cortex-w7-snap` | `~/.cache/cortex-wave-20261005h/snap` |

## Rules

- At most two adversary passes per unit; a small defect fix gets a coordinator review only.
- Every command runs with `TMPDIR` under the unit's scratch directory.
- Units whose scopes share a file are merged by the coordinator, who resolves the overlap and gates the result.

## Commits approval authorises

One commit per unit through `b10x-gates bot`; the merges into `wave/20261005h`; coordinator commits for docs and the changelog; the closing planning-store commit; the pull request into `main` and its merge.
