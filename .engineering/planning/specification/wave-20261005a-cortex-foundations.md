---
format: aep.planning-md/3
id: specification:wave-20261005a-cortex-foundations
kind: specification
status: implemented
title: 'Wave 20261005a: cortex 1.0 foundations'
relations:
- serves: vision:self-updating-instances
revision: 5
transitions:
- {from: "draft", to: "in_review", at: "2026-10-05T11:10:19Z", actor: "agent:claude", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-05T11:10:19Z", actor: "agent:claude", revision: 3}
- {from: "approved", to: "implemented", at: "2026-10-05T11:49:32Z", actor: "agent:claude", revision: 5}
---
## Wave 20261005a: cortex 1.0 foundations

Skill: `aep:implementing` 0.19.2, wave mode. Coordinator: session brain-and-cortex. Interactive run;
stage 1 stops for the operator.

## Units

| unit | story | serves | scope | branch | worktree id / path | build dir | scratch |
|---|---|---|---|---|---|---|---|
| int | (integration) | — | — | `wave/20261005a` | `cortex-w1-int` / `~/.local/state/worktree/trees/b10x/cortex/cortex-w1-int` | `~/.cache/b10x-target/cortex-w1-int` | `~/.cache/cortex-wave-20261005a/int` |
| U1 | `story:spec-standalone-types` | `vision:self-updating-instances` | 14 cited, 5 inferred | `unit/spec-standalone-types` | `cortex-w1-spec` / `~/.local/state/worktree/trees/b10x/cortex/cortex-w1-spec` | `~/.cache/b10x-target/cortex-w1-spec` | `~/.cache/cortex-wave-20261005a/spec` |
| U2 | `story:release-pipeline` | `vision:self-updating-instances` | 3 cited | `unit/release-pipeline` | `cortex-w1-release` / `~/.local/state/worktree/trees/b10x/cortex/cortex-w1-release` | `~/.cache/b10x-target/cortex-w1-release` | `~/.cache/cortex-wave-20261005a/release` |

Stage per unit: U1 proposed; U2 proposed. (Stage 2 updates this table.)

Dispatch types: `aep:implementor` per unit, then `aep:adversary` per unit (two passes at most).

## Selection

Computed with `aep plan artifact waves --kind story --status draft` (aep 0.68.0) after both
candidates were re-scoped by `aep:story-scoper` on 2026-10-05; wave 1 is the verb's first wave
unchanged. No collision between U1 and U2. Left out: every story of wave 2 onward, which depends on
U1 (the types and test seams) or collides with it on `spec/domains/instance.yaml`, `src/run.rs`,
`src/extract.rs`, `src/main.rs` or `tests/e2e.rs`.

## Pre-flight (2026-10-05)

| check | value |
|---|---|
| primary checkout | `~/beyond10x/cortex` on `main` `012e5a1`, clean except the uncommitted store edits of this proposal |
| other worktrees | none |
| free disk | 17 GB (floor 10 GB per AGENTS.md) |
| one measured tree build | 0.4–0.5 GB (`cortex-extract` 480 MB, `cortex-docs` 401 MB, today) |
| compiler cache | `sccache` installed; units run with `RUSTC_WRAPPER=sccache` |
| model budget | asked of the operator in the proposal |

## Commits approval authorises

Two unit commits series (one branch each), their merges into `wave/20261005a`, one opening store
commit (this page and the two stories moved to active), one closing store commit (evidence and moves
to implemented), and the merge of `wave/20261005a` into `main` through a bot PR. Nothing else: no
release, no tag, no next wave.

## `aep plan artifact waves --kind story --status draft`, verbatim

```
wave 1
  story:release-pipeline
  story:spec-standalone-types (inferred)
wave 2
  story:codex-model-backend (inferred)
  story:redaction-before-model (inferred)
  story:seen-documents-modelled
  story:store-backend-per-instance (inferred)
wave 3
  story:connectors-source-walks (inferred)
wave 4
  story:structured-source (inferred)
wave 5
  story:redaction-names-and-gate (inferred)
wave 6
  story:timer-runs-unattended (inferred)
wave 7
  story:file-records (inferred)
  story:first-web-instance (inferred)
  story:run-snapshots (inferred)
wave 8
  story:adopt-existing-store (inferred)
  story:document-time-as-valid-time (inferred)
  story:run-gate (inferred)
  story:structured-from-files-and-drops (inferred)
wave 9
  story:docs-for-1-0
  story:quality-judge (inferred)
wave 10
  story:release-1-0 (inferred)
  story:web-pages-cited-as-urls (inferred)
collision: story:adopt-existing-store story:connectors-source-walks src/state.rs
collision: story:adopt-existing-store story:file-records src/state.rs
collision: story:adopt-existing-store story:quality-judge src/main.rs
collision: story:adopt-existing-store story:run-snapshots generated
collision: story:adopt-existing-store story:run-snapshots spec/domains/instance.yaml
collision: story:adopt-existing-store story:run-snapshots spec/suite.json
collision: story:adopt-existing-store story:run-snapshots src/main.rs
collision: story:adopt-existing-store story:run-snapshots tests/conformance.rs
collision: story:adopt-existing-store story:seen-documents-modelled generated
collision: story:adopt-existing-store story:seen-documents-modelled spec/domains/instance.yaml
collision: story:adopt-existing-store story:seen-documents-modelled spec/suite.json
collision: story:adopt-existing-store story:seen-documents-modelled src/state.rs
collision: story:adopt-existing-store story:spec-standalone-types generated
collision: story:adopt-existing-store story:spec-standalone-types spec/domains/instance.yaml
collision: story:adopt-existing-store story:spec-standalone-types spec/suite.json
collision: story:adopt-existing-store story:spec-standalone-types src/main.rs
collision: story:adopt-existing-store story:spec-standalone-types tests/conformance.rs (inferred)
collision: story:adopt-existing-store story:store-backend-per-instance src/instance.rs
collision: story:adopt-existing-store story:store-backend-per-instance src/main.rs
collision: story:adopt-existing-store story:timer-runs-unattended src/state.rs (inferred)
collision: story:adopt-existing-store story:web-pages-cited-as-urls src/main.rs
collision: story:adopt-existing-store story:web-pages-cited-as-urls tests/conformance.rs
collision: story:codex-model-backend story:document-time-as-valid-time src/extract.rs
collision: story:codex-model-backend story:quality-judge src/extract.rs
collision: story:codex-model-backend story:spec-standalone-types src/extract.rs
collision: story:codex-model-backend story:web-pages-cited-as-urls src/extract.rs
collision: story:connectors-source-walks story:file-records src/sources.rs
collision: story:connectors-source-walks story:file-records src/state.rs
collision: story:connectors-source-walks story:redaction-before-model src/run.rs
collision: story:connectors-source-walks story:redaction-names-and-gate src/run.rs
collision: story:connectors-source-walks story:run-gate src/run.rs
collision: story:connectors-source-walks story:run-snapshots src/run.rs
collision: story:connectors-source-walks story:seen-documents-modelled src/state.rs
collision: story:connectors-source-walks story:spec-standalone-types src/run.rs
collision: story:connectors-source-walks story:spec-standalone-types src/sources.rs (inferred)
collision: story:connectors-source-walks story:structured-from-files-and-drops src/sources.rs
collision: story:connectors-source-walks story:structured-source src/run.rs
collision: story:connectors-source-walks story:structured-source src/sources.rs
collision: story:connectors-source-walks story:timer-runs-unattended src/run.rs (inferred)
collision: story:connectors-source-walks story:timer-runs-unattended src/state.rs (inferred)
collision: story:docs-for-1-0 story:spec-standalone-types website/docs/commands.md (inferred)
collision: story:docs-for-1-0 story:timer-runs-unattended README.md
collision: story:docs-for-1-0 story:web-pages-cited-as-urls website/docs/commands.md
collision: story:docs-for-1-0 story:web-pages-cited-as-urls website/docs/limits.md
collision: story:docs-for-1-0 story:web-pages-cited-as-urls website/docs/quickstart.md
collision: story:docs-for-1-0 story:web-pages-cited-as-urls website/docs/spec-file.md
collision: story:document-time-as-valid-time story:quality-judge src/extract.rs
collision: story:document-time-as-valid-time story:spec-standalone-types src/extract.rs
collision: story:document-time-as-valid-time story:web-pages-cited-as-urls src/evidence.rs
collision: story:document-time-as-valid-time story:web-pages-cited-as-urls src/extract.rs
collision: story:file-records story:seen-documents-modelled src/state.rs
collision: story:file-records story:spec-standalone-types src/sources.rs (inferred)
collision: story:file-records story:structured-from-files-and-drops src/sources.rs
collision: story:file-records story:structured-source src/sources.rs
collision: story:file-records story:timer-runs-unattended src/state.rs (inferred)
collision: story:quality-judge story:redaction-before-model src/lib.rs
collision: story:quality-judge story:run-gate src/lib.rs
collision: story:quality-judge story:run-snapshots src/lib.rs
collision: story:quality-judge story:run-snapshots src/main.rs
collision: story:quality-judge story:spec-standalone-types src/extract.rs
collision: story:quality-judge story:spec-standalone-types src/main.rs
collision: story:quality-judge story:store-backend-per-instance src/main.rs
collision: story:quality-judge story:structured-source src/lib.rs
collision: story:quality-judge story:web-pages-cited-as-urls src/extract.rs
collision: story:quality-judge story:web-pages-cited-as-urls src/main.rs
collision: story:redaction-before-model story:redaction-names-and-gate src/redact.rs (inferred)
collision: story:redaction-before-model story:redaction-names-and-gate src/run.rs
collision: story:redaction-before-model story:release-pipeline AGENTS.md
collision: story:redaction-before-model story:run-gate src/lib.rs
collision: story:redaction-before-model story:run-gate src/run.rs
collision: story:redaction-before-model story:run-snapshots src/lib.rs
collision: story:redaction-before-model story:run-snapshots src/run.rs
collision: story:redaction-before-model story:spec-standalone-types src/run.rs
collision: story:redaction-before-model story:structured-source src/lib.rs
collision: story:redaction-before-model story:structured-source src/run.rs
collision: story:redaction-before-model story:timer-runs-unattended src/run.rs (inferred)
collision: story:redaction-before-model story:web-pages-cited-as-urls AGENTS.md
collision: story:redaction-names-and-gate story:run-gate src/run.rs
collision: story:redaction-names-and-gate story:run-snapshots src/run.rs
collision: story:redaction-names-and-gate story:spec-standalone-types src/run.rs
collision: story:redaction-names-and-gate story:structured-source src/run.rs
collision: story:redaction-names-and-gate story:timer-runs-unattended src/run.rs (inferred)
collision: story:release-1-0 story:release-pipeline CHANGELOG.md (inferred)
collision: story:release-1-0 story:run-snapshots Cargo.lock
collision: story:release-1-0 story:run-snapshots Cargo.toml
collision: story:release-pipeline story:web-pages-cited-as-urls AGENTS.md
collision: story:run-gate story:run-snapshots src/lib.rs
collision: story:run-gate story:run-snapshots src/run.rs
collision: story:run-gate story:spec-standalone-types src/run.rs
collision: story:run-gate story:structured-source src/lib.rs
collision: story:run-gate story:structured-source src/run.rs
collision: story:run-gate story:timer-runs-unattended src/run.rs (inferred)
collision: story:run-snapshots story:seen-documents-modelled generated
collision: story:run-snapshots story:seen-documents-modelled spec/domains/instance.yaml
collision: story:run-snapshots story:seen-documents-modelled spec/suite.json
collision: story:run-snapshots story:seen-documents-modelled src/home.rs
collision: story:run-snapshots story:spec-standalone-types generated
collision: story:run-snapshots story:spec-standalone-types spec/domains/instance.yaml
collision: story:run-snapshots story:spec-standalone-types spec/suite.json
collision: story:run-snapshots story:spec-standalone-types src/home.rs (inferred)
collision: story:run-snapshots story:spec-standalone-types src/main.rs
collision: story:run-snapshots story:spec-standalone-types src/run.rs
collision: story:run-snapshots story:spec-standalone-types tests/conformance.rs (inferred)
collision: story:run-snapshots story:store-backend-per-instance src/main.rs
collision: story:run-snapshots story:store-backend-per-instance src/schedule.rs
collision: story:run-snapshots story:structured-source src/lib.rs
collision: story:run-snapshots story:structured-source src/run.rs
collision: story:run-snapshots story:timer-runs-unattended src/home.rs (inferred)
collision: story:run-snapshots story:timer-runs-unattended src/run.rs (inferred)
collision: story:run-snapshots story:timer-runs-unattended src/schedule.rs
collision: story:run-snapshots story:web-pages-cited-as-urls src/main.rs
collision: story:run-snapshots story:web-pages-cited-as-urls tests/conformance.rs
collision: story:seen-documents-modelled story:spec-standalone-types generated
collision: story:seen-documents-modelled story:spec-standalone-types spec/domains/instance.yaml
collision: story:seen-documents-modelled story:spec-standalone-types spec/suite.json
collision: story:seen-documents-modelled story:spec-standalone-types src/home.rs (inferred)
collision: story:seen-documents-modelled story:timer-runs-unattended src/home.rs (inferred)
collision: story:seen-documents-modelled story:timer-runs-unattended src/state.rs (inferred)
collision: story:spec-standalone-types story:store-backend-per-instance src/main.rs
collision: story:spec-standalone-types story:structured-from-files-and-drops src/sources.rs (inferred)
collision: story:spec-standalone-types story:structured-source src/run.rs
collision: story:spec-standalone-types story:structured-source src/sources.rs (inferred)
collision: story:spec-standalone-types story:timer-runs-unattended src/home.rs (inferred)
collision: story:spec-standalone-types story:timer-runs-unattended src/ports.rs (inferred)
collision: story:spec-standalone-types story:timer-runs-unattended src/run.rs (inferred)
collision: story:spec-standalone-types story:web-pages-cited-as-urls src/extract.rs
collision: story:spec-standalone-types story:web-pages-cited-as-urls src/main.rs
collision: story:spec-standalone-types story:web-pages-cited-as-urls tests/common/mod.rs (inferred)
collision: story:spec-standalone-types story:web-pages-cited-as-urls tests/conformance.rs (inferred)
collision: story:spec-standalone-types story:web-pages-cited-as-urls website/docs/commands.md (inferred)
collision: story:store-backend-per-instance story:structured-from-files-and-drops src/ekr.rs
collision: story:store-backend-per-instance story:timer-runs-unattended src/schedule.rs
collision: story:store-backend-per-instance story:web-pages-cited-as-urls .github/workflows/check.yml (inferred)
collision: story:store-backend-per-instance story:web-pages-cited-as-urls src/main.rs
collision: story:structured-from-files-and-drops story:structured-source src/sources.rs
collision: story:structured-from-files-and-drops story:structured-source src/structured.rs (inferred)
collision: story:structured-source story:timer-runs-unattended src/run.rs (inferred)
10 wave(s), 137 collision(s), 0 unassessed
```

## Outcome

Closed 2026-10-05. Both units merged into `wave/20261005a`; `task check` on `1caa195` EXIT=0, 45 tests (`~/.cache/cortex-wave-20261005a/int-gate.log`).

| unit | story | commit | merge | adversary passes (findings) | ledger pass 1 → 2 |
|---|---|---|---|---|---|
| U1 | `story:spec-standalone-types` | `b628e4d` | `1caa195` | 2 (3, then 6) | carried 0, new 6, resolved 3 |
| U2 | `story:release-pipeline` | `2a047ee` | `1e02578` | 2 (5, then 3 notes) | carried 0, new 3, resolved 5 |

- U2's pass-2 notes were fixed by the coordinator (one line each in `release.yml`, `CHANGELOG.md`, `AGENTS.md`) and re-checked with `check-release.sh` (0 failed) and the pass-2 `attack.sh` (1 of 23 red: `hostpath`, now documented in `AGENTS.md` as a property of an independent rebuild).
- U1's final correction was checked by the coordinator: 7 `src/run.rs` mutants from pass 2 each fail the unit's own suite (`spec/coord-verify/mutant-*.log`, EXIT=101 each).
- Carried: `website/docs/commands.md` patch → `story:docs-for-1-0` section "Carried from wave 20261005a".
- Not yet verified: the release workflow's first real run (`workflow_dispatch` after merge to `main`).

Agent cost (from the harness's per-agent usage; passes before the session's context compaction were not kept):

| agent | tokens | tool uses | duration |
|---|---|---|---|
| U2 correction 1 | 117 484 | 22 | 196 s |
| U2 adversary 2 | 93 792 | 22 | 748 s |
| U1 adversary 1 | 135 769 | 52 | 403 s |
| U1 correction 1 | 229 497 | 30 | 210 s |
| U1 adversary 2 | 153 910 | 61 | 507 s |
| U1 correction 2 | 253 533 | 14 | 137 s |
| U1, U2 implementation, U2 adversary 1 | not recorded | | |
