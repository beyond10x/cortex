---
format: aep.planning-md/3
id: story:review-instance-update
kind: story
status: implemented
title: Update, sources add and remove keep registry, scheduler and spec in agreement
summary: Findings 12 to 16 of the 0.7.2 review
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T01:54:00Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T01:54:00Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-24T00:21:05Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

`cortex update`, `cortex sources add` and `cortex remove` leave the registry, the scheduler and the
frozen spec in agreement, and report every step that did not complete.

## Work

Findings 12 to 16 of the external review of cortex 0.7.2 (2026-11-23):

- 12: a replacement spec without a source leaves that source Enabled in the registry and its unit
  files on disk (`src/main.rs`, `src/run.rs`). Reconcile removed sources during update.
- 13: changing a source's schedule rewrites its timer but `cortex sources` prints the old schedule.
  Refresh the source's metadata on update and keep its run and failure counters.
- 14: changing `serve.view_port` reports success, but the list and the viewer unit keep the old
  port. Apply the new port or refuse it.
- 15: repeating `sources add` for an existing identity resets its counters and re-enables a
  disabled source; a source whose parent instance does not exist is accepted (`src/main.rs`,
  `generated/cortex-model`, `src/ports.rs`, `spec/domains/instance.yaml`). Validate the parent and
  keep existing state.
- 16: `remove` returns `removed` with exit 0 after `systemctl disable --now` failed, and a second
  remove is refused as wrong-state (`src/schedule.rs`, `src/main.rs`). Report the incomplete stop
  and keep cleanup retryable.

## Acceptance

A test per finding (stand-in `systemctl`) reproduces the reviewed scenario, fails on 0.7.2's code
and passes on this change; spec changes pass `task compat` with reviewed acknowledgements.
