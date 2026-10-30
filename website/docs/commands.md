---
title: Commands
sidebar_position: 4
description: Every cortex command, its arguments and what it prints.
---

# Commands

Every command that changes something prints one JSON line,
`{"command": …, "outcome": …, "detail": …}`, and exits 0 for a successful outcome and 1 for a
refused one. An error before any outcome (an unreadable spec file, a home that cannot be locked)
goes to stderr as `cortex: <message>` with exit status 2. `cortex run` has six of its own:
3 for a disabled source, 4 when the store outcome is unknown (`store-outcome-unknown`), 5 when
Connectors stayed unavailable before anything was fetched (`connection-unavailable`), 6 when a
batch did not fit the model's window (`batch-too-large`), 7 when the model could not be reached
(`model-unavailable`) and 8 when the run was sent SIGTERM (`terminated`); see [Exit statuses of `cortex run`](./operating.md#exit-statuses-of-cortex-run).
`cortex list` and `cortex sources` print one JSON line per row.

The outcomes are the ones the specification declares; the
[generated reference](./reference/ess/cortex-instance.md#commands) lists each command's outcomes,
errors and events.

## Global options

| option | environment | default |
|---|---|---|
| `--home <dir>` | `CORTEX_HOME` | `~/.local/share/cortex` |
| `--connectors <bin>` | `CORTEX_CONNECTORS` | `connectors` |
| `--claude <bin>` | `CORTEX_CLAUDE` | `claude` |
| `--codex <bin>` | `CORTEX_CODEX` | `codex` |

:::note[The Codex backend does not extract]
`--codex` names the binary for a spec whose `model.backend` is `Codex`. In this release that
backend calls no model: every run of such an instance fails as `extraction-failed`, with a reason
naming `story:codex-model-backend`, and `codex` is never started; `cortex quality` and
`cortex schema` fail the same way (`judge-failed`, `propose-failed`). Use `Claude`, the default.
:::

## Instances

| command | does | outcomes |
|---|---|---|
| `cortex create --spec <file> [--no-extract] [--no-units] [--replace-binary] [--postgres-schema-config <file>]` | create an instance from a spec file and add its sources. `--no-extract` skips the seed documents, `--no-units` installs no systemd units, and `--replace-binary` replaces `<home>/bin/cortex` even when it is newer than this cortex (see [Operating](./operating.md#systemd-units); `adopt` and `update` take it too). For a `postgres` store, `--postgres-schema-config` names the `ekr.postgres/1` file of the schema-management role, and `ekr postgres-schema` creates the provider tables with it before the seed; without it the tables must already exist. `partial` is `created` for a seed that stopped before every seed document was extracted: its `seed` detail adds `documents_new`, `stopped`, `facts_refused` and `parts_rejected` (or carries `failed`), and `cortex run <name>/seed` extracts the rest | `created`, `partial`, `name-taken`, `connection-missing`, `seed-refused` |
| `cortex adopt --spec <file> --store <file> [--host <file>] [--seen <source>=<file>]… [--no-units] [--replace-binary]` | make an existing EKR store an instance, with its whole revision history, and add the spec's sources: nothing is seeded, extracted or written to the store named (see [Operating](./operating.md#adopting-an-existing-store)). `--store` is the SQLite database, which is copied into the instance, or for a `postgres` spec the `ekr.postgres/1` file its `store.value.config` names. `--host` is the `ekr.cli-host/1` document the store was seeded under. `--seen` starts a source from a `cortex.seen/1` file; without it the first run re-extracts every document. `adopted` reports the store's head as `revision`, and `seen_documents` and `seen_without_evidence` (seen documents the store holds no evidence of) | `adopted`, `name-taken`, `backend-mismatch`, `store-unreadable`, `store-held`, `seed-types-missing` |
| `cortex update <name> --spec <file> [--no-units] [--replace-binary]` | replace the instance's spec: sources, model and serve settings. New sources are added and every source's timer is (re)installed; the seed cannot change | `updated`, `seed-change-refused`, `not-active`, `no-such-instance` |
| `cortex remove <name>` | stop and delete the instance's timers and viewer that its home wrote (`units.kept` names any of the same name another home wrote); its directory and store stay | `removed`, `wrong-state`, `no-such-instance` |
| `cortex restore <name> <snapshot>` | put a `sqlite` store and the instance's `state/` back to a snapshot a run took before it applied anything (see [Operating](./operating.md#undoing-a-run)). It snapshots what it replaces as `<ms>-before-restore` and reports the name as `before_restore`, and `state_restored` (with the reason in `state_failed` when its copy of `state/` could not be written back); a running viewer is stopped and started again. It does not wait for the home's lock | `restored`, `busy`, `no-such-snapshot`, `backend-unsupported`, `no-such-instance` |
| `cortex quality <name> --sample <n>` | measure how often the instance's facts are supported by the evidence they cite (see [Measuring fact quality](#measuring-fact-quality)). It reads the store and writes nothing to it, so a removed instance is measured too. It holds the home's lock only while it draws the sample, so the timers' runs do not wait behind the model calls | `measured`, `sample-failed`, `judge-failed`, `no-such-instance` |
| `cortex eval run <name> --set <file> --runner mcp\|ask\|replay [--from <stamp>]` | ask each case of an eval set and score each answer with no model, by the fact strings it must contain and the node ids it must cite (see [Running an eval set](#running-an-eval-set)). It reads the store and writes nothing to it. It holds the home's lock only while it reads the set, the spec and a replayed run | `evaluated`, `set-invalid`, `runner-failed`, `no-such-instance` |
| `cortex eval case add <name> --set <file> --from-review <review id>` | append the case of a closed review item of the answer service to an eval set, creating the file when it does not exist (see [Adding a case from a review](#adding-a-case-from-a-review)). An open item is refused and the set file is left as it was | `added`, `set-invalid`, `no-such-review`, `review-open`, `no-expectations`, `case-exists`, `no-such-instance` |
| `cortex schema <name> [--sample <n>] [--dry-run \| --review]` | have the instance's model propose changes to the store's ontology from a sample of its facts (default 40) and apply each one EKR applies as a schema transaction of its own (see [Proposing schema changes](#proposing-schema-changes)). `--dry-run` records the proposals and applies none; `--review` records each one `pending` for a decision (see [Reviewing schema changes](#reviewing-schema-changes)). It holds the home's lock while it draws the sample and while it applies, not during the model calls | `applied`, `proposed` (a dry run), `reviewed`, `sample-failed`, `propose-failed`, `no-such-instance` |
| `cortex schema approve <name> <stamp> [--only <n>]… [--by <who>]` | approve the pending proposals of a review run: those `--only` names by their line (from 1), or every pending one | `approved`, `not-pending`, `no-such-run`, `no-such-instance` |
| `cortex schema reject <name> <stamp> [--only <n>]… --reason <text> [--by <who>]` | reject them, with the reason | `rejected`, `not-pending`, `no-such-run`, `no-such-instance` |
| `cortex schema apply <name> <stamp>` | apply the run's approved proposals, with no model call, in an EKR stage (`postgres`) or after a snapshot (`sqlite`), held to the spec's `gate` | `applied`, `not-approved`, `gate-failed`, `apply-failed`, `store-outcome-unknown`, `no-such-run`, `no-such-instance` |
| `cortex list` | instances: name, state, description, model, EKR version, seed digest, viewer address, and the version of `<home>/bin/cortex` their timers run (`binary_version`, null when none is recorded) | |
| `cortex mcp-line <name>` | print the `claude mcp add` line that serves the instance's store through `ekr mcp` | |
| `cortex ask serve --spec <file>` | answer questions over the store of the instance the spec names: `POST /ask` on the spec's `ask.listen` streams the events of `cortex.ask-events/1`, from a tool loop over `ekr mcp` against `model.gateway`, until stopped (see [Ask service](./ask-service.md)). A spec it cannot serve exits 2, naming the field | `answered` (`done`), `unidentified`, `rate-limited`, `limit-reached`, `model-failed`, `reader-failed` (`error`) |

`cortex update` refuses a spec whose `name` is not `<name>`. `create`, `adopt` and `update` name each unit they took over from a home that no longer holds the instance in `units.taken_over` (see [Operating](./operating.md#systemd-units)). Every command refuses a home whose path holds a line break.

### Measuring fact quality

`cortex quality <name> --sample <n>` draws `n` (1 to 1000) of the store's active facts with
`ekr sample`, each with the evidence it cites, seeded with the measurement's start in Unix
milliseconds. Each batch of 20 goes to the instance's model with no tools and a judge's own system
prompt, called as a run's extraction is, and shown as the spec's `redaction` policy shows a run's
documents. The model answers `yes`, `no` or `unclear`, with a reason, for each fact. `ekr
fact-quality` turns the verdicts into a pass rate with its Wilson interval at 95 %; `yes` counts as
passed, and `no` and `unclear` as failed. The measurement spends at most the spec's
`model.budget_usd`, and no more than its `model.daily_ceiling_usd` leaves of the UTC day (see
[the spec file](./spec-file.md#the-daily-ceiling)).

It writes two files under `quality/<UTC stamp>/` in the instance directory (the stamp is
`YYYYMMDDTHHMMSSZ`):

| file | holds |
|---|---|
| `verdicts.jsonl` | one `cortex.quality-verdict/1` object per fact: `fact` (the EKR assertion id), `verdict` and `reason`. A fact the model gave no verdict for is `unclear`; a verdict for a fact the batch did not hold is dropped |
| `fact-quality.json` | the `ekr.fact-quality/1` document as `ekr fact-quality` printed it: `judged`, `passed`, `failed`, `rate`, `lower`, `upper` |

A `measured` line's detail carries `stamp`, `revision` and `seed` (which draw the same sample
again), `judged`, `passed`, `unclear`, `rate` (null when the store holds no fact), `lower`,
`upper`, `cost_usd` (as a run reports it: null when an answer carried no cost) and `dir`.
`sample-failed` (EKR refused the size or could not read the store) asks no model and writes
nothing. `judge-failed` (a model call failed or found the budget or the daily ceiling spent, or `refuse_if_left`
refused what the judge would be shown) writes no `fact-quality.json`; `verdicts.jsonl` keeps the
verdicts of the batches judged before, and a failed model call's reason states the cost so far.

### Running an eval set

`cortex eval run <name> --set <file> --runner <runner>` asks each question of an eval set, in the
set's order, and scores each answer deterministically. The set is a JSON object whose `cases` are
`cortex.eval-case/1` objects, at least one, with distinct ids:

```json
{"cases": [
  {"format": "cortex.eval-case/1", "id": "widget",
   "question": "Who develops the Widget engine?",
   "expect_facts": ["Widget engine"], "expect_nodes": ["node-widget"]}
]}
```

A case is held to the published `cortex.instance.EvalCase` schema: `expect_facts` and
`expect_nodes` are both required lists of strings, no other key is admitted, `source` and
`review_id` are strings, and no expected fact or node is empty or white space only. A case the schema refuses makes the
set `set-invalid`.

An answer passes when it contains every `expect_facts` string (letter case and runs of white
space ignored) and cites every `expect_nodes` id exactly. An optional `source` says where a case
came from and is not scored.

| runner | answers with |
|---|---|
| `mcp` | the instance's model (the `claude` backend), with the store's MCP reader, `ekr mcp` as `cortex mcp-line` starts it, as its only tools (`--allowedTools mcp__ekr`, every built-in tool off). It answers `answer` and `cited_nodes`, and spends at most the spec's `model.budget_usd` and what its `model.daily_ceiling_usd` leaves of the UTC day, held to its token limits, as `cortex quality` is |
| `ask` | a running `cortex ask serve`. This release has none, so the runner answers `runner-failed` |
| `replay` | the answers the eval run `--from <stamp>` (a directory under `eval/`) recorded, by case id and question: a case whose id the run recorded under another question, or a run with no `summary.json` (a failed run), is `runner-failed`. It asks no model; two replays of one run write the same `results.jsonl`, byte for byte |

:::note[Redaction under the `mcp` runner]
With a `redaction` policy in the spec, the `mcp` runner's model reads the store through a
redacting MCP server in the `cortex` process instead of `ekr mcp` directly: MCP over HTTP on
`127.0.0.1`, under a random path and behind a random bearer header, for one case. The `--mcp-config`
that names it is a file only its owner reads, removed after the case: no secret is on the command
line. The server stops when the case ends, whatever connections are open. Each case is one redaction batch: its question and
every tool result are redacted as the [ask service](./ask-service.md#redaction) redacts them, a
result in which a `refuse_if_left` class is still detected reaches the model as a tool error saying
it was withheld by policy, a question that still holds such a class is recorded as a failed case
with the refusal as its answer and no model is asked, and the answer is restored before it is scored and written. Without a
policy the model reads `ekr mcp` as before.
:::

It writes two files under `eval/<UTC stamp>/` in the instance directory:

| file | holds |
|---|---|
| `results.jsonl` | one `cortex.eval-result/1` object per case, in the set's order: `id`, `question`, `answer`, `cited_nodes`, `passed`, `missing_facts` (the expected fact strings the answer does not contain) and `missing_nodes` (the expected node ids it does not cite). A line holds no time or cost |
| `summary.json` | written last, by a temporary file renamed into place, so it exists only when every result is on disk. `cortex.eval-run/1`: `stamp`, `instance_name`, `runner`, `cases`, `passed`, `facts_expected`, `facts_found`, `nodes_expected`, `nodes_found`, `model_calls`, `input_tokens`, `output_tokens`, `cost_usd`, and `from` for a replay |

An `evaluated` line's detail carries the counts of `summary.json`, `name` and `dir`;
`input_tokens`, `output_tokens` and `cost_usd` are null as soon as one answer carried no count or
cost, and 0 for `replay`. `set-invalid` (the set cannot be read or is not a `cortex.eval-case/1`
set, or `--from` is missing for `replay` or given for another runner) asks no model and writes
nothing. `runner-failed` (a model call failed or found the budget or the daily ceiling spent, no
ask service answered, the replayed run recorded no answer, or another question, for a case, or it wrote no `summary.json`) writes no
`summary.json`; `results.jsonl` keeps the results of the cases answered before, and the reason
states the cost so far.

### Adding a case from a review

`cortex eval case add <name> --set <file> --from-review <review id>` turns a closed review item of
the instance's answer service ([Answer records, votes and review](./ask-service.md#answer-records-votes-and-review))
into one case appended to the set file. The case's `id` and `review_id` are the review id
(`<answer id>:<cause>`), its `question` is the question of the item's answer record, and its
`expect_facts` and `expect_nodes` are those the reviewer sent when closing the item (empty when
they sent none). A set file that does not exist yet is created holding the one case; an existing
one is rewritten whole, through a temporary file renamed into place; a set path that is a symbolic
link is written at its target. It opens the answer records read-only: it creates no `ask.sqlite` (an
instance that served no question answers `no-such-review`) and migrates none, and writes nothing to
them or to the store.

| outcome | when | the set file |
|---|---|---|
| `added` | the item is closed; the detail carries `id` and `cases`, the set's cases with the new one | ends with the new case |
| `set-invalid` | the file exists and cannot be read or is not a `cortex.eval-case/1` set | unchanged |
| `no-such-review` | no review item carries the id, or its answer record is gone | unchanged |
| `review-open` | the item is still open: its expectations are written when it is closed | unchanged |
| `no-expectations` | the item is closed with no expected fact and no expected node (one closed before expectations were recorded has none): such a case passes every answer | unchanged, a missing file not created |
| `case-exists` | the set already holds a case with the review id as its `id` or `review_id` | unchanged |
| `no-such-instance` | no instance has the name | unchanged |

### Proposing schema changes

`cortex schema <name> [--sample <n>]` draws `n` (1 to 1000, default 40) of the store's active facts
with `ekr sample`, each with the evidence it cites, and reads the ontology in force with `ekr
ontology`. Each batch of 20 goes to the instance's model with no tools and its own system prompt,
shown as `cortex quality` shows it to the judge (the spec's `redaction` policy applied), beside the
ontology's type and property names. The model answers a list of proposals, each one change with a
reason, the ids of the facts it rests on and the labels (`E1`, `E2`, …) of the evidence it rests
on. A proposal citing a fact or a label its batch does not hold, citing no fact, or not in that form, is dropped.
The round spends at most the spec's `model.budget_usd`, and no more than its `model.daily_ceiling_usd`
leaves of the UTC day.

The changes the model may propose are a closed list:

| change | what cortex does |
|---|---|
| `add_node_type`, `add_edge_type`, `add_property` | asks EKR to apply it (`DefineNodeType`, `DefineEdgeType`, `ModifyProperty`). Every property cortex declares is optional, so no node the store holds becomes invalid |
| `widen_edge_type` | names an edge type the ontology declares and the node types to add to its sources (`add_source_types`), its targets (`add_target_types`) or both; asks EKR to apply it (`WidenEdgeType`) with each end written whole, what it holds now and the additions, so an end never loses a type. It is `invalid` when the edge type or a named node type is not in the ontology, or when it adds no type new to either end. Once applied, an edge between the newly admitted types is no longer rejected |
| `redeclare_property` | asks EKR to apply a new value kind or cardinality to a property the type declares itself (`ModifyProperty`); whether it is required and its constraints stay as declared. EKR refuses one the values the store holds do not fit |
| `remove_node_type`, `remove_edge_type`, `remove_property`, `merge_types`, `split_type` | records it for review: EKR removes, merges and splits nothing |

Without `--dry-run`, cortex takes the home's lock again and decides each proposal in order against
the ontology at the head: each one cortex can write is its own schema transaction and its own
schema version, so `ekr ontology --at <revision>`, with the `revision` the detail reports, still
answers the schema before. It writes `schema/<UTC stamp>/proposals.jsonl` in the instance
directory, one `cortex.schema-proposal/1` object per proposal not dropped: `change`, `reason`,
`facts` (EKR assertion ids), `evidence` (the EKR evidence ids the proposal and its facts cite) and
`status`:

| status | means |
|---|---|
| `applied` | committed; `revision` and `schema_version` say where |
| `refused` | EKR's validation refused it; `codes` holds EKR's issue codes and `note` its messages |
| `recorded-only` | a remove, merge or split, recorded and not applied |
| `invalid` | cortex could not write it against the ontology, and `note` says why: a type or property it names is not there; a type it adds already is, in any case; a property it adds is one the type, a type it inherits from or a type inheriting from it already declares, in any case; a property it redeclares is inherited; or a name it gives holds a placeholder of the `redaction` policy |
| `dry-run` | `--dry-run`: a change EKR would have been asked to apply |
| `pending` | `--review`: recorded for a decision, nothing applied |
| `approved` | `cortex schema approve` approved it; `cortex schema apply` applies it |
| `rejected` | `cortex schema reject` rejected it; `decision_reason` holds the reason. It is never applied |

Of the model's answer, only each proposal's `reason` has the `redaction` policy's placeholders
put back. A masked value never becomes a type or property name, which every later round would
show the model as it is; such a proposal is kept as the model wrote it.

EKR holds a transaction's evidence to its assertions, so a schema transaction cites none; the line
in `proposals.jsonl` is where an applied change's evidence is recorded. Each transaction cortex
wrote stays beside it as `schema-<nnnn>.yaml`, numbered by the proposal's line from `0000`.

An `applied` or `proposed` line's detail carries `stamp`, `revision` (the head the sample was drawn
at), `proposed`, `applied`, `refused`, `recorded_only`, `invalid`, `dropped`, `cost_usd` (as a run
reports it) and `dir`. `sample-failed` asks no model and writes nothing. `propose-failed` (a model
call failed or found the budget or the daily ceiling spent, `refuse_if_left` refused what the model would be shown, or
EKR could not read the ontology) applies nothing and writes no `proposals.jsonl`; its reason states
the cost so far.

### Reviewing schema changes

`cortex schema <name> --review` asks the model as above and applies nothing: it does not take the
lock again, and every proposal not dropped is recorded `pending`, except one whose name holds a
placeholder of the `redaction` policy, which is `invalid` as in any round. Its `reviewed` detail
names the run's `stamp` and counts the proposals `pending`. A person then decides each one:

```sh
cortex schema approve acme 20261017T101500Z --only 1 --by ops
cortex schema reject acme 20261017T101500Z --only 2 --reason "a maker is an organisation"
cortex schema apply acme 20261017T101500Z
```

`approve` and `reject` rewrite the decision of each proposal `--only` names, by its line in
`proposals.jsonl` counted from 1 (every pending one without `--only`), with `--by` as its
`decided_by` and, for `reject`, the reason as its `decision_reason`. Every proposal named must be
`pending`: a decided proposal is not decided again, and `not-pending` decides nothing. Their detail
counts the proposals `decided` and those still `pending`. Neither asks a model or touches the
store.

`apply` plans each `approved` proposal against the ontology at the head, as a round without
`--review` does, so one whose type was added meanwhile is `invalid` and nothing is applied for it.
Each other one is asked of EKR as a schema transaction of its own: the change as recorded, byte for
byte, with no model call. Proposals still `pending` stay so. The apply is held as a gated source
run is:

- On a `postgres` store the transactions run in an EKR stage (`ekr stage begin`). The stage is
  published at the head it began from only when EKR answered every transaction and the spec's
  `gate` passed, and abandoned otherwise: `gate-failed` and `apply-failed` leave the store's head
  where it was and every approved proposal `approved`. A publish whose outcome EKR cannot tell is
  `store-outcome-unknown`; the stage stays in `stage.json` and the next run of a source settles it.
- On a `sqlite` store the store is copied first and kept as the snapshot `<ms>-schema-<stamp>` once
  a transaction commits, which the detail names (`snapshot`) and [`cortex restore`](#instances) puts
  back. A gate that fails restores it.

The gate's checks on a number of `ekr quality` are read after the changes; checks on the run
report's measures (`documents_new`, `facts_refused`, `cost_usd`, …) belong to a source run and are
not read. `applied` counts the approved proposals by what was decided (`applied`, `refused`,
`recorded_only`, `invalid`) and gives the store's head after as `revision`; a proposal applied says
its `revision` and `schema_version` on its line. A run with no approved proposal is `not-approved`.

## Sources

A source id is `<instance>/<source>`.

| command | does | outcomes |
|---|---|---|
| `cortex run <instance>/<source> [--record-failure]` | run one source once. With `--record-failure`, which the timers pass, a failed run is also counted. Its exit status tells a scheduler which outcome it was (see [Exit statuses of `cortex run`](./operating.md#exit-statuses-of-cortex-run)): 0 `ran`, 1 a failed run or `no-such-source`, 3 `disabled`, 4 `store-outcome-unknown` (EKR could not say whether a store write of the run, a stage verb or `apply-extraction`, took effect; never counted against the source), 5 `connection-unavailable` (Connectors could not revalidate the source's connection before anything was fetched, after 4 attempts; never counted against the source), 6 `batch-too-large` (a part of an extraction batch did not fit the model's window after it was split; never counted against the source), 7 `model-unavailable` (the model could not be reached: `refused`, `not-loaded` or `gateway-down`; never counted against the source), 8 `terminated` (the run was sent SIGTERM and stopped at its next safe point, a gated run undone; never counted against the source). `<instance>/seed`, for an instance with no source of that name, extracts the seed documents no seed extraction applied yet, such as those a `partial` create left. Like a source run it keeps a snapshot named `<time>-seed` before it applies and is held to the spec's `gate`; on an adopted instance it answers `fetch-failed`, because the seed belongs to the store's earlier life | `ran`, `fetch-failed`, `extraction-failed`, `apply-refused`, `store-outcome-unknown`, `connection-unavailable`, `batch-too-large`, `terminated`, `model-unavailable`, `disabled`, `no-such-source` |
| `cortex run <instance>/<source> --fetch-only` | fetch the source as a run would, mask and scrub what it fetched, and count it against the source's seen state, without a model call or a write (see [Counting what a run would extract](./operating.md#counting-what-a-run-would-extract)). Exits 0 for `fetched`, 1 otherwise; refused beside `--record-failure` | `fetched`, `fetch-failed`, `no-such-source` |
| `cortex source enable <instance>/<source>` | enable a source that two failed runs in a row disabled, and its timer | `enabled`, `wrong-state`, `no-such-source` |
| `cortex source record-failure <instance>/<source> --reason <text>` | count one failed run; the second in a row disables the source and its timer | `counted`, `disabled`, `already-disabled`, `no-such-source` |
| `cortex source add --instance-name <name> --source-id <id> --name <name> --kind <Web\|Connectors\|Files> --schedule <calendar>` | register a source in the registry; `create` and `update` do this for the spec's sources | `added` |
| `cortex sources` | sources: id, kind, state, schedule, runs, consecutive failures, and the last run's outcome, time and reason (`last_outcome`, `last_run_at`, `last_reason`; see [Operating](./operating.md#seeing-a-failure)) | |

A `ran` line's detail carries `documents_new`, `documents_applied`, `cost_usd`, `facts_refused`
(facts the model cited unissued evidence for), `parts_rejected` (parts EKR rejected), `facts_reapplied` (facts marked `replaces: true` that had no current value to replace, applied again as plain assertions and stored; not counted in `parts_rejected`), `rejected` (each document a rejected part belongs to, with EKR's refusal, or the codes of the issues validation raised, such as `invalid-supersession`, and each `structured` record refused because its name or aliases match store nodes of more than one type under `store_lookup: AnyType`; present only when there is one), `too_large` (each document left out because it alone did not fit the model's window, by key; it is marked seen and not asked again until its text changes; present only when there is one), `unconverted` (each record of a `structured` source left out because a mapped value does not convert to its property's value kind, with the record's key, the property and the kind; present only when there is one), `masked`
(credential shapes replaced) and `stopped` (why the run ended early, if it did). `cost_usd` is
`"0.0000"` when no model was asked, the sum of the answers' costs, a failed or cut-off call's included, when every answer carried one,
and `null` when any answer carried none; a missing cost is never written as `0`. When the spec
has a `redaction` policy it also carries `redacted` (values replaced by placeholders in the
prompts sent, per class or rule) and `unrestored` (placeholders in the model's answers that had no
value to put back). A run the instance's daily ceiling stopped also carries
`daily_ceiling_reached: true`, and its `stopped` names the ceiling (see
[the spec file](./spec-file.md#the-daily-ceiling)).

The detail also carries `model_calls`, the model calls the run made (a call asked again after an
error answer that cost zero counts twice, and a failed call counts), and the tokens they reported:
`input_tokens`, `cache_read_input_tokens`, `cache_creation_input_tokens`, `output_tokens`, and
`fixed_input_tokens` and `documents_input_tokens`, an estimate of how the input splits between the
fixed part of each request and its documents (see
[Operating](./operating.md#what-a-run-spends)). A run with no model call reports `model_calls: 0`
and every token count `0`; a count is `null` when any answer did not report it, never `0`. The
run's `cortex.log` line carries the same fields.

A `fetched` line's detail carries `source_id`, `window` (`since` and `until`, RFC 3339), and the
counts `fetched`, `new`, `changed`, `held` and `unchanged`, and `unread`.

## Setup

| command | does |
|---|---|
| `cortex setup [--ekr-version <v>]` | install `ekr` at the version (default 0.0.34) with `cargo install` when it is missing |
| `cortex schema` | print the spec file's JSON Schema (with an instance name it proposes schema changes instead, see [Proposing schema changes](#proposing-schema-changes)) |
