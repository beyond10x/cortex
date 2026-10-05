---
format: aep.planning-md/3
id: specification:wave-20261005c-cortex-wave-2
kind: specification
status: implemented
title: 'Wave 20261005c: cortex 1.0 wave 2'
revision: 6
transitions:
- {from: "draft", to: "in_review", at: "2026-10-05T11:54:37Z", actor: "agent:claude", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-05T11:54:37Z", actor: "agent:claude", revision: 3}
- {from: "approved", to: "implemented", at: "2026-10-05T13:01:19Z", actor: "agent:claude", revision: 5}
---
## Wave 20261005c: cortex 1.0 wave 2

Opened 2026-10-05 by the coordinating session, `aep:implementing` 0.19.2 wave mode. It is computed wave 2 of `aep plan artifact waves --kind story` (2026-10-05).

**Approval:** the operator approved every wave up front on 2026-10-05: "I approve all waves upfront and now. do not ask for permission, you orchestrate this".

## Units

| unit | story | branch | worktree | build dir | scratch |
|---|---|---|---|---|---|
| U1 | `story:store-backend-per-instance` | `unit/store-backend-per-instance` | `cortex-w2-store` | `~/.cache/b10x-target/cortex-w2-store` | `~/.cache/cortex-wave-20261005c/store` |
| U2 | `story:seen-documents-modelled` | `unit/seen-documents-modelled` | `cortex-w2-seen` | `~/.cache/b10x-target/cortex-w2-seen` | `~/.cache/cortex-wave-20261005c/seen` |
| U3 | `story:redaction-before-model` | `unit/redaction-before-model` | `cortex-w2-redact` | `~/.cache/b10x-target/cortex-w2-redact` | `~/.cache/cortex-wave-20261005c/redact` |
| U4 | `story:codex-model-backend` | `unit/codex-model-backend` | `cortex-w2-codex` | `~/.cache/b10x-target/cortex-w2-codex` | `~/.cache/cortex-wave-20261005c/codex` |

## Selection

- The four stories' typed scopes share no file (`aep plan artifact show <id> --format json`, 2026-10-05).
- U1, U2 and U3 serve the cb3 goal `initiative:run-on-cortex` (PostgreSQL store, seen-state, redaction); U4 completes the model backend that wave 20261005a left refusing.
- At most two adversary passes per unit.
- EKR wave 20261005b runs at the same time with its own build directories.

## Commits approval authorises

One commit per unit through `b10x-gates bot`; the merges into `wave/20261005c`; the closing planning-store commit; the pull request into `main` and its merge.

## Outcome

Closed 2026-10-05. Three of four units merged into `wave/20261005c`; `task check` on the integration branch: EXIT=0, 146 tests (`~/.cache/cortex-wave-20261005c/int/gate-u3.log`).

| unit | story | commit | merge | adversary findings, pass 1 → 2 |
|---|---|---|---|---|
| U1 | `story:store-backend-per-instance` | `b6f6fd6` | — | 6 → 2 |
| U2 | `story:seen-documents-modelled` | `da7cfa7` | — | 8 (by reading) → 6 |
| U3 | `story:redaction-before-model` | `1a89081` | — | 14 → 10 |
| U4 | `story:codex-model-backend` | archived, not merged | — | none run |

- U4 left the wave: `codex exec` keeps a shell tool that can read local files, against the no-tools rule (`decision-blocker:codex-exec-keeps-shell`). Its tree is archived by `worktree archive`.
- Operator decision during the wave: PII stays out of the model, not out of the store (`story:redaction-before-model`).
- Filed: `story:postgres-credential-from-connectors`, `story:credential-mask-covers-titles`, `story:symlinked-seed-directory`; EKR `story:seed-if-absent`; beyond10x/ess#459, #461.
- The disk filled to 0 bytes once during the wave (EKR wave 20261005b build directories, 41 GB); U1's correction and U2's first adversary pass were cut short and re-run.
- Docs, CHANGELOG and status page: `24ecfd8`.
- Not yet verified: the PostgreSQL docker case on GitHub Actions (the pull request's CI run).

The branch was rebuilt as `wave/20261005c-r2` with one commit per unit: gitleaks read a fake test password in `tests/store_backend.rs` as a credential, and the pull-request check scans every commit. The first build stays on `wave/20261005c`.
