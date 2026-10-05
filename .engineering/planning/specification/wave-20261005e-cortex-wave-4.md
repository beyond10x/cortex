---
format: aep.planning-md/3
id: specification:wave-20261005e-cortex-wave-4
kind: specification
status: implemented
title: 'Wave 20261005e: cortex 1.0 wave 4'
revision: 5
transitions:
- {from: "draft", to: "in_review", at: "2026-10-05T14:13:38Z", actor: "agent:claude", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-05T14:13:38Z", actor: "agent:claude", revision: 3}
- {from: "approved", to: "implemented", at: "2026-10-05T14:48:54Z", actor: "agent:claude", revision: 5}
---
## Wave 20261005e: cortex 1.0 wave 4

Opened 2026-10-05 by the coordinating session, `aep:implementing` 0.19.2 wave mode.

**Approval:** the operator approved every wave up front on 2026-10-05: "I approve all waves upfront and now. do not ask for permission, you orchestrate this".

## Units

| unit | story | branch | worktree | build dir | scratch |
|---|---|---|---|---|---|
| U1 | `story:structured-source` | `unit/structured-source` | `cortex-w4-structured` | `~/.cache/b10x-target/cortex-w4-structured` | `~/.cache/cortex-wave-20261005e/structured` |
| U2 | `story:credential-mask-covers-titles` | `unit/credential-mask-covers-titles` | `cortex-w4-mask` | `~/.cache/b10x-target/cortex-w4-mask` | `~/.cache/cortex-wave-20261005e/mask` |

## Rules

- At most two adversary passes per unit; a small defect fix gets a coordinator review only.
- Every command runs with `TMPDIR` under the unit's scratch directory.
- Units whose scopes share a file are merged by the coordinator, who resolves the overlap and gates the result.

## Commits approval authorises

One commit per unit through `b10x-gates bot`; the merges into `wave/20261005e`; coordinator commits for docs and the changelog; the closing planning-store commit; the pull request into `main` and its merge.

## Outcome

Closed 2026-10-05. Both units merged into `wave/20261005e`; `task check` on the integration branch: EXIT=0, 223 tests.

| unit | story | commit | adversary findings |
|---|---|---|---|
| U1 | `story:structured-source` | `6e2ae44` | 7 (3 blockers), fixed or documented |
| U2 | `story:credential-mask-covers-titles` | `ae4b75f` | 7 (1 blocker), fixed |

One adversary pass each (the operator's rule: one review for risky changes). The two units overlapped in `src/run.rs`; the coordinator resolved it at merge.
