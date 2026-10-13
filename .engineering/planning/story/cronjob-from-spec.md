---
format: aep.planning-md/3
id: story:cronjob-from-spec
kind: story
status: draft
title: Each source runs as a Kubernetes CronJob realized from the ESS specification
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
- depends_on: story:scheduler-one-shot-run
- depends_on: story:container-image
- depends_on: story:headless-connectors-credentials
- depends_on: story:checkpoints-volume-contract
revision: 2
---
## Outcome

A cluster deployment of an instance runs each source as a Kubernetes CronJob whose manifests are
generated from cortex's ESS specification through an ESS realization, not written by hand. Host
systemd timers stay the default for a host install.

## Why

An EKR store on PostgreSQL served from containers on Kubernetes has nothing that writes it on a
schedule; cortex schedules only through systemd user timers. The CronJob is the cluster's
scheduler, and its manifests are derived from the specification so that the two cannot drift.

## Work

1. Spec first: model the scheduled job in `spec/` (the run of one source, its schedule, its
   image, its volumes and secrets; the exit statuses are those `story:scheduler-one-shot-run`
   specifies) in the form the ESS scheduled-job realization
   takes, and validate it with the `ess` release that ships that realization.
2. Generate the CronJob manifests per source from the specification and an instance spec; a
   `cortex` command or task writes them.
3. The container runs the one-shot runner of the scheduler story, with credentials, checkpoints
   and metrics from the stories of this epic.

## Acceptance

- The manifests generated for `examples/example.yaml` pass `kubectl apply --dry-run=client` (or
  the realization's own validation) and name one CronJob per source with the source's schedule.
- A generated manifest differs from what the specification generates only by failing a drift
  check.

## Depends on

The ESS scheduled-job realization (an upstream ESS story); the one-shot runner, the container
image, headless credentials and checkpoint stories of this epic.

## Not in this story

Release 1.0, which stays the host-timer release gated on the 7-day run; CronJob support ships in
the first release after it.
