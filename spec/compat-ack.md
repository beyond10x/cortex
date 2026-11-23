# Acknowledged specification changes

`task compat` compares `spec/` with `spec/` at the newest release tag (`ess verify diff
--fail-on breaking-or-unknown`) and fails on a breaking or unknown change that
[`compat-ack.json`](compat-ack.json) does not acknowledge. That file is ESS's
`ess-diff-acknowledgements/1`: the change ids, bound to the spec digests of both sides, and no
reasons. Each acknowledged change has its reason here, one row each; `tests/compat_ack.rs` fails
when an id has no row or a row no id.

The comparison: `v0.8.0` against the release being prepared. After a release, both files are
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
| `entity/cortex.instance.SeenDocument/field-added/fields_hash` | An `Optional` field of the per-source state file, written by cortex and read only by it. A state file of v0.8.0 lacks it and reads as before: a web page takes the fields of its next fetch as applied, a structured record is applied again once (no model call). |
| `command/cortex.instance.RunSource/outcome-response-payload-changed/fetched` | The `fetched` payload gains `skipped`, the line list a run already prints in its own `skipped`; every field of v0.8.0 keeps its name and type. A document named there is no longer counted `new` or `changed`, as the run sends it to no model (review finding 11). |
| `event/cortex.instance.SourceFetched/field-added/skipped` | The same field on the event: `fetched --fetch-only` output is read by operators and scripts as JSON, and an added member leaves each field they read unchanged. The event is carried on the outcome and not published, so no stored history holds the old shape. |
| `command/cortex.instance.UpdateInstance/input-added/no_wait` | `cortex update --no-wait`, new in this release. The input is optional; without it the command waits for the home's lock as in 0.8.0. |
| `command/cortex.instance.UpdateInstance/outcome-added/busy` | `busy`, exit 9, is answered only with `no_wait`, which no caller of 0.8.0 sends; without it the command never answers `busy`. |
| `command/cortex.instance.RemoveInstance/input-added/no_wait` | `cortex remove --no-wait`, new in this release. The input is optional; without it the command waits for the home's lock as in 0.8.0. |
| `command/cortex.instance.RemoveInstance/outcome-added/busy` | `busy`, exit 9, is answered only with `no_wait`, which no caller of 0.8.0 sends; without it the command never answers `busy`. |
| `command/cortex.instance.EnableSource/input-added/no_wait` | `cortex source enable --no-wait`, new in this release. The input is optional; without it the command waits for the home's lock as in 0.8.0. |
| `command/cortex.instance.EnableSource/outcome-added/busy` | `busy`, exit 9, is answered only with `no_wait`, which no caller of 0.8.0 sends; without it the command never answers `busy`. |
| `command/cortex.instance.RunSource/outcome-response-payload-changed/ran` | Story `run-report-cap-and-refusals`: the `ran` payload, `SourceRan`, carries `documents_left` and `stopped`, so a caller tells a run that `max_documents_per_run` capped (`stopped: document-cap`, the wanted documents left counted) from one that read every wanted document, without a second `--fetch-only` call. cortex keeps no history of `SourceRan` events; `cortex run` already printed `stopped` and now also prints `documents_left` on every `ran` line. |
| `event/cortex.instance.SourceRan/field-added/documents_left` | Same story: the wanted documents a run neither applied nor marked seen, `0` when it left none. No stored history of the event exists to read without it. |
| `event/cortex.instance.SourceRan/field-added/stopped` | Same story: why the run ended before applying every wanted document, `document-cap` when the cap alone left documents; absent when it applied every one. Optional, and no stored history of the event exists. |
| `command/cortex.instance.AddSource/outcome-added/no-such-instance` | `cortex source add` for an instance the home does not hold registered an orphan source; it is now refused (exit 1) and nothing changes. A caller that added such a source gets a refusal it did not get before (review of 0.7.2, finding 15). |
| `command/cortex.instance.AddSource/outcome-added/not-active` | `cortex source add` for a removed instance is refused (exit 1) and nothing changes, as `update` of a removed instance already is (finding 15). |
| `command/cortex.instance.AddSource/outcome-added/source-exists` | A repeated `cortex source add` for a source the registry holds reset its counters and re-enabled a disabled source; it is now refused (exit 1) and the source keeps its schedule, counters and state. `create`, `adopt` and `update` add only sources the registry does not hold, so they never meet it (finding 15). |
| `command/cortex.instance.RemoveInstance/outcome-added/stop-failed` | `cortex remove` answered `removed` (exit 0) after `systemctl disable --now` failed and deleted the unit files, so a second remove was refused as `wrong-state`. It now answers `stop-failed` (exit 1) naming each failure, deletes nothing and keeps the instance active, so the remove can be run again (finding 16). |
