---
format: aep.planning-md/3
id: architecture-decision-record:research-claim-model
kind: architecture-decision-record
status: proposed
title: A research run's claims are cortex records, decided one by one and applied as assertions in a gated stage
summary: 'Proposed: claims in research/<stamp>/claims.jsonl with cited evidence and the shared review lifecycle, until EKR offers a claim noun'
relations:
- decides: story:research-run
revision: 1
---
## Context

`story:research-run` has cortex answer an open question with cited claims a person reviews, not
with assertions written at once. Where a claim lives until it is decided, what it carries, and how
an approved one becomes an assertion decide what the spec models and whether the story can be built
before EKR changes. It is decided here once, before any implementation.

What exists, verified:

- The accepted `architecture-decision-record:answer-service-in-cortex` lists "Claim submission and
  decision in the store (an EKR request)" under what stays outside cortex, and says research runs
  depend on it.
- EKR has no claim noun at the version this tree pins (`.github/workflows/check.yml:21`, EKR
  0.0.34): its evidence kinds are `Url`, `Document`, `DatabaseRecord`, `GraphAssertion`,
  `Observation` and `HumanStatement` (EKR `crates/ekr-graph/src/evidence.rs:18-31`), and no planning
  artifact of the EKR checkout names claim submission.
- EKR has stages: a run's writes are held in a stage and published or abandoned whole (EKR
  `crates/ekr-kernel/src/stage.rs:1-13`); cortex uses one for a gated source run on `postgres`
  (`src/run.rs:751-754`).
- cortex already records a model's proposals as files beside the store and decides them itself:
  `schema/<stamp>/proposals.jsonl`, one `cortex.instance.SchemaProposal` per line
  (`spec/domains/instance.yaml:718-730`, `src/schema.rs:923`).
- An extraction admits only `!HumanStatement` evidence (`src/evidence.rs:24-25`).

## Options considered

| option | what it costs | why not (or why) |
|---|---|---|
| A. Wait for claims in EKR: cortex submits claims, EKR holds and decides them | nothing in cortex beyond submission; an EKR noun, its lifecycle and a decision verb upstream | the story waits on a request with no artifact upstream; the review happens in EKR's tools, apart from the schema and derivation reviews cortex will have |
| B. Claims as assertions written into an EKR stage that is published only after review | no new store noun; a stage held open for as long as a review takes | a stage is published or abandoned whole, so claims cannot be decided one by one; a long-open stage holds a tenant and blocks nothing else from drifting past its base |
| C. Claims as assertions applied at once with a marker (a property or low confidence) and retracted when rejected | no review path at all | the store holds unreviewed claims as facts, which is what the request rules out |
| D. (proposed) Claims are cortex's own records, `research/<stamp>/claims.jsonl`, decided one by one, and the approved ones applied as assertions inside the gated stage of `story:schema-proposals-on-approval` | a second record kind beside schema proposals and derived assertions, sharing one review lifecycle; cortex owns claim text and evidence until applied | buildable on the EKR this tree pins; one review path for every delta cortex proposes; moves to A later without changing the reviewer's view |

## Decision (proposed)

1. **A claim is a cortex record until decided**: one line of `research/<stamp>/claims.jsonl`,
   holding the claim as an assertion would state it (subject node, edge type or property, object
   node or value), its evidence, the question and the run, and a status in the review lifecycle
   shared with schema proposals and derived assertions (`pending`, `approved`, `rejected`,
   `applied`, `refused`, `invalid`).
2. **Evidence is what the run read**: a document of a configured source (source, key, passage) or
   an assertion of the store (its id). A claim citing anything else is dropped before it is
   recorded.
3. **An approved claim is applied as an assertion inside the gated stage**, carrying its document
   evidence as an extraction does (`!HumanStatement` naming the document, `src/evidence.rs`) and
   its assertion evidence as `GraphAssertion`.
4. **This narrows a line of the accepted answer-service record**: claims are held in cortex until
   EKR offers a claim noun. When it does, a later record may move claims to EKR (option A); the
   reviewer's commands stay the same.

## Consequences

- cortex owns claim records: questions, passages and claims may carry personal data, so their
  retention and redaction follow the instance's redaction policy as a run's documents do.
- One review lifecycle serves three kinds of delta; the three stories share its commands' shape.
- The line "Claim submission and decision in the store (an EKR request)" of
  `architecture-decision-record:answer-service-in-cortex` no longer blocks research runs if this
  record is accepted; the operator decides that together with this record.
- No implementation of `story:research-run` starts until the operator accepts this record.

## What ESS must model

In `spec/domains/instance.yaml`:

- entity `cortex.instance.ResearchRun` (identity: the stamp; `instance_name`, `question`,
  `cost_usd`, `model`, `outcome`), owned by `cortex.instance.Instance` via `instance_name`.
- entity `cortex.instance.ResearchClaim` (identity: the run's stamp and index; the stated
  assertion, `evidence`, `status`, `decision_reason`), owned by `ResearchRun`, with the review
  lifecycle shared with `cortex.instance.SchemaProposal`.
- union `cortex.instance.ClaimEvidence` (`document`: source, key, passage; `assertion`: id).
- commands `Research`, `ApproveClaims`, `RejectClaims`, `ApplyClaims` for
  `cortex.instance.Operator`, with `budget-exhausted` among `Research`'s outcomes.

## Status

Proposed, not accepted. It decides the claim model of `story:research-run`.
