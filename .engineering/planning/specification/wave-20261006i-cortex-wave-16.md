---
format: aep.planning-md/3
id: specification:wave-20261006i-cortex-wave-16
kind: specification
status: implemented
title: 'Wave 20261006i: cortex 1.0 wave 16'
revision: 8
transitions:
- {from: "draft", to: "in_review", at: "2026-10-06T11:40:47Z", actor: "agent:claude", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-06T11:40:48Z", actor: "agent:claude", revision: 3}
- {from: "approved", to: "implemented", at: "2026-10-07T02:56:54Z", actor: "human:timo", revision: 7}
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

| step | commit |
|---|---|
| U1 `story:postgres-credential-from-connectors` | `8514d95`, merged at `efa6f90` |
| main merged into the wave | `aecbed8`, `0f8ae03` |
| EKR pin 0.0.32; `upstream-blocker:ekr-postgres-password-file` cleared | `f84c401`, `ece3755` |
| security review pass 1: 4 findings, all introduced, no password leak | `review-result:security-postgres-credential-from-connectors-pass-1` |
| the four findings fixed by a fresh implementor in the integration tree (the unit's implementor could not be reached) | `d63e8f5` |
| coordinator commits: in-tree build reason without outside ids, AGENTS.md `## Serves` | `6f4a131`, `bd24e20` |

Integration tree `cortex-w16-int2` (managed), built into its own `target/`.

## Outcome

Closed 2026-10-07. `task check`, run step by step on `wave/20261006i` at `15842e3`: 8 steps, each exit 0; 37 test suites, 485 passed, 0 failed, 0 skipped (the Docker PostgreSQL case ran); `cargo test -- --list` names 485 tests. The local gate ran a debug `ekr` 0.0.32 built from the source tree of tag `0.0.32` (sha256 `df7377198e81d8bcba9cad5e7e6f33d0bddcc0acf2a45bb91f35077deaa08636`); the pull request CI builds `ekr` from the tag.

| agent | tokens | tool uses | wall time |
|---|---|---|---|
| security reviewer, pass 1 | 221,692 | 83 | 970 s |
| implementor, correction of pass 1 | 172,723 | 85 | 570 s |

Findings of pass 1: 4 (2 warning, 2 note), all `introduced`, all `fixed` in `d63e8f5`. No second pass: the correction was verified by the coordinator (no assertion dropped; the six review cases unchanged) and by the gate.

Left open: with the `ess` on `PATH` (0.55.0), `task docs-check` exits 201 ("requires ess 0.52.0 and this is ess 0.55.0, which is newer, and --strict-requires refuses a newer release", measured by the implementor); every other `--strict-requires` step of `task check` uses the same pin. The gate above ran the pinned 0.52.0. Moving the pin to the newest ESS is the next wave.
