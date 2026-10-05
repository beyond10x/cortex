---
format: aep.planning-md/3
id: specification:wave-20261005f-cortex-wave-5
kind: specification
status: implemented
title: 'Wave 20261005f: cortex 1.0 wave 5'
revision: 5
transitions:
- {from: "draft", to: "in_review", at: "2026-10-05T15:00:26Z", actor: "agent:claude", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-05T15:00:26Z", actor: "agent:claude", revision: 3}
- {from: "approved", to: "implemented", at: "2026-10-05T15:38:30Z", actor: "agent:claude", revision: 5}
---
## Wave 20261005f: cortex 1.0 wave 5

Opened 2026-10-05 by the coordinating session, `aep:implementing` 0.19.2 wave mode.

**Approval:** the operator approved every wave up front on 2026-10-05: "I approve all waves upfront and now. do not ask for permission, you orchestrate this".

## Units

| unit | story | branch | worktree | build dir | scratch |
|---|---|---|---|---|---|
| U1 | `story:redaction-names-and-gate` | `unit/redaction-names-and-gate` | `cortex-w5-names` | `~/.cache/b10x-target/cortex-w5-names` | `~/.cache/cortex-wave-20261005f/names` |

## Rules

- At most two adversary passes per unit; a small defect fix gets a coordinator review only.
- Every command runs with `TMPDIR` under the unit's scratch directory.
- Units whose scopes share a file are merged by the coordinator, who resolves the overlap and gates the result.

## Commits approval authorises

One commit per unit through `b10x-gates bot`; the merges into `wave/20261005f`; coordinator commits for docs and the changelog; the closing planning-store commit; the pull request into `main` and its merge.

## Outcome

Closed 2026-10-05. One unit merged into `wave/20261005f`; `task check` on the integration branch: EXIT=0, 244 tests.

| unit | story | commit | adversary findings |
|---|---|---|---|
| U1 | `story:redaction-names-and-gate` | `f3bbbf1` | 9 (1 blocker), fixed or documented |

Before commit the coordinator ran `gitleaks protect --staged` (no leaks) as well as `b10x-gates scan-text`.
