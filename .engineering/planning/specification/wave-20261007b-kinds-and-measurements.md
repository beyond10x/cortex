---
format: aep.planning-md/3
id: specification:wave-20261007b-kinds-and-measurements
kind: specification
status: implemented
title: 'Wave 20261007b: value kinds in the prompt, measurements on the events'
relations:
- specifies: story:prompt-names-value-kinds
- specifies: story:events-carry-measurements
revision: 5
transitions:
- {from: "draft", to: "in_review", at: "2026-10-07T06:23:20Z", actor: "human:timo", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-07T06:23:21Z", actor: "human:timo", revision: 3}
- {from: "approved", to: "implemented", at: "2026-10-07T06:56:49Z", actor: "human:timo", revision: 5}
---
## Wave 20261007b: value kinds in the prompt, measurements on the events

Opened 2026-10-07 by the coordinating session, `aep:implementing` 0.20.1 wave mode.

**Approval:** the operator approved every wave up front on 2026-10-05 ("I approve all waves upfront
and now. do not ask for permission, you orchestrate this").

## Selection

`aep plan artifact waves --kind story --status active`: wave 1 `story:codex-model-backend`,
`story:events-carry-measurements`, `story:first-web-instance`; wave 2
`story:prompt-names-value-kinds`; collision `story:codex-model-backend` × `story:prompt-names-value-kinds`
on `src/extract.rs` (cited); unassessed none. `story:codex-model-backend` is held by
`decision-blocker:codex-exec-keeps-shell` and `story:first-web-instance` is the running 7-day
instance, so neither is a unit; the two units share no file.

Left out: `story:postgres-run-undo`, blocked by `upstream-blocker:ekr-postgres-rewind` (found while
scoping); `story:structured-children` and `story:structured-compare-links` (both edit `spec/` and
`generated/`, which U2 regenerates); `story:docs-for-1-0` (it documents the features before it).

## Units

| unit | story | scope | branch | worktree | build dir | scratch |
|---|---|---|---|---|---|---|
| U1 | `story:prompt-names-value-kinds` | cited | `unit/prompt-names-value-kinds` | `cortex-w18-a` | `cortex-w18-a/target` | `cortex-w18-a/target/wave-scratch` |
| U2 | `story:events-carry-measurements` | cited (2 lines inferred) | `unit/events-carry-measurements` | `cortex-w18-b` | `cortex-w18-b/target` | `cortex-w18-b/target/wave-scratch` |

Integration branch `wave/20261007b`, tree `cortex-w18-int`.

## Rules

- One implementor (`aep:implementor`) per unit; one adversary pass (`aep:adversary`) on U2, which
  changes the spec, the generated port and two commands' output. U1 changes prompt text only: the
  gate decides.
- Every command runs with `TMPDIR` under the unit's scratch directory.
- The full `task check` runs once, on the integration branch, before the push.

## Commits approval authorises

One commit per unit through `b10x-gates bot`; the merges into `wave/20261007b`; coordinator commits
for the store and the changelog; the closing planning-store commit; the pull request into `main`
and its merge.

## Outcome

Closed 2026-10-07. U1 `story:prompt-names-value-kinds` (`289d952`, merged `15c2c2d`), U2 `story:events-carry-measurements` (`705a234`, adversary guard tests `2069597`, merged `9eb7e51`). `task check` on `7130266`, 8 steps each exit 0: 38 suites, 493 passed, 0 failed, 0 skipped; 493 names listed.

| agent | tokens | tool uses | wall time |
|---|---|---|---|
| implementor U1 | 115,014 | 60 | 389 s |
| implementor U2 | 192,939 | 105 | 1,002 s |
| adversary U2 pass 1 | 180,710 | 63 | 580 s |

U2 pass 1: 2 notes; F1 fixed in the store, F2 filed as `story:conformance-checks-field-shapes`. `story:postgres-run-undo` left the wave: `upstream-blocker:ekr-postgres-rewind`, planned against EKR staged runs.
