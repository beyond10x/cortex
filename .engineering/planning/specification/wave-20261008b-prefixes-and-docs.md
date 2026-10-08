---
format: aep.planning-md/3
id: specification:wave-20261008b-prefixes-and-docs
kind: specification
status: implemented
title: 'Wave 20261008b: parent identity prefixes and the 1.0 documentation'
relations:
- specifies: story:parent-identity-prefix-unmasked
- specifies: story:docs-for-1-0
revision: 5
transitions:
- {from: "draft", to: "in_review", at: "2026-10-08T13:59:06Z", actor: "human:timo", revision: 3}
- {from: "in_review", to: "approved", at: "2026-10-08T13:59:07Z", actor: "human:timo", revision: 4}
- {from: "approved", to: "implemented", at: "2026-10-08T13:59:07Z", actor: "human:timo", revision: 5}
---
## Wave 20261008b: parent identity prefixes and the 1.0 documentation

Opened 2026-10-08, `aep:implementing` 0.21.2 wave mode. Two units dispatched with wave 20261008a and finished here, on a branch from `main` after 0.2.1. Coordinator changes in the same pull request: the documentation site's `wait` job, the Use cases link and the identity sentence in the docs, and the 7-day run's restart record.

## Units

| unit | story | branch | worktree | scratch |
|---|---|---|---|---|
| U2 | `story:parent-identity-prefix-unmasked` | `unit/parent-identity-prefix-unmasked` | `cortex-w21-b` | `~/.cache/cortex-wave-20261008a/prefix` |
| U3 | `story:docs-for-1-0` | `unit/docs-for-1-0` | `cortex-w21-c` | `~/.cache/cortex-wave-20261008a/docs` |

Integration branch `wave/20261008b`, tree `cortex-w22-int`. Each tree builds into its own `target/`.

## Commits approval authorises

Unit and correction commits through `b10x-gates bot`; the merges into `wave/20261008b`; coordinator commits for the store, the changelog, the docs and the workflow; the pull request into `main` and its merge.

## Outcome

U2: `ea8b58e`, corrections `4e71a41` and `8d0a8b6`, merged `4d46b1c`. Adversary pass 1 found 3, all fixed in `4e71a41`. Pass 2 found 3 new ones, 0 carried; they were fixed in `8d0a8b6`, and the coordinator read that correction (no committed assertion changed). U3: `5d2e462`, merged `197ceab`; no adversary pass (documentation).

The documentation site failed on `main` at `e26f293` and `9226bec`. Its build-run lookup (the shared `project-site.yml`, one query, no retry) ran 11 s after the build finished and found none; the same query lists the run now. The `wait` job polls that query up to 30 times, 10 s apart, before `publish`.

### Gate

Integration gate on `wave/20261008b` at `4d46b1c` plus the coordinator's uncommitted edits (2026-10-08 13:55Z, log `~/.cache/cortex-wave-20261008a/gateb/gate.log`): fmt=0, drift=0, docs-check=0, clippy=0, test=0 (`cargo test -p cortex-cli -p cortex-docs`, 47 result lines, 555 passed, 0 failed), website=0. All 555 test names printed are listed by `cargo test -- --list` in the gated tree.

### Agent cost

| agent | tokens |
|---|---|
| U2 implementor, 4 rounds | ~427k |
| U2 adversary pass 1 | 82k |
| U2 adversary pass 2 | 80k |
| U3 implementor | 135k |
