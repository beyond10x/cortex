---
format: aep.planning-md/3
id: upstream-blocker:ekr-postgres-password-file
kind: upstream-blocker
status: cleared
title: EKR takes no PostgreSQL password from a separate file
relations:
- blocks: story:postgres-credential-from-connectors
withholds: test_result
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-07T02:24:47Z", actor: "human:timo", revision: 3}
---
## What would clear it

An EKR release in which an `ekr.postgres/1` configuration takes its password from a separate file (`password_file`), EKR `story:postgres-password-file`. The acceptance test of `story:postgres-credential-from-connectors` runs a real `ekr` that reads the password from `/proc/self/fd/3`. EKR 0.0.31 has no `password_file`; found when wave 20261006e opened, so the story left the wave with no work done.

## Cleared 2026-10-07

EKR 0.0.32 (released 2026-10-07T02:12:55Z, https://github.com/beyond10x/epistemic-knowledge-runtime/releases/tag/0.0.32): "An `ekr.postgres/1` configuration takes an optional `password_file` holding exactly `{"password": "..."}` ... An absolute path such as `/proc/self/fd/3` is used as written" (its release notes, § Added). cortex pins 0.0.32 on `wave/20261006i` (`f84c401`).
