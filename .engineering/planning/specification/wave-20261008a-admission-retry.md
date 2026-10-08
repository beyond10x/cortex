---
format: aep.planning-md/3
id: specification:wave-20261008a-admission-retry
kind: specification
status: implemented
title: 'Wave 20261008a: admission retry for the 7-day run'
relations:
- specifies: story:connectors-admission-timeout-retry
revision: 5
transitions:
- {from: "draft", to: "in_review", at: "2026-10-08T13:32:26Z", actor: "human:timo", revision: 3}
- {from: "in_review", to: "approved", at: "2026-10-08T13:32:26Z", actor: "human:timo", revision: 4}
- {from: "approved", to: "implemented", at: "2026-10-08T13:32:26Z", actor: "human:timo", revision: 5}
---
## Wave 20261008a: admission retry for the 7-day run

Opened 2026-10-08, `aep:implementing` 0.21.2 wave mode. N is one: the 7-day run restarts on a release with this fix before its next timer run, so the unit ships alone. `story:parent-identity-prefix-unmasked` and `story:docs-for-1-0` were dispatched at the same time on their own branches and close in wave 20261008b.

Selection: `aep plan artifact waves` placed all three in separate surfaces (`src/connectors.rs`; `src/structured.rs`; `website/docs/`, `README.md`); the scope lines are inferred from the story bodies.

## Units

| unit | story | branch | worktree | build dir | scratch |
|---|---|---|---|---|---|
| U1 | `story:connectors-admission-timeout-retry` | `unit/connectors-admission-timeout-retry` | `cortex-w21-a` | `cortex-w21-a/target` | `~/.cache/cortex-wave-20261008a/retry` |

Integration branch `wave/20261008a`, tree `cortex-w21-int`. Wave 20261008b units: U2 `cortex-w21-b` (`unit/parent-identity-prefix-unmasked`), U3 `cortex-w21-c` (`unit/docs-for-1-0`).

## Rules

- One implementor (`aep:implementor`), one adversary pass (`aep:adversary`) on U1.
- Package gates of `cortex-cli` on the exact commit before the push; the pull request's CI runs the full gate.

## Commits approval authorises

The unit commit and any adversary or fix commit through `b10x-gates bot`; the merges into `wave/20261008a`; coordinator commits for the store and the changelog; the pull request into `main` and its merge. The release 0.2.1 follows through the release process.

## Outcome

U1 `story:connectors-admission-timeout-retry`: `717842a`, adversary cases and a doc fix `8e2bcee`, format `652d25e`, merged `0e4bccc`. Adversary pass 1 found nothing (`review-result:adversary-connectors-admission-timeout-retry-pass-1`).

Package steps on `0e4bccc` (ess 0.55.0, EKR 0.0.32), each exit 0: `cargo fmt -p cortex-cli -p cortex-docs --check`, `task drift`, `cargo clippy -p cortex-cli --all-targets -- -D warnings`, `cargo test -p cortex-cli --no-fail-fast`: 43 result lines, 535 passed, 0 failed; both retry test files ran in this tree. The pull request's CI runs the full gate.

The `ess` on the machine's PATH is 0.56.0; with it `task generate` refuses the 0.55.0 pin and the clap ⇔ spec test fails. The units and this gate ran with 0.55.0 first on PATH.

| agent | tokens | tool uses | wall time |
|---|---|---|---|
| implementor U1 (2 rounds) | 101,821 | 39 | 859 s |
| adversary U1 | 50,998 | 12 | 295 s |
