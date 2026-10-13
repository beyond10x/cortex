---
format: aep.planning-md/3
id: specification:wave-20261013a-polish
kind: specification
status: approved
title: 'Wave 20261013a: polish'
relations:
- specifies: story:ess-pin-0-59
- specifies: story:dependency-pins-current
- specifies: story:docs-match-code
revision: 3
transitions:
- {from: "draft", to: "in_review", at: "2026-10-13T19:21:29Z", actor: "human:timo", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-13T19:21:30Z", actor: "human:timo", revision: 3}
---
## Wave 20261013a: polish

Opened 2026-10-13, `aep:implementing` wave mode, after cortex 0.2.2. A polish wave with a release at
its end: dependency pins current, documentation that matches the code, a `STATUS.md`, and every
gate green. The 7-day run `agent-tooling-3` is not touched by this wave; a change that would need
it restarted is planned, not applied.

Integration branch `wave/20261013a`, tree `cortex-w13a-int`.

## Units

| unit | story | runs |
|---|---|---|
| ess | `story:ess-pin-0-59` | implementor, own tree |
| pins | `story:dependency-pins-current` | coordinator, own tree, beside `ess` |
| docs | `story:docs-match-code` | coordinator, on the integration branch after `ess` and `pins` |

`ess` and `pins` both touch `.github/workflows/check.yml` and `AGENTS.md`, on different lines;
the coordinator merges them.

## Commits approval authorises

One commit per unit through `b10x-gates bot`, the merges into `wave/20261013a`, the closing
store commit, the pull request into `main` and its merge, then the release through `AGENTS.md`.

## Issues

None open when the wave opened (`gh issue list --state open`, 2026-10-13).
