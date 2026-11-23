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
| `entity/cortex.instance.SeenDocument/field-added/fields_hash` | An `Optional` field of the per-source state file, written by cortex and read only by it. A state file of v0.8.0 lacks it and reads as before: a web page takes the fields of its next fetch as applied, a structured record is applied again once (no model call). |
| `command/cortex.instance.RunSource/outcome-response-payload-changed/fetched` | The `fetched` payload gains `skipped`, the line list a run already prints in its own `skipped`; every field of v0.8.0 keeps its name and type. A document named there is no longer counted `new` or `changed`, as the run sends it to no model (review finding 11). |
| `event/cortex.instance.SourceFetched/field-added/skipped` | The same field on the event: `fetched --fetch-only` output is read by operators and scripts as JSON, and an added member leaves each field they read unchanged. The event is carried on the outcome and not published, so no stored history holds the old shape. |
