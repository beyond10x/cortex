---
title: Ask service
sidebar_position: 6
description: cortex ask serve answers questions over an instance's store as a stream of events.
---

# Ask service

`cortex ask serve --spec <instance spec>` answers questions over the store of the instance the spec
names. It is a long-running process of its own: `POST /ask` answers `text/event-stream`, one event
per step of a tool loop over the EKR MCP reader (`ekr mcp`, as [`cortex mcp-line`](./commands.md)
starts it) against the instance's model gateway. It reads the store only through the reader and
writes nothing to it. Each answer is streamed, and kept as a record in cortex's own file beside the
store, where readers vote on it and a review queue collects the answers to look at
([Answer records, votes and review](#answer-records-votes-and-review)). The specification is the domain
[`cortex.ask`](./reference/ess/cortex-ask.md).

## The `ask` section

The service is configured by the optional `ask` section of the instance spec
(`cortex.ask.AskSpec`), and its model is the spec's `model.gateway` and `model.model`:

```yaml
model:
  model: my-model
  budget_tokens: 2000000
  timeout_s: 120
  gateway: {base_url: "https://gateway.example", token_env: GATEWAY_TOKEN}
ask:
  listen: "127.0.0.1:8088"
  identity_header: x-asker
  limits: {max_steps: 8, max_result_bytes: 16384, max_seconds: 120, per_identity_per_minute: 10}
  link_templates: {node: "https://view.example/node/{id}", evidence: "https://view.example/evidence/{id}"}
  guidance: Answer in English, in at most five sentences.
  retention_days: 90
  slow_after_ms: 20000
```

| field | meaning |
|---|---|
| `listen` | the address the service binds, `host:port`; port `0` takes a free one |
| `identity_header` | the request header that carries the asker's identity, set by the guard in front of the service |
| `limits.max_steps` | the most model calls of one answer |
| `limits.max_result_bytes` | the most bytes of one tool result the model is shown |
| `limits.max_seconds` | the most seconds one answer takes, every model and tool call included |
| `limits.per_identity_per_minute` | the most questions of one identity in a sliding minute |
| `link_templates` | optional: the URL of a cited node or evidence, `{id}` replaced by its percent-encoded id |
| `guidance` | optional: text added to the service's system prompt |
| `retention_days` | optional: how many days an answer record is kept, with its votes and review items (default 90) |
| `slow_after_ms` | optional: an answer that took longer opens a `slow` review item; without it none does |
| `records` | required on a `postgres` store, refused on a `sqlite` one: the PostgreSQL schema of the answer records ([Records on a PostgreSQL store](#records-on-a-postgresql-store)) |

`cortex ask serve` refuses to start (exit 2, naming the field) a spec with no `ask` section, no
`model.gateway`, a limit, `retention_days` or `slow_after_ms` of 0 or less, an `identity_header`
that is no header name, or `ask.records` that does not fit the store (see
[Records on a PostgreSQL store](#records-on-a-postgresql-store)). It then opens its records
(exit 1 when it cannot) and purges the expired ones. Once bound it prints one line,
`{"listening": "<address>"}`, and serves until it is stopped.

:::note[Identity]
cortex verifies no token. The guard in front of the service verifies the asker and hands the
identity on in `identity_header`. A request without it, or with an empty one, answers 401 and asks
no model. Bind `listen` where only that guard reaches it.
:::

## The model route

The gateway is spoken to as an OpenAI-compatible endpoint: `POST <base_url>/v1/chat/completions`
(`<base_url>/chat/completions` when `base_url` already ends in `/v1`), with the value of the
variable `token_env` names as a bearer token, read when an answer starts. A missing or empty token
fails the answer with `model-failed`, naming the variable. The reader does not inherit that
variable, nor any other `ANTHROPIC_*` or `CLAUDE_*` variable: it reads a store and gets no model
credential. A `max_seconds` above ten years counts as ten years.

## The event contract

The contract is `cortex.ask-events/1` (`cortex.ask.AskEvent`). Each SSE event's `event:` line is
the event's name and its `data:` line the event as one JSON object, `{"event": <name>, "value":
<fields>}`:

| event | fields | when |
|---|---|---|
| `status` | `format` (`cortex.ask-events/1`), `step`, `message` | first, then before each model call |
| `tool` | `step`, `name`, `arguments`, `result_bytes`, `truncated`, `failed`, `withheld` | after each tool call the model asked for |
| `delta` | `text` | a piece of the model's text, in order |
| `discard` | `step` | withdraws the `delta`s since the last `discard`: text before a tool call, not the answer |
| `done` | `answer_id`, `answer`, `nodes`, `evidence`, `input_tokens`, `output_tokens`, `cached_tokens`, `context_tokens`, `steps`, `unrestored` | the last event of an answer |
| `error` | `reason`, `message`, `retry_after` (with `rate-limit` only) | the last event of a request with no answer |

`done.answer` is the answer with each citation marker, `[node:<id>]` or `[evidence:<id>]`, turned
into a Markdown link from `link_templates` (left as written without a template); `nodes` and
`evidence` are the cited ids, each once, in order. The token counts add up the gateway's `usage`
over every model call (`prompt_tokens`, `completion_tokens`, `prompt_tokens_details.cached_tokens`;
0 where not reported); `context_tokens` is the prompt tokens of the last call. `answer_id` is the
answer's UUIDv7, the id its record and its votes carry.

`error.reason` is one of:

| reason | status | meaning |
|---|---|---|
| `unidentified` | 401 | no identity header; no model was asked |
| `rate-limit` | 429 | the identity's questions of the minute are spent; `retry_after` (also the `Retry-After` header) is the whole seconds until the oldest leaves the window |
| `bad-request` | 400, or 413 | the body is not JSON with a non-empty `question`, or is above 1 MiB (413), or the question still holds a class `refuse_if_left` names (400); a refused body spends no question of the minute |
| `step-limit` | 200 | the model still asked for a tool at `max_steps` |
| `time-limit` | 200 | the answer passed `max_seconds` |
| `model-failed` | 200 | the gateway could not be reached, answered an error or no message |
| `reader-failed` | 200 | the reader could not be started or stopped answering |

A tool result is rewritten into compact text (each text that is JSON written without whitespace)
and cut at a character boundary to `max_result_bytes`; its `tool` event says whether it was cut.

## Redaction

With a [`redaction`](./spec-file.md#redaction) policy in the spec, the model reads the store only
redacted, as extraction shows it documents. One question is one redaction batch: one placeholder
mapping, held in memory for that answer and written nowhere.

- The question and each tool result are redacted before the model is given them, with the
  policy's classes, `known_names`, `rules` and `rare_limit`: the same value is the same
  placeholder (`[Email-1]`, `[Name-1]`) in every result of the answer, and a name replaced as rare
  stays replaced when a later result repeats it. A result is redacted before it is cut to
  `max_result_bytes`. Credentials and rules with a `replacement` are replaced irreversibly.
- A result in which a class the policy's `refuse_if_left` names is still detected is not given to
  the model: it gets a tool error saying the result was withheld by policy, naming the classes,
  and the `tool` event says `withheld: true`. The answer goes on.
- A question in which a `refuse_if_left` class is still detected, after it is redacted, is refused
  before anything is sent and before it spends a question of the minute: the answer is a 400 with
  an `error` event whose `reason` is `bad-request`, naming the classes and never what was found.
  No model is asked.
- Text in a tool result that already looks like a placeholder (`[Email-1]`) is shown to the model
  as a fresh placeholder that restores to that text, so it is never restored to another value.
  A withheld result adds none of its values to the answer's mapping.
- The answer is restored before the client sees it: each placeholder that has a value is replaced
  by it, in every `delta` and in `done.answer`. A `delta` that ends inside a possible placeholder
  holds that tail back until the next `delta` or `done`, so no client sees half a placeholder. A
  placeholder with no value (one the model changed or invented) stays as written and is counted in
  `done.unrestored`.

Without a policy, the question and the tool results reach the model unchanged, `withheld` is
always false and `unrestored` 0. Detection is by pattern, with the limits that
[Limits](./limits.md) states for extraction.

```text
event: status
data: {"event":"status","value":{"format":"cortex.ask-events/1","message":"starting the reader","step":0}}

event: tool
data: {"event":"tool","value":{"arguments":"{\"query\":\"widget\"}","failed":false,"name":"search","result_bytes":35,"step":1,"truncated":false,"withheld":false}}

event: done
data: {"event":"done","value":{"answer":"…","answer_id":"0192f3a1-7c2e-7d41-9b0a-3f5e6d7c8b9a","cached_tokens":100,"context_tokens":200,"evidence":[],"input_tokens":300,"nodes":["n-42"],"output_tokens":30,"steps":2,"unrestored":0}}
```

## Answer records, votes and review

Every answer the service gives, and every one that failed or hit a limit, is kept as a record
(`cortex.ask.AnswerRecord`) before its last event is sent: the question, the identity, the answer or
the reason there is none, the outcome (`answered`, `failed`, `limited`), the `tool` events, the
latency, the token counts of its `done` event and the time. A request refused before it started
(401, 429, 400) leaves no record. The records are cortex's own, never in the EKR store: on a `sqlite`
store they live in `instances/<name>/ask.sqlite` in the instance directory, on a `postgres` store in
a schema of their own ([below](#records-on-a-postgresql-store)). Either carries its schema version,
migrated when the service starts; one written by a newer cortex is refused. The question, the
answer, the `tool` events' arguments, a vote's comment and a close's
reason are kept with every credential shape masked (`[masked:<shape>]`), whatever the instance's
`redaction` policy; the mask is irreversible.

The record routes need the identity header too (401 without it) and answer JSON; a refusal is
`{"error": <reason>, "message": <text>}`.

| route | body | answers |
|---|---|---|
| `POST /ask/{answer_id}/vote` | `{"value": "up" \| "down", "comment"?: <text>}` | the vote as kept: `vote_id`, `answer_id`, `identity`, `value`, `comment`, `at`. 400 `bad-request` for another value, 404 `no-such-answer` for an id no record carries |
| `GET /review` | none | `{"reviews": [...]}`: every open review item, oldest first, with `review_id`, `answer_id`, `cause` and `opened_at` |
| `POST /review/{review_id}/close` | `{"reason": <text>}` | the item as closed. 400 without a reason, 404 `no-such-review` for an unknown id, 409 `review-not-open` for an item closed already |

An identity has one vote per answer: voting again changes it. An answer opens one review item per
cause, and an item, once opened, stays as it was whatever happens after:

| cause | opened when |
|---|---|
| `voted-down` | a `down` vote is cast on the answer, by anyone |
| `failed` | the answer's outcome is `failed` or `limited` |
| `slow` | the answer took longer than `slow_after_ms` |

A vote or a close that meets a purge or another close keeps to these answers: a vote whose answer
the purge takes answers 404 `no-such-answer` and leaves no vote or review item behind, and of two
closes of one item at once one closes it and the other answers 409 `review-not-open`.

A record older than `retention_days` is deleted with its votes and review items, in one
transaction, when the service starts (before the ready line) and every 24 hours after; any vote or
review item whose record is gone goes with them. A `retention_days` too large to count keeps every
record.

### Records on a PostgreSQL store

An instance on a `postgres` store keeps its records in a PostgreSQL schema of their own, never in
EKR's schema, and writes no `ask.sqlite`. Its spec must name that schema in `ask.records`:

```yaml
ask:
  # listen, identity_header, limits as above
  records:
    url: "postgres://cortex_ask@db.example:5432/brain"
    schema: cortex_ask
    ca_file: /etc/cortex/db-ca.pem
    connection: {adapter: postgres, profile: cortex-ask}
```

| field | meaning |
|---|---|
| `url` | a `postgres://` or `postgresql://` URL naming the records role, with no password. TLS is required unless the URL says `sslmode=disable` |
| `schema` | the records' schema: a lowercase identifier (`a-z`, digits, `_`, not starting with a digit or `pg_`, at most 63 bytes), never the schema of the store's `ekr.postgres/1` file |
| `ca_file` | optional: the PEM file the server's certificate is verified against; without it, the public web roots |
| `connection` | the Connectors connection holding the records role's `{"password": …}` document, by `connection` id or by `profile` |

The service never holds that password. When it starts, it launches its records process,
`cortex ask records --url <url> --schema <schema> [--ca-file <file>]`, with
`connectors connections launch --adapter <adapter> --connection <id> --consumer cortex-records`,
which hands that process the connection's document on descriptor 3. The records process connects,
creates the schema when it is missing, migrates the tables `answers`, `votes`, `reviews` and
`schema_version` in it to this cortex's schema version, and then runs the service's statements,
one JSON line each way on its standard input and output. A schema at a newer version is refused
(the service exits 1). Only the records process reads the password; the service, its output and
the instance directory never see it.

The operator, once per deployment:

1. Creates the records role, and either grants it `CREATE` on the database or creates the schema
   owned by it. It needs no privilege in EKR's schema.
2. Saves the role's password with `connectors connections connect` (the `{"password": …}`
   document), and allows the connection's adapter alias in `permissions.connections`.
3. Pins the `cortex` binary in the Connectors configuration as the consumer `cortex-records`. It
   needs no variable passed (`pass_env = []`).

`cortex ask serve` refuses before anything binds (exit 2, naming the field, never its value) a
`postgres` store without `ask.records`, a `sqlite` store with one, a schema that is the store's or
no lowercase identifier, a URL that carries a password, names no role or is not a PostgreSQL URL,
and a connection named by both id and profile.
