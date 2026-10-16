---
format: aep.planning-md/3
id: story:research-run
kind: story
status: draft
title: A research run writes cited claims for review, applied as assertions only once approved
summary: cortex research --question records pending claims citing what the run read; approve, reject and apply through the shared review path
relations:
- decomposes: epic:instance-answers-questions
- serves: vision:self-updating-instances
- depends_on: story:schema-proposals-on-approval
- depends_on: architecture-decision-record:research-claim-model
revision: 3
---
## Outcome

`cortex research <name> --question <text>` is a run that answers an open question with cited
claims for review, not with assertions:

1. It reads the instance's store and the documents of its configured sources that bear on the
   question, and has the instance's model state what it finds as claims, each citing the
   evidence it rests on (a document and the passage, or store assertions by id). A claim with no
   evidence the run actually read is dropped, as `cortex schema` drops a proposal citing what its
   batch does not hold.
2. The run writes its claims `pending` with the question, the run's stamp, its cost and its
   model, and applies nothing: the store's assertion count is unchanged.
3. A person approves or rejects each claim; the approved ones are applied inside the gated stage
   `story:schema-proposals-on-approval` introduces, each as an assertion carrying the claim's
   evidence. A rejected claim stays recorded with its reason.
4. The run is held to the spec's `budget_usd` and the daily ceiling as a source run is.

No implementing wave runs for this story until
`architecture-decision-record:research-claim-model` is accepted; where claims live and how one
becomes an assertion follow that record's proposal and change with it.

## Why

A downstream deployment wants research runs that propose cited claims a person reviews. Verified
in this tree:

- cortex writes assertions only by applying an extraction or a structured import, at once
  (`src/run.rs:950`, `src/run.rs:1530`); there is no pending claim, and nothing in `src/` names a
  claim or a research run (no match for `research` or `claim` as a noun in `src/*.rs`).
- Its model calls take no tools and read what cortex hands them (`src/extract.rs:484`, the
  `-p --tools ""` command), so a research run reads its sources through cortex, not through the
  model.
- The accepted `architecture-decision-record:answer-service-in-cortex` puts "Claim submission and
  decision in the store (an EKR request)" outside cortex and says research runs depend on it.
  EKR has no claim noun at the version this tree pins: its evidence kinds are `Url`, `Document`,
  `DatabaseRecord`, `GraphAssertion`, `Observation` and `HumanStatement` (EKR
  `crates/ekr-graph/src/evidence.rs:18-31`), and no planning artifact of the EKR checkout names
  claim submission. `architecture-decision-record:research-claim-model` decides whether cortex
  holds claims pending review itself or waits.
- An extraction admits only `!HumanStatement` evidence (`src/evidence.rs:24-25`), so an approved
  claim's document evidence is named as a run's is today.

Order: queued after `story:repository-documents-source` (no dependency), after
`story:schema-proposals-on-approval` (`depends_on`, the review and apply path) and after
`story:derivation-run` (no dependency; both write a reviewable delta, and this one comes last).

## Domain relations

Spec first, in `spec/domains/instance.yaml`, as the record proposes:

- entity `cortex.instance.ResearchRun` (identity: the stamp): `instance_name`, `question`,
  `cost_usd`, `model`, `outcome`. Relation: `cortex.instance.Instance` owns many via
  `instance_name`.
- entity `cortex.instance.ResearchClaim` (identity: the run's stamp and index): the claim as an
  assertion would state it (subject, predicate or edge type, object or value), `evidence:
  List<cortex.instance.ClaimEvidence>`, `status`, lifecycle as `cortex.instance.SchemaProposal`'s
  from `story:schema-proposals-on-approval`. Relation: `ResearchRun` owns many `ResearchClaim` via
  `stamp`.
- union `cortex.instance.ClaimEvidence` (`document`: source, key, passage; `assertion`: an EKR
  assertion id).
- commands `Research`, `ApproveClaims`, `RejectClaims`, `ApplyClaims` for
  `cortex.instance.Operator` (`:817`), with `budget-exhausted` among `Research`'s outcomes.
- UNMAPPED: if the record is decided for EKR-held claims instead, `ResearchClaim` is EKR's noun and
  cortex keeps only its id.

## Work

- Wait for `architecture-decision-record:research-claim-model` to be accepted.
- Spec first: the entities, the evidence union, the commands and their outcomes; validate,
  conformance suite, `task generate`. Then the code.
- `src/research.rs`: choose what to read for the question (the store's snapshot and the sources'
  documents), batches to the model with the answer held to the generated JSON Schema, citation
  checks, the pending record, apply through the shared review and stage path; spend through
  `src/spend.rs`.
- Docs: `website/docs/commands.md`, the operating page.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- With a stand-in model answering one claim citing a document the run read and one citing a
  document it did not: one claim is recorded `pending` with its evidence, the other is dropped and
  counted, and the store's assertion count is unchanged.
- `approve` then `apply`: the store holds the claim as an assertion whose evidence names the
  document; on a `postgres` stand-in the apply ran in a stage and nothing is published when the gate
  fails.
- `reject` records the reason; `apply` leaves a rejected claim out.
- A run whose spend reaches `budget_usd` stops before its next model call with `budget-exhausted`,
  and the claims recorded so far stay `pending`.

## Not in this story

- Reading the open web or any source the instance's spec does not configure.
- Claim submission and decision inside EKR, unless the accepted record chooses it.
- Personal instances on a shared schema (`story:instance-schema-from-another`).
- Derivation from the store by rule (`story:derivation-run`).
