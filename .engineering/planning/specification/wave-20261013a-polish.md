---
format: aep.planning-md/3
id: specification:wave-20261013a-polish
kind: specification
status: implemented
title: 'Wave 20261013a: polish'
relations:
- specifies: story:ess-pin-0-59
- specifies: story:dependency-pins-current
- specifies: story:docs-match-code
revision: 6
transitions:
- {from: "draft", to: "in_review", at: "2026-10-13T19:21:29Z", actor: "human:timo", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-13T19:21:30Z", actor: "human:timo", revision: 3}
- {from: "approved", to: "implemented", at: "2026-10-13T19:53:24Z", actor: "human:timo", revision: 5}
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

## Outcome

| unit | commits | what |
|---|---|---|
| ess | `88ed988` | `RunSource`'s `ran` outcome declared first (ESS-COMMAND-004), `requires: ess 0.59.0`, regenerated; no crate change needed |
| pins | `d9804d4`, `8730dd9` | EKR 0.0.33 (CI, `cortex setup`, examples, test helpers, pages), aep 0.71.2, task 3.54.0, newest artifact actions; spec-compat fixture |
| docs | `cdef276`, `8f524a0` | broken-pipe race in the prompt write; `STATUS.md`; changelog; upstream blockers for the CronJob and headless-credential stories |

Also planned in this wave: `epic:periodic-collection-reconcile` and its draft stories (`bb0053a`), the next wave.

Full gate on `8f524a0`: `task check` exit 0 (47 lanes, 555 passed, 0 failed), `task build` and `task website` exit 0, `aep plan artifact validate` valid. The release is 0.2.3, cut from the merge of this wave.
