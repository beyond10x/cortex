---
format: aep.planning-md/3
id: decision-blocker:codex-exec-keeps-shell
kind: decision-blocker
status: open
title: codex exec cannot run without a shell tool, so the Codex backend breaks the no-tools rule
relations:
- blocks: story:codex-model-backend
withholds: test_result
revision: 1
---
## What stops the story

The unit built in wave 20261005c (green: `task check` EXIT=0, `tests/codex.rs` 2 cases) runs `codex exec --sandbox read-only …` as the story's Acceptance writes it. That call keeps Codex's shell tool: the `read-only` sandbox blocks writes, not reads. A fetched document carrying instructions can make the model read local files, including `$CODEX_HOME` credentials, into its answer, and the answer is applied to the store. cortex's `AGENTS.md` rule is "a model that has no tools".

Checked 2026-10-05 against codex-cli 0.160.0: `codex exec --help` offers `--sandbox`, `--disable <FEATURE>` and `-c key=value`; `codex features list` shows no feature that removes the shell tool (only `shell_snapshot`, `powershell_shell_version`, browser, apps and computer-use features).

## What would clear it

One of:

1. A codex-cli release or config key that runs `codex exec` with no tools at all, shown by a test in which the stand-in is replaced by the real binary and asked to run a shell command (it must refuse or have no such tool).
2. A process sandbox around `codex exec` in which the model can read nothing but the empty working directory, with the credential supplied so that the model cannot read it (not a file in a readable `$CODEX_HOME`), shown the same way.

## Where the work is

The unit's change is archived, not merged: `worktree archive` of `cortex-w2-codex` (branch `unit/codex-model-backend`, base `6352006`), with its logs and the docs patch in `~/.cache/cortex-wave-20261005c/codex/`. It can be restored and rebased once this clears.

Decided by the coordinating session on 2026-10-05 under the operator's standing wave approval; the cb3 goal does not depend on it (`initiative:run-on-cortex`: extraction uses tool-less batch calls).
