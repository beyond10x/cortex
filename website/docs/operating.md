---
title: Operating an instance
sidebar_position: 5
description: Where instances live, what a run does, the systemd units, failures and the model call.
---

# Operating an instance

## Where things live

Everything lives under the home, `$CORTEX_HOME` or `~/.local/share/cortex`. cortex replaces
each file it rewrites (the registry, a status, a state file) through a temporary file it syncs
to disk, a rename, and a sync of the directory, so a crash leaves the old file or the new one.
A `create` or `adopt` that fails after it recorded the instance takes it out of the registry
again and removes its directory, so it can be retried. A source status file that cannot be read
fails `cortex sources`, naming the file, and a run leaves it as it is (`cortex.log` says why);
other commands list the source as if it had never run. To recover, delete that file: the next
run of the source writes it again, with its counts starting from zero.

| path | holds |
|---|---|
| `registry.json` | every instance and source with its state and counters |
| `cortex.lock` | the lock every writing command holds, so two commands on one home never interleave |
| `bin/cortex` | the copy of cortex the systemd units run |
| `bin/cortex.version` | the version of that copy, recorded when it is placed |
| `instances/<name>/instance.yaml` | the frozen spec file, beside copies of the seed files and instructions it names |
| `instances/<name>/store.sqlite` | the instance's EKR store, unless its spec names `store.backend: postgres`; then the store is in PostgreSQL, opened through the spec's `ekr.postgres/1` file |
| `instances/<name>/ask.sqlite` | with `cortex ask serve` on a `sqlite` store, its answer records, votes and review items (on a `postgres` store they live in the schema `ask.records` names instead) ([Ask service](./ask-service.md#answer-records-votes-and-review)); kept `ask.retention_days` (default 90) |
| `instances/<name>/host.json` | the EKR host document the store is opened with |
| `instances/<name>/meta.json` | the viewer's port and the directory the spec was created from |
| `instances/<name>/state/<source>.json` | what the source has already applied: each document's key, text hash and time |
| `instances/<name>/state/entities.json` | entity names from earlier runs, offered to the model so it reuses them |
| `instances/<name>/failures/<source>.json` | documents whose extraction failed (`cortex.failures/1`): each key's text hash, its consecutive runs failed with the document alone (`0` for one marked by a failed batch of several) and the last reason; outside `state/`, so undoing a run keeps it (see [What one run does](#what-one-run-does)) |
| `instances/<name>/runs/<time>-<source>/batch-<n>/` | per model call: the prompt, the model's answer, the extraction document applied and EKR's report, or claude's error text (`claude/error.txt`) |
| `instances/<name>/snapshots/<time>-<source>.sqlite` | copies of a `sqlite` store, each taken before a run applied anything; the newest `snapshots.keep` (default 3) stay |
| `instances/<name>/snapshots-state/<time>-<source>.json` | each snapshot's copy of `state/` |
| `instances/<name>/stage.json` | while a run on a `postgres` store with a `gate` runs: its EKR stage, the head it began from, whether its publish began, and `state/` from before it (see [Undoing a run](#undoing-a-run)) |
| `instances/<name>/cortex.log` | one JSON line per run: counts, cost, seconds, why it stopped |
| `instances/<name>/spend.json` | the spend of the day (`cortex.daily-spend/1`): the UTC day, what the instance's model answers that day cost (`spent_usd`), held to `model.daily_ceiling_usd`, and the tokens they reported (`spent_tokens`, absent until an answer reported any), held to `model.daily_ceiling_tokens`. Behind a `model.gateway` the costs its answers reported are summed in `uncounted_usd` instead of `spent_usd`, as reported and held to no ceiling (absent until such an answer). With a ceiling, a file cortex cannot read (not JSON or not `cortex.daily-spend/1`), or one of today whose `spent_usd` or `uncounted_usd` is not a number of 0 or more or whose `spent_tokens` is not a whole number of 0 or more, stops every model call of the instance until it is deleted; deleting it counts the day as nothing spent |
| `instances/<name>/status/<source>.json` | the source's last run (outcome, time, reason) and its runs by outcome |
| `instances/<name>/metrics.prom` | with `monitoring.metrics`, the runs of each source by outcome, in the Prometheus text format |

## systemd units

`cortex create` installs systemd user units, in `$XDG_CONFIG_HOME/systemd/user` or
`~/.config/systemd/user`:

- `cortex-<instance>-<source>.timer` and `.service` per source. The timer has
  `OnCalendar=<schedule>` and `Persistent=true`; the service runs
  `<home>/bin/cortex --home <home> run <instance>/<source> --record-failure`.
- `cortex-<instance>-view.service`, which runs `ekr view` on the store at the instance's port.

An instance and a source name may hold `-`, so two instances could name one unit: instance `a`
with source `b-c` and instance `a-b` with source `c` would both be `cortex-a-b-c`, and source
`b-view` of `a` would be the viewer of `a-b`. `create`, `adopt` and `update` refuse such a spec
before changing anything, with or without `--no-units`, naming the unit and the instance or
source that holds it in the home; a source named `view` is refused the same way. A source that
is disabled keeps its timer off through `update`: its units are written again, its timer is not
enabled until `cortex source enable`.

### Which process serves what

| process | serves |
|---|---|
| `cortex` (every command, the timers' runs included) | no port: it runs, prints one JSON line and exits; metrics are a file |
| `ekr view`, in the viewer unit | the store's viewer, at `serve.view_port` |
| `ekr mcp`, from `cortex mcp-line` | the store to one agent, over its stdio |
| `cortex ask serve --spec <file>` | `POST /ask`, the votes and the review queue at the spec's `ask.listen` ([Ask service](./ask-service.md)); it starts its own `ekr mcp` per answer |

cortex installs no unit for the ask service; run it under a unit or supervisor of your own, behind
the guard that verifies the asker and sets the spec's `ask.identity_header`.

Unit names carry no home, and every home of one user shares the unit directory. So each unit
records the home that wrote it as `X-CortexHome=<home>` in its `[Unit]` section (a unit an earlier
cortex wrote is read as belonging to the `--home` its service runs, or for a viewer the home of
its `EKR_HOST`). A home never changes another home's unit: `create`, `update` and `adopt` refuse,
before changing anything, when a unit they would write belongs to another home that holds an
instance of the same name (or whose registry cannot be read), and name that home; pass
`--no-units` to try a spec in a second home. `remove` deletes only its own home's units and names
any it kept in its `units.kept` detail, and `restore`, `source enable` and a source disabled by
`--record-failure` leave another home's viewer and timers alone.

A unit whose home was deleted, or no longer holds an active instance of that name, is orphaned: no
command of that home reaches it. The next `create`, `update` or `adopt` of the name in any home
takes it over, records its own home in it, and names it in its `units.taken_over` detail as
`<unit> (home <old home>)`. A home path holding a line break is refused by every command, since a
unit records its home on one line; so is any value a unit would hold on one line, such as a
schedule.

A unit runs with the systemd user manager's environment, not your shell's: its `PATH` holds no
user tool directory such as `~/.local/bin` or `~/.cargo/bin`, and its working directory is your
home directory. So the source services carry, from the shell that ran `cortex create` or
`cortex update`:

- `PATH`, `HOME`, the `XDG_*` directories and every `CONNECTORS_*` variable, so the tools are
  found on the same `PATH`, and `claude`'s sign-in and the default `ekr` under
  `~/.cache/cortex` under the same `HOME`;
- `CORTEX_CONNECTORS`, `CORTEX_CLAUDE` and `CORTEX_CODEX`: the binaries that command ran, from
  `--connectors`, `--claude` and `--codex` or their defaults. A path is made absolute; a bare name
  is looked up on the unit's `PATH`. `CORTEX_CODEX` is written for every source service, but the
  Codex model backend is not available in this release: an instance whose spec names
  `model.backend: Codex` fails every run without starting `codex` (see
  [Commands](./commands.md#global-options)).
- for an instance whose spec names a `model.gateway`, `PassEnvironment=<token_env>`: the name of
  the variable holding the gateway's token, never its value. systemd passes that variable from
  the user manager's environment, so set it there, for example with
  `systemctl --user set-environment GW_TOKEN=…` or a file under `~/.config/environment.d/`, and
  restart nothing: the next run reads it. A run that finds it unset or empty fails naming it,
  without starting `claude` (see [Spec file](./spec-file.md#a-model-gateway)).

A relative `--home` or `$CORTEX_HOME` is made absolute, so the units name the same home from any
directory. Run `cortex create` from a shell where `connectors` and `claude` work; after changing
either, `cortex update` writes the source units again. It writes the viewer unit only when the
update adds, drops or changes a PostgreSQL store's connection (see "A PostgreSQL store's password"
below).

The timers run only while your systemd user manager runs. To have them run while you are logged
out, enable lingering once: `loginctl enable-linger`.

The units run the copy at `<home>/bin/cortex`, one for every instance of the home. Rebuilding
cortex changes nothing for existing timers until a `create`, `update` or `adopt` installs the
units again and places the new binary. That command replaces the copy when it is the same version
or newer than the one recorded in `<home>/bin/cortex.version`, or when none is recorded. An older
cortex keeps the newer copy, because replacing it would move every timer of the home to the older
one; it writes the new instance's units all the same. Pass `--replace-binary` to replace it
anyway. The command's `binary` detail says what happened: `replaced`, the `old` version recorded
and the `new` one, and the `reason` when the copy was kept. `cortex list` shows the recorded
version as `binary_version`. `--no-units`, and a spec with no source, leave the copy alone.

## Exit statuses of `cortex run`

A scheduler or a script reads which outcome a run had from its exit status alone:

| exit status | outcome | means |
|---|---|---|
| 0 | `ran` | the source ran; `documents_new` 0 means nothing had changed |
| 1 | `fetch-failed`, `extraction-failed`, `apply-refused` | the run failed and applied nothing; with `--record-failure` the failure is counted |
| 1 | `no-such-source` | the home holds no such source |
| 2 | none | an error before any outcome, on stderr as `cortex: <message>` (an unreadable registry, a home that cannot be locked) |
| 3 | `disabled` | the source is disabled after two failed runs in a row; nothing ran. `cortex source enable` switches it back on |
| 4 | `store-outcome-unknown` | EKR could not say whether a store write of the run took effect (`publication outcome is unknown` from a stage verb, `apply-extraction` or a transaction's commit); not a failure of the source, so `--record-failure` counts nothing, and the next run settles the stage or extracts the batch again, and a gated `sqlite` run that had kept its snapshot is undone from it (see [Undoing a run](#undoing-a-run)) |
| 5 | `connection-unavailable` | Connectors answered a retryable failure to every attempt of `connections revalidate` (or of the `connections list` read before it) before the run fetched anything; not a failure of the source, so `--record-failure` counts nothing, and the next scheduled run starts as usual (see [Failures](#failures)) |
| 6 | `batch-too-large` | a part of several documents of an extraction batch was still too long for the model's window, or its answer still cut off, after the batch was split 3 deep; the reason names that part's documents and the batch size used. Not a failure of the source, so `--record-failure` counts nothing; the batches before it stay applied, and a later run extracts the rest. A part of one document does not end the run: it is left out alone, named in `too_large` and marked seen (see [The batch size](./spec-file.md#the-batch-size)) |
| 7 | `model-unavailable` | the model could not be reached: `detail.cause` is `refused` (`claude` could not be started, or the connection was refused), `not-loaded` (the server behind the gateway has not loaded the model) or `gateway-down` (HTTP 502, 503, 504 or 529 overloaded). Not a failure of the source, so `--record-failure` counts nothing, however long the model is down; the batches before it stay applied, and a later run extracts the rest (see [When the model cannot be reached](#when-the-model-cannot-be-reached)) |
| 8 | `terminated` | the run was sent SIGTERM and stopped at its next safe point; a gated run was undone first. Not a failure of the source, so `--record-failure` counts nothing (see [Stopping a run](#stopping-a-run)) |
| 143 | none | a second SIGTERM ended the process at once; nothing was recorded |

With `--fetch-only` (see [Counting what a run would extract](#counting-what-a-run-would-extract)),
`fetched` exits 0 and a fetch that failed exits 1 as `fetch-failed`.

The JSON line on stdout says the same, with its `detail`. With `--record-failure`, a failed run
prints a second line, the recorded failure's (`counted` or `disabled`); the exit status is the
run's. A `store-outcome-unknown`, `connection-unavailable`, `batch-too-large`,
`model-unavailable` or `terminated` run records no failure and prints no second line.

## Stopping a run

`cortex run` handles SIGTERM, which a Kubernetes Job's deadline or `systemctl stop` sends. The
run stops at its next safe point: at once during its fetch or a model call, whose process (with
its process group) is sent SIGTERM and, after 5 seconds, SIGKILL; between the pages of a paged
read and the files of a `files` source; before it uses what it fetched; before each batch; and in
the wait before a model call is asked again. A structured source stops before each record batch.
A run stopped during its fetch marks nothing seen and keeps its window, so the next run asks for
the same `since`. A gated run is then undone as a run its gate refuses is:
on `postgres` its stage is abandoned and `state/` put back, on `sqlite` its kept snapshot is
restored once a batch committed. An ungated run keeps the batches it applied, each marked seen,
and the next run extracts the rest. The run prints `terminated` with a `reason` saying where it
stopped and what was undone, records it in the source's status, the metric file and the hook,
and exits 8. Leave a grace period (Kubernetes' `terminationGracePeriodSeconds`, systemd's
`TimeoutStopSec`) of more than 5 seconds, so a child that ignores SIGTERM is killed before the
run is. A second SIGTERM ends the process at once with exit status 143, recording
nothing. `cortex run --fetch-only`, which writes nothing, ends on the first.

## What one run does

1. **Fetch** the source's documents (web pages or records through `connectors`, or local files).
2. **Mask** credential-shaped text in each document, then cut it to `max_chars_per_document`,
   and further, at a character boundary, to what its evidence holds: EKR 0.0.32 takes an
   evidence payload of at most 16,384 bytes, header included.
3. **Keep** documents whose key was never applied, and documents whose text hash changed once
   their last application is `refresh_after_days` old, at most `max_documents_per_run`. Two
   changes do not wait for that window: a record read from a file, and a change of a `structured`
   source whose text differs from the one last applied only in its links to tags.
4. **Issue evidence.** cortex mints one evidence item per document; the model never does. The
   item is observed when the document says it was written: a web page's published time or a
   record's `time`, read as RFC 3339, as a date alone (00:00 UTC) or as epoch seconds with an
   optional fraction (9 to 11 digits before it, as in a chat export's `ts`). A time that is none
   of these, or lies before 1970 or after the run's start, and a document with no time, are
   observed at the run's start. EKR dates each fact from the earliest observed time of the
   evidence it cites, so a fact is valid from when its source said it. Which documents a run
   keeps, and its window, still go by the run's own times, except that a run that leaves wanted
   documents unapplied moves its window to the earliest of their record times (see
   [the spec file](./spec-file.md#kind-connectors)).
5. **Extract** in batches of up to `model.batch_chars` characters (UTF-8 bytes), 60,000 unless the spec gives a
   size or a gateway's window sets one (see [The batch size](./spec-file.md#the-batch-size)); a
   batch too long for the model's window is split and asked again. Each batch is one `claude -p` call that
   answers in the JSON form of `ekr schema ekr.extraction-document/1`. The prompt carries the
   instance's description, its instructions, the store's node types with each property's value
   kind (`Release(version: String, release_date: String)`), its edge types, and the known
   entities (below). When the spec has a `redaction` policy, every value it finds in the
   documents and the known entities is shown to the model as a placeholder (`[Email-1]`), and each
   placeholder in the answer is put back before merging; the mapping exists for that one call.
   When a document says a property's value changed (a new owner, a new status), the model marks
   that fact `replaces: true`, and EKR supersedes every active value of the same entity and
   property from the new one's time on, so one value stays current and the older one is kept,
   superseded by it. EKR rejects such a fact when there is no current value to replace
   (`replacement-without-active-assertion`); cortex then applies that fact again in the same batch
   without `replaces`, so its value is stored as a plain assertion, and counts it in the run's
   `facts_reapplied` instead of `rejected`. When the batch holds several such facts for one entity
   and property, the earliest (by its document's time) is stored that way and each later one is
   applied after it, still replacing, so one value stays current here too. If that second apply
   fails in the store, the run stops after the batch and the documents whose values it carried
   are not marked seen, so a later run extracts them again. EKR also rejects it when the current value is dated
   later than the replacement (`invalid-supersession`), as when an older document is read after a
   newer one, and the newer value then stays current; that rejection is listed in the run's
   `rejected` and the rest of the document applies.
6. **Merge.** A fact citing an evidence id cortex did not issue for that batch is refused; cortex
   adds the evidence items itself and records each web page as a `WebPage` node.
7. **Apply** the document with `ekr apply-extraction`, then record the batch's documents as seen.
   A document a part EKR rejected belongs to is seen too, since EKR would reject that part again
   on the same text; the run names it in `rejected`, with EKR's refusal or the codes of the issues
   validation raised.
   Before the run's first apply, a `sqlite` store and `state/` are copied, and kept as a
   snapshot once that apply commits (see [Undoing a run](#undoing-a-run)); a copy that cannot be
   taken fails the run with nothing applied.
8. **Check** the run against the spec's [`gate`](./spec-file.md#gate), when it applied
   something. A failed check undoes the run (see [Failures](#failures)).

**Known entities.** A run reads the store once, at its first batch (`ekr snapshot`), and keeps
what it read, adding after each apply the nodes that batch's document named (a new one named by
the least of its first aliases, as EKR names it). A commit another writer makes while the run is
going is not seen by that run; the next run reads it. Before each batch is sent, the run
takes every node, of any type, whose canonical name or one of whose aliases equals one of the
batch's terms, both lowercased: each document's key and, for a record, its id (after the first
`#` of a file record's key `<path>#<id>`, so an id holding `#` is taken whole, after
`<adapter>:<operation>:` of a Connectors record's). The prompt lists these nodes first, under
their own type, marked as in the store and with their other aliases:

```text
Known entities:
- Repository: acme/api (in the store; also repo:acme/api); billing-service
```

so the model reuses a node that an adoption or another writer put in the store instead of
creating it again under another type. Then come the names the run's own earlier batches
extracted (`state/entities.json`), at most 50 per type; a run name that a store hit of its type
already shows is left out. The 50 bound holds for run names only: every store hit of the batch is
listed. Matching is exact after lowercasing (`str::to_lowercase`, not full case folding: `STRASSE`
does not match `Straße`); a similar name is not a hit. A store node's name and aliases are another
writer's text: they are masked for credential shapes on every run, with or without a `redaction`
policy, and then pass through the policy like every known name; a line break or another control
character in a type, name or alias is written as a space, so each type keeps its one line. The
store's ontology names the prompt lists (node types with their properties and value kinds,
relations with their source and target types) are treated the same way: masked, then the
policy, and each written on one line. Behind
a model window, a batch whose store hits make its prompt's fixed part longer than the batch size
allowed for is split in two before it is sent (see
[the batch size](./spec-file.md#the-batch-size)). `ekr snapshot` reads the
whole graph (nodes, edges, assertions and evidence) at the head, once per run, so on a large
store the first batch takes that read's time on top of the model call; a `structured` source
calls no model and makes no such read.

A `structured` source skips steps 5 and 6: each batch of records is mapped to an extraction
document by the source's `mapping` and applied without a model call, at a cost of 0 (see
[the spec file](./spec-file.md#kind-structured)). It masks every string in a record rather than
the text as a whole, and never cuts a record. Each mapped property is written in the value kind
its mapping names or the store declares, else `String`. A record whose value does not convert to
that kind is left out of its batch alone and named in the run's `unconverted`, in its result and
its `cortex.log` line: one entry per value, with the record's key (`document`), the `property` and
its `value_kind`. The field is present only when there is one. The record is marked seen, so
later runs skip it until its source text changes. A mapping whose `value_kind` differs from the kind the store declares
fails the run as `apply-refused`, naming the node type, the property and both kinds, before
anything is applied.

The run stops starting batches when its `budget_usd` is spent, or when the instance's spend of
the UTC day reaches `model.daily_ceiling_usd` (see
[the spec file](./spec-file.md#the-daily-ceiling)). With a limit in tokens, it also stops when its
calls have used `model.budget_tokens`, when the instance's tokens of the UTC day reach
`model.daily_ceiling_tokens`, or when an answer reported no `usage` or a count in it below 0, so
its tokens could not be counted (see [the spec file](./spec-file.md#limits-in-tokens)). Each stop is checked before a
batch, keeps the batches before it applied, and names the limit in `stopped`.

When the model answers that it failed on a batch of several documents (an error answer, or an
answer with no structured output), the run stops there as at any failed batch (after one call),
and the batch's documents are marked. A later run does not ask that batch whole again: it asks
its unmarked documents together, then each marked document alone, those that failed alone fewer
times first, so one document that fails every time does not stop the others, and a failing call
that spends the run's `budget_usd` still leaves the next run a document further. Only an answer
marks or counts a document: an error answer that cost nothing came before any model was asked (a
sign-in or a usage limit), and a call that timed out, crashed or left no JSON answer says nothing
about the documents; neither marks nor counts one. A document that fails alone is counted in
`failures/<source>.json` in the instance directory, beside `state/` and not in it, so a run its
gate undoes keeps the count. Below 3 consecutive failures the run stops at that document: when an
earlier document was applied, the run ends as `ran` with `stopped` set, naming the document and its
count; when none was, nothing was applied and the run fails as `extraction-failed`. From the third
consecutive failure on, while its text is the one that last failed, the document is skipped: it
is named in `skipped` (`<key>: extraction failed with the document alone on 3 consecutive runs
(<reason>); skipped until its text changes`), in the run's result and in `cortex.log`, it is not
sent to the model, it holds no window, and the batches after it run. A changed text keeps the
count and is asked once more alone: applied, it drops the count; failed, it is skipped again. A
run that read the source's whole listing (no window, nothing left unread) drops the counts of
documents the source no longer lists.

### Counting what a run would extract

`cortex run <instance>/<source> --fetch-only` runs steps 1 and 2 and the comparison of step 3,
and stops there. It fetches over the window a run would ask about, masks and scrubs each document
as a run does, and compares its content hash with the source's seen state:

```json
{"command":"run","outcome":"fetched","detail":{"source_id":"news/feed",
 "window":{"since":"2026-10-15T09:50:00Z","until":"2026-10-15T10:00:00Z"},
 "fetched":20,"new":3,"changed":1,"held":0,"unchanged":16,"unread":[]}}
```

`new` counts documents whose key was never applied, `changed` those a run would extract now,
`held` those whose change waits for `refresh_after_days`, and `unchanged` the rest; `fetched` is
their sum. `max_documents_per_run` does not cut the counts. `unread` lists, masked as a run's
`stopped` is, why the fetch left records unread. Use it to check a new source, connection or
schedule, and to count the documents a run at your cadence would send to the model, before
paying for extraction.

Nothing else happens: no model call, no snapshot and no PostgreSQL stage, nothing written to the
store, `state/<source>.json`, the source's status, `metrics.prom` or `cortex.log`, no hook
call, and no run or failure counted. A fetch that fails answers `fetch-failed` and leaves the
window where it was. A disabled source is counted too; `<instance>/seed` is not a source and
answers `fetch-failed`. `--fetch-only` cannot be combined with `--record-failure`. It holds the
home's lock while it fetches, as a run does.

## Failures

A run that fails (`fetch-failed`, `extraction-failed`, `apply-refused`) applies nothing and leaves
the source's seen state where it was. The timers pass `--record-failure`, so a failure is counted;
the second failure in a row disables the source and stops its timer. A successful run resets the
count. `cortex source enable <instance>/<source>` switches it back on.

A run that ends because EKR cannot say whether one of its store writes took effect (`ekr stage
begin`, `ekr stage publish`, `ekr apply-extraction` or the commit of a transaction answering
`publication outcome is unknown; resolve the exact retained occurrence`) fails as
`store-outcome-unknown` and exits 4. The store failed, not the source: `--record-failure` counts
nothing, so `consecutive_failures` does not move and the source is never disabled by it. Its
status, metric and hook report it as any failed run's do. The next run settles the stage, as it
settles the stage of a run that was killed (see [Undoing a run](#undoing-a-run)). A run in no
stage and under no gate marks no document of the batch whose apply was unknown as seen, nor of any
later batch, so the next run extracts that batch again. A `sqlite` run held to a `gate` that had
already kept its snapshot is undone instead, as a run its gate refuses is: the snapshot is
restored, `state/` is put back, and the `cortex.log` line says so; the run still fails as
`store-outcome-unknown`, exits 4 and counts nothing. Nothing is lost; a batch that did land is written a
second time, a second set of assertions and evidence for its documents, as any re-extraction
writes (see [Adopting an existing store](#adopting-an-existing-store)). The batches the run
applied before it stay applied and seen.

Before a source's first read through Connectors, cortex lists its connection and revalidates it
when it is not `ready`. When Connectors answers either with a failure it marks retryable
(`error.data.kind: operational` and `next_action: retry_explicitly`, such as `unavailable` at
`dispatch`), cortex tries again: at most 4 attempts in all, pausing 5 s, 10 s, then 20 s between
them. `CORTEX_REVALIDATE_ATTEMPTS` (default 4, at least 1) and `CORTEX_REVALIDATE_PAUSE_MS`
(default 5000, the first pause, doubled after each attempt) change the bound. A refusal (a
missing, revoked or not-granted connection, an authentication or scope failure) is not retried
and fails the run as `fetch-failed`. A revalidation still failing after the last attempt, before
the run fetched anything, fails the run as `connection-unavailable` and exits 5, its `reason`
naming the last error. Connectors failed, not the source: `--record-failure` counts nothing, so
`consecutive_failures` does not move and the source is never disabled by it; its status, metric
and hook report it as any failed run's do, and the next scheduled run starts as usual. A
revalidation that fails after the run fetched documents (a lapse renewed mid-run) is retried the
same way and, if it still fails, fails the run as `fetch-failed`, counted.

A run that fails a check of the spec's `gate` fails as `apply-refused` and is counted the same
way. Its `reason` names each failed check with its measure and value
(`facts_refused = 1 (max 0)`). On a `sqlite` store the run is undone first: the store and
`state/` are put back to the snapshot taken before it, as `cortex restore` does (a running viewer
is stopped and started again, an attached reader such as `ekr mcp` does not stop it, and the store
as the run left it is kept as `<now>-before-restore`), and the `reason` names that snapshot. If
the store's restore is refused (another connection held its write lock for 5 seconds) or fails,
`state/` is still put back, so the next run fetches the run's documents again; the `reason` says
the store still holds the run. `cortex.log` records the run's line with `gate.failed`, each
failed check's measure, bounds and value, `gate.restored`, the snapshot restored (`null` when the
store was not), and `gate.reason`. On a `postgres` store the run ran in an EKR stage, which is
abandoned, so nothing of the run reaches the store, and `state/` is put back (see
[Undoing a run](#undoing-a-run)); `gate.restored` is `null`, and the `reason` names the stage. A
run on a `sqlite` store with no snapshot (`snapshots.keep: 0`) cannot be undone: it still fails,
and its `reason` says it was not undone.

### Seeing a failure

Every run of a source records its outcome on the source, whatever it is, including `disabled`:
`cortex sources` shows `last_outcome`, `last_run_at` (when the run started, UTC to the second)
and `last_reason` (the run's `reason`; `null` for `ran` and `disabled`). The status is
`instances/<name>/status/<source>.json`. Each write of an instance's status files and its metric
file holds `instances/<name>/status/.lock` for that write only, so no count is lost to a
concurrent writer; it is held for milliseconds, never during a run or a hook. A run of
`<instance>/seed` is not a source's and records no status.

With [`monitoring.metrics`](./spec-file.md#monitoring), every run of a source then rewrites
`instances/<name>/metrics.prom` whole, through a temporary file and a rename, in the Prometheus
text format. Point a node exporter's textfile collector, or a sidecar, at it; cortex serves no
port. It holds these series per source of the instance:

```text
# TYPE cortex_source_runs_total counter
cortex_source_runs_total{source="example/news",outcome="fetch-failed"} 1
# TYPE cortex_source_last_run_timestamp_seconds gauge
cortex_source_last_run_timestamp_seconds{source="example/news"} 1791962441.269
# TYPE cortex_source_last_run_duration_seconds gauge
cortex_source_last_run_duration_seconds{source="example/news"} 12.408
# TYPE cortex_source_last_run_documents gauge
cortex_source_last_run_documents{source="example/news",kind="read"} 3
cortex_source_last_run_documents{source="example/news",kind="applied"} 3
# TYPE cortex_source_documents_total counter
cortex_source_documents_total{source="example/news",kind="read"} 41
cortex_source_documents_total{source="example/news",kind="applied"} 40
# TYPE cortex_source_model_tokens_total counter
cortex_source_model_tokens_total{source="example/news",kind="input"} 52310
cortex_source_model_tokens_total{source="example/news",kind="cached"} 18004
cortex_source_model_tokens_total{source="example/news",kind="output"} 6120
```

| series | type | holds |
|---|---|---|
| `cortex_source_runs_total{source, outcome}` | counter | the source's runs under each outcome (`ran`, `fetch-failed`, `extraction-failed`, `apply-refused`, `store-outcome-unknown`, `connection-unavailable`, `batch-too-large`, `terminated`, `model-unavailable`, `disabled`), every outcome written, 0 for one that never happened |
| `cortex_source_last_run_timestamp_seconds{source}` | gauge | when the last run started |
| `cortex_source_last_run_duration_seconds{source}` | gauge | how long the last run took, to the moment it recorded its outcome, whatever the outcome |
| `cortex_source_last_run_documents{source, kind}` | gauge | the last run's documents: `read`, the new or changed documents it selected to extract (`documents_new`), and `applied` (`documents_applied`). Present when the last run ended `ran`, and when it applied a batch and then ended with another outcome (`extraction-failed`, `apply-refused`, `store-outcome-unknown`, `batch-too-large`, `terminated`, `model-unavailable`): then it reports what the run read and applied before it ended. Absent after a run that ended before it applied a batch |
| `cortex_source_documents_total{source, kind}` | counter | the same, added up over every run that reports them, whatever its outcome; a run that ended before it applied a batch adds nothing |
| `cortex_source_model_tokens_total{source, kind}` | counter | the model tokens those runs reported: `input` (`input_tokens` and `cache_creation_input_tokens`), `cached` (`cache_read_input_tokens`) and `output` (`output_tokens`), each summed over the answers that reported it, a failed answer included. An answer that did not report a count adds nothing to it |

A run that applied batches and then ended with another outcome (`extraction-failed`,
`apply-refused`, `store-outcome-unknown`, `batch-too-large`, `terminated` or `model-unavailable`)
and kept its batches adds the documents it read and applied
and the tokens its answers reported, as a run that ended `ran` does; a run that ended before it
applied a batch adds none. A run that was undone afterwards (its gate refused it, or a gated run
ended `terminated` or `store-outcome-unknown` and was undone) adds the documents it read and the
tokens it spent but no applied document, as the store holds none of them. Every label is the source id or a
fixed name: no series holds a credential, a prompt, a question or a document's content. The
figures are kept in the status file and start from the first run cortex recorded them for.

With [`monitoring.hook`](./spec-file.md#monitoring), each run that fails calls the hook once,
after its line is printed and its failure recorded: `<command…> <instance>/<source> <outcome>`, with
the outcome `fetch-failed`, `extraction-failed`, `apply-refused`, `store-outcome-unknown`,
`connection-unavailable`, `batch-too-large`, `terminated` or `model-unavailable`, or
`disabled` when that run's
recorded failure disabled the source. A run that ran or found the source disabled calls no
hook. The hook runs in the instance directory, in a process group of its own, with cortex's
environment, so a secret it needs (a webhook URL, a token) stays in its own configuration, never
in the spec. A relative program path holding a `/` (`./hooks/notify`) is read against the spec
file's directory, as seed paths are; a bare name is looked up on `PATH`. The call lasts
`timeout_s` seconds at most (default 10), during which a run holding the home's lock keeps it;
then the hook's whole process group is killed. A process the hook starts in the background and
leaves running when it exits is neither waited for nor killed. `cortex.log` records each call as
a line with `source`, `outcome` and `hook.result`: `called` (exit status 0), `failed` (with
`hook.status`, or `hook.reason` when it could not start)
or `timed-out`. Neither a failed hook nor a metric file that cannot be written changes the run's
outcome or its exit status; the latter is recorded in `cortex.log` as `metrics.failed`.

If Connectors refuses a read because a connection's validation lapsed, cortex revalidates the
connection once and retries; that revalidation is itself retried as above.

When `claude` answers an error, the `extraction-failed` reason keeps the error text it gave
(`claude answered an error (success), cost 0.0000 USD: Not logged in …`), and the batch's
`claude/error.txt` under the run directory holds the same text. The text is masked for credential
shapes as fetched text is, and cut at a character boundary to 1,000 bytes (`…` marks a cut). An
error answer that cost zero, the shape of an answer given before any model was asked such as a
sign-in or usage-limit message, is asked once more after 60 seconds; if that answer is valid the
run goes on and no failure is counted, and a second error fails the run as above. An error answer
with a cost above zero, or with no cost reported, is not asked again.

An error answer saying the request is too long for the model's window, or that the answer was
cut off at the output limit, is neither asked again after a wait nor a failure: the batch is split
in two and each half asked (see [The batch size](./spec-file.md#the-batch-size)). A part of one
document that still does not fit is left out alone: the run names it in `too_large`, marks it seen
so it is not asked again until its text changes, and goes on with the batches after it. A part of
several documents that still does not fit after 3 splits ends the run as `batch-too-large` with
exit status 6. The batches before it stay applied and seen; that part and the batches after it are
not marked seen, so a later run extracts them. No failure is recorded, so it never disables the
source; its status, metric and hook report it as for `store-outcome-unknown`. A gated run that
fails its gate ends `apply-refused` as any other, and its reason also names the part that did not
fit.

### When the model cannot be reached

A model call that fails because the model cannot be reached ends the run as `model-unavailable`,
exit status 7, instead of `extraction-failed`, so a model outage never disables a healthy source.
cortex tells it by the call's error text, in any case: `refused` when `claude` could not be
started (or `timeout` could not run it) or the error says `connection refused`, `ECONNREFUSED`,
`Connection error` or `Unable to connect`; `not-loaded` for `model not loaded`, `no models loaded`,
`model is loading` or `currently loading`; `gateway-down` for an `API Error` with status 502, 503,
504 or 529, `Bad Gateway`, `Service Unavailable`, `Gateway Timeout` or `overloaded`. A call that timed out, any other
error and an answer that is not a valid document still end `extraction-failed`, which
`--record-failure` counts. An error answer that cost zero is asked once more first, as every one
is. The `reason` holds the call's error text, masked for credential shapes and bounded; never the
prompt. The batches applied before the model became unavailable stay applied and seen, and are
held to the gate; the rest are extracted by a later run.

## Undoing a run

Before a run's first `apply-extraction` on a `sqlite` store, cortex copies the store, through
SQLite's online backup, and every file of `state/` (each source's seen state and the known entity
names) to hidden temporary files. Once that apply commits, the copies become the snapshot
`<time>-<source>`, where `<time>` is the run's start in Unix milliseconds, the same time that
names the run's directory under `runs/` and its `at` in `cortex.log`:
`instances/<name>/snapshots/<time>-<source>.sqlite` and
`instances/<name>/snapshots-state/<time>-<source>.json`. `cortex run <name>/seed` is a run too,
and its snapshot is `<time>-seed`; the seed `create` extracts into a new store takes none. A run
that applies nothing, or whose
first apply is refused, keeps no snapshot. A copy a killed run left behind is removed by the next
snapshot.

Of the snapshots cortex named, `<digits>-<label>`, the instance keeps the newest
`snapshots.keep` (default 3) and removes older ones with their side files. Any other file in
`snapshots/` is never removed: to keep a snapshot past rotation, copy its `.sqlite` (and its
`.json` in `snapshots-state/`) under a name of your own, such as `pinned-before-incident`.

`cortex restore <name> <snapshot>` puts the instance back to a snapshot, undoing the run that
followed it and every later run, of every source. `<snapshot>` is the name of any `.sqlite` file
in `snapshots/`, without the extension. To undo the last run, restore the newest snapshot:

1. It refuses as `busy` while another cortex command holds the home's lock; it does not wait.
2. If the viewer (`cortex-<name>-view.service`) is running, it stops it. It waits up to 5
   seconds for the store's write lock and refuses as `busy` if another connection held it all
   that time, then starts the viewer again. A reader that holds the store open, such as an
   `ekr mcp` session, does not hold the write lock and does not stop the restore.
3. It snapshots the store and `state/` as they are, as `<now>-before-restore`, which counts
   toward `keep`, and reports the name as `before_restore`.
4. It writes the snapshot into the live store through SQLite's online backup, in one
   transaction, and puts `state/` back to the snapshot's copy, then starts the viewer if it was
   running. The snapshot stays: a restore never removes the snapshot it restored from, so with
   `snapshots.keep: 1` two remain until the next run's snapshot.

Because `state/` is put back, the next runs apply again what the undone runs applied, at the
cost of their model calls. A snapshot with no copy of `state/` in `snapshots-state/` (one you
copied without it) puts back the store only, and the restore reports `state_restored: false`.
So does a copy of `state/` that cannot be written back once the store is restored: the restore
still answers `restored`, with the store at the snapshot, `state/` as it was, and the reason in
`state_failed`.

To undo a restore, restore its `before_restore` snapshot.

A reader attached during a restore keeps a consistent view: a read it has started finishes on
the store as it was, and its next read sees the restored store. After the copy, cortex waits up
to 5 seconds for such reads to finish and writes SQLite's write-ahead log back into the store
file, because until then EKR 0.0.32 refuses to open the store (`store-replaced`). A connection that
keeps one read transaction open for longer delays that: the restore still succeeds, cortex says
so on stderr, and EKR opens the store again once that reader lets go and the next connection to
close the store writes the log back.

A `postgres` store has no snapshot; its backup is the operator's database backup, and
`cortex restore` answers `backend-unsupported`.

A run on a `postgres` store that the spec holds to a `gate` can still be undone, through EKR's
staged runs (`ekr stage`, EKR 0.0.33). Before its first batch, cortex begins a stage (`ekr stage
begin`, which copies the store into a tenant of its own and leaves the store's head where it is,
and answers the head it began at), and writes `instances/<name>/stage.json` with the stage, that
head and a copy of `state/`. Every `ekr` of the run joins the stage with `EKR_STAGE`, which a
Connectors launch passes on, so the run's reads see its own commits and its commits stay in the stage:

- A run that passes its gate is published, `ekr stage publish <stage> --expect-head <head>`:
  every revision it committed lands in the store at once. `stage.json` records that the publish
  began before it does. If the store's head moved since the run began, EKR refuses
  (`stage-head-moved`) and publishes nothing; the stage is abandoned, the run fails as
  `apply-refused` with the refusal in its `reason`, and `state/` is put back. If EKR answers
  that the publish's outcome is unknown, the publish may have landed: the run fails as
  `store-outcome-unknown`, `stage.json` stays with the publish recorded as begun, `state/` is
  not put back and the stage is not abandoned, so the next run publishes it at the same head.
- A run whose `ekr stage begin` answers that its outcome is unknown fails as
  `store-outcome-unknown` before it applies anything; the stage EKR may hold as `Begun` is
  abandoned by the next run.
- On a `sqlite` store, a run held to a `gate` whose later `apply-extraction`, or whose
  dropped-values commit, answers that its outcome is unknown is undone from its snapshot as a
  run that fails its gate is (see above): the store and `state/` are restored, so no batch stays
  in the store without having been held to the gate. It fails as `store-outcome-unknown`, exits 4
  and counts nothing; `cortex.log` records that the snapshot was restored.
- A run whose `ekr apply-extraction`, or a transaction's commit, answers that its outcome is
  unknown fails as `store-outcome-unknown` and leaves its stage in `stage.json`, neither published
  nor abandoned, whatever it applied before: the next run abandons the stage and puts `state/`
  back, so the run's documents are fetched and extracted again.
- A run that fails its gate is abandoned (`ekr stage abandon`): `ekr head` answers the head from
  before the run, and `state/` is put back from `stage.json`.
- A run that fails part-way, before its gate, keeps what it applied, as on `sqlite`: its stage
  is published; a refused publish is said in the run's `reason`, and `state/` is put back. A
  publish whose outcome is unknown keeps `stage.json` and `state/` as they are instead, and the
  next run publishes the stage at the same head; the run keeps its own outcome.
- A run that committed nothing in its stage, whatever its end, has the stage abandoned and
  keeps `state/` as it left it, as on `sqlite`.

A `stage.json` left by a killed run, or by an abandon that failed, is settled by the next run of
the instance. A stage whose publish had begun is published again at the same head, which
completes a publish the kill interrupted, and `state/` is kept as it is; any other stage, or one
whose publish is now refused because the head moved, is abandoned and `state/` put back. Then
every stage of the store still `Begun` in `ekr stage list` (which lists every stage the store
has had), such as one a run began and was killed before it wrote `stage.json`, is abandoned: each
holds a copy of the store. Beginning a stage copies the store, so it takes longer as the store
grows. A run on a `postgres` store with no `gate` runs in no stage.

### Undoing a schema apply

`cortex schema apply <name> <stamp>`, which applies the approved proposals of a review run (see
[Reviewing schema changes](./commands.md#reviewing-schema-changes)), is held the same way, gate or
none. First it settles a `stage.json` a killed run left, as a run does. On a `sqlite` store it
copies the store and `state/` before its first transaction and keeps the copy as the snapshot
`<time>-schema-<stamp>` once one commits; `cortex restore <name> <time>-schema-<stamp>` puts the
ontology back as it was before the apply, and a gate the changes fail restores it at once. On a
`postgres` store it begins a stage, writes `stage.json`, asks every transaction and `ekr quality`
in the stage, and publishes the stage only when EKR answered every transaction and the gate
passed; otherwise it abandons the stage and `ekr head` is where it was. A `stage.json` it leaves
behind is settled by the next run of a source as a run's is.

## Adopting an existing store

`cortex adopt --spec <file> --store <file>` makes an EKR store that cortex did not create an
instance, with its whole revision history. Nothing is seeded, no seed document is extracted and
nothing is written to the store named; `cortex run <name>/seed` refuses an adopted instance, whose
seed belongs to the store's earlier life. The spec's sources are added, and the units installed, as
`create` does.

- **SQLite.** The spec names no `store`, or `store.backend: sqlite`, and `--store` is the database
  file, or a symlink to it. cortex copies it, from a read-only connection on the file it resolves
  to, through SQLite's online backup to `instances/<name>/store.sqlite`, and the instance grows
  the copy. The copy holds every revision, those still in the store's `-wal` included, as when a
  viewer or an `ekr mcp` session holds the store open. The database and its `-wal` are not
  written; SQLite itself creates or updates the `-shm` beside them, and on a store at rest it also
  leaves an empty `-wal` there. The copy is the store at the moment it is taken: stop every
  process that writes to the old file first, since later writes to it do not reach the instance.
- **Free space.** Before copying, cortex checks the free space under the home (statvfs): the store,
  its `-wal` and a 64 MiB margin must fit. If they do not, `cortex adopt` exits 2 naming the bytes
  needed and the bytes free. A copy that fails while writing (a full disk, a file-size limit)
  exits 2 with `cannot copy --store into …`; it is not `store-unreadable`, which means the store
  itself could not be read.
- **PostgreSQL.** The spec names `store.backend: postgres`, and `--store` is the same
  `ekr.postgres/1` file its `store.value.config` names. The store is used where it is; cortex
  provisions no tables. One lineage, the resolved configuration file and the host's tenant, is
  grown by one instance: adopting it while another active instance of the home grows it answers
  `store-held`, naming that instance, and `create` refuses it as `seed-refused`. cortex records
  each `postgres` instance's lineage in `instances/<name>/meta.json`.
- **The host.** EKR opens a store only under the tenant and the authority (its agents and
  validation profile) it was seeded with; under another host it answers "the lineage has no seed"
  or `bootstrap-authority-mismatch`, and adopting answers `store-unreadable`. `--host <file>` names
  the `ekr.cli-host/1` document the store was seeded under, which is copied as
  `instances/<name>/host.json`. Without it, cortex writes its own host for tenant `<name>`, which
  opens only a store cortex created. A run adds node and edge types only under a validation
  profile that admits schema changes, such as the `ekr.p2-apply/1` of cortex's own host.
- **The seed.** Every node and edge type that the spec's `seed.ekr_seed` and `seed.schema` declare
  must be in the store's ontology; otherwise adopting answers `seed-types-missing` and lists them.
  The seed files are frozen with the spec, as for `create`, so `update` refuses to change them.
- **Seen documents.** Each source starts from the `cortex.seen/1` file `--seen <source>=<file>`
  names, such as that source's `state/<source>.json` in another home, or empty; a second `--seen`
  for one source exits 2. With an empty state the first run fetches every document and applies
  each whose content hash it has not seen, which re-extracts the documents the store already
  holds: a second set of assertions and evidence for them, at the model's cost. A document a seen
  file names is not extracted again while its text is unchanged, whether or not the store holds
  evidence of it. `adopted` therefore reports `seen_documents`, every document the seen files name,
  and `seen_without_evidence`, those of them the store holds no evidence of under any identity
  cortex gives a document (its URL, `file:<key>` or `record:<key>`). A non-zero
  `seen_without_evidence` means the seen file is from another store, or the store lost those
  documents' evidence: to have them extracted, remove their keys from `state/<source>.json`.
- **Seen by key alone.** The hash cortex remembers is of a document's text as cortex renders,
  scrubs and masks it, which only cortex can compute. An adopter that knows only which documents
  its store holds names each by its key and `applied_at` (milliseconds since the Unix epoch), with
  no `hash`. The key is the one cortex gives the document, masked: a page's URL, a file's absolute
  path, and for a record read from a file, the file's absolute path, `#` and the record's id. For
  a files source with `records: {format: JsonLines, id: ts}` over `/srv/archive/*.jsonl`:

  ```json
  {"format": "cortex.seen/1",
   "documents": {"/srv/archive/2026-09.jsonl#1.0": {"applied_at": 1}}}
  ```

  The key is absolute even when the spec names the files by a relative path (`paths: [archive]`,
  read against the directory the spec file was created from): a relative key such as `archive/2026-09.jsonl#1.0` matches
  no document, so that record is extracted again. A run's `state/<source>.json` shows the keys
  as cortex gives them.

  The source's first run records the hash of each such document it fetches, keeps its
  `applied_at` and does not extract it; the run's report counts them as `hashed`, apart from
  `documents_new`. A later run extracts the document again when its text changes. A key no run
  has fetched yet keeps no hash. The hash is written with the rest of the run's state, so a run
  its gate undoes puts it back with `state/`, and the next run records it again. An entry with
  no `applied_at` (missing or `null`), or one whose `applied_at` is not an integer or whose
  `hash` is neither a string nor `null`, makes `cortex adopt` exit 2 naming its key.

`adopted` reports the store's head as `revision`. `backend-mismatch`, `store-unreadable`,
`store-held` and `seed-types-missing` leave no instance directory and register nothing.

## The model call

Extraction runs `claude -p` in an empty directory under the run's directory, isolated from the
user's configuration:

- `ANTHROPIC_API_KEY` is removed from its environment, so the signed-in account is used, or the
  gateway the spec names: with a `model.gateway`, exactly `ANTHROPIC_BASE_URL`,
  `ANTHROPIC_AUTH_TOKEN` (the value of `token_env`, read when the call is made) and, when given,
  `CLAUDE_CODE_MAX_CONTEXT_TOKENS` are set from the spec; without one, those three are removed
  too (see [Spec file](./spec-file.md#a-model-gateway));
- `--tools ""`, `--setting-sources ""`, `--strict-mcp-config`, `--disable-slash-commands` and
  `--no-session-persistence`: no tools, no settings, no MCP servers, no slash commands;
- `--json-schema` with EKR's extraction schema, and `--max-budget-usd` with what is left of the
  run's budget, or of the instance's daily ceiling when less is left of it;
- the call is stopped after `timeout_s` seconds.

The model sees fetched text and nothing else. Fetched text is untrusted: it is masked for
credential shapes, stored as evidence and sent to a model that cannot act on it, with the
personal data a `redaction` policy names replaced by placeholders (see [Limits](./limits.md)).

### What a run spends

Besides `cost_usd`, a run's result and its `cortex.log` line say how many model calls it made
(`model_calls`) and the tokens those calls reported in `claude`'s `usage`: `input_tokens`,
`cache_read_input_tokens`, `cache_creation_input_tokens` and `output_tokens`, each summed over the
calls. A call asked again after an error answer that cost zero counts as two calls, and a failed
call counts. A count is `0` for a run with no model call and `null` as soon as one answer did not
report it (or reported it below 0); it is never written as `0` for a count nobody reported.

Each request has a fixed part, sent again with every batch: the system prompt, EKR's extraction
schema, the instance's description and instructions, the store's ontology and the known entity
names. The rest is the batch's documents. `claude` reports input tokens per call, not per part,
so `fixed_input_tokens` and `documents_input_tokens` are an estimate: each call's input tokens,
cache reads and writes included, apportioned by the bytes of the two parts. They add up to
`input_tokens` + `cache_read_input_tokens` + `cache_creation_input_tokens`. A source that runs
often with one to three documents per run can read from them whether the fixed part dominates
what it spends.

## Connections by profile

cortex holds no credential; every source that reads through Connectors, and a `postgres` store's
`ekr`, names a Connectors connection. Where that connection's id changes from one start to the
next, as in a container whose Connectors state is created anew each time (`connectors setup
init`, then `connectors connections connect`), name it by profile instead of by id
(`profile: tavily.api-key`; [Spec file](./spec-file.md#naming-a-connection)). Each cortex
command resolves the profile once, the first time it needs the connection, to the one connection
`connectors connections list --adapter <adapter>` lists with that profile and a state other than
`revoked`, and uses that id for the rest of the command. Connect exactly one connection per
adapter and profile: with none, or two (an old one not revoked), `cortex create` answers
`connection-missing` and a run `fetch-failed`, naming the adapter, the profile, the count and
the matching ids, and no credential is used. Two things outlive a command and carry the id the
profile resolved to when they were written, not the profile: a `postgres` store's viewer unit and
the line `cortex mcp-line` prints. After the connection is connected anew they name the old id
until they are written again: `cortex update` resolves the profile anew and rewrites the viewer
unit when the id changed, and the MCP line is printed again after each connect.

## A PostgreSQL store's password

A `postgres` store whose spec names `store.value.connection` gets its password from Connectors:
every `ekr` that opens the store, the viewer's and the MCP line's included, starts through
`connectors connections launch --consumer ekr`, and `cortex update` rewrites and starts the viewer
unit when an update adds, drops or changes the connection ([Spec file](./spec-file.md#store)). Any
other update leaves the viewer unit alone, so a viewer the operator stopped stays stopped.

The one `ekr` cortex starts directly is the provisioning of `cortex create
--postgres-schema-config <file>` when the spec names no `store.value.schema_connection`: it runs
`ekr postgres-schema --config <file>` itself. That file is the operator's own schema-role
configuration, given on the command line for this one call and never copied into the instance, so
its password stays wherever the operator keeps it. Name `schema_connection` to start that `ekr`
through Connectors too; `<file>` must then name `"password_file": "/proc/self/fd/3"` and carry no
`password` key, or `cortex create` refuses it as `seed-refused` before any `ekr` runs.

The answer records of `cortex ask serve` on a `postgres` store have a password of their own, the
records role's, in the connection `ask.records.connection` names. Only the records process reads
it: the service starts `cortex ask records` through `connectors connections launch --consumer
cortex-records`, which hands it the document on descriptor 3. A deployment pins the `cortex`
binary as the consumer `cortex-records` in the Connectors configuration, with no variable passed
([Ask service](./ask-service.md#records-on-a-postgresql-store)).

## Serving a store

`cortex list` prints each instance's viewer address, `http://127.0.0.1:<port>/`.
`cortex mcp-line <name>` prints a `claude mcp add --transport stdio cortex-<name> -- … ekr mcp`
line with the instance's host document and store, which registers the store as an MCP server.
For a `postgres` store that names a `connection`, the line, like the viewer unit, starts
`ekr mcp` through `connectors connections launch … --args '["mcp"]'`, so the server gets the
database password from Connectors (see [Spec file](./spec-file.md#store)).
