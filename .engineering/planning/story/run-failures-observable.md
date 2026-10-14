---
format: aep.planning-md/3
id: story:run-failures-observable
kind: story
status: draft
title: A failed scheduled run shows as a source status, a metric and a hook call
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
- depends_on: story:scheduler-one-shot-run
revision: 1
---
## Outcome

A cluster operator sees a failed or skipped scheduled run without reading `cortex.log`: each source
shows its last outcome, when it happened and why, a run leaves a metric a monitoring system collects,
and an optional hook is called when a run fails or a source is disabled.

## Why

Verified in this tree:

- "Failures" (`website/docs/operating.md:145-166`): a failing run is counted with
  `--record-failure`, the second failure in a row disables the source and stops its timer
  (`:147-150`); a gate failure is recorded in `cortex.log` (`:160-162`). `cortex.log` holds one JSON
  line per run (`:28`).
- Partly present: `cortex run` prints one JSON line with its outcome and exits 1 on a failure
  (`src/main.rs:222-232`), and the `Sources` view lists each source `state` and
  `consecutive_failures` (`spec/domains/instance.yaml:1518-1529`). So a scheduler can read the exit
  status of the run it started.
- Missing: the view holds no last outcome, time or reason (`:1518-1529`); nothing in `src/` or
  `spec/` writes a metric or calls an alert hook (searched for `metric`, `prometheus`, `alert` and
  `webhook`; the only `webhook` matches are credential shapes in `src/redact.rs:363-370`).
- Not confirmed: whether the events the spec declares (`emits:`, for example
  `spec/domains/instance.yaml:1130`) leave the process anywhere a monitor could read; read the spec
  only, not the generated event path.

## Work

- Spec first: add the last outcome, its time and its reason to `cortex.instance.Source` and the
  `Sources` view; add the metric output and the hook as declared configuration of the instance or
  home, with their outcomes; validate with `ess specify validate` and the conformance suite, then
  `task generate`. The concrete forms (the metric format and where it is written; a hook that is a
  command, so any secret it needs stays outside cortex) are settled in that spec change and agreed
  when the story is proposed.
- A run records its outcome on the source in every case, including `disabled` and the lock-held
  outcome of `story:scheduler-one-shot-run`.
- Document the status, the metric and the hook on the operating page "Failures".

## Acceptance

- After a run fails with `fetch-failed`, `cortex sources` shows that source with last outcome
  `fetch-failed`, its time and its reason.
- After the same run, the metric output counts one more failed run for that source.
- When the second failure in a row disables a source, the configured hook is called once with the
  source id and the outcome `disabled`.

## Not in this story

Exit statuses for lock held and disabled (`story:scheduler-one-shot-run`); the overspend signal of
the daily ceiling (`story:daily-instance-budget` reports its own outcome, which this status then
shows); dashboards or alert routing of any cluster.
