---
format: aep.planning-md/3
id: story:run-cost-split
kind: story
status: implemented
title: A run reports its model calls and the fixed prompt's tokens apart from the documents'
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-15T03:31:23Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-15T03:31:23Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-15T15:25:17Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

A run's report and its `cortex.log` line say how many model calls the run made and how its input
tokens split between the fixed part of each prompt (the system prompt, the instance description
and instructions, the ontology and the known entity names) and the documents, besides the one
`cost_usd` it reports today. An operator can then tell whether the fixed part dominates the cost
of a run that carries one to three documents.

## Why

A downstream consumer runs cortex at a 10-minute cadence, so most runs carry 1-3 documents, and the
fixed part of the prompt, repeated on every call, may dominate the daily cost. Today a run reports
a single `cost_usd`. Verified in this tree:

- `claude -p` runs with `--output-format json` (`src/extract.rs:281`), and of its JSON answer the
  code reads `total_cost_usd`, `is_error`, `subtype` and `structured_output` only
  (`src/extract.rs:312-337`); `Answer` carries the document and `cost_usd` (`:54-58`). The answer's
  `usage` object (`input_tokens`, `cache_creation_input_tokens`, `cache_read_input_tokens`,
  `output_tokens`) is not read. Its field names are to be confirmed against the pinned `claude`
  before the spec names them.
- `extract::prompt` builds one string: description, instructions, existing node types and
  relations, known entities, then `Extract the knowledge in these documents.` and the documents
  (`src/extract.rs:170-219`, the split at `:202`); the system prompt and the JSON schema go as
  flags (`:278-280`). The fixed part is rebuilt for every batch (`src/run.rs:600-626`); batches
  are cut at `BATCH_CHARS` (`src/run.rs:854-868`).
- `Report` holds `cost_usd` and no call or token count (`src/run.rs:38-68`); the `cortex.log` line
  (`src/run.rs:1154-1170`) and the `ran` result (`src/main.rs:1716-1725`) print `cost_usd` only.
  The `SourceRan` event has `documents_new`, `documents_applied` and `cost_usd`
  (`spec/domains/instance.yaml:1447-1452`). The cost rule (a missing cost is never `0`) is held by
  `tests/cost.rs`.

## Domain relations

No new noun: new fields of the `cortex.instance.SourceRan` event
(`spec/domains/instance.yaml:1447-1452`), emitted by `RunSource`'s `ran` outcome (`:1056-1072`).

## Work

- Spec first: in `spec/domains/instance.yaml`, add to `cortex.instance.SourceRan` an Integer
  `model_calls` and optional Integer fields for the input tokens (as `claude` reports them, cache
  reads and writes apart), the output tokens, and the input tokens of the fixed part and of the
  documents, with a comment stating how the split is made; set them `{generated: true}` in the
  `ran` payload. Run `ess specify validate` and the conformance suite green, then `task generate`.
  Only then change the code.
- `src/extract.rs`: read `usage` into `Answer` and `Refused` beside `cost_usd`; a missing field is
  `None`, never `0`, as for cost. `extract::prompt` returns, or a sibling function gives, the byte
  length of its fixed part and of its documents part.
- The split: `claude` reports input tokens per call only, not per part of the prompt. The fixed
  part's tokens are the call's input tokens (including cache reads and writes) apportioned by the
  bytes of the fixed part (system prompt, schema and the prompt up to the documents) against the
  bytes of the documents part, and named an estimate in the spec comment and the docs.
- `src/run.rs`: count every model call the run made, refused ones included, and sum the token
  fields per run with the rule of `add_cost`; write them into `Report`, the `ran` result and the
  `cortex.log` line. A run with no model call reports `model_calls: 0` and token fields `0`.
- Tests: extend `tests/cost.rs` with a stand-in `claude` that answers `usage`; one with no `usage`.
- Document the fields on the operating page and in `website/docs/commands.md`.

## Acceptance

- `ess specify validate` passes with the new `SourceRan` fields, and `task drift` is clean after
  `task generate`.
- A run of two batches against a stand-in `claude` answering
  `usage: {input_tokens: 1000, cache_read_input_tokens: 200, cache_creation_input_tokens: 0,
  output_tokens: 50}` per call reports `model_calls: 2`, input tokens 2000, cache reads 400 and
  output tokens 100 in its result and its `cortex.log` line, and its fixed and documents input
  tokens add up to the input tokens it reports.
- With a stand-in whose answer has no `usage`, the run reports `model_calls` and `null` token
  fields, never `0`, and `cost_usd` as today.
- A run with nothing new reports `model_calls: 0`.

## Not in this story

A daily ceiling (`story:daily-instance-budget`); prompt caching or shrinking the fixed part;
the same split for `cortex quality` and `cortex schema`, which make their own calls
(`src/quality.rs:611`, `src/schema.rs:475`).
