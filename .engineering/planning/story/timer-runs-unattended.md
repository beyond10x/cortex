---
format: aep.planning-md/3
id: story:timer-runs-unattended
kind: story
status: draft
title: Scheduled runs work with nobody at the keyboard
relations:
- decomposes: epic:first-web-instance
- serves: vision:self-updating-instances
- depends_on: story:ci-runs-task-check
- depends_on: story:seen-documents-modelled
- depends_on: story:store-backend-per-instance
- depends_on: story:redaction-names-and-gate
- depends_on: story:connectors-source-walks
scope:
- confidence: cited
  path: README.md
- confidence: inferred
  path: src/home.rs
- confidence: inferred
  path: src/ports.rs
- confidence: inferred
  path: src/run.rs
- confidence: cited
  path: src/schedule.rs
- confidence: inferred
  path: src/state.rs
revision: 9
---
## Outcome

The systemd user timers `cortex create` installs, one per source (`README.md:40`), run each source on
its schedule with nobody at the keyboard.

## Work

- Find why the smoke instance has no timer listed (`systemctl --user list-timers`, 2026-10-05), then
  create an instance whose units `src/schedule.rs` installs on this machine.
- A timer runs `cortex run` under the user session with no TTY: check that `connectors` reaches the
  Secret Service keyring and `claude -p` its OAuth credentials from a systemd unit, and fix the unit
  environment where not.
- Observe 24 h of runs with no manual step.

## Acceptance

Over one 24 h window with no manual step, every run of the instance's timers appears in
`journalctl --user` with exit status 0, and `cortex sources` shows each source's `runs` equal to its
number of journal entries in that window. The journal excerpt and the `cortex sources` output are
recorded as evidence.

Failure handling (`RecordFailure`, `consecutive_failures`) is not judged here: the conformance
suite's `RecordFailure` scenarios cover it. Note found by review: `src/ports.rs:175` disables a
source when it fails with one failure already counted, i.e. on its second consecutive failure.

## Depends on

`story:ci-runs-task-check` (fixes this story forces land behind the gate),
`story:seen-documents-modelled` (it changes the run-state files this story observes),
`story:store-backend-per-instance` (both edit `src/schedule.rs`), `story:redaction-names-and-gate`
(a fix this story forces in `src/run.rs` lands after it).

## Scope

`src/schedule.rs`, `README.md` (operating section). A fix the observation forces may touch
`src/run.rs:128-232`, `src/ports.rs:175,260`, `src/state.rs` and `src/home.rs:172`; the last two
are also `story:seen-documents-modelled`'s, which lands first.
