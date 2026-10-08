---
format: aep.planning-md/3
id: epic:standalone-1-0
kind: epic
status: active
title: cortex 1.0 runs any instance on its own
relations:
- serves: vision:self-updating-instances
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T14:07:50Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-08T14:07:50Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":1}}}
---
## Outcome

cortex 1.0: anyone can install a released cortex, start an instance on the store backend they
choose, feed it from web, Connectors, structured or file sources, keep personal data away from the
model, recover from a bad run, adopt a store they already have, and read the documentation that
says all of it. No feature depends on one company's setup.

## Starting point (2026-10-05)

| fact | source |
|---|---|
| Store backend is hard-coded: `EKR_BACKEND=sqlite` | `src/ekr.rs` `Store::cmd` |
| No personal-data redaction; only credential masking before the model | `src/mask.rs`, AGENTS.md hard rules |
| Every source goes through the model; no deterministic import | `src/run.rs`, `SourceSettings` in `spec/domains/instance.yaml` |
| One model backend: `claude -p` | `src/extract.rs:161-174` |
| No snapshot before a run; no adopt; no release, no CHANGELOG, version 0.1.0 | `src/run.rs`, `Cargo.toml:3`, no `CHANGELOG.md` |
| The new types are drafted and validate: `StoreSpec` (sqlite / postgres), `ModelBackend`, `RedactionPolicy`, `StructuredSource` with `RecordMapping`, `SnapshotPolicy`; `AdoptInstance` and `RestoreSnapshot` are `UNMAPPED:` | `spec/domains/instance.yaml` on branch `draft/standalone-spec`; `ess specify validate --path spec`: "cortex v1 — 3 file(s), valid"; `ess verify conform synthesize`: 35 scenarios, 0 refusals |

## Decided

- Store backend is configurable per instance: `sqlite` (default) or `postgres` through EKR's
  PostgreSQL provider (operator, 2026-10-05).

## Stories and order

The `depends_on` edges are the order; `aep plan artifact waves` computes the waves from them and
from each story's typed scope. Every edge's reason (the file both stories edit, or the behaviour one
needs) is written in the dependent story's "Depends on" section.

| story | after |
|---|---|
| `spec-standalone-types` | nothing; it also moves the shared test fixtures to `tests/common/` and makes model cost optional, so the feature stories below touch disjoint files |
| `store-backend-per-instance`, `redaction-before-model` | `spec-standalone-types` |
| `structured-source` | `redaction-before-model` (`src/run.rs`) |
| `run-snapshots` | `structured-source` (`src/run.rs`), `seen-documents-modelled` (spec), `store-backend-per-instance` (`src/main.rs`) |
| `adopt-existing-store` | `run-snapshots`, `store-backend-per-instance` |
| `release-pipeline` | nothing; it cuts no release |
| `docs-for-1-0` | every feature story and `release-pipeline` |
| `release-1-0` | `release-pipeline`, `docs-for-1-0`, `timer-runs-unattended` |

The spec story adds no refusals for the new settings: between it and the story that gives a setting
its behaviour, `main` parses the setting without acting on it, and no release is cut in between.

## Not in this epic

Company-specific migration work (it belongs to the company's own repository); a hosted runner other
than systemd; multi-user access control on the MCP server; Slack reads (a Connectors story).

`story:codex-model-backend` left this epic on 2026-10-08. `decision-blocker:codex-exec-keeps-shell`
has blocked it since 2026-10-05 (`codex exec` keeps a shell tool, which breaks the rule that the
model has no tools), and the 1.0 documentation states the Codex backend as unavailable. 1.0 ships
with the Claude backend only. The story stays active and serves the vision directly; it lands in a
later release once the blocker clears.
