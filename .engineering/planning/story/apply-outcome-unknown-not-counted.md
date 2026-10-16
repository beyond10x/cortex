---
format: aep.planning-md/3
id: story:apply-outcome-unknown-not-counted
kind: story
status: active
title: An unknown outcome of a store write fails a run without counting against the source
summary: An apply-extraction (or other store write) whose outcome is unknown fails the run as store-outcome-unknown, uncounted, and the next run loses no batch
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T13:49:47Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-16T13:49:47Z", actor: "human:timo", revision: 4}
---
## Outcome

A source run whose `ekr apply-extraction` (or any other store write of the run outside the stage
verbs) answers that its outcome is unknown ("publication outcome is unknown") fails as
`store-outcome-unknown`, exit status 4, as an unknown stage verb does since 0.5.1: it is not
recorded as a failure of the source and never disables it. What the run leaves is settled so that
the next run loses no batch (a batch that did land may be written a second time, as a re-extraction writes it):

- In a stage (a gated run on a `postgres` store): the stage is abandoned by the next run, which
  fetches the run's documents again (`state/` put back, as for a run killed in its stage).
- Outside a stage (an ungated `postgres` store, or `sqlite`): the run's documents are not marked
  seen past the batch whose apply was unknown, so the next run extracts that batch again: no
  batch is lost. A batch that did land under the fault is written a second time, a second set of
  assertions and evidence, exactly as any re-extraction writes one (`cortex adopt`'s help,
  `website/docs/operating.md` "Adopting an existing store"); EKR has no content-hash idempotence
  for `apply-extraction`, and cortex does not look the batch up in the store.

## Why

The adversary review of `story:store-outcome-unknown-not-counted` found the scope edge: EKR returns
the same fault (`StoreError::UnknownCommit`) from a commit outside the stage verbs, and cortex
still counts it as `apply-refused` there. The story's reason holds there too: nothing about the
source was wrong, and a second such store fault would switch off a healthy source.

Verified in the wave of 0.5.1:

- `src/ekr.rs` `Store::apply` maps every failure of `apply-extraction` to an error string, which
  the run turns into `Failure::Apply`, so `--record-failure` counts it.
- `ekr::outcome_unknown` (added in 0.5.1) tells the fault by its message; it is applied to the
  stage verbs only.

## Domain relations

No new noun: the `store-outcome-unknown` outcome of `cortex.instance.RunSource` covers this case;
its `external` and summary grow to name store writes besides the stage verbs.

## Work

- Spec first: widen the outcome's `external` and summary; validate, conformance suite,
  `task generate`.
- `src/run.rs`: an unknown outcome from `apply-extraction` (and any other store write of a run)
  fails the run as `Failure::StoreOutcomeUnknown`; inside a stage the stage is left for the next
  run's recover; outside, the seen state is not advanced past the unknown batch.
- Docs: the operating page's exit statuses and "Undoing a run".

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- A stand-in `ekr` whose `apply-extraction` answers "publication outcome is unknown" after
  committing, on an ungated `postgres` store: two runs with `--record-failure` exit 4 as
  `store-outcome-unknown`; `consecutive_failures` stays 0; the third run (no fault) extracts the
  batch again and marks it seen, so no batch is lost; the store holds one set of the batch's
  assertions and evidence per run whose apply landed (two after the faulted runs, a further one
  after the third), its nodes once, as re-extraction writes.
- The same inside a stage (gated `postgres` run): the run exits 4; the next run abandons the stage,
  puts `state/` back and applies the batch once.
- The same before committing: the next run applies the batch once.
- Every other `apply-extraction` refusal is still `apply-refused` and counted (existing tests
  unchanged and green).

## Not in this story

- Resolving the retained occurrence inside EKR.
- An EKR exit status or code for the fault; the message stays the signal until EKR gives one.
