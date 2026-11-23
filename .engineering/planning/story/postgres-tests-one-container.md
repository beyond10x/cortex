---
format: aep.planning-md/3
id: story:postgres-tests-one-container
kind: story
status: active
title: PostgreSQL tests share one server
summary: The test step ends within 120 s and the budget is enforced
relations:
- serves: vision:self-updating-instances
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T21:16:52Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T21:16:52Z", actor: "human:timo", revision: 3}
---
## Outcome

The test step of `task check` ends within 120 s on CI and is enforced: `TEST_BUDGET` is `enforce`.

## Work

The step ran 124 s on CI at #95's head 8caf59e7 (1414 passed, 11 skipped). The 35 PostgreSQL tests
held 168 of 946 test-seconds; nine of the ten slowest tests are PostgreSQL tests (11.8-28.3 s each),
each paying for a PostgreSQL server of its own.

- Start one PostgreSQL server per test run, shared by every PostgreSQL test (a setup step before the
  tests, or a server keyed by a digest under `target/`; an in-process global does not count under
  nextest, which runs each test in its own process). Each test takes its own database or schema so
  tests stay independent.
- Switch `TEST_BUDGET` to `enforce`.

## Acceptance

On CI the test step prints a wall time under 120 s with `TEST_BUDGET=enforce`; the PostgreSQL tests
start no server of their own (a count of server starts per run is 1); no test runs over 60 s.
