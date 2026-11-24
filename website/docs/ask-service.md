---
title: Ask service
sidebar_position: 6
description: cortex ask serve answers questions over an instance's store as a stream of events.
---

# Ask service

`cortex ask serve --spec <instance spec>` answers questions over the store of the instance the spec
defines. It is a long-running process of its own. `POST /ask` answers `text/event-stream`, one
event per step of a tool loop. The loop runs over the EKR MCP reader against the instance's model
gateway. The reader is `ekr mcp`, started as [`cortex mcp-line`](./commands.md) starts it. The
service reads the store only through the reader and does not write to it.

Each answer is streamed and kept as a record in cortex's own file beside the store. Askers vote on
the records, and a review queue collects the answers to look at, as
[Answer records, votes and review](#answer-records-votes-and-review) describes. The specification
is the domain [`cortex.ask`](./reference/ess/cortex-ask.md).

## The `ask` section

The optional `ask` section of the instance spec (`cortex.ask.AskSpec`) configures the service. Its
model is the spec's `model.gateway` and `model.model`:

```yaml
model:
  model: my-model
  budget_tokens: 2000000
  timeout_s: 120
  gateway: {base_url: "https://gateway.example", token_env: GATEWAY_TOKEN}
ask:
  listen: "127.0.0.1:8088"
  identity_header: x-asker
  limits:
    max_steps: 8
    max_result_bytes: 16384
    max_seconds: 120
    per_identity_per_minute: 10
    max_tool_calls: 16
  max_in_flight: 8
  link_templates:
    node: "https://view.example/node/{id}"
    evidence: "https://view.example/evidence/{id}"
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
| `limits.max_tool_calls` | optional: the most tool calls one step makes (default 16); the calls past it are not made, the model is told so for each, and a second `status` event of the step says how many were not made |
| `max_in_flight` | optional: the most answers the service works on at once, over every identity (default 8); a question past it answers 429 `busy` at once |
| `link_templates` | optional: the URL of a cited node or evidence, `{id}` replaced by its percent-encoded id |
| `guidance` | optional: text added to the service's system prompt |
| `retention_days` | optional: how many days an answer record is kept, with its votes and review items (default 90) |
| `slow_after_ms` | optional: an answer that took longer opens a `slow` review item; without it none does |
| `records` | required on a `postgres` store, refused on a `sqlite` one: the PostgreSQL schema of the answer records ([Records on a PostgreSQL store](#records-on-a-postgresql-store)) |

`cortex ask serve` exits 2 without starting, with the field in its message, when the spec has:

- no `ask` section, or no `model.gateway`
- a limit, `retention_days`, `slow_after_ms` or `max_in_flight` of 0 or less
- an `identity_header` that is no header name
- `ask.records` that does not fit the store, listed under
  [Records on a PostgreSQL store](#records-on-a-postgresql-store)

It then opens its records and purges the expired ones. It exits 1 when it cannot open them. Once
bound it prints one line, `{"listening": "<address>"}`, and serves until it is stopped.

:::note[Identity]
cortex verifies no token. The guard in front of the service verifies the asker and hands the
identity on in `identity_header`. A request without it, or with an empty one, answers 401 and asks
no model. Bind `listen` where only that guard reaches it.
:::

## The model route

The service calls the gateway as an OpenAI-compatible endpoint at
`POST <base_url>/v1/chat/completions`. When `base_url` already ends in `/v1`, the path is
`<base_url>/chat/completions`. The bearer token is the value of the variable in `token_env`, read
when an answer starts. A missing or empty token fails the answer with `model-failed`, and the
service's log gives the variable. The reader inherits neither that variable nor any other
`ANTHROPIC_*` or `CLAUDE_*` variable, so it gets no model credential.

A `max_seconds` above ten years counts as ten years. A gateway answer above 4 MiB is not read past
that and fails the answer with `model-failed`.

Each answer starts its own reader and kills it when the answer ends. Readers are not pooled, so no
answer inherits a reader that another answer left stuck or mid-call. At most `max_in_flight`
readers run at once, and at most as many model calls. An answer holds its slot until its reader is
stopped and its model call has ended, also when no client reads it any more.

## The event contract

The contract is `cortex.ask-events/1` (`cortex.ask.AskEvent`). Each SSE event's `event:` line is
the event's name, and its `data:` line is the event as one JSON object,
`{"event": <name>, "value": <fields>}`:

| event | fields | when |
|---|---|---|
| `status` | `format` (`cortex.ask-events/1`), `step`, `message` | first, then before each model call, and after a reply whose tool calls were cut to `max_tool_calls` |
| `tool` | `step`, `name`, `arguments`, `result_bytes`, `truncated`, `failed`, `withheld` | after each tool call the model asked for |
| `delta` | `text` | a piece of the model's text, in order |
| `discard` | `step` | withdraws the `delta`s since the last `discard`: text before a tool call, not the answer |
| `done` | `answer_id`, `answer`, `nodes`, `evidence`, `unsupported`, `input_tokens`, `output_tokens`, `cached_tokens`, `context_tokens`, `steps`, `unrestored` | the last event of an answer |
| `error` | `reason`, `message`, `retry_after` (with `rate-limit` only) | the last event of a request with no answer |

`done.answer` is the answer with each citation marker, `[node:<id>]` or `[evidence:<id>]`, turned
into a Markdown link from `link_templates`. Without a template the marker is left as written.
`nodes` and `evidence` are the cited ids, each once, in order.

A citation counts only when the reader returned its id: the id occurs as a whole word in a tool
result the model was shown in this answer. A marker with any other id is an unsupported reference.
It is written `[unsupported node:<id>]` (or `evidence`) and is not linked. It is left out of
`nodes` and `evidence` and listed once in `unsupported` as `node:<id>` or `evidence:<id>`.

The token counts add up the gateway's `usage` over every model call: `prompt_tokens`,
`completion_tokens` and `prompt_tokens_details.cached_tokens`, 0 where not reported.
`context_tokens` is the prompt tokens of the last call. `answer_id` is the answer's UUIDv7, the id
its record and its votes carry.

`error.reason` is one of:

| reason | status | meaning |
|---|---|---|
| `unidentified` | 401 | no identity header; no model was asked |
| `rate-limit` | 429 | the identity's questions of the minute are spent; `retry_after` (also the `Retry-After` header) is the whole seconds until the oldest leaves the window |
| `busy` | 429 | the service is working on `max_in_flight` answers already; no model was asked and no question of the minute was spent |
| `bad-request` | 400, or 413 | the body is not JSON with a non-empty `question`, or is above 1 MiB (413), or the question still holds a class `refuse_if_left` names (400); a refused body spends no question of the minute |
| `step-limit` | 200 | the model still asked for a tool at `max_steps` |
| `time-limit` | 200 | the answer passed `max_seconds` |
| `model-failed` | 200 | the gateway could not be reached, answered an error or no message |
| `reader-failed` | 200 | the reader could not be started or stopped answering |
| `cut-off` | 200 | the model's reply ended at its token limit (`finish_reason` `length`) or at a content filter (`content_filter`); its text is no answer and is not streamed |
| `refused` | 200 | the model declined: its chat completion carried a non-empty `message.refusal` (the Chat Completions refusal). `message` is the refusal text; no `delta` is streamed and no tool call it asked for is made |

The `error` event of an answer that started ends its `message` with `(request <answer_id>)`, the id
of its record. The `message` of `model-failed` and `reader-failed` is a fixed text. What the
gateway answered, the reader's program path and any other internal detail go only to the service's
standard error, on a line that carries that request id. The service masks every line it writes
there for credential shapes first, as it masks a record.

A client that closes the stream before its last event stops the answer at once. No model is asked
again and the reader process is stopped. The answer is recorded as `client-closed`, with the token
counts and steps of the model calls already made. It is no failure and opens no review item,
`slow` included.

A client that leaves while the model is asked does not cut the call. The service waits for the
call, up to `max_seconds`, so its tokens are counted, and makes none of its tool calls. Until the
call ends, the answer keeps its `max_in_flight` slot.

A tool result is rewritten into compact text: each text that is JSON is written without
whitespace. The result is then cut at a character boundary to `max_result_bytes`, and its `tool`
event says whether it was cut.

## Redaction

With a [`redaction`](./spec-file.md#redaction) policy in the spec, the model reads the store only
redacted, as extraction shows it documents. One question is one redaction batch: one placeholder
mapping, held in memory for that answer and written nowhere.

- The question and each tool result are redacted with the policy's classes, `known_names`, `rules`
  and `rare_limit` before the model is given them. The same value is the same placeholder
  (`[Email-1]`, `[Name-1]`) in every result of the answer. A name replaced as rare stays replaced
  when a later result repeats it. A result is redacted before it is cut to `max_result_bytes`.
  Credentials and rules with a `replacement` are replaced irreversibly.
- A result in which a class from the policy's `refuse_if_left` is still detected is not given to
  the model. The model gets a tool error that says the result was withheld by policy and lists the
  classes, and the `tool` event says `withheld: true`. The answer goes on.
- A question that still holds a `refuse_if_left` class after it is redacted is refused before
  anything is sent and before it spends a question of the minute. The answer is a 400 with an
  `error` event whose `reason` is `bad-request`. The event lists the classes and not what was
  found.
- Text in a tool result that already looks like a placeholder (`[Email-1]`) is shown to the model
  as a fresh placeholder that restores to that text, so it is not restored to another value. A
  withheld result adds none of its values to the answer's mapping.
- The answer is restored before the client sees it: each placeholder that has a value is replaced
  by it, in every `delta` and in `done.answer`. A `delta` that ends inside a possible placeholder
  holds that tail back until the next `delta` or `done`, so no client sees half a placeholder. A
  placeholder with no value, because the model changed or invented it, stays as written and is
  counted in `done.unrestored`.

Without a policy, the question and the tool results reach the model unchanged. `withheld` is then
false and `unrestored` is 0. Detection is by pattern, with the limits that [Limits](./limits.md)
states for extraction.

A stream with one tool call and an answer, without a policy. The service sends each `data:` line
as one line of JSON. Here each is broken after a comma to fit the page:

```text
event: status
data: {"event":"status","value":{"format":"cortex.ask-events/1",
  "message":"starting the reader","step":0}}

event: tool
data: {"event":"tool","value":{"arguments":"{\"query\":\"widget\"}",
  "failed":false,"name":"search","result_bytes":35,"step":1,
  "truncated":false,"withheld":false}}

event: done
data: {"event":"done","value":{"answer":"…",
  "answer_id":"0192f3a1-7c2e-7d41-9b0a-3f5e6d7c8b9a",
  "cached_tokens":100,"context_tokens":200,"evidence":[],
  "input_tokens":300,"nodes":["n-42"],"output_tokens":30,"steps":2,
  "unrestored":0,"unsupported":[]}}
```

## Answer records, votes and review

The service keeps each answer it gives, and each one that failed or hit a limit, as a record
(`cortex.ask.AnswerRecord`) before its last event is sent. A record holds:

- the question and the identity
- the answer, or the reason there is none
- the outcome: `answered`, `failed`, `limited`, `client-closed`, `cut-off` or `refused`
- the `tool` events, the latency and the time
- the token counts and steps: those of its `done` event, or for every other outcome those of the
  model calls made before it ended, 0 when none was made

A `refused` record keeps the model's refusal text as its `answer`, with the reason `refused`. A
request refused before it started (401, 429, 400) leaves no record.

The records are cortex's own and are not in the EKR store. On a `sqlite` store they live in
`instances/<name>/ask.sqlite` in the instance directory. On a `postgres` store they live in a
schema of their own. Either carries its schema version and is migrated when the service starts. A version written by a newer
cortex is refused.

Whatever the instance's `redaction` policy, the service masks every credential shape
(`[masked:<shape>]`) in the question, the answer, the `tool` events' arguments, a vote's comment,
and a close's reason and expected facts. The mask is irreversible.

The record routes also need the identity header (401 without it) and answer JSON. A refusal is
`{"error": <reason>, "message": <text>}`. A body above 64 KiB is refused with 413 `bad-request`.

Before it answers any request, an unknown path or method included, the service reads the request
body to the end. A body above the route's limit, or one the answer does not use, is discarded as
it is read, up to 16 MiB and for at most 10 seconds. So a client that is still sending the body
reads the answer.

A failure of the records answers 500 `store-failed` with a fixed message and a
`request_id`. The detail, such as a database path or SQL text, goes only to the service's
standard error, on a line that carries that `request_id`.

| route | body | answers |
|---|---|---|
| `POST /ask/{answer_id}/vote` | `{"value": "up" \| "down", "comment"?: <text>}` | the vote as kept: `vote_id`, `answer_id`, `identity`, `value`, `comment`, `at`. 400 `bad-request` for another value, 404 `no-such-answer` for an id no record carries |
| `GET /review` | none | `{"reviews": [...]}`: every open review item, oldest first, with `review_id`, `answer_id`, `cause` and `opened_at` |
| `POST /review/{review_id}/close` | `{"reason": <text>, "expect_facts": [<text>, …], "expect_nodes": [<node id>, …]}` | the item as closed, with `expect_facts` and `expect_nodes`: what a right answer contains and cites, which `cortex eval case add --from-review` makes an eval case (both optional, empty when absent). 400 without a reason or with an expectation list that is not a list of strings that are not blank, 404 `no-such-review` for an unknown id, 409 `review-not-open` for an item closed already |

An identity has one vote per answer, and voting again changes it. An answer opens one review item
per cause. Once opened, an item stays as it is, whatever happens after:

| cause | opened when |
|---|---|
| `voted-down` | a `down` vote is cast on the answer, by anyone |
| `failed` | the answer's outcome is `failed`, `limited`, `cut-off` or `refused` (a `client-closed` answer opens none) |
| `slow` | the answer took longer than `slow_after_ms` |

A vote whose answer a purge deletes at the same time answers 404 `no-such-answer` and leaves no
vote or review item behind. Of two closes of one item at once, one closes it and the other answers
409 `review-not-open`.

A record older than `retention_days` is deleted with its votes and review items in one
transaction. The purge runs when the service starts, before the ready line, and every 24 hours
after. Any vote or review item whose record is gone is deleted with them. A `retention_days` too
large to count keeps every record.

### Records on a PostgreSQL store

An instance on a `postgres` store keeps its records in a PostgreSQL schema of their own, not in
EKR's schema, and writes no `ask.sqlite`. Its spec must give that schema in `ask.records`:

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

The service does not hold that password. When it starts, it launches its records process,
`cortex ask records --url <url> --schema <schema> [--ca-file <file>]`, with
`connectors connections launch --adapter <adapter> --connection <id> --consumer cortex-records`.
That command hands the process the connection's document on descriptor 3.

The records process connects and creates the schema when it is missing. It migrates the tables
`answers`, `votes`, `reviews` and `schema_version` in it to this cortex's schema version. Then it
runs the service's statements, one JSON line each way on its standard input and output. A schema
at a newer version is refused, and the service exits 1. Only the records process reads the
password. The service, its output and the instance directory do not see it.

Record calls are served one at a time in the order they arrive. A call waits at most 30 seconds for
the calls before it, for a start of the records process and for the answer to its statement. The
records process runs each statement with a `statement_timeout` of 30 seconds.

While the database stalls, a vote, `GET /review` or a close answers `store-failed` (500). An answer
whose record cannot be kept still ends, and its record is logged as not kept. A records process
that did not answer in time is stopped, and the next call starts another. A call that only waited
behind others fails without stopping it. A start that fails fails the calls that waited through it,
and they do not each start another.

The operator, once per deployment:

1. Creates the records role, and either grants it `CREATE` on the database or creates the schema
   owned by it. It needs no privilege in EKR's schema.
2. Saves the role's password with `connectors connections connect` (the `{"password": …}`
   document), and allows the connection's adapter alias in `permissions.connections`.
3. Pins the `cortex` binary in the Connectors configuration as the consumer `cortex-records`. It
   needs no variable passed (`pass_env = []`).

`cortex ask serve` exits 2 before anything binds when the records configuration has one of these
faults. Its message gives the field and not its value.

- a `postgres` store without `ask.records`, or a `sqlite` store with one
- a schema that is the store's or no lowercase identifier
- a URL that carries a password, has no role or is not a PostgreSQL URL
- a connection given by both id and profile

## Figures

cortex draws figures as SVG with its own code, from a typed figure,
[`cortex.ask.FigureSpec`](./reference/ess/cortex-ask.md). It does not draw from markup a model
wrote. A figure is one of three kinds:

| kind | holds | drawn as |
|---|---|---|
| `bar-chart` | a title and bars, each a label and a count of 0 or more | one horizontal bar per entry, in the order given, as long as its count against the largest |
| `timeline` | a title and entries, each an RFC 3339 instant and a label | one row per entry, earliest first, each with its instant in UTC |
| `relationship-graph` | a title, nodes (a store node or evidence id and a label) and edges (a source id, a target id and a label) | the nodes on a circle in the order given, each edge a line with an arrowhead |

The SVG uses only the elements `svg`, `g`, `rect`, `line`, `path`, `circle`, `text` and `title`,
and only geometry, fill, stroke and text attributes, whose values are numbers, colours and
keywords. It holds no script, no `foreignObject`, no `style`, no event-handler attribute and no
`href`. Every title and label is the text of a `text` or `title` element, escaped (`<`, `>`, `&`,
`"`, `'`), with each control character drawn as U+FFFD. The same figure gives the same bytes on
every run.

A figure over a cap is not cut. It is refused whole with `cortex.ask.FigureTooLarge`, which gives
the cap:

| cap | at most |
|---|---|
| `bars` | 50 bars |
| `entries` | 200 timeline entries |
| `nodes` | 100 graph nodes |
| `edges` | 300 graph edges |
| `bytes` | 262,144 bytes of SVG |

A figure holding a value it cannot be drawn from is refused with `cortex.ask.FigureInvalid`, which
gives the value. Such values are a negative count, an instant that is not RFC 3339, a node id given
twice, or an edge whose source or target is no node of the graph.

The renderer is not yet reachable from `POST /ask`: no endpoint or event draws a figure in this
release.
