---
format: aep.planning-md/3
id: upstream-blocker:ess-scheduled-job-realization
kind: upstream-blocker
status: open
title: ESS does not realize a scheduled job as a Kubernetes CronJob yet
relations:
- blocks: story:cronjob-from-spec
revision: 1
---
## What would clear it

An ESS release whose runtime realizes a scheduled job from a specification (ESS story `story:runtime-realizes-a-scheduled-job`): from a modelled job (its schedule, image, command, volumes and secrets), ESS generates the Kubernetes CronJob manifests and checks them for drift. cortex's `story:cronjob-from-spec` models its per-source run in that form.
