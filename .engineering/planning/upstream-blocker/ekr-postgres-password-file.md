---
format: aep.planning-md/3
id: upstream-blocker:ekr-postgres-password-file
kind: upstream-blocker
status: open
title: EKR takes no PostgreSQL password from a separate file
relations:
- blocks: story:postgres-credential-from-connectors
withholds: test_result
revision: 1
---
## What would clear it

An EKR release in which an `ekr.postgres/1` configuration takes its password from a separate file (`password_file`), EKR `story:postgres-password-file`. The acceptance test of `story:postgres-credential-from-connectors` runs a real `ekr` that reads the password from `/proc/self/fd/3`. EKR 0.0.31 has no `password_file`; found when wave 20261006e opened, so the story left the wave with no work done.
