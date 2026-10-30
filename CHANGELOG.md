# Changelog

Every user-visible change to cortex. Versions are the `version` in `Cargo.toml`; a release is the
tag `v<version>`. The process is in [AGENTS.md](AGENTS.md#cutting-a-release).

## Unreleased

### What 1.0 holds

1.0 is the first release whose interface is stable. It adds no feature over 0.5.0; it is 0.5.0's
feature set after a real instance ran for 7 days with nobody at the keyboard.

- **Instances.** One spec file makes an instance: an EKR store on SQLite or PostgreSQL, a viewer,
  an MCP line, and one systemd user timer per source. `cortex update` changes sources, model and
  serve settings; `cortex adopt` turns an existing store into an instance without reseeding;
  `cortex remove` keeps the data.
- **Sources.** Web (search, or listed sites read as pages or crawled), Connectors (records of any
  admitted operation, with pages, child calls and a time window), files (local files or records
  read from them, with lookups), and structured imports that need no model call, map child
  operations as linked nodes, link changes to the first tag that ships them, and end what a source
  dropped.
- **Runs.** Only new or changed documents are extracted, by a model call with no tools. cortex
  mints every evidence id, dates facts by when their source said them, supersedes a changed value,
  and writes extracted relations as edges. Credentials are masked before storage; a `redaction`
  policy keeps personal data from the model and a gate refuses a batch it still finds. A snapshot
  before each run makes `cortex restore` undo a bad one, and a gated run on PostgreSQL runs in an EKR
  stage it abandons on failure; two failed runs in a row disable a source, and a daily ceiling
  holds model spend. A metric file, a failure hook and `cortex sources` show how each run ended.
  `cortex quality` judges a sample of facts; `cortex schema` proposes schema changes.
- **Project.** Specified in ESS with 115 conformance scenarios; CI runs `task check` on every pull
  request; each release carries an `x86_64-unknown-linux-gnu` binary checked against `SHA256SUMS`.

Open limits in 1.0 ([Limits](https://beyond10x.github.io/cortex/docs/limits)):

- `model.backend: Codex` is accepted and fails every run; extraction uses `claude -p` only.
- A web page is cited as a `HumanStatement` naming its URL, until EKR admits URL evidence.
- Linux with systemd user units only, and one released target.

### Fixed

- A run sent SIGTERM during its fetch ran the fetch out before it stopped. The fetch is now a safe
  point: the `connectors` call in flight is ended with its process group (SIGKILL after 5
  seconds), a paged or `files` read stops between pages and files, and the run ends `terminated`,
  exit 8, with nothing marked seen and its window kept. A run stopped in the wait before a model
  call is asked again says so in its reason, instead of saying a call was ended.
- A Connectors source failed every run against Connectors 0.60.0, which removed `operations
  invoke`. cortex now calls `connectors invoke <connector>/<instance> <operation> --connection
  <id> --input-stdin`, with the connector and instance read from the connection's `connections
  list` entry, which carries them from Connectors 0.54.0 on, and needs Connectors 0.60.0 or newer.
  An older one fails the run naming that version: from 0.54.0 to 0.59.x by its parse refusal of
  the `invoke` command line, before 0.54.0 by the list entry without a connector and instance,
  naming the connection. A failure envelope that carries its code only in `error.code`, as a
  parse refusal does, is now named by that code instead of "unknown at unknown". The input goes
  on standard input, so no record field or cursor is in the process list, and each call is one
  `invoke`, without a description read before it. A `result` that is not a JSON object fails the
  call naming its JSON type.
- A model call that failed or was cut off at the output limit counted in the day's spend but not
  against the run's `budget_usd` or in the run's `cost_usd`, so a split batch was given more budget
  than remained and the run reported, and a gate's `cost_usd` check read, less than it spent. Every
  costed answer now counts in all three. A run with a failed call that carried no cost now reports
  `cost_usd` null, as `cortex quality`, `cortex schema` and `cortex eval` do.
- A model call that had already finished when the run was sent SIGTERM was dropped as ended, and
  what it cost counted nowhere. Its answer is now read: its cost and tokens count in the day's
  spend and the run's, and the run still ends `terminated`.
- A fact the model marked `replaces: true` with no current value to replace was rejected by EKR
  (`replacement-without-active-assertion`) and its value lost, its document marked seen. cortex now
  applies such a fact again in the same batch without `replaces`, so the value is stored as a plain
  assertion; it is not counted in `parts_rejected` or listed in `rejected`, and the run reports it
  in `facts_reapplied` (in the `ran` detail, and in the `cortex.log` line when not 0). Several
  such facts of one entity and property are applied in their documents' time order, the later
  ones still replacing, so one value stays current. A re-apply the store fails stops the run and
  leaves the documents it carried unseen, so a later run extracts them again.
- `cortex ask serve` recorded an answer whose reader closed the stream as `model-failed`, with no
  tokens, and opened a review item for it. It is now recorded with the outcome `client-closed`,
  the token counts and steps of the model calls already made, and no review item, `slow`
  included. A reader who leaves while the model is asked frees the answer's `max_in_flight` slot
  at once, and no tool call of that reply is made.
- `cortex ask serve` streamed and recorded a model reply cut off at its token limit or by a
  content filter as an answer. Such a reply (`finish_reason` `length` or `content_filter`) now
  ends the stream with an `error` event, reason `cut-off`, is recorded with the outcome `cut-off`
  and its tokens, and opens a `failed` review item. An older cortex cannot read a record with
  either new outcome.
- `cortex ask serve` had no bound on answers in flight, tool calls per step, the gateway's
  answer, or the bodies of votes and review closes. `ask.max_in_flight` (default 8) caps the
  answers worked on at once: a question past it answers 429 `busy` at once and spends no question
  of the minute. `ask.limits.max_tool_calls` (default 16) caps the tool calls one step makes; the
  calls past it are not made, the model is told so, and a `status` event says how many. A gateway
  answer above 4 MiB fails the answer with `model-failed`, and a vote or close body above 64 KiB
  answers 413. The redaction policy is compiled once when the service starts, not twice per
  question. Readers stay one per question, killed when the answer ends.
- `cortex ask serve` sent internal text to HTTP clients: the database path and SQL error of a
  record route's 500, and the gateway's answer or the reader's program path in an `error` event.
  A record route's failure now answers `store-failed` with a fixed message and a `request_id`;
  an answer's `error` event carries a fixed message for `model-failed` and `reader-failed`, ending
  `(request <answer_id>)`. The detail goes to standard error on a line naming the same id.

## 0.6.1 — 2026-10-17

### Added

- `metrics.prom` reports what each run of a source did: `cortex_source_last_run_duration_seconds`,
  `cortex_source_last_run_documents{kind="read|applied"}`, `cortex_source_documents_total{kind}`
  and `cortex_source_model_tokens_total{kind="input|cached|output"}`, beside the runs by outcome.
  No series holds a credential, a prompt or document content.
- `cortex run` handles SIGTERM: the run stops at its next safe point (ending a model call in
  flight), a gated run is undone, and it ends `terminated`, exit 8, with its status, metric file
  and hook written; `--record-failure` does not count it. A second SIGTERM exits 143 at once.
- A run whose model cannot be reached (connection refused, `claude` not startable, the model not
  loaded, a gateway answering 502, 503 or 504, an API answering 529 overloaded) ends
  `model-unavailable`, exit 7, with its cause (`refused`, `not-loaded`, `gateway-down`), instead
  of `extraction-failed`; `--record-failure` does not count it, so a model outage never disables
  a source. Batches applied before it stay. A stop that also ends the model call or the
  `connectors` fetch still ends `terminated`, and the wait before a model call is asked again
  ends on SIGTERM.
- Before each batch, a run looks the batch's document keys and record ids up in the store, across
  every node type, and shows the model each node it finds (type, name, aliases, marked as in the
  store) ahead of its own known names, so a node an adoption or another writer stored is reused
  instead of created again under another type. The 50-per-type bound holds for the run's own
  names only. Names match lowercased. A store node's name and aliases are masked for credential
  shapes on every run and written on one line, and behind a model window they count in the
  batch's fixed part.
- A structured mapping's `alias_templates` add fixed-format aliases from record fields
  (`repo:{namespace}/{path}`), so a source's records resolve to an adopted store's nodes of the
  mapping's own node type. A
  template whose field a record lacks yields no alias; another placeholder syntax is refused when
  the spec is read.
- A figure renderer for the ask service (`cortex.ask.FigureSpec`): a bar chart, a timeline or a
  relationship graph drawn as SVG by cortex's own code, from typed values only. The SVG holds only
  allow-listed elements and attributes, with every label as escaped text; the same figure gives
  the same bytes; a figure over a cap (50 bars, 200 timeline entries, 100 nodes, 300 edges,
  256 KiB) is refused whole. No endpoint draws one yet.

### Changed

- The reason of a model call that exited without a JSON answer is masked for credential shapes
  and bounded, as an error answer's is.

### Fixed

- An RFC 3339 instant with a character outside ASCII (`2026-01-01T00:00:0éZ`) made a figure's
  timeline or a structured record's date panic; it is now refused as no instant. `Z` followed by
  an offset (`…00Z01:00`) is refused instead of read as the offset.

## 0.6.0 — 2026-10-17

### Added

- `cortex ask serve --spec <instance spec>` answers questions over an instance's store as a
  `text/event-stream`: a tool loop over the EKR MCP reader (`ekr mcp`) against the spec's
  `model.gateway`, configured by the spec's new `ask` section. It reads the store only through the
  reader and writes nothing to it.
- With a `redaction` policy, the ask service and `cortex eval run --runner mcp` show the model only
  redacted store contents: one placeholder mapping per answer, held in memory; a result in which a
  `refuse_if_left` class is still detected is withheld from the model.
- Every answer is kept as an answer record with its tool calls, latency, tokens and outcome.
  Readers vote on it (`POST /ask/{id}/vote`, one changeable vote per identity); an answer voted
  down, failed or slower than `slow_after_ms` opens a review item (`GET /review`,
  `POST /review/{id}/close`); records older than `retention_days` are purged. On a `sqlite` store
  they live in `ask.sqlite` in the instance directory, on a `postgres` store in a PostgreSQL schema
  of their own (`ask.records`), never in EKR's. The PostgreSQL records are reached by
  `cortex ask records`, which Connectors launches with the records role's password; the service
  never reads it, and the connection requires TLS unless the URL says `sslmode=disable`.
- `cortex eval run <name> --set <file> --runner mcp|ask|replay` asks each case of an eval set and
  scores each answer with no model call, by the facts it must contain and the nodes it must cite.
- `cortex eval case add <name> --set <file> --from-review <review id>` appends the case of a closed
  review item to an eval set: the question of its answer record and the facts and nodes the
  reviewer expected, with `review_id` naming the item. An open item, an unknown one and one the set
  already holds, and one closed with no expected fact or node, are refused, and the set file is left
  as it was. `POST /review/{id}/close` takes the expectations as `expect_facts` and `expect_nodes`
  beside `reason`.

## 0.5.6 — 2026-10-17

### Added

- `cortex schema <name> --review` has the model propose ontology changes and records each one
  `pending` in `schema/<stamp>/proposals.jsonl`, applying none. `cortex schema approve <name>
  <stamp> [--only <n>…]` and `cortex schema reject <name> <stamp> [--only <n>…] --reason <text>`
  decide each proposal once. `cortex schema apply <name> <stamp>` applies exactly the approved
  proposals, each planned again against the ontology at the head (a type added meanwhile is
  recorded `invalid`), without a model call: on a `postgres` store inside an EKR stage published
  only when the spec's `gate` passes and abandoned otherwise, on a `sqlite` store after a snapshot
  `cortex restore` puts back. An apply that fails leaves its proposals `approved` and the store as
  it was. `cortex schema <name>` without `--review` behaves as before.
- A schema proposal can widen an existing edge type: `widen_edge_type` names the edge type and the
  node types to add to its sources and targets. cortex writes EKR's `WidenEdgeType` with each end
  whole, and records a widening of an unknown edge or node type, or one that adds nothing,
  `invalid`. After it is applied, a fact joining the newly admitted types is no longer refused.

## 0.5.5 — 2026-10-17

### Fixed

- A rare name in underscore italics (`_Rowan_`), in a file name after an underscore
  (`CV_Rowan.pdf`, `2024_Quill.pdf`, the `Quill` of `Rowan_Quill`) or written straight after CJK
  text (`会议上Rowan发言了`) is replaced by `RareName` and found by `refuse_if_left: [RareName]`
  (issue #74). Before, the underscore or the CJK letter before it made it part of a longer word,
  so it reached the model and the check did not see it. An underscore and a letter of a script
  without case now bound a word, as a space does. A word glued directly to a lower-case letter
  (`useFoo`) is still part of an identifier, and so is a word after underscores when the run of
  letters and digits before them begins with a lower-case letter (`snake_case_Identifier`). A
  word now also ends at a CJK letter after it, so `Rowan` is counted alone; a Latin letter
  without case (`ǀ`, `ʔ`) stays part of a word (`Nǀaqriaq`). The rare head
  of a word whose tail is a known name (`Schmidt` of `Schmidt-Müller`) has been replaced since
  0.5.4; a test now holds it.

## 0.5.4 — 2026-10-16

### Fixed

- An extraction batch fits the model it is sent to (issue #72). Behind a gateway with a 64k-token
  window, 60,000 characters of merge-request text were too long for the model (`Prompt is too
  long`) or drew an answer cut off at the output limit, and each such run counted as a failure
  of the source. Now `model.batch_chars` (at least 1,000) sets the batch size, and without it a
  gateway's `max_context_tokens` sets it, less the output reserve (the new
  `model.gateway.max_output_tokens`, passed to `claude` as `CLAUDE_CODE_MAX_OUTPUT_TOKENS`, else
  a quarter of the window) and the request's fixed part. A batch `claude` answers as too long, or
  whose answer was cut off (`Prompt is too long`, or an OpenAI-compatible server's `maximum
  context length`), is split in two and asked again, at most 3 splits deep. One document that
  alone still does not fit is left out, named in the run's new `too_large` field and `cortex.log`,
  and marked seen until its text changes; the run goes on. A part of several documents that still
  does not fit ends the run as the new outcome `batch-too-large`, exit status 6, which
  `--record-failure` does not count: the batches before it stay applied and a later run extracts
  the rest. The size counts UTF-8 bytes of document text, as the 60,000 before it did. A run's
  report and `cortex.log` line state `batch_chars` and `splits`, as does the `SourceRan` event.
- A capitalised word with hyphens that a batch holds more than `rare_limit` times is kept whole
  by `RareName` (issue #71). Before, the part after its first hyphen was replaced as a rare name
  (`Co-[Name-1]` for a frequent `Co-Authored-By` trailer), and with `refuse_if_left: [RareName]`
  the part left standing (`Co`) was then found as a rare name, so the whole batch was refused.
  Now no part of a word is a rare name unless the whole word is, and the `refuse_if_left` check
  reads words the same way. A hyphenated name glued to an underscore or to CJK text
  (`_Jean-Luc_`) still has its rare tail replaced; only a word glued to a lower-case letter, an
  identifier such as `useFoo-Bar`, is skipped whole. The rare head of a word before a span another
  class hides (`Co` of `Co-Jane`, `Jane` a known name) is replaced too, so the check does not
  refuse it, and a text cut at `max_chars` inside a frequent word (`Co-Auth`) is not refused.
- A transient Connectors failure of `connections revalidate` no longer counts against a source.
  A container that started a run within a minute of its own start saw `connections revalidate`
  answer `unavailable` at `dispatch` (`kind: operational`, `next_action: retry_explicitly`); the
  run failed as `fetch-failed`, was counted, and two such runs disabled the source
  (beyond10x/connectors#174). Now a failure Connectors marks retryable, from the revalidation or
  the `connections list` read before it, is tried again: at most 4 attempts, pausing 5 s, 10 s,
  then 20 s (`CORTEX_REVALIDATE_ATTEMPTS`, `CORTEX_REVALIDATE_PAUSE_MS`). One that remains before
  the run fetched anything fails the run as the new outcome `connection-unavailable`, exit
  status 5, which `--record-failure` does not count and which never disables the source; its
  status, metric and hook report it. A refusal is not retried and is counted as before.

## 0.5.3 — 2026-10-16

### Fixed

- Behind a model gateway (`model.gateway`), runs are bounded in tokens only, and no run stops
  over a dollar spend that never happened. `claude` prices a gateway's answer at its default
  model's rate, so `budget_usd` and `daily_ceiling_usd` could stop runs of a self-hosted or
  otherwise unpriced model. Now, with a gateway, the cost `claude` reports counts against no
  `budget_usd`, no `daily_ceiling_usd` and no dollar budget of `cortex quality` or
  `cortex schema`, `--max-budget-usd` is not passed to `claude`, and a run gate check on
  `cost_usd` checks nothing, so it no longer undoes a run over that cost. The cost is still
  recorded as reported, marked as not counted: a run's report and `cortex.log` line carry
  `cost_counted: false`, the `SourceRan` event `cost_counted`, and `spend.json` sums it in
  `uncounted_usd`.

### Added

- With `model.gateway`, `model.budget_usd` may be omitted, and `model.budget_tokens` is required:
  `create`, `update` and `adopt` refuse a gateway spec without it (exit status 2, naming
  `model.budget_tokens`, before anything is written). Without a gateway `model.budget_usd` stays
  required, and a spec without it is refused naming `model.budget_usd`. A gateway spec that still
  gives `budget_usd`, `daily_ceiling_usd` or a gate check on `cost_usd` is accepted, and
  `create`, `update` and `adopt` print one line on stderr saying they bound nothing behind the
  gateway. `create`, `update` and `adopt` also refuse a `budget_usd` or `daily_ceiling_usd` with
  more digits than a finite number holds (exit status 2, naming the field); before, such a value
  was read as an infinite budget. An instance created with a gateway by an earlier release, whose
  spec has no `budget_tokens`, keeps running with its dollar limits counted as before, until an
  `update` gives it one; an instance's own spec is not held to the finite rule either.

## 0.5.2 — 2026-10-16

### Fixed

- A `structured` source writes each mapped property in the value kind the store holds for it.
  Before, cortex declared and wrote every mapped property as a `String`, so a batch mapping a
  property the store declared `Integer` or `Boolean` was refused whole on a PostgreSQL store, and
  on a SQLite store the property was redeclared `String`. A property mapping takes an optional
  `value_kind` (`String`, `Integer`, `Decimal`, `Boolean`, `Timestamp`); without one, cortex uses
  the kind the store's ontology declares for that property of the node type, and `String` when it
  declares none. Each value is converted to the kind (`"4"` to `4`, `"false"` to `false`, RFC 3339
  text to a `Timestamp`). A record whose value does not convert is left out of its batch alone and
  named, with its key and the property, in the run's new `unconverted` field and in `cortex.log`;
  the rest of the batch is applied. The record is marked seen, so later runs skip it (and it no
  longer counts against `max_documents_per_run`) until its source text changes. A `value_kind` that differs from the kind the store declares is
  refused when the spec is read (`create`, `update`, a run), naming the node type, the property
  and both kinds.

- A run whose `ekr apply-extraction`, or the commit of the transaction that ends dropped values,
  answers that its outcome is unknown ("publication outcome is unknown") no longer counts against
  the source: it fails as `store-outcome-unknown` with exit status 4, as an unknown stage verb has
  since 0.5.1. In a stage (a gated run on `postgres`) the run leaves its stage in `stage.json`, and
  the next run abandons it and puts `state/` back; a gated `sqlite` run that had kept its snapshot
  is undone: the snapshot is restored and `state/` put back, as for a run its gate refuses, and
  the run's `cortex.log` line says so, so no batch stays in the store without being held to the
  gate; outside these (an ungated run) the batch is not marked seen, so the next run extracts it again: nothing is
  lost, and a batch that did land is written a second time, a second set of assertions and
  evidence, as any re-extraction writes it. Before, such a
  run was `apply-refused` and counted, and a second one disabled the source.

## 0.5.1 — 2026-10-16

### Fixed

- A run on a `postgres` store whose `ekr stage begin` or `ekr stage publish` answers that its
  outcome is unknown ("publication outcome is unknown") no longer counts against the source. It
  fails as `store-outcome-unknown` with exit status 4; `--record-failure` records nothing, so a
  store fault never disables a source, and the status file, metric and failure hook name the
  outcome. A publish with an unknown outcome keeps `stage.json` and `state/` as they are, and the
  next run publishes that stage at the same head; a stage whose begin was unknown is abandoned by
  the next run. Before, such a run was `apply-refused` and counted, and a second one disabled the
  source.

## 0.5.0 — 2026-10-16

### Added

- `model.gateway` sends the model calls through a gateway: `base_url` (an http:// or https://
  URL) is passed to `claude` as `ANTHROPIC_BASE_URL`, the value of the environment variable that
  `token_env` names as `ANTHROPIC_AUTH_TOKEN`, read when each call is made, and the optional
  `max_context_tokens` as `CLAUDE_CODE_MAX_CONTEXT_TOKENS`. A source's timer unit names the
  variable with `PassEnvironment=` and never holds the token. `ANTHROPIC_API_KEY` is always
  removed, and without a gateway the three gateway variables are removed as well. A `token_env`
  that is not a variable name, or names a variable cortex writes into a unit, any `XDG_*`, any
  `ANTHROPIC_*` or any `CORTEX_*` variable, is refused, as is a `max_context_tokens` of 0 or less.
- Limits in tokens, for models whose cost `claude` cannot state: `model.budget_tokens` bounds a
  run, a quality measurement and a schema proposal, and `model.daily_ceiling_tokens` bounds an
  instance per UTC day across its sources and commands (`spend.json` gains `spent_tokens`). A
  call's tokens are its answer's input, cache and output tokens together. An answer whose tokens
  cannot be counted (no `usage`, a count below 0, a sum that does not fit) stops the run before its
  next batch under either limit, without reporting the daily ceiling reached.

## 0.4.0 — 2026-10-15

### Added

- A spec may name a Connectors connection by `profile` of its adapter instead of by id
  (`profile: tavily.api-key`), on a source and on a `postgres` store's `connection` and
  `schema_connection`. Each command resolves it to the one connection of that adapter and profile
  that is not `revoked`; none or several is refused naming the adapter, the profile, the count and
  every matching id. A `postgres` store's viewer unit holds the id resolved when it was written,
  and `cortex update` rewrites it after a reconnect. The id form stays valid.
- The `NextLink` paging style reads the next page's cursor out of a link in the page body
  (`next: _links.next`), and `stop_older_than_since: true` ends a walk at the first record whose
  `time` is older than the run's `{since}`; the rule needs a `time` and is refused without one.
- `cortex run <instance>/<source> --fetch-only` fetches a source as a run would and prints its
  window and how many fetched documents are new, changed, held or unchanged, with no model call and
  no write to the store, the seen-state or `cortex.log`. A disabled source is fetched too.
- A run's result and `cortex.log` line carry `model_calls`, the input, cache and output tokens the
  calls reported, and an estimate of how the input splits between each prompt's fixed part and the
  documents (`fixed_input_tokens`, `documents_input_tokens`).
- `cortex adopt --seen <source>=<file>` takes entries with a key and `applied_at` and no `hash`:
  the source's first run records the hash of each such document it fetches and does not extract
  it. A key is the one cortex gives the document (for a files source, the file's absolute path).
  An entry with no `applied_at`, or whose fields are not of their types, is refused naming its key.

### Changed

- The specification requires ess 0.60.0.

## 0.3.1 — 2026-10-15

### Added

- `{since_epoch}` and `{until_epoch}` in a string value of a Connectors or structured source's
  `inputs` are replaced by the run's window as whole seconds since the Unix epoch, the same
  instants `{since}` and `{until}` name, for an operation whose bounds are epoch seconds. Any other
  text in braces is passed to the operation as written.

## 0.3.0 — 2026-10-15

### Added

- A run on a PostgreSQL store held to a gate can be undone: it runs in an EKR stage (`ekr stage
  begin`), which is published when the gate passes and abandoned when it fails, with `state/` put
  back. A stage a killed run left is settled by the next run. SQLite runs and runs without a gate
  are unchanged.
- `model.daily_ceiling_usd` holds an instance's model calls, across all its sources and
  `quality` and `schema`, to a spend per UTC day (`spend.json`). A run that reaches it stops
  before its next batch, keeps what it applied, and counts no failure.
- `monitoring` in the spec file: `metrics` writes a Prometheus text file after every run, and
  `hook` calls a command once per failed run. `cortex sources` shows each source's last outcome,
  its time and its reason.
- `cortex run` exits by outcome: 0 for `ran`, 1 for a failed run or an unknown source, 3 for a
  disabled source, 2 for an error before any outcome.

### Changed

- When `claude` answers an error, the run's reason and `batch-<n>/claude/error.txt` keep the text
  it gave, masked and bounded. An error answer that cost nothing is asked once more after a
  minute before the run fails.
- `cortex setup` installs EKR 0.0.34 by default, and the examples pin it.

## 0.2.3 — 2026-10-13

### Changed

- `cortex setup` installs EKR 0.0.33 by default, and the examples pin it.
- cortex is specified and generated with ESS 0.59.0; CI checks with ESS 0.59.0 and aep 0.71.2.

### Fixed

- A `claude` that exits before reading the prompt is reported by its exit status and its own
  error output, not as `cannot send the prompt: Broken pipe`.

### Documentation

- The install commands in the README and the quickstart name the current release; they named
  0.2.0 after 0.2.1 and 0.2.2 were out.
- `STATUS.md` states the repository's state against named targets, each row dated and naming the
  command it was read from.

## 0.2.2 — 2026-10-08

### Fixed

- A structured source whose adapter, operation, `files` path or glob ends in a credential name
  (`vault.secret`) no longer applies its unchanged records again on every run: that part of its
  identities is written in hex, and values applied under the earlier identities still end. Sources
  whose prefix masking leaves alone keep their identities.
- The documentation site waits until GitHub lists the finished build run before it publishes.

### Documentation

- Install from the release tarball checked against `SHA256SUMS`; a Use cases page; the Codex
  backend is documented as unavailable; EKR limits re-checked against 0.0.32.

## 0.2.1 — 2026-10-08

### Fixed

- A run retries a Connectors invocation once when Connectors answers `timeout` at `admission`
  (its own 30 s deadline ran out); a second timeout answers `fetch-failed` as before, and nothing
  else is retried.

## 0.2.0 — 2026-10-07

### Added

- A `structured` source with a Connectors input can call `children` operations per parent record
  and map each child record to its own node, linked to its parent by the edge its `parent:` names and
  dated by its `time` field. A child call that fails for one parent is named in `skipped`, and that
  parent's earlier children of the operation are kept.
- A `structured` source can link each merged change to the first tag that ships it: a `links`
  entry names the tag and change child operations and a compare operation, cortex calls the compare
  once per tag (from the tag before it), and each change gets one edge, named by the link's
  `relation`, to the earliest tag whose comparison holds it. A change merged before its tag gets its
  edge in the run that first sees the tag. A failed compare call or a tag with no usable `order`
  value is named in `skipped`, and that parent's edges already stored are kept.

### Changed

- cortex is specified and generated with ESS 0.55.0 (was 0.52.0); `spec/suite.json` is
  `ess-conformance/34`, and the conformance runner checks `expect_event_values` steps.
- The extraction prompt names each existing property with its value kind
  (`Release(version: String, release_date: String)`), so the model answers in the kind the store
  declares and EKR rejects fewer facts as `extraction-value-mismatch`.
- The `QualityMeasured` event carries the measurement's `rate` and `cost_usd`, and
  `SchemaChangesProposed` its `cost_usd`; `cortex quality` and `cortex schema` print them from the
  event, under the same keys and in the same forms as before.

## 0.1.0 — 2026-10-07

### Added

- `cortex schema <instance> [--sample N] [--dry-run]` proposes ontology changes from a sample of facts
  and evidence and applies the ones EKR accepts; every proposal is recorded under `schema/<stamp>/`.
- `cortex quality <instance> --sample N` judges a sample of facts against their evidence with the
  instance's model and reports a pass rate with its interval under `quality/<stamp>/`.
- A `structured` source reads JSON files, and `dropped: Supersede` ends what a source no longer
  lists: a changed value is superseded, a value no longer listed is retracted.
- The model marks a property value a document says changed as `replaces: true`, so the older value is
  superseded; a run names an EKR validator rejection by its issue codes (`invalid-supersession`).
- Facts are dated by their document's time: evidence `observed_at` comes from the document's `time` or
  `published` (RFC 3339, a bare date or epoch seconds), else the run start.
- A `records` spec can map its author field and inline `<@id>` mentions through a JSON lookup file
  (`lookup`), and can name `fallback_text` templates used when `text` renders empty.
- `<home>/bin/cortex`, the copy every timer of a home runs, has its version recorded beside it.
  `create`, `update` and `adopt` replace it only with the same or a newer cortex and say so in
  their `binary` detail; an older cortex keeps it, naming both versions, unless `--replace-binary`
  is given. `cortex list` shows the version as `binary_version`.
- The test harness no longer fails with `Text file busy`: stand-ins are written by a child
  shell, never held open for writing by a test process.
- A run gate: `gate.checks` bound the run report's numbers or `ekr quality` measures, and a run
  that fails its gate is undone from the snapshot taken before it. A restore now writes into the
  live store, so an attached `ekr mcp` reader no longer blocks it.
- `cortex adopt` turns an existing EKR store into an instance without reseeding: a SQLite store is
  copied with its `-wal`, a PostgreSQL store is used in place, `--host` passes the store's own host
  document and `--seen` carries what each source has seen.
- A files source can yield one document per JSON line or per markdown section, with ids, authors
  and threads; unreadable lines and files are named in `skipped`.
- Run snapshots: a run that applies something first copies the store and `state/`; `cortex restore`
  puts both back after snapshotting the current state, so a restore can itself be undone.
- `examples/agent-tooling.yaml`: a web instance that searches agent-tooling news daily and crawls
  the MCP specification weekly.
- Scheduled runs work with nobody at the keyboard: timers carry the `connectors`, `claude` and
  `codex` paths and `HOME` cortex was created with, and a relative home or a spec given as a bare
  file name no longer breaks them. Run `loginctl enable-linger` once to keep timers running while
  logged out.
- Redaction classes `Url`, `known_names` and `RareName` replace links and personal names with
  placeholders the model sees and the store does not keep; `Credential` masks secrets for good.
  `refuse_if_left` fails a run, before any model call, when a listed class survives.
- A `structured` source imports records from a Connectors operation with no model call and no
  cost: paths map record fields to a node, its aliases, properties and relations. A record's
  identity is `<adapter>:<operation>:<id>`; nodes are named `<name> (<identity>)`, and a plain name
  merges only when the spec maps it as an alias.
- Credentials are masked in a document's title, description and URL as in its text, including
  percent-encoded names, URL passwords and prefixed names such as `client_secret=`; documents
  whose URLs differ only in a credential are one document.
- Connectors sources walk every page (`Token`, `PageNumber`, `Keyset`), call a child operation
  per parent and append its records, and read a window: `{since}`/`{until}` in the inputs open
  at the last successful run (with a 5-minute overlap) and stay open for changes held back,
  runs that stopped and child calls that failed, so no record is skipped. A child call that
  fails on 3 runs in a row is skipped.
- A seed directory that is a symlink, and symlinked files inside one, are copied as regular files.
- An instance's store lives on the backend its spec names: SQLite as before, or PostgreSQL through
  an `ekr.postgres/1` file (`cortex create --postgres-schema-config` provisions the schema). A
  create refuses a tenant that already holds a store; refusals name fields, never their values.
- Redaction before the model: emails, phone numbers, card numbers, IP addresses and spec rules
  become placeholders such as `[Email-1]` in everything the model sees, and are restored before
  the extraction is applied, so the store keeps the real values. A rule with a `replacement` is
  irreversible. Runs log `redacted` and `unrestored` counts, and `cortex run` and `cortex create`
  print them.
- The specification declares what a source has seen (`SeenDocument`) and the seed digest an
  update compares; `cortex list` prints `seed_digest`. An update naming a `..` or absolute path,
  or a renamed seed file, is a seed change.
- The `cortex` command line: `create`, `update`, `remove`, `run`, `source add`, `source enable`,
  `list`, `sources`, `mcp-line`, `setup` and `schema`. Each command prints its outcome as one JSON line;
  a refusal exits 1, a usage or environment failure exits 2.
- Instances: one EKR SQLite store per instance under `$CORTEX_HOME` (default
  `~/.local/share/cortex`), seeded from the spec file's schema, with a viewer and one systemd user
  timer per source. Two failed runs of a source in a row disable its timer; `cortex source enable`
  turns it back on.
- Sources: `web` (search queries or listed sites, read through any Connectors adapter implementing
  `datasource.websearch/v1alpha1`, `tavily` by default), `connectors` (the records of any
  operation a Connectors adapter admits) and `files` (local files matching a glob). A source names
  a Connectors connection id; cortex holds no secret.
- Runs keep only new or changed documents, mask credential shapes, mint the evidence for every
  document, extract facts and relations through an isolated `claude -p` call checked against
  `ekr schema`, refuse a fact citing evidence cortex did not issue, and apply the result with
  `ekr apply-extraction`.
- Connectors answers are read as the CLI returns them, and a connection whose validation lapsed
  is revalidated before a read and renewed once when a read is refused at admission.
- The specification of cortex in ESS (`spec/`, ESS 0.52.0), with the model, the spec file's serde
  types, its JSON Schema and 35 conformance scenarios generated from it.
- A documentation site, published at <https://beyond10x.github.io/cortex/>.
- CI: `task check` and the planning-store validation on every pull request and push to `main`.
- A release workflow: a published GitHub Release `v<version>` gets
  `cortex-<version>-x86_64-unknown-linux-gnu.tar.gz` and `SHA256SUMS` attached; a manual run
  builds the same two files as a workflow artifact.
- A PostgreSQL store can name a Connectors `connection` (and a `schema_connection` for
  provisioning): every `ekr` that opens the store then starts through
  `connectors connections launch --consumer ekr` and reads the password from descriptor 3 as its
  `password_file`, so no file cortex writes holds it.

### Changed

- The EKR pin is 0.0.32 (`cortex setup` default, the examples and CI): its `ekr.postgres/1` takes a
  `password_file`.

### Fixed

- A second home no longer takes over or deletes another home's systemd units for an instance of
  the same name. Each unit records its home as `X-CortexHome=`; `create`, `update` and `adopt`
  refuse a unit another home wrote, naming that home, and `remove`, `restore`, `source enable` and
  `--record-failure` leave it alone. Units written by an earlier cortex are read as belonging to
  the home their service runs. A unit whose home was deleted, or holds no active instance of that
  name, is taken over by the next `create`, `update` or `adopt` and named in `units.taken_over`.
  A home path holding a line break is refused by every command, and no unit is written with a
  value holding one.
- A source named `entities` or `seed` is refused by `create`, `update` and `adopt`, naming the
  reserved name. Its seen documents would have shared `state/entities.json` with the known entity
  names, or `state/seed.json` with the seed, and a source named `entities` was disabled after
  another source ran.
- Evidence over 16 KB is no longer lost: each evidence payload is cut, at a character boundary, to
  the 16,384 bytes EKR 0.0.30 takes, on a source run and on the seed alike, and the text the model
  is shown is cut to what the payload holds. A structured record's evidence leads with its mapped
  values, so the cut never takes a value its facts cite; a record whose mapped values alone are
  over the bound is named in `skipped` and not applied. A run names each document a part EKR
  rejected belongs to, with EKR's refusal, in `rejected`.
- A seed extraction that stops before every seed document is extracted (its budget spent, a model
  call or an apply failed) makes `create` answer `partial` instead of `created`, with the seed's
  `documents_new`, `documents_applied`, `stopped`, `facts_refused` and `parts_rejected`, or
  `failed`. The documents it left are not recorded as seen, and `cortex run <name>/seed` extracts
  them, keeping a snapshot first and held to the gate as a source run is; it refuses an adopted
  instance. A seed that finishes still answers `created`.
