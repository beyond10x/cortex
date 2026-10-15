---
format: aep.planning-md/3
id: story:research-run
kind: story
status: draft
title: A research run answers an open question with cited claims submitted for review
summary: cortex research --question writes claims, not assertions; waits for claim submission in EKR
relations:
- decomposes: epic:instance-answers-questions
- serves: vision:self-updating-instances
revision: 1
---
## Outcome

`cortex research <instance> --question <text>` answers an open question by reading the instance's
sources and the store, and writes what it finds as cited claims submitted for review, not as
assertions. Each claim carries its evidence and the run that produced it; a reviewer decides it in
the store, and only an approved claim becomes an assertion.

## Why

A downstream deployment wants research runs for its personal instances and has only an empty
design branch. Verified in this tree: cortex writes assertions only by applying an extraction or a
structured import, at once (`src/run.rs`, `store.apply`; `src/structured.rs`); there is no pending
claim. The store-side noun, a claim submitted and decided in EKR, is an EKR request, not yet in the
EKR this tree pins (inferred from the request; EKR not checked here). This story waits for it.

## Domain relations

- entity `cortex.instance.ResearchRun` (identity: the stamp): `instance_name`, `question`,
  `claims: List<String>` (EKR claim ids), `cost_usd`, `outcome`. Relation:
  `cortex.instance.Instance` owns many via `instance_name`.
- The claim itself is EKR's noun; cortex holds only its id.

## Work

- Spec first: the entity and the command `Research` with its outcomes; validate, conformance, `task
  generate`. Then the code.
- Confirm EKR's claim submission is released and pinned; if not, file an upstream blocker against
  this story before any code.
- `src/research.rs`: plan reads over the sources, extract claims with evidence, submit them; held
  to `budget_usd` and the daily ceiling as a run is.
- Docs: `commands.md`.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- With a stand-in EKR that accepts claims and a stand-in model answering one claim with evidence,
  a research run submits one claim, the store's assertion count is unchanged, and the run's record
  names the claim id.
- A run whose spend reaches `budget_usd` stops before its next model call and says so.

## Not in this story

- Deciding claims: EKR's.
- Personal instances on a shared schema (`story:instance-schema-from-another`).
