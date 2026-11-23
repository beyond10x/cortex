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
| `command/cortex.instance.RunSource/input-added/no_wait` | `cortex run --no-wait`, new in this release. The input is optional; a run without it waits for the home's lock as in 0.7.2. |
| `command/cortex.instance.RunSource/outcome-added/busy` | `busy`, exit 9, is answered only with `no_wait`, which no caller of 0.7.2 sends; a run without it never answers `busy`. |
| `command/cortex.instance.RunSource/input-added/path` | An `Optional` input, the flag `cortex run --path <dir>`; a run without it reads the spec's `paths` and keys documents as before. |
| `command/cortex.instance.RunSource/outcome-added/path-refused` | Answered only to a run given `--path`, which no caller before this release could pass; every run without it answers the outcomes of v0.7.2. |
| `command/cortex.instance.RunSource/outcome-response-payload-changed/ran` | Story `run-report-cap-and-refusals`: the `ran` payload, `SourceRan`, carries `documents_left` and `stopped`, so a caller tells a run that `max_documents_per_run` capped from one that read every wanted document without a second `--fetch-only` call. cortex keeps no history of `SourceRan` events; `cortex run` prints both keys on every `ran` line, and `stopped` was there already. |
| `event/cortex.instance.SourceRan/field-added/documents_left` | Same story: the wanted documents a run neither applied nor marked seen, `0` when it left none. No stored history of the event exists to read without it. |
| `event/cortex.instance.SourceRan/field-added/stopped` | Same story: why the run ended before applying every wanted document, `document-cap` when the cap alone left documents; absent when it applied every one. Optional, and no stored history of the event exists. |
