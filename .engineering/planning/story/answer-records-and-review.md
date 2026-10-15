---
format: aep.planning-md/3
id: story:answer-records-and-review
kind: story
status: draft
title: Every answer is kept with its votes, and the failed ones are queued for review
summary: Answer records, one changeable vote per identity, a review queue and retention in cortex's own tables
relations:
- decomposes: epic:instance-answers-questions
- serves: vision:self-updating-instances
- depends_on: architecture-decision-record:answer-service-in-cortex
- depends_on: story:ask-service
revision: 1
---
## Outcome

Every answer `cortex ask serve` gives is kept as an answer record with its tool calls, latency,
tokens and outcome. A reader casts one vote per answer and may change it (`POST /ask/{id}/vote
{value, comment?}`). An answer voted down, failed or slower than the spec's threshold becomes a
review item; `GET /review` lists the open ones and `POST /review/{id}/close` closes one with a
reason. Records older than the spec's `retention_days` are purged with their votes and review
items.

**No implementation starts until the operator accepts
`architecture-decision-record:answer-service-in-cortex`.**

## Why

A downstream deployment keeps this in a PostgreSQL schema of its own beside the store, with its own
migrate and purge commands. Verified in this tree: cortex owns no tables; its state is files under
the instance directory (`website/docs/operating.md`, instance directory table), and a PostgreSQL
store is EKR's, opened only through `ekr` (`spec/domains/instance.yaml`,
`cortex.instance.PostgresStore`). `Cargo.toml` has `rusqlite` and no PostgreSQL client. Where the
tables live is the record's decision 2: a separate `ask.sqlite` in the instance directory, or a
schema of their own (not EKR's) on PostgreSQL.

## Domain relations

New entities in `spec/domains/ask.yaml`:

- `cortex.ask.AnswerRecord` (identity `answer_id`, a UUIDv7): `instance_name`, `identity`,
  `question`, `answer`, `tool_calls`, `latency_ms`, token counts, `outcome`
  (`answered | failed | limited`), `at`. Relation: `cortex.instance.Instance` owns many
  `AnswerRecord` via `instance_name`.
- `cortex.ask.Vote` (identity: `answer_id` and `identity`): `value` (`up | down`), optional
  `comment`. Relation: `AnswerRecord` owns many `Vote` via `answer_id`, at most one per identity.
- `cortex.ask.ReviewItem` (identity `review_id`): `answer_id`, `cause` (`voted-down | failed |
  slow`), lifecycle `Open → Closed` with a close reason. Relation to `AnswerRecord`:
  `UNMAPPED: one review item per answer, or one per cause` (cardinality not settled).

## Work

- Spec first: the three entities, the commands `Vote`, `CloseReview`, `PurgeExpired`, the view
  `OpenReviews`, and `retention_days` and `slow_after_ms` in `AskSpec`; validate, conformance,
  `task generate`. Then the code.
- Storage as the record decides, with a schema version and a migration on start.
- The vote and review endpoints in the service; a purge on a timer (or on start and daily).
- Docs: the service page; `operating.md`'s instance directory table gains `ask.sqlite`.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- After one answer, the store of records holds one `AnswerRecord` whose token counts equal those of
  the `done` event.
- One identity votes `down` then `up` on one answer: one `Vote` remains, `up`, and no new
  `ReviewItem` is opened by the second vote.
- A `down` vote, a failed answer and an answer slower than `slow_after_ms` each open one
  `ReviewItem` with the matching `cause`; `GET /review` lists them; closing one removes it from the
  list.
- With `retention_days: 1`, a record dated two days ago is gone after a purge, with its votes and
  review items; one dated today stays.
- No table is created in EKR's schema (checked on a PostgreSQL test store by listing the tables of
  EKR's schema before and after).

## Not in this story

- The service and its stream (`story:ask-service`).
- Turning reviewed answers into eval cases (`story:eval-cases-from-review`).
- Counts and topics over the records (`story:vitals-and-topics`).
