---
format: aep.planning-md/3
id: story:postgres-credential-from-connectors
kind: story
status: draft
title: A PostgreSQL store's credential comes from a Connectors connection
relations:
- decomposes: epic:organisation-scale-instance
- depends_on: story:store-backend-per-instance
- serves: vision:self-updating-instances
revision: 1
---
## Outcome

A PostgreSQL-backed instance gets its database credential from a Connectors connection held in Connectors' secret backends. Neither cortex nor any file cortex writes ever holds the password.

## Why

Decided for the cb3 move (cb3 `initiative:run-on-cortex`, "Decided": "Database credentials for the PostgreSQL store come from a Connectors connection held in Connectors' secret backends; cortex never sees them", operator, 2026-10-05). `story:store-backend-per-instance` (wave 20261005c) shipped the backend with file paths only. EKR 0.0.30 reads the password from the connection file that the `ekr.postgres/1` file names. cortex never reads that file, and a test checks that the password appears nowhere cortex writes. The channel itself was not designed.

## Work

1. Design how `ekr` obtains the credential at process start from Connectors, without cortex in the path. Two options: an `ekr.postgres/1` variant naming a Connectors connection, or a Connectors-launched `ekr`. Record the decision in this story.
2. The EKR change and any Connectors change it needs go in their own repositories' stories. This story wires cortex to it.

## Acceptance

The docker PostgreSQL case in `tests/store_backend.rs` runs with the credential held only by a stand-in Connectors connection. No file under the instance home, no unit file and no process argv cortex starts contains the password.
