---
format: aep.planning-md/3
id: story:create-rollback-drops-postgres-lineage
kind: story
status: draft
title: A rolled-back create on PostgreSQL leaves no seeded lineage, so the retry succeeds
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
revision: 2
---
## Outcome

A `cortex create` on a PostgreSQL store that rolls back after `seed_instance` leaves no seeded
lineage, so the retry the docs promise (`website/docs/operating.md:15`, "so it can be retried")
succeeds. Today the rollback removes the instance directory but leaves the lineage, and a retry
answers "tenant already holds a store".

## Acceptance

- On a PostgreSQL store, a `create` that fails after seeding (for example a failing `save_meta`)
  leaves no lineage for the tenant, and a second `create` with the same spec exits 0 (new test
  against a PostgreSQL instance, or a seam test that observes the drop call; say which).
- A rollback that cannot drop the lineage reports that it is left, naming the tenant, instead of
  claiming the create can be retried.

## Source

- `review-result:adversary-w30a-cli-pass-1`, judgement J1 (`src/main.rs:932`), INFEASIBLE in that
  pass: no PostgreSQL run.
- Wave 20261030a coordinator review of the unit reports.
