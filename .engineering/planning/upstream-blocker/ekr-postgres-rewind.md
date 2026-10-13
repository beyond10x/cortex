---
format: aep.planning-md/3
id: upstream-blocker:ekr-postgres-rewind
kind: upstream-blocker
status: cleared
title: EKR cannot put a PostgreSQL store's head back
relations:
- blocks: story:postgres-run-undo
revision: 4
transitions:
- {from: "open", to: "cleared", at: "2026-10-13T10:32:51Z", actor: "human:timo", revision: 4}
---
## What would clear it

An EKR release with staged runs: a stage forked from the base at the head, every one-shot verb opening it with `EKR_STAGE=<id>` or `--stage <id>` (`ontology`, `snapshot`, `quality`, `propose`, `validate`, `commit`, `apply-extraction`), `ekr stage publish <id> --expect-head <rev>` and an abandon. The design is chosen (2026-10-07; `story:postgres-run-undo` § Design); the release is not out. A rewind command is not coming: EKR keeps committed revisions.

## Found

2026-10-07, scoping `story:postgres-run-undo` against EKR 0.0.32: `ekr --help` lists no revert, rewind, reset or restore-point command; `ekr migrate --help` says "PostgreSQL sources are refused"; every `ekr operations` kind (`RetractAssertion`, `SupersedeAssertion`, `DeleteEdge`) is a new transaction that moves the head forward. The story's acceptance (`ekr head` equal to the head before a gated-out run) cannot hold on PostgreSQL without it.

## Cleared

EKR 0.0.33 (https://github.com/beyond10x/epistemic-knowledge-runtime/releases/tag/0.0.33, published 2026-10-08) ships staged runs, read on 2026-10-13 from a binary built from the tag (`cargo install --locked --git … --tag 0.0.33 ekr`, `ekr 0.0.33`):

- `ekr stage --help` lists `begin`, `publish`, `abandon` and `list`; `publish` takes `--expect-head <revision>` and lands every revision of the run at once or none; an abandoned stage leaves `ekr head` where it was. The help names SQLite and PostgreSQL stores; the release notes add that a stage can be begun from a PostgreSQL store.
- `--stage` is in the `--help` of each verb this blocker names: `ontology`, `snapshot`, `quality`, `propose`, `validate`, `commit`, `apply-extraction` (one match each).

`story:postgres-run-undo` can now be built on the chosen design against EKR 0.0.33. Not yet checked: a stage begun and abandoned on a live PostgreSQL store; that is the story's own acceptance.
