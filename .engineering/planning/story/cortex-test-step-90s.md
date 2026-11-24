---
format: aep.planning-md/3
id: story:cortex-test-step-90s
kind: story
status: active
title: Test step under 90 s
summary: The CI test step ends under 90 s on every run
relations:
- serves: vision:self-updating-instances
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T00:17:13Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T00:17:13Z", actor: "human:timo", revision: 3}
---
## Outcome

Cortex's CI test step ends under 90 s on every run, leaving 30 s of the 120 s budget for the next
features.

## Work

CI test step on main varies 78-117 s over five runs and measured 111-113 s on PR #117 (1429 tests).
Known costs from the CI report of PR #113: the first PostgreSQL tests wait on the shared server's
start (23.8 s and 18.0 s), `w30b_ask_adversary` drain tests take about 10 s each,
`runout_adversary::a_second_sigterm_exits_143_at_once_and_records_nothing` 9.3 s, 24 tests over 5 s.

- Measure first: per-test times from two CI runs of main (the nextest JUnit artifact or the log),
  summed per module, and the critical path at the step's thread count.
- Start the shared PostgreSQL server before the tests start (a setup step of the test task) so no
  test waits on it.
- Cut fixed sleeps and wall-clock waits to event waits; reuse one built fixture per run where a test
  rebuilds the same instance.
- Lower the budget to the measured need: keep `TEST_BUDGET=enforce` at 120 s and add a printed
  warning line over 90 s.

## Acceptance

Two CI runs of the PR print a test-step wall time under 90 s; no test over 60 s; the ten slowest tests
are printed; no test was deleted or weakened (each changed test names in its commit body what it
still checks).
