---
format: aep.planning-md/3
id: story:run-fetch-only
kind: story
status: draft
title: A fetch-only run counts what a source would deliver without a model call or a write
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
revision: 1
---
## Outcome

`cortex run <instance>/<source> --fetch-only` fetches the source as a run would, masks and scrubs
what it fetched, compares it with the source's seen-state by its content hash, and prints the
window and per-source counts. It makes no model call, writes nothing to the store, to the
seen-state or to `cortex.log`, takes no snapshot, and counts no run and no failure. Example:

```json
{"source":"news","window":{"since":"2026-10-15T09:50:00Z","until":"2026-10-15T10:00:00Z"},
 "fetched":20,"new":3,"changed":1,"held":0,"unchanged":16,"unread":[]}
```

## Why

A downstream consumer running cortex at a 10-minute cadence needs to verify a new source,
connection or schedule without paying for extraction, and to count the documents per run for a
cost estimate. Today the only way to see what a source delivers is a full run. Verified in this
tree:

- `cortex run` has one flag besides the source id, `--record-failure` (`src/main.rs:129-135`).
  The command is `cortex.instance.RunSource` (`spec/domains/instance.yaml:1033`) with the input
  `source_id` only; its `ran` outcome increments `runs` and resets `consecutive_failures`
  (`:1056-1072`).
- `run::run` computes the window from the seen-state (`src/run.rs:163-172`), fetches
  (`:176-190`) and hands the result to `process` (`src/run.rs:300`), which masks and scrubs each
  document (`:400-460`), selects what the seen-state wants (`:478-495`, `SeenState::wants` at
  `src/state.rs:150-161`, `holds` at `:166-174`) and goes on to snapshot, extract and apply; a
  failed run writes `pending_since` into the seen-state (`src/run.rs:208-218`).
- The clap test holds each flag of `run` to an input of `RunSource` or to the `STEERING` list
  (`src/main.rs:1973-1983`, `:2019-2050`).

## Domain relations

No new noun. The mode is an input of `cortex.instance.RunSource`, whose subject is
`cortex.instance.Source` (`spec/domains/instance.yaml:556`); the counts compare against the
source's `SeenDocument`s (relation `seen`, `:567-571`) without changing them.

## Work

- Spec first: in `spec/domains/instance.yaml`, add an optional Boolean input `fetch_only` to
  `cortex.instance.RunSource` and an outcome `fetched` for it, taken when the source is `Enabled`
  and the fetch succeeds, that updates nothing on the `Source` and emits an event carrying the
  window and the counts `fetched`, `new`, `changed`, `held`, `unchanged` and `unread`; a failed
  fetch in this mode answers `fetch-failed` as today. Run `ess specify validate` and the
  conformance suite green, then `task generate`. Only then change the code.
- `src/run.rs`: split `process` so the fetch, the mask and scrub and the seen-state comparison run
  without the store, the snapshot, the model, `finish` and `log`; `--fetch-only` stops there and
  returns the counts. `new` is a key the seen-state does not carry; `changed` is one `wants`
  would extract now; `held` is one `holds` keeps back; `unchanged` is the rest. Every count is
  taken after masking and scrubbing, as a real run's hash is. A failed fetch in this mode writes no
  `pending_since`.
- `src/main.rs`: the `--fetch-only` flag (in conflict with `--record-failure`) and the printed
  result; `src/ports.rs`: the `fetched` outcome. The `unread` lines are masked as a run masks them.
- Tests in `tests/e2e.rs` with the stand-in `connectors` and a stand-in `claude` that fails when
  called.
- Document the flag in `website/docs/commands.md` (`## Sources`) and on the operating page.

## Acceptance

- `ess specify validate` passes with `fetch_only` on `RunSource` and its `fetched` outcome, and
  the clap test in `src/main.rs` passes with `--fetch-only` on `run`.
- On a source with 20 documents of which the seen-state holds 16 unchanged, 1 changed past
  `refresh_after_days` and none held, `cortex run <instance>/<source> --fetch-only` exits 0 and
  prints `fetched: 20, new: 3, changed: 1, held: 0, unchanged: 16` with the window's `since` and
  `until` as RFC 3339.
- After that command the stand-in `claude` was never invoked, and `state/<source>.json`, the
  store file, `registry.json` (`runs`, `consecutive_failures`) and `cortex.log` are byte-identical
  to before, with no new entry under `runs/` or `snapshots/`.
- A `--fetch-only` run whose fetch fails answers `fetch-failed`, leaves `state/<source>.json`
  unchanged (no `pending_since`) and counts no failure.
- `cortex run <instance>/<source> --fetch-only --record-failure` is refused by the command line.

## Not in this story

A cost estimate in dollars (counts only; `story:run-cost-split` records what a run spent);
fetch-only for every source of an instance at once; a dry run of extraction.
