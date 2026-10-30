---
format: aep.planning-md/3
id: story:spec-matches-design
kind: story
status: draft
title: The specification says what the design pages and the code say
summary: Resolve the 32 pre-existing and undecided findings of the 2026-10-30 design review of spec/
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
revision: 1
---
## Outcome

`spec/` says what the design pages and the code say. The pre-existing findings of
`review-result:design-review-w30a-spec-pass-1` (31 pre-existing, 1 undecided) are each resolved:
the spec changed, the page changed, or an `UNMAPPED:` marker names the rule the spec cannot hold.

## Acceptance

- Each of the 9 `contradicts` findings is gone. Where the code shows which side is right, that side wins:
  - the eval `ask` runner always answers `runner-failed`: `src/eval.rs:429-433`
  - the answer record is kept before the last event: `src/ask/serve.rs:310`
  - the Operator may run `RunSource` and `RecordFailure`: `README.md:82-85`, `commands.md:275`
- `cortex.ask.Ask` declares the `bad-request` refusals of `ask-service.md:112`.
- The external texts of `apply-refused`, `seed-refused`, `seed-change-refused` and `extraction-failed`
  name every refusal that answers with them.
- `SourceRan`, `InstanceAdopted` and `SnapshotRestored` carry the report fields the commands print,
  or a marker says why not.
- The `undecided` finding ("exceeds the run budget before any batch", `instance.yaml:1896`) is
  answered: the code at `src/run.rs:1088-1107` is read and the external text says what it covers.
- `task drift`, `task docs-check` and `tests/conformance.rs` pass.

## Findings it closes

Every finding in the fenced block of `review-result:design-review-w30a-spec-pass-1` whose `origin`
is `pre-existing` or `undecided`. The 2 `introduced` findings (`facts_reapplied` on `SourceRan`;
the record routes' 413 and 500 on `CastVote` and `CloseReview`) are fixed in wave 20261030a's
`ess` unit.
