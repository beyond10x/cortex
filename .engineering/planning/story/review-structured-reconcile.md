---
format: aep.planning-md/3
id: story:review-structured-reconcile
kind: story
status: implemented
title: A structured source retracts only on evidence and retries what EKR did not apply
summary: Findings 1, 2, 3, 5 and 6 of the 0.7.2 review
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T01:53:59Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T01:53:59Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-24T00:21:07Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

A structured source retracts a record's facts only on evidence that the record is gone, and a
record EKR did not apply is retried, never counted as applied.

## Work

Findings 1, 2, 3, 5 and 6 of the external review of cortex 0.7.2 (2026-11-23):

- 1: with `{since}` and `dropped: Supersede`, an incremental window that returns only the changed
  record retracts the unchanged record's facts (`src/sources.rs`, `src/run.rs`, `src/structured.rs`).
  Retract a missing identity only after an authoritative full listing or an explicit deletion.
- 2: a provider that repeats a pagination token after a partial page makes fetch-only report no
  unread pages and the run retract the unseen record. A pagination cycle is incomplete and keeps
  the assertions of unseen records.
- 3: a record that returns unchanged after its retirement is called unchanged and its facts stay
  retracted (`src/run.rs`, `src/state.rs`). Retirement invalidates the saved fingerprint, or presence
  is tracked apart from it.
- 5: an EKR row reported ambiguous (two existing nodes match the record's aliases) is counted as
  applied and marked seen (`src/structured.rs`, `src/ekr.rs`). Read EKR's per-record ambiguity
  before advancing checkpoints or reporting success.
- 6: with a one-record run limit, a rejected first record is selected on every run and blocks a
  valid record behind it (`src/state.rs`, `src/run.rs`). The rejected record keeps its retry; other
  eligible identities progress.

## Acceptance

An end-to-end test per finding, against a real `ekr` 0.0.34, reproduces the reviewed scenario and
fails on 0.7.2's code and passes on this change; the documentation in `website/docs/spec-file.md`
describes the new retraction and retry rules.
