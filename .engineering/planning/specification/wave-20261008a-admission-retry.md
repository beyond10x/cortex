---
format: aep.planning-md/3
id: specification:wave-20261008a-admission-retry
kind: specification
status: draft
title: 'Wave 20261008a: admission retry for the 7-day run'
relations:
- specifies: story:connectors-admission-timeout-retry
revision: 1
---
## Wave 20261008a: admission retry for the 7-day run

Opened 2026-10-08, `aep:implementing` 0.21.2 wave mode. N is one: the 7-day run restarts on a release with this fix before its next timer run, so the unit ships alone. `story:parent-identity-prefix-unmasked` and `story:docs-for-1-0` were dispatched at the same time on their own branches and close in wave 20261008b.

Selection: `aep plan artifact waves` placed all three in separate surfaces (`src/connectors.rs`; `src/structured.rs`; `website/docs/`, `README.md`); the scope lines are inferred from the story bodies.

## Units

| unit | story | branch | worktree | build dir | scratch |
|---|---|---|---|---|---|
| U1 | `story:connectors-admission-timeout-retry` | `unit/connectors-admission-timeout-retry` | `cortex-w21-a` | `cortex-w21-a/target` | `~/.cache/cortex-wave-20261008a/retry` |

Integration branch `wave/20261008a`, tree `cortex-w21-int`. Wave 20261008b units: U2 `cortex-w21-b` (`unit/parent-identity-prefix-unmasked`), U3 `cortex-w21-c` (`unit/docs-for-1-0`).

## Rules

- One implementor (`aep:implementor`), one adversary pass (`aep:adversary`) on U1.
- Package gates of `cortex-cli` on the exact commit before the push; the pull request's CI runs the full gate.

## Commits approval authorises

The unit commit and any adversary or fix commit through `b10x-gates bot`; the merges into `wave/20261008a`; coordinator commits for the store and the changelog; the pull request into `main` and its merge. The release 0.2.1 follows through the release process.
