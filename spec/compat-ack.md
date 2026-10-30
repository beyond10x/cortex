# Acknowledged specification changes

`task compat` compares `spec/` with `spec/` at the newest release tag (`ess verify diff
--fail-on breaking-or-unknown`) and fails on a breaking or unknown change that
[`compat-ack.json`](compat-ack.json) does not acknowledge. That file is ESS's
`ess-diff-acknowledgements/1`: the change ids, bound to the spec digests of both sides, and no
reasons. Each acknowledged change has its reason here, one row each; `tests/compat_ack.rs` fails
when an id has no row or a row no id.

The comparison: `v0.6.1` against the release being prepared. After a release, both files are
reset (AGENTS.md, "The specification comes first").

| change | reason |
|---|---|
| `type/cortex.ask.AnswerOutcome/variant-added/client-closed` | An answer whose reader left is recorded as `client-closed`, no longer as `failed` with a review item; a reader of the records has to accept the new outcome (CHANGELOG, Unreleased, Fixed). |
| `type/cortex.ask.AnswerOutcome/variant-added/cut-off` | An answer whose reply hit the token limit or a content filter is recorded as `cut-off` instead of as answered; a reader of the records has to accept the new outcome, and an older cortex cannot read such a record (CHANGELOG, Unreleased, Fixed). |
| `type/cortex.ask.AskLimits/field-added/max_tool_calls` | An `Optional` field of the spec file; a spec without it parses as before and takes the default 16. |
| `type/cortex.ask.AskSpec/field-added/max_in_flight` | An `Optional` field of the spec file; a spec without it parses as before and takes the default 8. |
| `type/cortex.instance.ChildCall/field-added/follow` | An `Optional` field of the spec file; a `connectors` child without it is called for every parent, as before. |
| `type/cortex.instance.ChildCall/field-added/text` | An `Optional` field of the spec file; a `connectors` child without it adds its records as raw JSON, as before. |
| `type/cortex.instance.ConnectorsSource/field-added/key` | An `Optional` field of the spec file; a `connectors` source without it keys documents by `id`, as before. |
| `type/cortex.instance.RecordMapping/field-added/store_lookup` | An `Optional` field of the spec file; a mapping without it meets store nodes of its own type, as before. |
| `type/cortex.instance.StructuredChild/field-added/follow` | An `Optional` field of the spec file; a `structured` child without it is called for every parent, as before. |
| `command/cortex.ask.Ask/outcome-added/busy` | A new refusal, 429 `busy` past `max_in_flight` answers at once; a caller already handles 429 for `rate-limited`. |
| `command/cortex.ask.Ask/outcome-added/client-closed` | The outcome of an answer whose reader left; nobody reads its last event, and its record carries it. |
| `command/cortex.ask.Ask/outcome-added/cut-off` | A new `error` event, reason `cut-off`, where the reply was cut off; a caller already handles `error` events that end a stream. |
| `command/cortex.ask.CastVote/outcome-added/bad-request` | The vote route answered 413 and 400 `bad-request` already (website/docs/ask-service.md); the specification now states it. No behaviour changed. |
| `command/cortex.ask.CastVote/outcome-added/store-failed` | The vote route answered 500 `store-failed` already; the specification now states it. No behaviour changed. |
| `command/cortex.ask.CloseReview/outcome-added/bad-request` | The close route answered 413 and 400 `bad-request` already; the specification now states it. No behaviour changed. |
| `command/cortex.ask.CloseReview/outcome-added/store-failed` | The close route answered 500 `store-failed` already; the specification now states it. No behaviour changed. |
| `command/cortex.instance.ApplySchemaProposals/outcome-condition-changed/apply-failed` | `apply-failed` now also covers a head that cannot be read after the publish and a `proposals.jsonl` that cannot be written; the outcome and its exit status are the ones a caller already handles. |
| `command/cortex.instance.ProposeSchemaChanges/outcome-added/store-outcome-unknown` | A proposal EKR could not say took effect stops the round as `store-outcome-unknown`, exit 4 as a run's, instead of being recorded `refused`. |
| `command/cortex.instance.ProposeSchemaChanges/outcome-condition-changed/propose-failed` | `propose-failed` now also covers a `proposals.jsonl` that cannot be written; the outcome is the one a caller already handles. |
| `command/cortex.instance.RunSource/outcome-response-payload-changed/ran` | `SourceRan` gains `facts_reapplied`, which `cortex run` printed already. |
| `command/cortex.instance.UpdateInstance/outcome-added/rekey-refused` | `cortex update` refuses a change that would re-key a source's seen documents, exit 1, where it re-extracted them before. |
| `event/cortex.instance.SourceRan/field-added/facts_reapplied` | An added field of the run's result. cortex stores no `SourceRan` event, so no earlier copy lacks it; a reader of `cortex run`'s output saw the field already. |
