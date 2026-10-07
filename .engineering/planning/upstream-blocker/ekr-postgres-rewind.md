---
format: aep.planning-md/3
id: upstream-blocker:ekr-postgres-rewind
kind: upstream-blocker
status: open
title: EKR cannot put a PostgreSQL store's head back
relations:
- blocks: story:postgres-run-undo
revision: 2
---
## What would clear it

An EKR release with staged runs: a stage forked from the base at the head, every one-shot verb opening it with `EKR_STAGE=<id>` or `--stage <id>` (`ontology`, `snapshot`, `quality`, `propose`, `validate`, `commit`, `apply-extraction`), `ekr stage publish <id> --expect-head <rev>` and an abandon. The design is chosen (2026-10-07; `story:postgres-run-undo` § Design); the release is not out. A rewind command is not coming: EKR keeps committed revisions.

## Found

2026-10-07, scoping `story:postgres-run-undo` against EKR 0.0.32: `ekr --help` lists no revert, rewind, reset or restore-point command; `ekr migrate --help` says "PostgreSQL sources are refused"; every `ekr operations` kind (`RetractAssertion`, `SupersedeAssertion`, `DeleteEdge`) is a new transaction that moves the head forward. The story's acceptance (`ekr head` equal to the head before a gated-out run) cannot hold on PostgreSQL without it.
