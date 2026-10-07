---
format: aep.planning-md/3
id: specification:wave-20261007c-structured-children
kind: specification
status: approved
title: 'Wave 20261007c: structured children, conformance field shapes'
relations:
- specifies: story:structured-children
- specifies: story:conformance-checks-field-shapes
revision: 3
transitions:
- {from: "draft", to: "in_review", at: "2026-10-07T07:43:31Z", actor: "human:timo", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-07T07:43:31Z", actor: "human:timo", revision: 3}
---
## Wave 20261007c: structured children, conformance field shapes

Opened 2026-10-07 by the coordinating session, `aep:implementing` 0.20.1 wave mode.

**Approval:** the operator approved every wave up front on 2026-10-05 ("I approve all waves upfront
and now. do not ask for permission, you orchestrate this").

## Selection

`aep plan artifact waves --kind story --status active`: wave 1 `story:codex-model-backend`,
`story:conformance-checks-field-shapes`, `story:first-web-instance`, `story:structured-children`;
collisions none; unassessed none. `story:codex-model-backend` is held by
`decision-blocker:codex-exec-keeps-shell` and `story:first-web-instance` is the running 7-day
instance, so neither is a unit. Left out: `story:structured-compare-links` (depends on
`story:structured-children`), `story:postgres-run-undo` (blocked upstream), `story:docs-for-1-0`
(it documents the features before it).

## Units

| unit | story | scope | branch | worktree | build dir | scratch |
|---|---|---|---|---|---|---|
| U1 | `story:structured-children` | cited (2 lines inferred) | `unit/structured-children` | `cortex-w19-a` | `cortex-w19-a/target` | `cortex-w19-a/target/wave-scratch` |
| U2 | `story:conformance-checks-field-shapes` | cited | `unit/conformance-checks-field-shapes` | `cortex-w19-b` | `cortex-w19-b/target` | `cortex-w19-b/target/wave-scratch` |

Integration branch `wave/20261007c`, tree `cortex-w19-int`.

## Decisions taken for U1

The story leaves these open; the coordinator decided them before dispatch:

- `children` sits on the `connectors` input of a structured source (`StructuredConnectors`), so
  the type itself says children need a Connectors walk.
- A child's identity includes its parent's id, so same-named children of two parents stay two nodes.
- A child is linked to its parent by one edge whose name the child mapping gives (`parent:`).
- A child call that fails for one parent: the parent and its other children are imported, the run's
  `skipped` names the parent and the operation with the reason, and that parent's earlier children of
  the failed operation are not ended as dropped in that run.
- The new path of a renamed parent is not added as an alias (EKR's `apply-extraction` never adds
  aliases to a matched node); the old path stays an alias, which is what the acceptance asks.

## Rules

- One implementor (`aep:implementor`) per unit; one adversary pass (`aep:adversary`) on U1. U2
  changes the test harness only: the gate decides, and a mutant run proves the new check fails.
- Every command runs with `TMPDIR` under the unit's scratch directory.
- The full `task check` runs once, on the integration branch, before the push.

## Commits approval authorises

One commit per unit through `b10x-gates bot`, plus the adversary's test commit; the merges into
`wave/20261007c`; coordinator commits for the store and the changelog; the closing planning-store
commit; the pull request into `main` and its merge.
