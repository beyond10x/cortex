---
format: aep.planning-md/3
id: specification:wave-20261006i-cortex-wave-16
kind: specification
status: approved
title: 'Wave 20261006i: cortex 1.0 wave 16'
revision: 4
transitions:
- {from: "draft", to: "in_review", at: "2026-10-06T11:40:47Z", actor: "agent:claude", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-06T11:40:48Z", actor: "agent:claude", revision: 3}
---
## Wave 20261006i: cortex 1.0 wave 16

Opened 2026-10-06 by the coordinating session, `aep:implementing` 0.19.2 wave mode.

**Approval:** the operator approved every wave up front on 2026-10-05: "I approve all waves upfront and now. do not ask for permission, you orchestrate this".

## Units

| unit | story | branch | worktree | build dir | scratch |
|---|---|---|---|---|---|
| U1 | `story:postgres-credential-from-connectors` | `unit/postgres-credential-from-connectors` | `cortex-w16-a` | `~/.cache/b10x-target/cortex-w16-a` | `~/.cache/cortex-wave-20261006i/a` |

## Rules

- At most two adversary passes per unit; a small defect fix gets a coordinator review only.
- Every command runs with `TMPDIR` under the unit's scratch directory.
- Units whose scopes share a file are merged by the coordinator, who resolves the overlap and gates the result.

## Commits approval authorises

One commit per unit through `b10x-gates bot`; the merges into `wave/20261006i`; coordinator commits for docs and the changelog; the closing planning-store commit; the pull request into `main` and its merge.

## State

| step | state |
|---|---|
| U1 `story:postgres-credential-from-connectors` | `8514d95`, merged into `wave/20261006i` at `efa6f90`; its trees `cortex-w16-a` and `cortex-w16-int` were finished |
| main merged into the wave | `aecbed8` |
| EKR pin 0.0.32 | `f84c401`; `upstream-blocker:ekr-postgres-password-file` cleared |
| integration tree | `cortex-w16-int2` (managed), building into its own `target/`; the local gate runs a debug `ekr` 0.0.32 copied to `target/ekr-0.0.32/ekr`, sha256 `df7377198e81d8bcba9cad5e7e6f33d0bddcc0acf2a45bb91f35077deaa08636`, built from the source tree of tag `0.0.32`; CI builds `ekr` from the tag |
| security review of U1 | next, in the integration tree |
| gate, pull request, close | after the review |
