---
format: aep.planning-md/3
id: story:scheduler-one-shot-run
kind: story
status: draft
title: An external scheduler runs one source and reads why it did not run from the exit status
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
revision: 1
---
## Outcome

An external scheduler (a Kubernetes CronJob, or any process supervisor) runs one source with one
`cortex` invocation and tells from its exit status alone whether the source ran, failed, was
skipped because another command held the home, or was skipped because the source is disabled;
none of it needs systemd.

## Why

Verified in this tree:

- Scheduling is systemd user units only: `website/docs/limits.md:14-16` ("Timers and the viewer are
  systemd user units; there is no other scheduler"). `src/schedule.rs:458` (`install_source`) writes
  a `Type=oneshot` service whose `ExecStart` is `cortex --home <home> run <instance>/<source>
  --record-failure` (`:492`) and a timer with `OnCalendar` (`:501`), then `systemctl daemon-reload`
  and `enable --now` (`:506-507`).
- `epic:standalone-1-0` put "a hosted runner other than systemd" out of its scope
  (`.engineering/planning/epic/standalone-1-0.md:59-60`).
- `cortex run <instance>/<source>` is already a one-shot command an external scheduler can call:
  the gap is its exit statuses, not the command. Every writing command waits for the home lock
  (`src/home.rs:42-49`, `Home::lock`; `src/main.rs:264`); only `restore` uses the non-waiting
  `Home::try_lock` (`src/home.rs:54`, `src/main.rs:263`). So a run started while another holds the
  lock blocks instead of answering "lock held".
- Exit statuses are 0 for an outcome with `ok`, 1 for any other declared outcome and 2 for an error
  (`src/main.rs:222-236`). The `disabled` outcome of `run` (`src/main.rs:1787`) exits 1, as
  `fetch-failed` (`:1763`) does, so a scheduler cannot tell "source disabled" from a failure.
- With `--record-failure`, the failure that disables a source also stops its systemd timer
  (`src/main.rs:1826`, `set_source_timer(.., false)`); without systemd that call fails and is only
  reported in the outcome detail (`units.failed`). Inferred from the code, not run in a container.

## Work

- Spec first: the new outcomes of `RunSource` (lock held; and, if the exit-status mapping is
  modelled, its statuses) and any new flag (for example a non-waiting lock for `run`) go into
  `spec/domains/instance.yaml` first, validated with `ess specify validate` and the conformance
  suite, then `task generate`; the clap test in `src/main.rs` holds the command line to the spec.
- `cortex run` gains a mode in which it does not wait for the home lock and answers a declared
  "lock held" outcome with a distinct exit status; a run of a disabled source answers its own
  distinct exit status.
- Recording a failure without systemd: disabling a source must not depend on `systemctl` when the
  home has no units (`cortex create --no-units`); a disabled source is then refused by `run`
  itself (`src/ports.rs:319`).
- Document the exit statuses on the operating page and in `website/docs/limits.md`, and remove the
  "systemd only" limit for scheduled runs.

## Acceptance

- With another command holding the home lock, `cortex run <instance>/<source>` in the new mode exits
  with the documented "lock held" status at once and the source runs counter stays unchanged.
- A run of a disabled source exits with the documented "disabled" status, distinct from the status
  of `fetch-failed`.
- On a home created with `--no-units` and no `systemctl` on `PATH`, the second recorded failure in a
  row moves the source from `Enabled` to `Disabled` with no `units.failed` in the outcome.

## Not in this story

A container image (`story:container-image`); reporting a failure to a metric or alert hook
(`story:run-failures-observable`); where the home lives in a container
(`story:checkpoints-volume-contract`); writing the CronJob manifests of any deployment.
