---
format: aep.planning-md/3
id: story:batch-fits-model-window
kind: story
status: implemented
title: An extraction batch fits the model's window, and one that does not is split and retried, not counted against the source
refs:
- provider: github
  reference: beyond10x/cortex#72
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T18:18:11Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-16T18:18:11Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-17T02:05:50Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

An extraction batch fits the model it is sent to, and a batch that still does not fit is split
and retried instead of failing the run:

1. Size. `model.batch_chars` (optional, at least 1,000) sets the most characters of document text
   in one batch. Without it, behind a gateway that gives `max_context_tokens`, the size is derived:
   the input room is `max_context_tokens` minus the output reserve (`model.gateway.max_output_tokens`
   when given, else a quarter of `max_context_tokens`); the batch size is 1.5 characters per token
   of that room, minus the request's fixed part (system prompt, schema, instructions, ontology,
   known names) in characters, at least 4,000 and at most 60,000. Otherwise it is 60,000, as now.
   `model.gateway.max_output_tokens`, when given, is passed to `claude` as
   `CLAUDE_CODE_MAX_OUTPUT_TOKENS`, beside the gateway's fixed pass-through set. The size a run used
   is stated in its report and `cortex.log` line (`batch_chars`).
2. Split. When `claude` answers that the request is too long for the model's window, or that its
   answer was cut off at the output limit, the batch is split in two halves by documents and each
   half is asked again, recursively, at most **3 splits deep** (a batch becomes at most 8 parts). A
   one-document part that is still too long is not split further. The calls of the failed attempts
   count against the token and dollar limits as any call does.
3. Not counted. A run in which a part still does not fit after the bound ends with outcome
   `batch-too-large`, exit status 6, its reason naming the documents of that part and the size used:
   the batches before it are applied and stay seen; that part and the batches after it are not
   marked seen, so a later run (with a smaller `model.batch_chars`) extracts them. No failure is
   recorded (`--record-failure` counts nothing) and the source is never disabled by it; its status,
   metric and hook report it as for `store-outcome-unknown`.

## Why

Issue #72. Behind `model.gateway`, a self-hosted model with a 64k-token window received 60,000
characters of merge-request text per batch: with a 24,000-token output cap the request was over
the window (`Prompt is too long`, 41,537 input tokens); with a 16,000-token cap the answer was cut
off. Each run counted as a failure; one more disables the source. The fixed 60,000 characters fit
the hosted model's window, not a smaller one.

Verified in the code:

- `src/run.rs` `BATCH_CHARS = 60_000` and `batches` cut every source's documents the same way.
- `cortex.instance.ModelGateway.max_context_tokens` is already passed to `claude` as
  `CLAUDE_CODE_MAX_CONTEXT_TOKENS` (`src/extract.rs`), but not used for batching.
- An `is_error` answer from `claude` becomes the run's failure (`src/extract.rs` around
  `answer["is_error"]`), which `--record-failure` counts.

## Domain relations

No new noun. `cortex.instance.ModelSpec` gains `batch_chars: Optional<Integer>`;
`cortex.instance.ModelGateway` gains `max_output_tokens: Optional<Integer>`; a new
`cortex.instance.RunSourceOutcome` variant `BatchTooLarge` with an error carrying `reason`, beside
`StoreOutcomeUnknown`; `SourceRan` (or the run's report) carries `batch_chars` and the number of
splits.

## Work

- Spec first: the fields, the outcome, and comments stating the derivation, the split bound and
  that the outcome is not counted; the refusals (`batch_chars` below 1,000, `max_output_tokens` not
  below `max_context_tokens` or not above 0: exit 2 naming the field, at create/update/adopt);
  validate, conformance suite, `task generate`, `task drift`.
- `src/run.rs`: the size from the spec; split-and-retry around a batch's extraction; the outcome.
- `src/extract.rs`: tell "too long" and "cut off at the output limit" from other errors, as
  `claude -p` reports them (name the exact signals in the spec comment); pass
  `CLAUDE_CODE_MAX_OUTPUT_TOKENS`.
- `src/main.rs`: exit status 6, no `RecordFailure`.
- Tests first, with the stand-in `claude`: `tests/batch_fit.rs`.
- Docs: spec-file page `model` section, `website/docs/limits.md`, `website/docs/operating.md`
  "Failures"; CHANGELOG naming issue #72.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- Size: with `max_context_tokens: 65536` and `max_output_tokens: 24000` and no `batch_chars`, the
  documents the stand-in `claude` receives in one request are at most the derived size; with
  `batch_chars: 5000` they are at most 5,000 characters; without a gateway, 60,000 as before.
- Split: a stand-in that answers "too long" for any request over N characters of documents and
  answers normally under it: the run succeeds, every document is extracted, and the report counts
  the splits. The same for an answer cut off at the output limit.
- Bound: a stand-in that answers "too long" always: after 3 splits the run ends `batch-too-large`,
  exit 6; with `--record-failure` the failure count is unchanged and two such runs leave the
  source enabled; the batches before it are applied and the rest are extracted by a later run.
- Other errors are not split: an `is_error` answer of another kind fails and is counted as today.
- `max_output_tokens` reaches `claude` as `CLAUDE_CODE_MAX_OUTPUT_TOKENS`; without it, the variable
  is not passed.

## Not in this story

- Counting tokens exactly before a call (a tokenizer per model).
- Batching for `cortex quality` and `cortex schema`, which keep their sizes.
