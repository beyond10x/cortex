---
format: aep.planning-md/3
id: upstream-blocker:ekr-postgres-rewind
kind: upstream-blocker
status: open
title: EKR cannot put a PostgreSQL store's head back
relations:
- blocks: story:postgres-run-undo
revision: 1
---
## What would clear it

An EKR release with a command that puts a PostgreSQL store's head back to an earlier revision (revision number and root) in one database transaction, refusing when the head is no longer where the caller expects (for example `ekr rewind --to <rev> --expect-head <rev>`), or a restore point it can create and restore. It must run through the Connectors launch like every other `ekr` of a connected store, so cortex never holds the credential.

## Found

2026-10-07, scoping `story:postgres-run-undo` against EKR 0.0.32: `ekr --help` lists no revert, rewind, reset or restore-point command; `ekr migrate --help` says "PostgreSQL sources are refused"; every `ekr operations` kind (`RetractAssertion`, `SupersedeAssertion`, `DeleteEdge`) is a new transaction that moves the head forward. The story's acceptance (`ekr head` equal to the head before a gated-out run) cannot hold on PostgreSQL without it.
