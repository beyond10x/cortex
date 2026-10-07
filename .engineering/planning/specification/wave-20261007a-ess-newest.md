---
format: aep.planning-md/3
id: specification:wave-20261007a-ess-newest
kind: specification
status: implemented
title: 'Wave 20261007a: the newest ESS (0.55.0)'
relations:
- specifies: story:ess-pin-newest
revision: 6
transitions:
- {from: "draft", to: "in_review", at: "2026-10-07T03:19:41Z", actor: "human:timo", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-07T03:19:41Z", actor: "human:timo", revision: 3}
- {from: "approved", to: "implemented", at: "2026-10-07T03:39:57Z", actor: "human:timo", revision: 5}
---
## Wave 20261007a: the newest ESS (0.55.0)

Opened 2026-10-07 by the coordinating session, `aep:implementing` 0.20.1 wave mode. N is one: the
pin move regenerates `generated/` and may touch `src/model_map.rs`, which every spec-changing story
also touches, so no other story can share the wave. `story:codex-model-backend` is held by its
decision blocker.

**Approval:** the operator approved every wave up front on 2026-10-05 ("I approve all waves upfront
and now. do not ask for permission, you orchestrate this").

## Units

| unit | story | branch | worktree | build dir | scratch |
|---|---|---|---|---|---|
| U1 | `story:ess-pin-newest` | `unit/ess-pin-newest` | `cortex-w17-a` | `cortex-w17-a/target` | `cortex-w17-a/target/wave-scratch` |

Integration branch `wave/20261007a`, tree `cortex-w17-int`.

## Rules

- One implementor (`aep:implementor`); the unit is a toolchain move, so no adversary: the full
  gate on the integration branch and the pull request's CI decide.
- Every command runs with `TMPDIR` under the unit's scratch directory.

## Commits approval authorises

The unit commit through `b10x-gates bot`; its merge into `wave/20261007a`; coordinator commits for
the store and the changelog; the closing planning-store commit; the pull request into `main` and
its merge.

## Outcome

Closed 2026-10-07. U1 `story:ess-pin-newest` merged at `442e51b` (unit commits `84c925f`, `e53fa74`). `task check` with `ess` 0.55.0, every step exit 0, in the unit tree on `e53fa74`: 37 test suites, 485 passed, 0 failed, 0 skipped (the Docker PostgreSQL case ran); `cargo test -- --list` printed 485 entries. The wave head differs from `e53fa74` only in `CHANGELOG.md` and this store, so no second local gate ran; the pull request CI runs `task check` on the head.

| agent | tokens | tool uses | wall time |
|---|---|---|---|
| implementor, U1 | 157,555 | 94 | 1,027 s |

Found on the way: the four tracked `generated/*/.ess-output/state.json` records held the generating tree's absolute path under the home directory, hex-encoded as path components (`UnixBytes1`), so the text scans did not see it. `84c925f` stops tracking them; earlier commits still hold them.

Left open: with 0.55.0, an `Optional` `{generated: true}` payload field is read from the context port (`generate_optional_<t>`), so the two `UNMAPPED:` notes on `QualityMeasured` and `SchemaChangesProposed` no longer state a limit of the pinned ESS (measured by the implementor with a scratch probe). Moving `rate` and `cost_usd` onto those events is a separate story.
