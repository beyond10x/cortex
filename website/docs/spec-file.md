---
title: Spec file
sidebar_position: 3
description: The cortex.instance/1 file one instance is created from, section by section.
---

# Spec file

One YAML file in the `cortex.instance/1` format defines one instance. Its exact types are
`InstanceSpec` and the types it uses in the
[generated specification reference](./reference/ess/cortex-instance.md#instancespec); its JSON
Schema is [`instance-spec.schema.json`](https://beyond10x.github.io/cortex/schemas/instance-spec.schema.json),
and `cortex schema` prints the same document. This page says what each section does.

The repository's `examples/example.yaml` is a complete file with two web sources.
`examples/agent-tooling.yaml` is another, with its own seed ontology
(`examples/seed/agent-tooling.yaml`): a daily news search on three queries, a weekly crawl of the
Model Context Protocol specification, and a budget of $0.50 per run.

## Top level

```yaml
format: cortex.instance/1
name: example
description: Developments in open-source vector databases and the companies behind them.
ekr: {version: "0.0.34"}
seed: {schema: seed/schema.yaml, documents: []}
model: {model: claude-sonnet-5-5, budget_usd: "1", timeout_s: 900}
sources: []
serve: {}
```

| field | meaning |
|---|---|
| `format` | must be `cortex.instance/1` |
| `name` | the instance name: 1 to 63 lowercase letters, digits or `-`, not starting with `-` |
| `description` | what the instance is about; every extraction prompt carries it |
| `ekr` | the EKR version the instance runs on, and optionally the binary |
| `seed` | what the store starts from |
| `model` | the model that extracts, its per-run budget, its optional daily ceiling and its timeout |
| `sources` | the data sources and their schedules |
| `serve` | the viewer's port |
| `store` | optional: the store's backend, SQLite or PostgreSQL ([`store`](#store)) |
| `redaction` | optional: what is kept from the model, and what refuses a run ([`redaction`](#redaction)) |
| `snapshots` | optional: how many snapshots of a `sqlite` store are kept ([`snapshots`](#snapshots)) |
| `gate` | optional: the checks a run is held to ([`gate`](#gate)) |
| `monitoring` | optional: what a run reports besides its printed line ([`monitoring`](#monitoring)) |
| `ask` | optional: the ask service's settings ([Ask service](./ask-service.md#the-ask-section)) |

Paths in `seed` and `model.instructions` are relative to the spec file's directory and may not
leave it. `cortex create` copies the spec file and every file they name into the instance
directory, so the instance keeps working when the originals move.

## `ekr`

| field | meaning |
|---|---|
| `version` | the pinned EKR version |
| `bin` | optional path to the `ekr` binary; `~/` is expanded. Without it, cortex uses `~/.cache/cortex/bin/<version>/bin/ekr`, where `cortex setup` installs it |

## `seed`

| field | meaning |
|---|---|
| `schema` | optional `ekr.extraction-document/1` file applied right after the store is seeded, usually the starting ontology (`examples/seed/schema.yaml`). A rejected part refuses the whole create |
| `ekr_seed` | optional EKR seed file; without it, cortex seeds an empty ontology and an empty graph at revision 0 |
| `documents` | directories whose files are extracted once when the instance is created, through the same pipeline as a source run (recorded as the source `seed`), unless `cortex create --no-extract` is given. A seed that stops before every document is extracted (its budget spent, a model call or an apply failed) makes `create` answer `partial`; `cortex run <name>/seed` extracts the documents it left |

The seed of an existing store cannot change: `cortex update` with a different `seed` section or
different seed files answers `seed-change-refused`.

## `model`

| field | meaning |
|---|---|
| `model` | the model name passed to `claude --model` |
| `backend` | optional: `Claude` (the default) runs `claude -p`. `Codex` is accepted by `create` and `update` but extracts nothing in this release: every run fails as `extraction-failed`, naming `story:codex-model-backend` (see [Commands](./commands.md#global-options)) |
| `budget_usd` | the spending limit of one run, in US dollars, as a decimal string; above 0. A run stops starting new batches once it is spent. Required without a `gateway`; with one it may be omitted, and bounds nothing (see [A model gateway](#a-model-gateway)) |
| `daily_ceiling_usd` | optional: the most the instance's model calls spend in one UTC calendar day, in US dollars, as a decimal string; above 0. Absent means no ceiling. See [The daily ceiling](#the-daily-ceiling). Bounds nothing behind a `gateway` |
| `budget_tokens` | optional: the most tokens one run's model calls use, a whole number above 0. Absent means no limit in tokens. Required with a `gateway`. See [Limits in tokens](#limits-in-tokens) |
| `daily_ceiling_tokens` | optional: the most tokens the instance's model calls use in one UTC calendar day, a whole number above 0. Absent means no ceiling in tokens. See [Limits in tokens](#limits-in-tokens) |
| `timeout_s` | the time limit of one model call, in seconds; above 0 |
| `instructions` | optional text file whose content is added to every extraction prompt |
| `gateway` | optional: a model gateway every `claude` call goes through, with no interactive sign-in. See [A model gateway](#a-model-gateway) |
| `batch_chars` | optional: the most characters of document text in one extraction request, counted as UTF-8 bytes (a non-Latin character counts 2 to 3), a whole number of at least 1,000; `create`, `update` and `adopt` refuse a smaller one. Absent, it is derived from a gateway's window, else 60,000. See [The batch size](#the-batch-size) |

### The batch size

A run sends its documents to the model in batches, one `claude -p` call each, of at most
`batch_chars` characters of document text; a longer document goes alone. A character is counted
as its UTF-8 bytes, as before this setting: a Latin letter counts 1, a non-Latin character (Greek,
Cyrillic, CJK) 2 to 3, for a given size, a derived one and the default alike. Without `batch_chars`,
behind a `gateway` that gives `max_context_tokens`, the size follows the model's window: the input
room is `max_context_tokens` less the output reserve (`max_output_tokens` when given, else a
quarter of `max_context_tokens`), and the size is 1.5 characters per token of that room, less the
characters of the request's fixed part (the system prompt, the answer's schema, the instance's
description and instructions, the ontology and the known entity names), at least 4,000 and at most
60,000. Otherwise it is 60,000. A gateway with `max_context_tokens: 65536` and
`max_output_tokens: 24000` leaves 41,536 tokens of room, 62,304 characters before the fixed part.
A run's report and its `cortex.log` line state the size it used as `batch_chars`. A batch's own
fixed part counts too: when the store nodes its documents name (the
[known entities](./operating.md)) make its fixed part longer than the one the size
was derived with, and its documents' text plus that growth is over the size, a batch of more than
one document is split in two before it is sent, and the split is counted in `splits`.

When `claude` answers that a request is too long for the model's window (`Prompt is too long`,
or `maximum context length` from an OpenAI-compatible server behind a gateway),
or that the answer was cut off at the output limit (`exceeded the … output token maximum`), the
batch is split in two halves by documents and each half is asked again, at most 3 splits deep (a
batch becomes at most 8 parts); a part of one document is not split. The report and `cortex.log`
count the splits (`splits`). Such an answer is not asked again after a wait, and every call counts
against the token and dollar limits as any call does.

A part of one document that still does not fit is left out alone, and the run goes on with the
batches after it and ends `ran`. The run's report and its `cortex.log` line name that document in
`too_large`, and it is marked seen, so no later run asks it again until its text changes; no
`batch_chars` makes one document smaller. A part of several documents that still does not fit
after 3 splits ends the run as `batch-too-large`, exit status 6, which is not counted against the
source (see [Operating](./operating.md#exit-statuses-of-cortex-run)); a later run extracts it, and
a smaller `batch_chars` makes its batches smaller.

### The daily ceiling

```yaml
model: {model: claude-sonnet-5-5, budget_usd: "1", daily_ceiling_usd: "5", timeout_s: 900}
```

`daily_ceiling_usd` covers every model call of the instance in one UTC calendar day: the runs of
all its sources, the seed extraction, `cortex quality` and `cortex schema`. Each call is given
(`--max-budget-usd`) the lesser of what is left of its own `budget_usd` and what is left of the
ceiling today. Every answer that states a cost adds it to the spend of the day, a failed answer
included; an answer with no cost adds nothing. The day starts at 00:00 UTC.

Once the spend of the day reaches the ceiling, no call is made until the next UTC day:

- a run stops before its next batch and answers `ran`, with the batches before it applied,
  `stopped` naming the ceiling (`daily ceiling of 5 USD reached: 5.0000 USD spent on 2026-10-14
  (UTC)`) and `daily_ceiling_reached: true`. A run stopped before its first batch applies nothing
  and makes no model call. Either way it counts no failure, and the documents it did not extract
  are extracted by a run on a later day;
- `cortex quality` answers `judge-failed` and `cortex schema` answers `propose-failed`, the reason
  naming the ceiling.

The spend of the day is `spend.json` in the instance directory (see
[Operating](./operating.md#where-things-live)); it is kept whether or not the spec names a ceiling,
and `cortex restore` leaves it as it is. A ceiling changed with `cortex update` holds from the next
run or command, against what the day has spent so far.

### A model gateway

```yaml
model:
  model: my-open-model
  budget_tokens: 200000
  daily_ceiling_tokens: 2000000
  timeout_s: 900
  gateway: {base_url: "https://gateway.example", token_env: GW_TOKEN, max_context_tokens: 128000}
```

| field | meaning |
|---|---|
| `base_url` | the gateway's URL, passed to `claude` as `ANTHROPIC_BASE_URL`. It starts with `http://` or `https://`; `create`, `update` and `adopt` refuse an empty one or another scheme |
| `token_env` | the name of the environment variable holding the gateway's token, never the token itself. Its value is read when each call is made and passed as `ANTHROPIC_AUTH_TOKEN`. A variable name (letters, digits and `_`, not starting with a digit), and not one cortex writes into a unit by value (`PATH`, `HOME`, a `CONNECTORS_*` or a `CORTEX_*` variable), nor any `XDG_*` or `ANTHROPIC_*` variable; `create`, `update` and `adopt` refuse another |
| `max_context_tokens` | optional: passed as `CLAUDE_CODE_MAX_CONTEXT_TOKENS`; a whole number above 0, `create`, `update` and `adopt` refuse another. Sets [the batch size](#the-batch-size) when `batch_chars` is absent |
| `max_output_tokens` | optional: passed as `CLAUDE_CODE_MAX_OUTPUT_TOKENS`, the most tokens of one answer; a whole number above 0 and below `max_context_tokens`, `create`, `update` and `adopt` refuse another. The output reserve of [the batch size](#the-batch-size) |

cortex passes exactly that set to `claude`, from a run in your shell, a timer unit and a runner that
calls `cortex run` directly alike; `model` names the gateway's model (`--model`). An
`ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`, `CLAUDE_CODE_MAX_CONTEXT_TOKENS` or
`CLAUDE_CODE_MAX_OUTPUT_TOKENS` in cortex's own environment is replaced by the spec's, or removed
when the spec does not set it, and `ANTHROPIC_API_KEY` is always removed. A missing or empty
`token_env` fails the call before `claude` starts, with a reason naming the variable: a run
answers `extraction-failed`. Without a `gateway`, `ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`,
`CLAUDE_CODE_MAX_CONTEXT_TOKENS` and `CLAUDE_CODE_MAX_OUTPUT_TOKENS` are removed as well, so an
instance never reaches a gateway by accident.

A source's timer unit names the variable with `PassEnvironment=`, so the token reaches a scheduled
run from the systemd user manager's environment (see
[Operating](./operating.md#systemd-units)). The gateway applies to the `Claude` backend; the
`Codex` backend fails every call before starting anything in this release, so it reaches no gateway.

Behind a gateway the instance is bounded in tokens only. `claude` prices each answer at its
default model's rate, from its own price table, not at what the gateway's model costs, so that
figure bounds nothing:

- `budget_tokens` is required: `create`, `update` and `adopt` refuse a gateway spec without it
  (exit status 2, naming `model.budget_tokens`, before anything is written). `budget_usd` may be
  omitted. An instance created with a gateway by an earlier release, whose spec has no
  `budget_tokens`, keeps running with its dollar limits counted as before, until an `update`
  gives it one.
- The cost an answer reports counts against no `budget_usd`, no `daily_ceiling_usd` and no
  dollar budget of a `cortex quality` measurement or a `cortex schema` round, and no
  `--max-budget-usd` is passed to `claude`. A [run gate](#gate) check on `cost_usd` checks
  nothing. The token limits ([Limits in tokens](#limits-in-tokens)) hold as without a gateway.
- A gateway spec that still gives `budget_usd`, `daily_ceiling_usd` or a gate check on
  `cost_usd` is accepted, and `create`, `update` and `adopt` print one line on stderr saying they
  bound nothing behind the gateway.
- The reported cost is still recorded, as not counted: a run's report and its `cortex.log` line
  carry `cost_usd` as reported and `cost_counted: false`, and `spend.json` sums it in
  `uncounted_usd`, leaving `spent_usd` as it is.

### Limits in tokens

```yaml
model: {model: claude-sonnet-5-5, budget_usd: "1", budget_tokens: 200000, daily_ceiling_tokens: 2000000, timeout_s: 900}
```

The dollar limits rely on the cost `claude` states in each answer, and for a model `claude` cannot
price that figure is missing or not the real cost (see [Limits](./limits.md#the-model)). The
token limits bound the same runs and calls by what each answer reports in its `usage`: input
tokens, cache reads, cache writes and output tokens, added together.

- `budget_tokens` bounds one run: before each batch, a run whose calls have used `budget_tokens`
  or more stops, `stopped` naming the token budget (`token budget of 200000 tokens spent: 201500
  tokens used`). It bounds each `cortex quality` measurement and each `cortex schema` round the
  same way, as `budget_usd` does: before a further call, one whose calls have used
  `budget_tokens` or more fails with that reason.
- `daily_ceiling_tokens` bounds the instance's calls in one UTC calendar day, across the runs of
  all its sources, the seed extraction, `cortex quality` and `cortex schema`, as
  `daily_ceiling_usd` does. Every answer that reports a `usage` adds its tokens to the day's, a
  failed answer included. Once the day's tokens reach the ceiling, a run stops before its next
  batch with `daily_ceiling_reached: true` and `stopped` naming it (`daily ceiling of 2000000
  tokens reached: 2001500 tokens used on 2026-10-15 (UTC)`); `cortex quality` and `cortex schema`
  are refused with that reason.
- Under either token limit, an answer whose tokens cannot be counted stops the run before its
  next batch, `stopped` saying its tokens could not be counted; the batches before it stay
  applied. That is an answer that reports no `usage`, or a count in it below 0, or counts whose
  sum (with the tokens already used) does not fit a signed 64-bit integer. Such a stop is not a
  reached ceiling: `daily_ceiling_reached` is not set. `cortex quality` and `cortex schema` are
  refused the same way under either limit.

A token limit cannot be passed to `claude` the way `--max-budget-usd` is, so a single call is not
cut short: a run can pass its limit by the tokens of its last call. The token limits apply beside
the dollar limits; whichever is reached first stops the run. Behind a `gateway` they are the only
limits ([A model gateway](#a-model-gateway)). Without a token limit, runs behave as before. The day's tokens are `spent_tokens` in `spend.json`; a file written without it counts as
no tokens used.

## `sources`

Each source has a `name` (same rules as the instance name, unique within the instance), a
`schedule`, `settings` and a `policy`. Its id is `<instance>/<source>`.

`entities` and `seed` are reserved: a source's seen documents are `state/<source>.json`, and
cortex keeps the known entity names in `state/entities.json` and the seed's seen documents in
`state/seed.json`. `create`, `update` and `adopt` refuse a spec with a source of either name and
name it. An instance created with one before this rule keeps loading; rename the source with
`update`.

`schedule` is a systemd calendar expression; it is written into the timer as `OnCalendar=`, for
example `"*-*-* 06:00:00"` for every day at 06:00.

`settings` is `kind` plus `value`, where `kind` is `web`, `connectors`, `files` or `structured`.

### Naming a connection

A source that reads through Connectors, and a `postgres` store, name their Connectors connection
in one of two forms, exactly one per place; `create`, `update` and `adopt` refuse both or neither
with a reason naming the fields:

| form | written as | means |
|---|---|---|
| by id | `connection: conn_…` | that connection |
| by profile | `profile: <profile>` | the one live connection of the adapter with that profile |

A profile is resolved each time cortex needs the connection, once per command: `cortex create`
checks it, and every later command that uses the connection (a run, `quality`, `schema`,
`mcp-line`) resolves it again. It resolves to the
one connection that `connectors connections list --adapter <adapter>` lists with that `profile`
and a `state` other than `revoked` (a `pending` one is revalidated as by id). The adapter is the
source's own `adapter` (for `kind: web`, `tavily` when absent) or the store reference's `adapter`.
None, or more than one, is refused: `cortex create` answers `connection-missing`, a run
`fetch-failed`, each with a reason naming the adapter, the profile, the count and every matching
id. cortex reads the first page of the list only, so a list with a further page is refused too.
The frozen spec keeps the profile, never the id it resolved to, so an instance whose Connectors
state is rebuilt with new connection ids keeps working, as in a container that runs
`connectors connections connect` at every start ([Operating](./operating.md#connections-by-profile)).

```yaml
settings:
  kind: web
  value:
    profile: tavily.api-key
    input: {input: search, value: {queries: [widget engine], policy: {topic: news, max_results: 5, include_domains: [], exclude_domains: []}}}
```

### `kind: web`

Websites with their title, description and content, through a Connectors adapter that implements
`datasource.websearch/v1alpha1`.

| field | meaning |
|---|---|
| `adapter` | optional; `tavily` when absent |
| `connection` or `profile` | the Connectors connection, by id or by profile of `adapter` ([Naming a connection](#naming-a-connection)) |
| `input` | `input: search` or `input: sites`, with its `value` |

`input: search` runs `websearch.search` once per query, always asking for full content:

| field | meaning |
|---|---|
| `queries` | the search queries |
| `policy.topic` | `general` or `news` |
| `policy.time_range` | optional: `day`, `week`, `month` or `year` |
| `policy.max_results` | results per query |
| `policy.include_domains`, `policy.exclude_domains` | domain lists; an empty list is not sent |
| `policy.country`, `policy.language` | optional |

`input: sites` reads listed URLs:

| field | meaning |
|---|---|
| `urls` | the start URLs |
| `mode` | `Pages` reads exactly these URLs in one `websearch.fetch`; `Crawl` runs `websearch.crawl` from each |
| `policy` | optional crawl policy: `max_depth`, `max_breadth`, `limit`, `select_paths`, `exclude_paths`, `instructions`, `allow_external`. Without it a crawl is limited to 20 pages |

A website with no content keeps its description as its text; one with neither is dropped. A URL
returned twice in one run counts once.

### `kind: connectors`

The records of any operation a Connectors adapter admits.

| field | meaning |
|---|---|
| `adapter`, `connection` or `profile`, `operation` | what to invoke; the connection by id or by profile of `adapter` ([Naming a connection](#naming-a-connection)) |
| `inputs` | a list of JSON inputs; the operation is invoked once per input. `{since}`, `{until}`, `{since_epoch}` and `{until_epoch}` in a string value are replaced by the run's window |
| `records` | dotted path to the array of records in the answer, for example `issues` |
| `id` | dotted path to a record's stable id |
| `time` | optional dotted path to the record's publication time, which dates its evidence and the facts drawn from it ([how](./operating.md#what-one-run-does)) |
| `text` | templates whose `{a.b}` placeholders are filled from the record; the non-empty results, joined, are the document's text |
| `paging` | optional: how to read every page of the answer (below) |
| `child` | optional: an operation invoked once per record, whose records are added to that record's text (below) |

A record without an id, or whose text is empty, is skipped. A document's key is
`<adapter>:<operation>:<id>`; a key read twice in one run counts once.

**The window.** `{since}` and `{until}` are instants in RFC 3339, in UTC and to the second
(`2026-10-05T12:34:56Z`). `{until}` is the start of this run. `{since}` is 5 minutes before the
start of the source's last successful run, or, before the first, the start of this run minus the
policy's `refresh_after_days`. The 5 minutes cover a record a provider shows late; a document read
again in them is applied again only if its text changed.

`{since_epoch}` and `{until_epoch}` are the same two instants as whole seconds since the Unix
epoch, a string of digits (`1790000000`), for an operation whose bounds are epoch seconds, such as
a chat history's `oldest`:

```yaml
inputs:
  - {channel: "C1", oldest: "{since_epoch}"}
```

The value stays a JSON string. Any other text in braces, such as `{since_ms}`, is passed to the
operation as written.

With `refresh_after_days: 0` the first run's window is empty (`{since}` equals `{until}`), so
records older than the 5 minutes before that run are never read. Set `refresh_after_days` to 1 or
more to read that much history on the first run.

A run is successful when it applied every new or changed document it fetched and read every
record. A run that is not successful (it stopped early, failed, left documents beyond
`max_documents_per_run`, or left records unread) sets the `{since}` of every later run until one
succeeds, so the next run asks again for what it did not apply. When the run read every record
and every new or changed document it did not apply has a readable `time`, that `{since}` is the
earliest of those times less 5 minutes, never before the run's own `{since}`: under a backlog
the window moves forward past what was applied, and one document that keeps failing holds it
only at its own time. Otherwise it is the run's own `{since}`. It is never after a change the run
held back by `refresh_after_days`, so a run that fails leaves that change in the next window.
That holds before the first successful run too: the window does not slide with the clock. A
document skipped after failing alone on 3 consecutive runs (see
[Operating](./operating.md#what-one-run-does)) holds no window. Records are left unread by a walk
that reaches `max_pages` with pages left, by an empty page that still names a next one, by a
`NextLink` link without its parameter, and by a child call that fails for a record, which leaves
that record out of the run. Each of these says so in `stopped`, in the run's result and in
`cortex.log`; the other records still apply.

A child call that fails for the same record on 3 runs in a row stops holding the window: from the
third failure on, the record is left out and named in `skipped` (`<operation>: child call failed
for <key> on 3 runs; skipped`), in the run's result and in `cortex.log`, but not in `stopped`.
The call is still tried on every run, and one success resets the count.

A changed document whose last application is younger than `refresh_after_days` is held back, not
lost. Every run works out, from the documents it holds back, the earliest window start that covers
each held change: the earliest last application among them, less the 5 minutes. No later run's
`{since}` is after that; a run that holds nothing back clears it.

**`paging`** has the fields `style` (`PageNumber`, `Token`, `Keyset` or `NextLink`), `param` (the
dotted path of the input field to advance), `next` (optional, the dotted path in the answer of the
next page, token, key or link), `max_pages` and `stop_older_than_since` (optional). With `next`,
its value is the next `param`, whatever the style, except `NextLink`. Without it, a `PageNumber`
walk asks for the page after the current one (the input's `param`, or 1 when absent), and a
`Token`, `Keyset` or `NextLink` walk reads one page. A walk stops at an empty page, at a missing
`next`, at a page or token it has already asked for, or after `max_pages` pages.

A `NextLink` walk reads a URL at `next`, relative or absolute, and asks next for the
percent-decoded value of its query parameter named by the last segment of `param`:
`_links.next: "/api/pages?cursor=abc&limit=50"` with `param: cursor` asks for `cursor: abc`, and
`https://host/api/pages?limit=50&cursor=a%2Fb` for `cursor: a/b`. A missing, null or empty link
ends the walk. A link without that parameter, or with it empty, ends it too and leaves records
unread: the run says `<adapter>.<operation>: the next link names no <param>` in `stopped`.

With `stop_older_than_since: true`, the walk ends at the first record whose `time` is older than
the run's `{since}`; that record and the ones after it on its page are not taken. An operation that
lists newest-modified first and takes no time filter is so read down to the last run and no
further. A walk the rule ends has read everything the window asks for: it leaves nothing unread
and the run can succeed. A record whose time is missing or cannot be read does not stop the walk.
A walk that reaches `max_pages` before the rule fires leaves pages unread as without it, and the
run keeps its `{since}`. Only a `connectors` source's own `paging` takes the rule, and only with a
`time`; a spec that sets it elsewhere, or without `time`, is refused when it is loaded.

```yaml
operation: pages.changed
records: results
id: id
time: version.createdAt
paging: {style: NextLink, param: cursor, next: _links.next, max_pages: 20, stop_older_than_since: true}
```

:::caution[PageNumber paging can skip records]
A `PageNumber` walk asks for pages by offset. When a write moves a record within the result set
while the walk runs, the records behind it shift, and one can land on a page already read and be
skipped. The next run reads it only when its stamp lies in the 5 minutes its `{since}` reaches
back; a skipped record stamped earlier is never read. Prefer `Token` or `Keyset` paging, or ask the
provider for a sort that writes do not reorder, such as by creation time or id.
:::

**`child`** has the fields `operation` (on the same adapter and connection), `input` (JSON whose
`{a.b}` placeholders are filled from the parent record), `records` (the dotted path to the array
of records in its answer) and an optional `paging`, walked the same way. Each child record is
added to the parent's text as one paragraph of JSON. The change hash covers the whole text,
children included, so a child field that changes on every call (a duration, a signed URL) makes
the parent extracted again after each `refresh_after_days`.

Child records count toward the document's `max_chars_per_document`. Text past it, child records
included, does not reach the model: a long parent's last children are cut off. The run counts the
cut documents it shows the model as `truncated` (absent when there are none), and a cut document
whose whole text changed is extracted again.

:::note[After upgrading]
cortex now detects a change on a document's whole text, before the cut. A source's state from an
earlier version holds the hash of the cut text, so every document longer than
`max_chars_per_document` looks changed once and is extracted once more, after its
`refresh_after_days`.
:::

### `kind: files`

| field | meaning |
|---|---|
| `paths` | directories to read; relative paths are read against the directory the spec file was created from, and `~/` is expanded |
| `glob` | matched against each file's path relative to its directory, for example `**/*.md` |
| `records` | optional: read each file as records, one document per record (below). Without it, each file is one document |

Files that are not UTF-8 text, or are empty, are skipped. A document's key is the file's path.

**`records`** has the fields:

| field | meaning |
|---|---|
| `format` | `WholeFile` (each file is one document, as without `records`), `JsonLines` (each line is a JSON record) or `MarkdownSections` (each `##` section is a record) |
| `id` | dotted path to a record's id |
| `time` | optional dotted path to the record's publication time, which dates its evidence and the facts drawn from it ([how](./operating.md#what-one-run-does)) |
| `author` | optional dotted path to the record's author |
| `text` | templates whose `{a.b}` placeholders are filled from the record; the non-empty results, joined, are the record's text |
| `fallback_text` | optional templates tried in order only when every `text` template renders empty; the first non-empty result is the record's text |
| `thread` | optional dotted path to the field that groups records into threads |
| `filters` | a list, possibly empty, of `field` (a dotted path), `values` and `include`. With `include: true` only records whose field equals one of the values are read; with `include: false` those records are left out. A record is read only when it passes every filter |
| `lookup` | optional path of a JSON file holding one object of strings, an id to a name; read against the spec file's directory as `paths` are. It maps the author and every `<@id>` in the text (below) |

A record of a `MarkdownSections` file is one level-two section, with the fields `heading` (the
heading's text) and `body` (the lines up to the next level-two heading, trimmed), so
`id: heading` and `text: ["{body}"]` read each section as written. A level-two heading is a
`## ` line, or a one-line paragraph underlined by `-` characters (`Setup` over `-----`). A
deeper heading stays in its section. Neither form counts inside a fenced code block, which closes
only on a fence of its own character (backtick or tilde) at least as long as the one that opened
it. A `---` front matter block at the top of the file and text before the first heading are not
read. A section whose key is already taken in its file, such as a second `## Notes`, gets
`-2` after its id (`<file path>#Notes-2`), the third `-3`, counted in file order.

A `JsonLines` file is read line by line, and its blank lines are ignored. A leading byte-order mark
is ignored in both formats. What cannot be read is left out and named in `skipped`, in the run's
result and in `cortex.log`, while the rest of the file is still read:

| left out | named as |
|---|---|
| a JSON line that is not UTF-8 | `<path>: line <n> is not UTF-8; skipped` |
| a JSON line that is not JSON | `<path>: line <n> is not JSON; skipped` |
| a markdown file that is not UTF-8 | `<path>: not UTF-8 text; skipped` |
| a markdown file with no level-two heading | `<path>: no ## section; skipped` |

An empty file is skipped and named nowhere, as without `records`.

A record without an id, whose text and every `fallback_text` are empty, or that a filter leaves
out, is skipped. A document's key is `<file path>#<id>`, and its evidence is cited as
`file:<file path>#<id>`; a key read twice in one run counts once. With `author`, the text reads
`<author>: <text>`. Records are read in file order, and files by name, one entry of `paths` after
the other.

**Lookup.** With `lookup`, the author's value and every `<@id>` in the text are replaced by the
name the file gives that id: with the file `{"U1": "Ana", "U2": "Ben"}`, a record by `U1` with the
text `hi <@U2>` reads `Ana: hi @Ben`. A labelled mention `<@U2|ben>` is looked up by its id and
becomes `@Ben` too. An id the file does not hold, or maps to a blank name, is kept as written
(`U9`, `<@U9>`). A leading byte-order mark in the file is ignored, as in record files. The file
is read once per run, before any record; when it is missing, cannot be read or
is not a JSON object of strings, the source's run fails with `fetch-failed` and a reason naming
the file. Names enter the text before the `redaction` policy is applied, so the policy treats
them as any other name in the text.

**Threads.** With `thread`, a record that has the field carries the earlier records of that thread
in the run after its own text, as paragraphs marked `[context]`: the nearest first (the record it
replies to), then older ones. The context holds at most `max_chars_per_document` characters of
their text; where that cuts it, a last paragraph `[context cut]` says so. A record without the
field carries no context and starts the thread named by its own id: a later record whose field
names that id sees it, and not an earlier file's thread of the same id.

**Redaction.** Thread context repeats earlier records, and every record repeats its file's name as
its title. With a `redaction` policy, `RareName` counts a name over each record's own text only,
not over its context, key or title, so a name written once stays rare when it is repeated as
context. A name in the context is replaced like any other, by the same placeholder as in the
record it came from when both are sent in one batch.

A record whose text changed is extracted again on the next run, whatever `refresh_after_days`:
the window holds back only whole files and the other kinds of source. Its thread context is part
of its text, so a record whose nearer thread records changed is extracted again too.

### `kind: structured`

Records turned into entities, properties and relations by a fixed `mapping`, with no model call:
a structured run costs 0.

| field | meaning |
|---|---|
| `input` | `from: connectors` with its `value`: `adapter`, `connection` or `profile`, `operation`, `inputs` and an optional `paging`, as for [`kind: connectors`](#kind-connectors), window included, and optional `children` (below). Or `from: files` with its `value`: `paths` and `glob`, as for [`kind: files`](#kind-files); each matching file is one JSON document (a leading byte-order mark is ignored) |
| `records` | path to the array of records in the answer or in each file, for example `$.people` |
| `mapping` | how a record becomes an entity (below) |
| `dropped` | optional: `Keep` (the default) leaves every value the source asserted earlier; `Supersede` ends each value the source no longer lists (below) |

A file of a `from: files` input that cannot be read, is not JSON, or holds no array at `records`
fails the run with `fetch-failed`, naming the file, and nothing is applied or ended. So does a
path under which no file matches `glob` (a file moved away, a mount not there yet), naming the
path and the glob.

A path is dotted and may start at the root: `$.contact.email` and `contact.email` are the same,
and `$` alone is the answer itself.

| `mapping` field | meaning |
|---|---|
| `node_type` | the node type of every record's entity |
| `id` | path to the record's stable id |
| `name` | path to the entity's name |
| `aliases` | paths to further names; each holds a value or a list of values. A mapped alias resolves across records and sources (below) |
| `properties` | `property`, `path` and an optional `value_kind`: the value at the path is asserted for the property, in the property's value kind (below) |
| `relations` | `relation`, `target_type` and `target_name`: a relation from the record's entity to the entity of `target_type` named by the value at the `target_name` path, one per value when it holds a list |
| `alias_templates` | optional: fixed-format aliases, each a text with `{path}` placeholders filled from the record (below) |
| `store_lookup` | optional: `OwnType` (the default) lets a record meet store nodes of its `node_type` only; `AnyType` writes it onto a store node of another type its name or an alias names (below) |

A record without an id or a name is skipped, and a record whose id was already read in the run
counts once.

**Identity.** A record's identity is `<adapter>:<operation>:<id>`, or `files:<paths>:<glob>:<id>`
for a `from: files` input, its `paths` as written joined by `,` (for example
`files:registry:*.json:P-1`), so one glob under two roots is two record sets. In a path and the
glob, `%`, `:` and `,` are written `%25`, `%3A` and `%2C`. An adapter, operation, path or glob
that masking would read where it stands, with the `:` after it, as an assigned value (one ending
in a credential's name, `vault.secret`) is written as `%x` and the hex of its bytes instead
(`dir:%x7661756c742e736563726574:<id>`); a part masking leaves alone keeps its text. When masking or a `redaction`
rule with a `replacement` would change the id, the identity carries the first 16 hex digits of the
id's SHA-256 in its place, so the id is never stored and two such ids stay two records. The
identity is the record's document key, and its evidence reads `Source: record:<identity>`. EKR treats things of one node
type that share any alias as one thing, so the entity's aliases are, in order:

1. `<name> (<identity>)`, which names the node, for example `Ada Lovelace (directory:people.list:P-1)`;
2. the identity;
3. the mapped `aliases`, as the spec lists them;
4. each of `alias_templates`, rendered from the record.

The bare name and the bare id are not aliases. Two records of one name, or one record's id equal
to another's alias, are two nodes, and a record whose name changes stays the same node. A mapped
alias resolves on purpose: records, and nodes from other sources, that share one are one node. To
merge a record with nodes known by its plain name, map the name as an alias too
(`aliases: ["$.name"]`). EKR adds no alias to a node an extraction matches, so a record whose
mapped alias changes (a renamed path) keeps its node and its old alias, and the new value is not
added as an alias; the property holding it is set as any changed value is.

**Alias templates.** An alias template matches a source's key scheme to the aliases another writer
gave a store's nodes, for example a store adopted with repositories known as `repo:acme/api`:

```yaml
mapping:
  node_type: Repository
  id: "$.id"
  name: "$.path"
  aliases: []
  properties: []
  relations: []
  alias_templates: ["repo:{namespace}/{path}"]
```

A record `{"id": 7, "namespace": "acme", "path": "api"}` then has the alias `repo:acme/api`, and
EKR resolves it to the store's `Repository` node of that alias instead of creating another. A
placeholder is `{path}`, a dotted path into the record as in `aliases` (`$.` allowed before it);
the value there must be a scalar. A record that lacks a field a template names, or holds no scalar
or blank text there, gets no alias from that template, with no error. The spec check refuses
(`create`, `update`, `adopt` and a run alike, naming the source and the template) an empty
template, a template that names no field (every record would share its alias), and any other
placeholder syntax: an unclosed `{`, a lone `}`, `{}`, a nested `{`, or a character in a path
other than ASCII letters, digits, `_`, `-` and the dots between names (a non-ASCII letter is
refused). A template alias matches a store node of the mapping's own `node_type` only, unless
the mapping sets `store_lookup: AnyType`.

**Store lookup.** EKR resolves an alias within one node type, so by default (`store_lookup:
OwnType`, or no `store_lookup`) a record whose alias another writer gave a node of another type
becomes a second node holding that alias. With `store_lookup: AnyType`, the run reads the store
(`ekr snapshot`) before each batch and looks for the nodes, of any type, whose canonical name or
one of whose aliases equals the record's name or one of its aliases (its alias templates
included), both lowercased. In the example above with `store_lookup: AnyType` added, a store whose
`Project` node holds `repo:acme/api` gets the record's properties and relations on that `Project`
node, and no `Repository` node:

- matches all of one type other than `node_type`: the record is written onto that node, as that
  node's type, and a relation or a child's parent relation naming the record in the same run
  names that node;
- matches of more than one type (its own included): the record is refused alone, named in the
  run's `rejected` with the types (`its name or aliases match store nodes of more than one type
  (store_lookup: AnyType): Project, Service`), and not marked seen, so the next run tries it
  again; the rest of its batch is applied;
- a match whose type declares a mapped property in another value kind than the mapping writes:
  refused alike, naming the type, the property and both kinds;
- no match, or matches of its own type only: written as with `OwnType`.

**Values.** Each mapped property is declared and written in one value kind: the `value_kind` its
mapping names (`String`, `Integer`, `Decimal`, `Boolean` or `Timestamp`), else the kind the
store's ontology already declares for that property of the node type (`ekr ontology`), else
`String`. A record's value is converted to the kind:

| kind | a value converts when it is |
|---|---|
| `String` | any scalar, stored as text (a number or a boolean as written in JSON) |
| `Integer` | a JSON integer, or text of a decimal integer (`"4"`) |
| `Decimal` | a JSON number, or text of a decimal number (`"12.50"`), stored as that text |
| `Boolean` | `true` or `false`, in JSON or as text in any case (`"false"`, `"TRUE"`) |
| `Timestamp` | RFC 3339 text (`2026-10-01T00:00:00Z`), stored in milliseconds since the epoch |

A property whose value is absent, empty or null is not set, nor is a `String` property whose
value is a list or an object. A record with a value that does not convert (`"12a"` for an
`Integer`, `"maybe"` for a `Boolean`, a list for any kind but `String`) is left out of its batch
alone: none of its values is applied, the rest of the batch is, and the run names the record's key
and the property in `unconverted` (in its result and in `cortex.log`). The record is marked
seen, like an applied one, so later runs skip it, and it does not count against the run's limit,
until its source text changes. A `value_kind` that differs from the
kind the store declares for that property is refused when the spec is read: `create` answers
`seed-refused` and keeps no instance, `update` answers `seed-change-refused` and changes nothing,
and a run, whose store came to declare another kind since, fails with `apply-refused` before it
applies anything. Each names the source, the node type, the property and both kinds. Changing the
kind of a property the store already holds is a schema change ([`cortex schema`](./commands.md)).
A changed value adds an assertion of the new value. With `dropped: Keep` the old one stays active beside it, so a
property can hold every value a record has had.

**Dropped values.** With `dropped: Supersede`, after a run cortex reads the store
(`ekr snapshot`) for the active assertions this source made, unless the run selected no record
and its listing (every record's key and text), the records it holds back and the parents whose
child call failed are those of the last run that ended the dropped values: such a run reads
nothing of the store, and begins no stage. A commit another writer made in between is compared
by the next run whose listing changed. The assertions this source made are those whose evidence is all
`record:<identity>` of this source's identities. It compares each with what the record lists now
and ends the ones it no longer lists, in transactions of their own (`runs/<run>/ends-<n>.yaml`):

- a property value that changed is **superseded** by the record's new value, from the new value's
  valid time on, so `ekr snapshot --valid-at` still answers the old value for earlier instants;
- a value with no replacement (the record is no longer listed, or no longer has the property or the
  relation) is **retracted**, with a reason naming the record: EKR 0.0.31 supersedes an assertion
  only by another. The node itself stays. A retracted relation's edge is removed when no other
  active assertion of that relation joins the two nodes.

A child record is ended as any record of the source is: a child its parent no longer lists, and
every child of a parent the source no longer lists, has its values ended, its relation to the
parent among them.

The run reports `superseded` and `retracted` when it ended any. Nothing is ended when the fetch
left records unread (a `max_pages` reached, an empty page that named a next one), or when the
source lists no record at all: an empty list is read as a list that failed, so a source cannot
drop its last record. A record whose current text the run did not apply (held back by
`refresh_after_days`, beyond `max_documents_per_run`, rejected, or over the evidence bound) keeps
its values. Values are ended at most 1,000 to a transaction; when a later transaction of a run
fails, the edges of relations an earlier one retracted stay, and later runs do not remove them.

Values another source asserted are never ended. A `kind: connectors` source cites its records as
a structured source does, so `create` and `update` refuse a `dropped: Supersede` structured source
that reads the adapter and operation a `connectors` source of the spec reads, naming both. Two
structured sources whose identities share one prefix (the same adapter and operation, or the same
paths and `glob`) are one source here, as they are one record set: a run of one ends the values of
records only the other lists.

**Relations.** A relation target names a record of the same run by that record's identity (the
target taken as an id) or, when exactly one record of the run has it, by its name or a mapped
alias; the relation then names the target by all that record's aliases. Any other target of the
mapping's own node type is named by the target text and its identity as an id, so it reaches a
record of an earlier run by its id, or a node by a mapped alias; a target of another node type is
named by the target text. The mapping's node type, its properties, the target types and the
relations are added to the store's ontology where it does not hold them yet.

**Children.** A `from: connectors` input may list `children`: operations called once for each
record the source read, whose records become nodes of their own, each linked to its parent's node.

| `children` field | meaning |
|---|---|
| `operation` | the child operation, on the input's `adapter` and `connection`. An operation appears once among a source's `children`: `create` and `update` refuse a second entry |
| `input` | the operation's input: `{a.b}` in a string value is filled from the parent record, as a [`kind: connectors`](#kind-connectors) `child` is |
| `records` | path to the array of records in the child's answer |
| `paging` | optional, as for the input |
| `mapping` | how a child record becomes an entity, as `mapping` above, relations included |
| `time` | optional dotted path to the child record's publication time, which dates its evidence and the facts drawn from it ([how](./operating.md#what-one-run-does)); without it, a child's facts are dated by the run |
| `parent` | the name of the relation from each child record's node to its parent's node |
| `follow` | optional: a rule the parent record must match for the child to be called for it (below) |

A child record's identity is `<prefix>/<operation>:<parent>:<id>`: the source's prefix, the child
operation, its parent's identity after the source's prefix (the parent's id, or its digest), with
`%` and `:` written `%25` and `%3A`, and its own id; for example
`forge:projects.list/tags.list:1:v1.0`. So the children of two parents stay two nodes even when
their names and ids are equal, and a child's identity never holds an id masking or a rule would
change. Masking reads a credential's name followed by `:` and six characters or more as an
assigned value, so a parent whose part would end that way (an id such as `top-secret`) has it
written as `%x` and the hex of its bytes in its children's identities
(`forge:projects.list/tags.list:%x746f702d736563726574:v1.0.0`), and so has a child operation
whose name would (`…client_secret`). A child record without an id or a name by its mapping is
skipped, and one whose identity was already read in the run counts once. A mapped alias of a child
resolves across parents as any mapped alias does: mapping a tag's name as an alias makes the
same-named tags of two projects one node.

Each child record has one relation, named by `parent`, to its parent's node, which it names by all
the parent's aliases, so a parent the run does not apply (over the evidence bound) is still named
`<name> (<identity>)`. The ontology gets each child's node type, properties and relations, and
the `parent` relation from the child's node type to the parent's; children with one `parent` name
share one edge type.

Every run calls each child once for every parent it reads (more with `paging`), whether or not the
parent changed, unless the child has a `follow` rule. Then it is called only for the parents the
rule matches; `field` is a dotted path into the parent record, as `{a.b}` placeholders name one:

| `follow` | the parent matches when |
|---|---|
| `{rule: equals, value: {field: archived, value: false}}` | the value at `field` equals `value`, of the same JSON kind: `false` is not `"false"`; numbers compare by value |
| `{rule: within_days, value: {field: last_activity_at, days: 30}}` | the value at `field` is a text holding an RFC 3339 instant, a date alone (midnight UTC) or epoch seconds, no earlier than `days` days before the run's start; `days` above 0 |
| `{rule: at_least, value: {field: star_count, number: 1}}` | the value at `field` is a JSON number no less than `number`; `number` above 0 |

A parent without the field, or with a value of another kind, does not match. A parent the rule
does not match is still read and applied as a node; only its records of that child are not asked
for, and none of its earlier ones is ended (`dropped: Supersede`), as after a failed call. The run
counts those parents per child in `skipped`
(`tags.list: 2 parents not followed by its rule; …`).

A child call that fails for one parent (Connectors answers `forbidden`, say, or the
answer has no array at `records`) does not fail the run: the parent and its other children are
read and applied, and the run names the parent's identity and the operation, with the reason, in
`skipped`. That parent's records of that operation are not read in that run, and none of its
earlier ones is ended (`dropped: Supersede`). The failure does not hold the window: with `{since}`
in the `inputs`, a parent the next run does not list is not asked again. A child whose walk leaves
pages unread (its `max_pages` reached) leaves records unread as the input's walk does: the run
says so in `stopped`, and ends nothing.

**Links.** A `from: connectors` input with `children` may list `links`: each links every record
of one child (a change) to the first record of another child (a tag) whose comparison holds it,
by one relation, so an agent can ask which tag shipped a change.

| `links` field | meaning |
|---|---|
| `operation` | the compare operation, on the input's `adapter` and `connection` |
| `input` | the operation's input: `{a.b}` in a string value is filled from the parent record, as a child's is, and `{from.a.b}` and `{to.a.b}` from the two tags that bound the range, `to` the tag and `from` the tag before it. The first tag has none before it, so its call fills `{from.…}` with nothing and asks for everything up to the first tag. A parent field named `from` or `to` cannot be read here |
| `records` | path to the array of records in the compare answer |
| `paging` | optional, as for the input |
| `tags` | the child operation whose records are the tags. `create` and `update` refuse one that names no child of the source, naming the field |
| `changes` | the child operation whose records are the changes, refused the same way |
| `order` | dotted path to a time in each tag record, read as a `time` is ([how](./operating.md#what-one-run-does)); the tags are ordered by it, earliest first, and tags of one time by identity |
| `change_id` | dotted path to the value of a compare record that is a change's id, as the changes' mapping reads its `id` |
| `relation` | the name of the relation from a change's node to its tag's node (`shipped_in`, say) |

For each parent it reads, the run orders the parent's tags and makes one compare call per tag,
from the tag before it to it. A compare record whose `change_id` value is the id of one of that
parent's changes links that change to the tag; the change is matched by the identity the run gives
it, so the tags and changes of two parents stay apart even when their names and ids are equal. A
change two ranges hold is linked to the earliest tag only; a change no range holds has no link. For
a parent with no change read, no compare call is made; otherwise every run makes one per tag,
whether or not anything changed. cortex reads only what the compare answers: it tells no branches
apart, and a change reverted after its tag keeps its link.

A run writes the links it finds into each change's record, after cleaning, as the member
`cortex.links` (a list of `{"relation", "tag"}`, each tag by its identity), and the change's
mapped values name each tag under the relation. So a link is part of the text the run compares with
the text it last applied: a change applied before its tag existed is applied again in the run that
first sees the tag, and gains its edge, whatever `refresh_after_days`. The run remembers each
change's text without its links too, so a change whose text differs from the one it last applied
only in its links is not held back by the refresh window; one whose text changed besides waits, as
any document does. The change names its tag by all the tag's aliases, and the
ontology gets the relation from the changes' node type to the tags'. A `cortex.links` member the
provider wrote into a change is replaced, and none is read from any other record; no mapping path
reads it, as a path splits at `.`. With `dropped: Supersede`, a link a change no longer has is ended
as any value is.

A link that cannot be found in full for one parent does not fail the run: a compare call that fails
(Connectors answers `forbidden`, say) or leaves pages unread, a tag with no time at `order`, or
tags not all read (their child call failed or left pages unread). The run names the parent's
identity, the compare operation and the reason, the tag by its identity, in `skipped`. The changes
the calls before it linked keep their links; the parent's other changes get none in that run, as
the range not read may hold a change a later range holds too. With a tag out of order no range is
known, so no call is made for that parent. With `dropped: Supersede`, the run ends none of the
links of that relation the parent's changes hold in the store, for as long as the failure lasts,
however many runs that is. Every other value of those
changes ends as it would without the link, so a change the parent no longer lists loses its other
values while its stored link stays. A change that loses its link for the run is applied again
without it, at once whatever `refresh_after_days`, and again with it once a run finds it. The
failure does not hold the window, as a failed child call does not.

**Evidence and rejections.** Each record is stored as the evidence every fact from it cites: a
`Source:` header, its mapped values (`Mapped values:`, one `<label>: <value>` line each for its
id, name, aliases, properties and relation targets, for a child record its parent's identity
under the `parent` name, and for a change the identity of each tag it is linked to under the
link's `relation`), then the record's JSON, cut at a character
boundary to 16,384 bytes (EKR 0.0.32's bound on one evidence payload). The mapped values come
first, so the cut never takes a value a fact cites. A record whose header and mapped values alone
are over the bound is not applied: the run names it in `skipped` with the reason and records it as
seen, so it is read again only once it changes. Before that, every string in the record has credential
shapes masked and the `redaction` policy's rules with a `replacement` applied; no model sees a
record, so nothing is replaced by a placeholder. `max_chars_per_document` does not cut a record,
and EKR rejects a text value over 65,536 bytes (EKR 0.0.32's string limit). A record with any part
EKR rejects is counted in `parts_rejected`, named in `rejected` with EKR's refusal, is not counted in `documents_applied` and is not marked
seen; the run is then not successful, so later runs read the record and try it again until it
applies. Shorten or unmap the value at the source to clear it. Change detection,
`refresh_after_days` and `max_documents_per_run` work as for any source.

### `policy`

| field | meaning |
|---|---|
| `refresh_after_days` | a document whose text changed is extracted again only when it was last applied at least this many days ago; 0 to 36500. A record read from a file ([`kind: files`](#kind-files)) and a `structured` source's change whose text changed only in its links ([`kind: structured`](#kind-structured)) wait for nothing |
| `change` | how a change is detected; `ContentHash` is the only value |
| `max_documents_per_run` | the most documents one run extracts; above 0 |
| `max_chars_per_document` | longer text is cut to this many characters; above 0. Text is also cut, at a character boundary, to what a 16,384-byte evidence payload holds after its header (EKR 0.0.32's bound), so about 5,400 characters of a three-byte script such as Chinese fit. A change is detected on the whole text, before the cut, and the run counts the cut documents it shows the model as `truncated` |

## `redaction`

Optional. What is kept from the model, and what refuses a run. See [Limits](./limits.md) for
what each class finds.

```yaml
redaction:
  classes: [Url, Email, Phone, Credential, RareName]
  rules: [{name: employee-id, pattern: "EMP-[0-9]{6}", replacement: ""}]
  known_names: [Jane Doe, Rowan]
  rare_limit: 1
  refuse_if_left: [Phone]
```

| field | meaning |
|---|---|
| `classes` | built-in classes replaced in what the model is shown: `Email`, `Phone`, `IpAddress`, `PaymentCard`, `Url` and `RareName` by placeholders that are restored before storage, `Credential` irreversibly |
| `rules` | regular expressions, each with a `name`; an empty `replacement` gives a `[<name>-1]` placeholder that is restored, a non-empty one replaces the match before storage |
| `known_names` | optional names replaced by `[Name-1]`-style placeholders wherever they appear as whole words, in any case, and restored |
| `rare_limit` | optional; for `RareName`, the most times a capitalised word may appear in a batch to be taken as a name. Default 1 |
| `refuse_if_left` | optional classes that fail the run, before any model call, when they are still found in what the model would be shown |

## `snapshots`

Optional. How many snapshots of a `sqlite` store the instance keeps; see
[Operating](./operating.md#undoing-a-run).

```yaml
snapshots: {keep: 3}
```

| field | meaning |
|---|---|
| `keep` | the newest snapshots cortex named that are kept; a new snapshot removes the oldest beyond it. Files of other names in `snapshots/` are never removed. 0 takes none. Without a `snapshots` section, 3 |

## `gate`

Optional. Checks a source run is held to once it applied something; a run that fails one is
undone and fails as `apply-refused`: on a `sqlite` store from the snapshot taken before it, on a
`postgres` store by abandoning the EKR stage it ran in. See
[Operating](./operating.md#failures).

```yaml
gate:
  checks:
    - {measure: facts_refused, max: "0"}
    - {measure: assertions.active, min: "100"}
```

| field | meaning |
|---|---|
| `checks` | the checks, each with a `measure` and a `min`, a `max` or both. Without a `gate` section, or with no checks, a run is not checked |
| `measure` | a number of the run report `cortex run` prints: `documents_new`, `documents_applied`, `facts_refused`, `parts_rejected`, `masked`, `truncated`, `unrestored` or `cost_usd`. Any other name is the dotted path of a number in what `ekr quality` prints for the store after the run, such as `assertions.active` or `sharing_nodes` |
| `min`, `max` | decimals, written as strings; the check fails when the measure is below `min` or above `max`. Both are compared at 4 decimal places, the precision `cortex run` prints a cost at, so a cost printed as `0.3000` is within `max: "0.3"`. A measure that cannot be read (a misspelt name, a `cost_usd` no answer carried, an `ekr quality` that fails) fails its check. Behind a [model gateway](#a-model-gateway) a `cost_usd` check checks nothing: the cost is what `claude` reported, and it bounds nothing |

## `monitoring`

Optional. What a run of a source reports besides its printed line, `cortex.log` and the source's
status in `cortex sources`; see [Operating](./operating.md#failures).

```yaml
monitoring:
  metrics: true
  hook: {command: [/usr/local/bin/notify-oncall, --channel, brains], timeout_s: 10}
```

| field | meaning |
|---|---|
| `metrics` | `true` rewrites `metrics.prom` in the instance directory after every run of a source, in the Prometheus text format, for a textfile collector or a sidecar to read: runs by outcome, the last run's time, duration and documents, and documents and model tokens over time (see [Failures](./operating.md#failures)). Without it, no metric file is written |
| `hook` | optional: a command called once for each run that fails, with the source id and the outcome appended as its last two arguments |
| `hook.command` | the program and its first arguments, run without a shell in the instance directory, in a process group of its own. It must name a program. A relative path holding a `/` is read against the spec file's directory; a bare name is looked up on `PATH`. It holds no secret: a hook that needs one reads it itself |
| `hook.timeout_s` | how long the call may last, in seconds, 1 to 86400; then the hook's process group is killed. Without it, 10 |

## `serve`

| field | meaning |
|---|---|
| `view_port` | optional port for `ekr view` on 127.0.0.1, 0 to 65535; without it, `cortex create` takes the first free port from 18900 |

## `store`

Optional. Where the instance's EKR store lives: `{backend: sqlite}` (the default, without
`store`) is `store.sqlite` in the instance directory; `{backend: postgres, value: {...}}` is EKR's
PostgreSQL provider. A `sqlite` store cannot be placed elsewhere yet: a spec that names
`store.value.path` is refused by `cortex create` (`seed-refused`) and `cortex update`
(`seed-change-refused`), with a reason naming `store.value.path`.

```yaml
store:
  backend: postgres
  value:
    config: ~/brain/app.json
    connection: {adapter: postgres, connection: <connection id>}
    schema_connection: {adapter: postgres, profile: <profile>}
```

`connection` and `schema_connection` each name their `adapter` and either a `connection` id or a
`profile` of that adapter ([Naming a connection](#naming-a-connection)); a profile is resolved
before every launch, to the id `connectors connections launch --connection` is given.

| field | meaning |
|---|---|
| `config` | absolute path, or one starting with `~/`, to the application role's `ekr.postgres/1` file. cortex passes the path to `ekr` |
| `connection` | optional Connectors connection holding the application role's password. With it, every `ekr` that opens the store (runs, `create`, `adopt`, `quality`, `schema`, the viewer and the MCP line) starts through `connectors connections launch --consumer ekr`, which hands `ekr` the connection's `{"password": …}` document on descriptor 3; `config` must then name `"password_file": "/proc/self/fd/3"` and carry no `password` key. Without it, `ekr` starts directly and reads the password where `config` says |
| `schema_connection` | optional Connectors connection of the schema-management role, used the same way for the `ekr postgres-schema --config` that `cortex create --postgres-schema-config` runs: the file `--postgres-schema-config` names must then name `"password_file": "/proc/self/fd/3"` and carry no `password` key. It needs `connection` |

With `connection`, cortex never reads the password: the operator saves it once with
`connectors connections connect`, and pins the `ekr` binary in the Connectors configuration as
the consumer `ekr`, with `pass_env = ["EKR_"]` so the `EKR_HOST`, `EKR_BACKEND` and `EKR_STORE`
cortex sets reach it, and the connections' adapter alias in `permissions.connections`. cortex
removes every other `EKR_*` variable of its own environment from the launches it starts, so none
from the operator's shell reaches the launched `ekr`. That needs
an `ekr` whose `ekr.postgres/1` reads `password_file` (EKR 0.0.32 or later). `cortex create` refuses
a spec whose connection Connectors does not list, or whose profile names not exactly one live
connection (`connection-missing`). A `config` without that
`password_file`, or with a `password` key, is refused by `cortex create` (`seed-refused`),
`cortex update` (`seed-change-refused`) and `cortex adopt` (exit 2), and with `schema_connection`
`cortex create` refuses such a `--postgres-schema-config` file (`seed-refused`) before any `ekr`
runs. Each refusal names the fields, never their values. cortex does not read the connection file
a `config` names; an `ekr` refuses one that carries a password while `password_file` is set.
