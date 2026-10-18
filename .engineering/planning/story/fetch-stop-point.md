---
format: aep.planning-md/3
id: story:fetch-stop-point
kind: story
status: active
title: A run sent SIGTERM while it fetches stops within a bounded time
summary: 'the fetch is a safe point: its child ended on SIGTERM, run ends terminated'
relations:
- serves: vision:self-updating-instances
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-18T00:28:45Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-18T00:28:45Z", actor: "human:timo", revision: 3}
---
## Outcome

A run sent SIGTERM while it fetches stops within a bounded time: the `connectors` fetch (or a web
or files read) is ended, nothing is marked seen, the window does not move, and the run ends
`terminated`, exit 8, not counted by `--record-failure`. A run sent SIGTERM while it waits before a
model call is asked again says so in its reason, instead of saying a model call in flight was
ended.

## Why

`story:run-metrics` made SIGTERM a stop at the run's safe points: before it uses what it fetched,
before each batch and while a model call runs. A fetch is not one of them, so a run sent SIGTERM
during a slow fetch waits for the whole fetch; the spec states it, and a Kubernetes Job whose grace
period is shorter than the fetch kills the run with nothing recorded. Verified by
`tests/runout_adversary.rs::a_run_sent_sigterm_during_a_slow_fetch_ends_within_a_bounded_time`
(ignored: the run took 20 s to end). The same adversary noted that a stop during the retry wait
reports "the call was ended" although no call was in flight.

## Domain relations

- No new noun. The `terminated` paragraph of `RunSource` names the fetch as a safe point and the
  reason's wording for each point.

## Work

- Spec first: the paragraph; validate, `task generate`, `task drift`.
- Code: the fetch's child process (`src/sources.rs`, `src/connectors.rs`) is waited for as a model
  call is (`extract::wait_or_terminate`): on SIGTERM its process group is ended, SIGKILL after the
  grace. A paged fetch checks the flag between pages.
- The retry wait in `extract::Model::ask` gives its own reason.

## Acceptance

- `ess specify validate` passes and `task drift` is clean.
- The ignored case above runs and passes: the run ends within 10 s of SIGTERM, `terminated`, exit
  8, the source's consecutive failures unchanged, and the window's `since` unchanged.
- A run sent SIGTERM in the retry wait reports a reason naming the wait, not a call in flight.

## Not in this story

- Keeping the pages a stopped fetch already read.
