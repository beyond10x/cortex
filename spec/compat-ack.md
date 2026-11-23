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
| `command/cortex.instance.UpdateInstance/input-added/no_wait` | `cortex update --no-wait`, new in this release. The input is optional; without it the command waits for the home's lock as in 0.7.2. |
| `command/cortex.instance.UpdateInstance/outcome-added/busy` | `busy`, exit 9, is answered only with `no_wait`, which no caller of 0.7.2 sends; without it the command never answers `busy`. |
| `command/cortex.instance.RemoveInstance/input-added/no_wait` | `cortex remove --no-wait`, new in this release. The input is optional; without it the command waits for the home's lock as in 0.7.2. |
| `command/cortex.instance.RemoveInstance/outcome-added/busy` | `busy`, exit 9, is answered only with `no_wait`, which no caller of 0.7.2 sends; without it the command never answers `busy`. |
| `command/cortex.instance.EnableSource/input-added/no_wait` | `cortex source enable --no-wait`, new in this release. The input is optional; without it the command waits for the home's lock as in 0.7.2. |
| `command/cortex.instance.EnableSource/outcome-added/busy` | `busy`, exit 9, is answered only with `no_wait`, which no caller of 0.7.2 sends; without it the command never answers `busy`. |
