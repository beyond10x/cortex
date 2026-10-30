---
format: aep.planning-md/3
id: story:snapshot-writes-synced
kind: story
status: draft
title: Snapshot files are synced before rename and survive a crash
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
revision: 2
---
## Outcome

Snapshot files survive a crash: every snapshot write and rename is synced, as `write_atomic` is
since `story:durable-home-writes`. Today snapshot state and store copies are written and renamed
without fsync, and a restore depends on them.

## Acceptance

- Each snapshot file write calls `sync_all` before its rename, and the snapshot directory is synced
  after the rename (unit test through a seam, or code read in review if no seam exists; say which).
- The snapshot tests in `tests/snapshots.rs` stay green.

## Source

- `review-result:adversary-w30a-cli-pass-1`, judgement J3 (`src/snapshot.rs:192` at that pass's
  head).
- Wave 20261030a coordinator review of the unit reports.
