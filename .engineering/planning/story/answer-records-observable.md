---
format: aep.planning-md/3
id: story:answer-records-observable
kind: story
status: draft
title: Answer records and closed reviews are observable to the conformance suite
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
revision: 1
---
## Outcome

The conformance suite kills the mutants that ESS hardening pass 1 found surviving on the ask
domain: a stored answer record and a closed review's expectations are observable through a view
or an event field, and the review queue's order is pinned.

## Why

`review-result:hardening-w30a-pass-1` (mutation audit, 165 mutants): 8 RecordAnswer field
mix-ups survive and 2 are unwitnessed because no view or event exposes stored answer records
(`spec/domains/ask.yaml:741`); CloseReview's `expect_facts` and `expect_nodes` can be swapped
undetected although AddEvalCase relies on them (`ask.yaml:916`); the review queue's order is
unpinned because every review in a scenario carries the same time; a vote change is not
observable.

## Work

1. Declare a view (or event fields) over answer records and closed reviews that the
   implementation already answers, or the smallest new read the CLI can serve.
2. A clock seam in `src/ports.rs` so scenarios can give reviews distinct times.
3. Re-run `ess verify conform mutate --emit` with `CORTEX_CONFORMANCE_SUITE` and
   `CORTEX_CONFORMANCE_REPORT` (tests/conformance.rs) and `--collect`.

## Acceptance

- The 10 RecordAnswer mutants and the CloseReview swap are killed in a re-run audit.
- `task compat` acknowledges the added view or fields.
