---
format: aep.planning-md/3
id: upstream-blocker:ess-scheduled-job-realization
kind: upstream-blocker
status: cleared
title: ESS does not realize a scheduled job as a Kubernetes CronJob yet
relations:
- blocks: story:cronjob-from-spec
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-30T09:39:33Z", actor: "human:timo", revision: 3}
---
## What would clear it

An ESS release whose runtime realizes a scheduled job from a specification (ESS story `story:runtime-realizes-a-scheduled-job`): from a modelled job (its schedule, image, command, volumes and secrets), ESS generates the Kubernetes CronJob manifests and checks them for drift. cortex's `story:cronjob-from-spec` models its per-source run in that form.

## Cleared (2026-10-30)

ESS `story:runtime-realizes-a-scheduled-job` is `implemented` in the ESS planning store (moved
2026-10-16 on a recorded `test_result`); the commit that closed it is first contained in ESS tag
`0.62.0`. Read from `origin/main` of `beyond10x/ess` during the store reconcile of 2026-10-30.
