---
format: aep.planning-md/3
id: story:timer-runs-unattended
kind: story
status: implemented
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
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T15:42:25Z", actor: "agent:claude", revision: 10, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-10-05T15:42:25Z", actor: "agent:claude", revision: 11, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-10-05T16:11:41Z", actor: "agent:claude", revision: 13, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
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

Landed 2026-10-05 in `fb905ce` (wave 20261005g, merged `ba07060`).

- **Files:** `src/schedule.rs` (CORTEX_CONNECTORS, CORTEX_CLAUDE, CORTEX_CODEX and HOME in source units), `src/home.rs` (an absolute home), `src/spec.rs` (a bare spec file name keeps `.` as its base), `src/main.rs` (two call sites), `README.md`, `website/docs/operating.md`, `tests/unattended.rs` (new, 4 cases)
- **Inferred lines, not needed:** `src/ports.rs`, `src/run.rs`, `src/state.rs`; failure handling worked unchanged under a real timer
- **Acceptance, split by the coordinator:** a 15-minute window of real systemd user timers on the operator machine with a stand-in model: 15 runs, all exit 0, `cortex sources` `runs: 15`, and the failure path counting then disabling (`~/.cache/cortex-wave-20261005g/timer/live/evidence/`). The 24 h window on a real instance is recorded by `story:first-web-instance`.
- **Left open:** the viewer unit exits 143 on stop, so `cortex remove` leaves it listed as failed (`SuccessExitStatus=143` would fix it); `%` and `$` in a home path or PATH are not escaped in unit files
- **Review:** coordinator review of the diff; no adversary pass
