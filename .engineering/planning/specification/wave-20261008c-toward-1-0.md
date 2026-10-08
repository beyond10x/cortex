---
format: aep.planning-md/3
id: specification:wave-20261008c-toward-1-0
kind: specification
status: approved
title: 'Wave 20261008c: toward 1.0'
relations:
- specifies: story:release-1-0
revision: 3
transitions:
- {from: "draft", to: "in_review", at: "2026-10-08T14:24:03Z", actor: "human:timo", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-08T14:24:04Z", actor: "human:timo", revision: 3}
---
## Wave 20261008c: toward 1.0

Opened 2026-10-08, `aep:implementing` wave mode, after cortex 0.2.2 was released
(https://github.com/beyond10x/cortex/releases/tag/v0.2.2, PR https://github.com/beyond10x/cortex/pull/60).
Everything for 1.0 except the version bump, the tag and the Release, which wait for the 7-day run
of `story:first-web-instance` (read on day 8, 2026-10-16). No behaviour change, so no
specification change. The coordinator did every unit; no implementor or adversary was dispatched
(store moves, changelog, documentation and one workflow).

Integration branch `wave/20261008c`, tree `cortex-w23-int`. Release tree `cortex-rel-022`.

## Units

| unit | commit | what |
|---|---|---|
| store | `9c51af0` | the three epics active; Codex out of the 1.0 epic; `web-pages-cited-as-urls` no longer waits on Codex; `release-1-0` depends on `first-web-instance` and is active |
| docs site | `c1d6823` | `project-site.yml` pinned at `fffaaeb2`, whose run lookup retries; the `wait` job removed |
| changelog | `5cd26e1` | "What 1.0 holds" and the open limits, under Unreleased |
| rehearsal | `d89e7cb` | the acceptance rehearsed on `v0.2.2` in an empty home (`story:release-1-0` § Rehearsal); install commands fixed |

## Commits approval authorises

Coordinator commits through `b10x-gates bot`, the merge of `origin/main` after 0.2.2, the pull
request into `main` and its merge.

## Open after this wave

- `story:release-1-0`: bump to 1.0.0, release through `AGENTS.md`, run the acceptance on `v1.0.0`,
  after the 7-day run passes.
- The documentation site's deploy step fails when GitHub has not listed the uploaded `github-pages`
  artifact yet, and every re-run of that job finds the earlier attempt's artifact as well (run
  37791160685, attempts 1 and 2). Both steps are in the shared `project-site.yml`; the fix is
  there, and the pin moves when it lands.
