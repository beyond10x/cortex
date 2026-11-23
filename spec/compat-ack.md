# Acknowledged specification changes

`task compat` compares `spec/` with `spec/` at the newest release tag (`ess verify diff
--fail-on breaking-or-unknown`) and fails on a breaking or unknown change that
[`compat-ack.json`](compat-ack.json) does not acknowledge. That file is ESS's
`ess-diff-acknowledgements/1`: the change ids, bound to the spec digests of both sides, and no
reasons. Each acknowledged change has its reason here, one row each; `tests/compat_ack.rs` fails
when an id has no row or a row no id.

The comparison: `v0.7.2` against the release being prepared. After a release, both files are
reset (AGENTS.md, "The specification comes first").

| change | reason |
|---|---|
| `type/cortex.ask.AnswerOutcome/variant-added/refused` | Review finding 20: a model refusal was recorded as `answered` with an empty answer. A reader of the records meets the new outcome `refused`; `outcome_of` reads it and the review queue opens a `failed` item for it. |
| `type/cortex.ask.AskDone/field-added/unsupported` | Review finding 21: a citation of an id the reader never returned was linked and listed in `nodes`. `done` now lists such references apart; a strict decoder of the event stream needs the field. |
| `command/cortex.ask.Ask/outcome-added/refused` | Review finding 20: a Chat Completions refusal ends the answer with an `error` event whose reason is `refused`, no `done`. |
| `command/cortex.ask.Ask/outcome-error-payload-changed/limit-reached` | Review finding 24: the error now carries the steps and token counts of the model calls made, so the record keeps them. |
| `command/cortex.ask.Ask/outcome-error-payload-changed/model-failed` | Review finding 24: as `limit-reached`, for a model failure after a model call. |
| `command/cortex.ask.Ask/outcome-error-payload-changed/reader-failed` | Review finding 24: as `limit-reached`, for a reader failure after a model call. |
| `error/cortex.ask.LimitReached/field-added/cached_tokens` | Review finding 24: usage of the model calls made before the limit. |
| `error/cortex.ask.LimitReached/field-added/context_tokens` | Review finding 24: usage of the model calls made before the limit. |
| `error/cortex.ask.LimitReached/field-added/input_tokens` | Review finding 24: usage of the model calls made before the limit. |
| `error/cortex.ask.LimitReached/field-added/output_tokens` | Review finding 24: usage of the model calls made before the limit. |
| `error/cortex.ask.LimitReached/field-added/steps` | Review finding 24: the model calls made before the limit. |
| `error/cortex.ask.ModelFailed/field-added/cached_tokens` | Review finding 24: usage of the model calls that answered before the failure. |
| `error/cortex.ask.ModelFailed/field-added/context_tokens` | Review finding 24: usage of the model calls that answered before the failure. |
| `error/cortex.ask.ModelFailed/field-added/input_tokens` | Review finding 24: usage of the model calls that answered before the failure. |
| `error/cortex.ask.ModelFailed/field-added/output_tokens` | Review finding 24: usage of the model calls that answered before the failure. |
| `error/cortex.ask.ModelFailed/field-added/steps` | Review finding 24: the model calls that answered before the failure. |
| `error/cortex.ask.ReaderFailed/field-added/cached_tokens` | Review finding 24: usage of the model calls made before the reader failed. |
| `error/cortex.ask.ReaderFailed/field-added/context_tokens` | Review finding 24: usage of the model calls made before the reader failed. |
| `error/cortex.ask.ReaderFailed/field-added/input_tokens` | Review finding 24: usage of the model calls made before the reader failed. |
| `error/cortex.ask.ReaderFailed/field-added/output_tokens` | Review finding 24: usage of the model calls made before the reader failed. |
| `error/cortex.ask.ReaderFailed/field-added/steps` | Review finding 24: the model calls made before the reader failed. |
