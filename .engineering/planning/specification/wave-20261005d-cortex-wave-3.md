---
format: aep.planning-md/3
id: specification:wave-20261005d-cortex-wave-3
kind: specification
status: implemented
title: 'Wave 20261005d: cortex 1.0 wave 3'
revision: 5
transitions:
- {from: "draft", to: "in_review", at: "2026-10-05T13:10:34Z", actor: "agent:claude", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-05T13:10:34Z", actor: "agent:claude", revision: 3}
- {from: "approved", to: "implemented", at: "2026-10-05T14:10:03Z", actor: "agent:claude", revision: 5}
---
## Wave 20261005d: cortex 1.0 wave 3

Opened 2026-10-05 by the coordinating session, `aep:implementing` 0.19.2 wave mode.

**Approval:** the operator approved every wave up front on 2026-10-05: "I approve all waves upfront and now. do not ask for permission, you orchestrate this".

## Units

| unit | story | branch | worktree | build dir | scratch |
|---|---|---|---|---|---|
| U1 | `story:connectors-source-walks` | `unit/connectors-source-walks` | `cortex-w3-walks` | `~/.cache/b10x-target/cortex-w3-walks` | `~/.cache/cortex-wave-20261005d/walks` |
| U2 | `story:symlinked-seed-directory` | `unit/symlinked-seed-directory` | `cortex-w3-symlink` | `~/.cache/b10x-target/cortex-w3-symlink` | `~/.cache/cortex-wave-20261005d/symlink` |

## Selection

- `story:connectors-source-walks` is the next story on the cb3 goal's path (`initiative:run-on-cortex`: organisation-scale sources).
- `story:symlinked-seed-directory` (filed in wave 20261005c) touches only `src/instance.rs`; the two typed scopes share no file.
- `story:credential-mask-covers-titles` shares `src/run.rs` with U1 and waits for the next wave.
- At most two adversary passes per unit. Every command runs with its own `TMPDIR` under the unit's scratch directory (a shared temporary directory blocked `ess generate` in wave 20261005c).

## Commits approval authorises

One commit per unit through `b10x-gates bot`; the merges into `wave/20261005d`; the closing planning-store commit; the pull request into `main` and its merge.

## Outcome

Closed 2026-10-05. Both units merged into `wave/20261005d`; `task check` on the integration branch: EXIT=0, 177 tests (`~/.cache/cortex-wave-20261005d/int/gate.log`).

| unit | story | commit | merge | adversary findings, pass 1 → 2 |
|---|---|---|---|---|
| U1 | `story:connectors-source-walks` | `c3fa691` | `3079840` | 8 → 6 |
| U2 | `story:symlinked-seed-directory` | `aa32a84` | `101d821` | none run (small fix) |
