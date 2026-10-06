---
format: aep.planning-md/3
id: specification:wave-20261006e-cortex-wave-12
kind: specification
status: implemented
title: 'Wave 20261006e: cortex 1.0 wave 12'
revision: 6
transitions:
- {from: "draft", to: "in_review", at: "2026-10-06T07:36:01Z", actor: "agent:claude", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-06T07:36:01Z", actor: "agent:claude", revision: 3}
- {from: "approved", to: "implemented", at: "2026-10-06T07:43:44Z", actor: "agent:claude", revision: 6}
---
## Wave 20261006e: cortex 1.0 wave 12

Opened 2026-10-06 by the coordinating session, `aep:implementing` 0.19.2 wave mode.

**Approval:** the operator approved every wave up front on 2026-10-05: "I approve all waves upfront and now. do not ask for permission, you orchestrate this".

## Units

| unit | story | branch | worktree | build dir | scratch |
|---|---|---|---|---|---|
| U1 | `story:extraction-supersedes` | `unit/extraction-supersedes` | `cortex-w12-a` | `~/.cache/b10x-target/cortex-w12-a` | `~/.cache/cortex-wave-20261006e/a` |
| U2 | `story:postgres-credential-from-connectors` | `unit/postgres-credential-from-connectors` | `cortex-w12-b` | `~/.cache/b10x-target/cortex-w12-b` | `~/.cache/cortex-wave-20261006e/b` |

## Rules

- At most two adversary passes per unit; a small defect fix gets a coordinator review only.
- Every command runs with `TMPDIR` under the unit's scratch directory.
- Units whose scopes share a file are merged by the coordinator, who resolves the overlap and gates the result.

## Commits approval authorises

One commit per unit through `b10x-gates bot`; the merges into `wave/20261006e`; coordinator commits for docs and the changelog; the closing planning-store commit; the pull request into `main` and its merge.

## Unit b left the wave

Unit b (`story:postgres-credential-from-connectors`) left the wave before any work: `upstream-blocker:ekr-postgres-password-file`. The story stays active and blocked.

## Outcome

Closed 2026-10-06. One unit merged into `wave/20261006e`; the integration tree equals the unit tree outside `.engineering`, whose `task check` exited 0 with 407 tests. Unit b (`postgres-credential-from-connectors`) left before work (see above).

| unit | story |
|---|---|
| a | `story:extraction-supersedes` |
