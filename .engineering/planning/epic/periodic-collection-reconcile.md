---
format: aep.planning-md/3
id: epic:periodic-collection-reconcile
kind: epic
status: active
title: Instances are collected and reconciled on a schedule from containers
relations:
- serves: vision:self-updating-instances
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T19:29:10Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T19:29:11Z", actor: "human:timo", revision: 3}
---
## Outcome

An instance whose EKR store is on PostgreSQL, served from containers under an external scheduler
(a Kubernetes CronJob), is written on a schedule by cortex with no desktop session: collection
runs unattended, changed and deleted source records are reconciled into the store, a bad run can
be undone, and a failure or an overspend is seen by the operator of that cluster.

## Why

A downstream deployment went live on 2026-10-13 with an EKR store on PostgreSQL served from
containers on Kubernetes, and nothing writes that store on a schedule. Read against cortex `main`
`e598c7e` and v0.2.2, nine gaps stand between cortex and that deployment (each to be verified by
its story before it is drafted as fact):

1. Scheduling is systemd user timers only; there is no one-shot runner an external scheduler can
   call, with exit statuses for "lock held" and "source disabled".
2. A release carries only the x86-64 tarball and `cortex setup` builds EKR with cargo; there is no
   container image.
3. Source and PostgreSQL credentials are reached only through `connectors connections`, whose
   custody needs a desktop Secret Service; headless custody is a Connectors change, and the cortex
   side consumes it.
4. Extraction runs `claude -p` on an interactive sign-in and removes `ANTHROPIC_API_KEY`; there is
   no unattended model credential. Design only: entering a key and any spend it carries are the
   operator's.
5. Checkpoints (`state/<source>.json`, `registry.json`, the lock) live only on local disk under
   `$CORTEX_HOME`; they move into the store, or the volume contract is documented.
6. A bad run on PostgreSQL cannot be undone (`story:postgres-run-undo`).
7. A changed document is re-extracted only after `refresh_after_days`, nothing from its earlier
   text is retracted, and a record deleted at the source is never retracted for model-extracted
   sources.
8. Failures are reported only by one `cortex.log` line and a failed systemd unit; there is no
   status an external scheduler reads, no metric and no alert hook.
9. Budget is per run only (`budget_usd`); there is no daily ceiling per instance across sources.

Already present, no story: incremental Connectors windows, content-hash idempotence per key,
`dropped: Supersede` for structured sources.

## Not in this epic

Choosing or paying for a model credential; operating the downstream deployment.
