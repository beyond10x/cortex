---
format: aep.planning-md/3
id: story:codex-model-backend
kind: story
status: active
title: An instance can extract with Codex instead of Claude
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
- depends_on: story:spec-standalone-types
scope:
- confidence: cited
  path: src/extract.rs
- confidence: inferred
  path: tests/codex.rs
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T11:54:37Z", actor: "agent:claude", revision: 10, decided_on: {"recorded":{"review_outcome":6}}}
- {from: "proposed", to: "active", at: "2026-10-05T11:54:37Z", actor: "agent:claude", revision: 11, decided_on: {"recorded":{"review_outcome":6}}}
---
## Outcome

An instance can extract with Codex instead of Claude, chosen by `model.backend`.

## Work

- `src/extract.rs`: a backend trait over the one call `Model::ask` makes today
  (`src/extract.rs:161-174`); `Claude` keeps its isolation flags.
- `Codex` runs, with the prompt on standard input, as
  `codex exec -m <model> --sandbox read-only --skip-git-repo-check --ephemeral --ignore-user-config --ignore-rules --output-schema <schema file> --json -`
  (flags from `codex exec --help`, codex-cli 0.160.0), in an empty directory, and reads the final
  agent message from the `--json` event stream.
- Codex reports no dollar cost: its answer's cost is `None`, which the budget loop prepared by
  `story:spec-standalone-types` treats as consuming no dollar budget. The run stays bounded by
  `policy.max_documents_per_run` and `model.timeout_s`. No new spec field.

## Acceptance

`tests/codex.rs` runs a `files` source with `model.backend: Codex` against a stand-in `codex` that
fails unless its arguments equal the command line above with the instance's model and a schema file
that parses as JSON Schema, and the run report shows `"cost_usd": null` with the stand-in's facts
applied.

## Depends on

`story:spec-standalone-types`.

## Scope

`src/extract.rs`, `tests/codex.rs` (new).
