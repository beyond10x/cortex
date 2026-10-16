---
format: aep.planning-md/3
id: story:connections-revalidate-retry
kind: story
status: draft
title: A transient revalidation failure is retried, and one that remains before any fetch is not counted against the source
refs:
- provider: github
  reference: beyond10x/connectors#174
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
revision: 1
---
## Outcome

A transient Connectors failure of `connections revalidate` is retried within a fixed bound, and
one that remains before the run's first fetch fails the run as `connection-unavailable`, which is
not counted against the source and never disables it:

1. Retry: when `connections revalidate` (or the `connections list` read before it) answers an
   operational failure that Connectors marks retryable (`error.data.kind: operational` with
   `next_action: retry_explicitly`, such as `code: unavailable` at `stage: dispatch`), cortex runs
   it again, at most **4 attempts in all**, pausing **5 s, 10 s, then 20 s** between them (doubling
   from 5 s; 35 s of pauses at most). `CORTEX_REVALIDATE_ATTEMPTS` (default 4, at least 1) and
   `CORTEX_REVALIDATE_PAUSE_MS` (default 5000, the first pause) override the bound, as
   `CORTEX_STATUS_POLLS` does for the status reads. A refusal (a missing, revoked or not-granted
   connection, an authentication or scope failure) is not retried.
2. Not counted: a run whose revalidation still fails after the last attempt, before any document
   of the run was fetched, ends with outcome `connection-unavailable`, exit status 5, its reason
   naming the last error. No failure is recorded (`--record-failure` counts nothing), the source is
   never disabled by it, and its status, metric and hook report it as for `store-outcome-unknown`.
   The next scheduled run starts as usual.
3. A revalidation that fails after the run fetched documents (a lapse renewed mid-run) is retried
   the same way and, if it still fails, fails the run as today.

## Why

A downstream deployment running cortex from containers saw `connections revalidate` answer
`{"code":"unavailable","kind":"operational","next_action":"retry_explicitly","stage":"dispatch"}`
on 2 of 5 runs in one day, each within a minute of the container's start; each run counted as a
failure, and the source was disabled. Connectors tracks the cause and asks callers to retry
(beyond10x/connectors#174); nothing about the source was wrong.

Verified in the code:

- `src/connectors.rs` `Connectors::revalidate` runs `connections revalidate` once; only an
  unconfirmed outcome (`next_action: retry_status`) is followed up (`await_ready`).
- `ensure_ready` runs before every invocation; its error becomes the run's failure, which
  `--record-failure` counts (`src/main.rs` `record_failure`).
- `store-outcome-unknown` (exit 4) is the existing pattern of an outcome that is reported and not
  counted.

## Domain relations

No new noun: a new `cortex.instance.RunSourceOutcome` variant `ConnectionUnavailable` with an error
type carrying `reason`, beside `StoreOutcomeUnknown`.

## Work

- Spec first: the `ConnectionUnavailable` outcome and its error type in
  `spec/domains/instance.yaml`, its comment stating the bound, the backoff and that it is not
  counted; validate, conformance suite, `task generate`, `task drift`.
- `src/connectors.rs`: the bounded retry around `revalidate`; tell retryable from refusal by the
  envelope's `kind` and `next_action`.
- `src/run.rs`, `src/main.rs`: the outcome before the first fetch, exit status 5, no
  `RecordFailure`; status, metric and hook as for `store-outcome-unknown`.
- Tests first, with the stand-in `connectors` the existing tests use: `tests/connections_revalidate_retry.rs`.
- Docs: `website/docs/operating.md` "Failures" (exit statuses), CHANGELOG Fixed entry naming
  connectors#174's symptom.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- A stand-in `connectors` whose revalidate answers the retryable failure twice and then succeeds:
  the run succeeds, and revalidate was invoked 3 times.
- One that answers it on every call: revalidate is invoked 4 times; the run ends
  `connection-unavailable`, exit status 5; with `--record-failure`, the source's failure count is
  unchanged and two such runs in a row leave the source enabled.
- One that answers a refusal (`not_granted`, or an authentication failure): revalidate is invoked
  once and the run fails and is counted as today.
- The pauses follow `CORTEX_REVALIDATE_PAUSE_MS` (tests set it to a few milliseconds) and double.

## Not in this story

- The cause of the failure inside Connectors (beyond10x/connectors#174).
- Retrying any other Connectors operation.
