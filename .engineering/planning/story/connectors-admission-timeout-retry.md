---
format: aep.planning-md/3
id: story:connectors-admission-timeout-retry
kind: story
status: active
title: A run retries a Connectors invocation that timed out at admission once
relations:
- decomposes: epic:first-web-instance
- serves: vision:self-updating-instances
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T13:07:56Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-08T13:07:56Z", actor: "human:timo", revision: 3}
---
## Outcome

A scheduled run whose Connectors invocation answers `timeout` at `admission` retries that invocation once before it answers `fetch-failed`.

## Why

The `agent-tooling` news run of 2026-10-08 00:30 CEST exited 1 with `tavily.websearch.search: timeout at admission` after 82 s (journalctl). Run again by hand at 2026-10-08 ~13:30 UTC, the same revalidate, describe and invoke sequence succeeded (revalidate 10.3 s, describe 0.3 s, invoke 2.2 s). An invocation refused at admission was not sent to the provider, so one retry repeats no provider call. cortex today maps every refusal other than `not_granted` and `not_found` at admission to `InvokeError::Failed` and does not retry it (`src/connectors.rs`, `envelope`).

## Work

- The `fetch-failed` outcome of `RunSource` in `spec/domains/instance.yaml` says a timeout at admission is retried once; regenerate.
- `src/connectors.rs`: `envelope` classifies `timeout` at `admission`; the invocation retries it once (fresh `operations describe`, same input); a second timeout answers `fetch-failed` with the same reason as today.
- A test with a stand-in `connectors` that times out at admission once, then answers: the run applies; and one that times out twice: `fetch-failed`, one retry made.

## Acceptance

With a stand-in `connectors` answering `{"code":"timeout","stage":"admission"}` on the first `operations invoke` and a result on the second, `cortex run` exits 0 and the stand-in records exactly two invocations; with two timeouts it exits 1 with `fetch-failed` and the stand-in records two invocations.

## Scope

- `src/connectors.rs` (inferred)
- `spec/domains/instance.yaml`, the `fetch-failed` outcome of `RunSource` (inferred)
- `tests/` a new test file (inferred)
