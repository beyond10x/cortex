---
format: aep.planning-md/3
id: specification:wave-20261005h-cortex-wave-7
kind: specification
status: implemented
title: 'Wave 20261005h: cortex 1.0 wave 7'
revision: 5
transitions:
- {from: "draft", to: "in_review", at: "2026-10-05T16:13:32Z", actor: "agent:claude", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-05T16:13:32Z", actor: "agent:claude", revision: 3}
- {from: "approved", to: "implemented", at: "2026-10-05T18:48:44Z", actor: "agent:claude", revision: 5}
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

## Outcome

Closed 2026-10-05. Three units merged into `wave/20261005h`; `task check` on the integration branch: EXIT=0, 295 tests (the first run failed one test with `Text file busy`, a harness race filed as `story:test-stand-ins-never-busy`; the re-run passed).

| unit | story | commit | adversary findings |
|---|---|---|---|
| U1 | `story:file-records` | `4dfee0f` | 11 (1 blocker), fixed |
| U2 | `story:first-web-instance` | `dbc0f4f` | none run (example files) |
| U3 | `story:run-snapshots` | `d795671` | 10, fixed or documented |

All three implementors and two adversaries stopped once on an API rate limit and were resumed. The pinned `ess` 0.52.0 had to be reinstalled, and `ess` 0.53.0 on PATH refuses the pin, so every gate ran with `ESS=` set to the pinned binary.
