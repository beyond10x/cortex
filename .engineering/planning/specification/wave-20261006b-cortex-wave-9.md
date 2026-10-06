---
format: aep.planning-md/3
id: specification:wave-20261006b-cortex-wave-9
kind: specification
status: implemented
title: 'Wave 20261006b: cortex 1.0 wave 9'
revision: 5
transitions:
- {from: "draft", to: "in_review", at: "2026-10-06T01:11:25Z", actor: "agent:claude", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-06T01:11:25Z", actor: "agent:claude", revision: 3}
- {from: "approved", to: "implemented", at: "2026-10-06T01:53:06Z", actor: "agent:claude", revision: 5}
---
## Wave 20261006b: cortex 1.0 wave 9

Opened 2026-10-06 by the coordinating session, `aep:implementing` 0.19.2 wave mode.

**Approval:** the operator approved every wave up front on 2026-10-05: "I approve all waves upfront and now. do not ask for permission, you orchestrate this".

## Units

| unit | story | branch | worktree | build dir | scratch |
|---|---|---|---|---|---|
| U1 | `story:test-stand-ins-never-busy` | `unit/test-stand-ins-never-busy` | `cortex-w9-busy` | `~/.cache/b10x-target/cortex-w9-busy` | `~/.cache/cortex-wave-20261006b/busy` |

## Rules

- At most two adversary passes per unit; a small defect fix gets a coordinator review only.
- Every command runs with `TMPDIR` under the unit's scratch directory.
- Units whose scopes share a file are merged by the coordinator, who resolves the overlap and gates the result.

## Commits approval authorises

One commit per unit through `b10x-gates bot`; the merges into `wave/20261006b`; coordinator commits for docs and the changelog; the closing planning-store commit; the pull request into `main` and its merge.

## Outcome

Closed 2026-10-06. One unit merged into `wave/20261006b`; the integration tree equals the unit tree outside `.engineering`, whose `task check` exited 0 with 339 tests.

| unit | story | commit |
|---|---|---|
| U1 | `story:test-stand-ins-never-busy` | `335f41c` |
