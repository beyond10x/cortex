---
format: aep.planning-md/3
id: story:prompt-names-value-kinds
kind: story
status: draft
title: The extraction prompt names each property's value kind
relations:
- decomposes: epic:first-web-instance
- serves: vision:self-updating-instances
scope:
- confidence: cited
  path: src/extract.rs
revision: 2
---
## Outcome

The extraction prompt names each existing property with its value kind, so the model answers a
property in the kind the store declares and EKR rejects fewer facts as
`extraction-value-mismatch`.

## Why

On 2026-10-07 the `agent-tooling` instance's news run stopped after 7 of 15 new documents: the
model answered `Release.release_date` as `value_kind: Timestamp` (`1789689600000`), the seed schema
declares it `String`, and `ekr apply-extraction` 0.0.30 refused the whole batch
(`extraction-value-mismatch: the property's type does not hold the value: facts[3]:
Release.release_date`).

Reproduced on a fresh store from copies of the run's seed, seed schema and both batches (`batch-0`, then `batch-1`):

| `ekr` | `batch-1` |
|---|---|
| 0.0.30 | exit 2, the error above; nothing of the batch applied |
| 0.0.32 (the pin on `main`) | exit 0; `rejected` holds 1 part, `facts[3]` `extraction-value-mismatch`; the rest committed |

So on `main` the run no longer stops: a rejected part is counted in `parts_rejected` and the run
goes on (`src/run.rs`, `tests/evidence_bound.rs`). The fact itself is still lost. The value comes
from the model, and EKR's check is right. The prompt lists properties by name alone:
`Existing node types: … Release(version, release_date); …` (`src/extract.rs`, `prompt`).

## Work

- `extract::prompt` writes each property with its value kind, for example
  `Release(version: String, release_date: String)`, from the ontology (`ekr ontology`).
- A test with a stand-in `claude` that asserts the kinds appear in the prompt it was given.

## Acceptance

- The prompt of a run on an instance whose ontology declares `Release.release_date` as `String`
  contains `release_date: String`.
- A run whose model answers one property in another kind applies the batch's other facts, counts
  `parts_rejected` 1 and exits 0 (EKR 0.0.32).

## Not modelled

The prompt text is not part of cortex's ESS specification; `RunSource`'s outcome already carries
`parts_rejected`.
