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
ekr: {version: "0.0.30"}
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
| `model` | the model that extracts, its per-run budget and its timeout |
| `sources` | the data sources and their schedules |
| `serve` | the viewer's port |

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
| `documents` | directories whose files are extracted once when the instance is created, through the same pipeline as a source run (recorded as the source `seed`), unless `cortex create --no-extract` is given |

The seed of an existing store cannot change: `cortex update` with a different `seed` section or
different seed files answers `seed-change-refused`.

## `model`

| field | meaning |
|---|---|
| `model` | the model name passed to `claude --model` |
| `budget_usd` | the spending limit of one run, in US dollars, as a decimal string; above 0. A run stops starting new batches once it is spent |
| `timeout_s` | the time limit of one model call, in seconds; above 0 |
| `instructions` | optional text file whose content is added to every extraction prompt |

## `sources`

Each source has a `name` (same rules as the instance name, unique within the instance), a
`schedule`, `settings` and a `policy`. Its id is `<instance>/<source>`.

`schedule` is a systemd calendar expression; it is written into the timer as `OnCalendar=`, for
example `"*-*-* 06:00:00"` for every day at 06:00.

`settings` is `kind` plus `value`, where `kind` is `web`, `connectors`, `files` or `structured`.

### `kind: web`

Websites with their title, description and content, through a Connectors adapter that implements
`datasource.websearch/v1alpha1`.

| field | meaning |
|---|---|
| `adapter` | optional; `tavily` when absent |
| `connection` | the Connectors connection id |
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
| `adapter`, `connection`, `operation` | what to invoke |
| `inputs` | a list of JSON inputs; the operation is invoked once per input. `{since}` and `{until}` in a string value are replaced by the run's window |
| `records` | dotted path to the array of records in the answer, for example `issues` |
| `id` | dotted path to a record's stable id |
| `time` | optional dotted path to the record's publication time |
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

With `refresh_after_days: 0` the first run's window is empty (`{since}` equals `{until}`), so
records older than the 5 minutes before that run are never read. Set `refresh_after_days` to 1 or
more to read that much history on the first run.

A run is successful when it applied every new or changed document it fetched and read every
record. A run that is not successful (it stopped early, failed, left documents beyond
`max_documents_per_run`, or left records unread) keeps its `{since}` as the lower bound of every
later run until one succeeds, so the next run asks for them again. That holds before the first
successful run too: the window does not slide with the clock. Records are left unread by a walk
that reaches `max_pages` with pages left, by an empty page that still names a next one, and by a
child call that fails for a record, which leaves that record out of the run. Each of these says so
in `stopped`, in the run's result and in `cortex.log`; the other records still apply.

A child call that fails for the same record on 3 runs in a row stops holding the window: from the
third failure on, the record is left out and named in `skipped` (`<operation>: child call failed
for <key> on 3 runs; skipped`), in the run's result and in `cortex.log`, but not in `stopped`.
The call is still tried on every run, and one success resets the count.

A changed document whose last application is younger than `refresh_after_days` is held back, not
lost. Every run works out, from the documents it holds back, the earliest window start that covers
each held change: the earliest last application among them, less the 5 minutes. No later run's
`{since}` is after that; a run that holds nothing back clears it.

**`paging`** has the fields `style` (`PageNumber`, `Token` or `Keyset`), `param` (the dotted path
of the input field to advance), `next` (optional, the dotted path in the answer of the next page,
token or key) and `max_pages`. With `next`, its value is the next `param`, whatever the style.
Without it, a `PageNumber` walk asks for the page after the current one (the input's `param`, or
1 when absent), and a `Token` or `Keyset` walk reads one page. A walk stops at an empty page, at a
missing `next`, at a page or token it has already asked for, or after `max_pages` pages.

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
| `time` | optional dotted path to the record's publication time |
| `author` | optional dotted path to the record's author |
| `text` | templates whose `{a.b}` placeholders are filled from the record; the non-empty results, joined, are the record's text |
| `thread` | optional dotted path to the field that groups records into threads |
| `filters` | a list, possibly empty, of `field` (a dotted path), `values` and `include`. With `include: true` only records whose field equals one of the values are read; with `include: false` those records are left out. A record is read only when it passes every filter |

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

A record without an id, whose text is empty, or that a filter leaves out, is skipped. A document's
key is `<file path>#<id>`, and its evidence is cited as `file:<file path>#<id>`; a key read twice
in one run counts once. With `author`, the text reads `<author>: <text>`. Records are read in file
order, and files by name, one entry of `paths` after the other.

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
| `input` | `from: connectors` with its `value`: `adapter`, `connection`, `operation`, `inputs` and an optional `paging`, as for [`kind: connectors`](#kind-connectors), window included. `from: files` is accepted but does not run yet: its run fails with `fetch-failed` |
| `records` | path to the array of records in the answer, for example `$.people` |
| `mapping` | how a record becomes an entity (below) |
| `dropped` | accepted; `Keep` and `Supersede` do not act yet, and nothing a source made earlier is superseded |

A path is dotted and may start at the root: `$.contact.email` and `contact.email` are the same,
and `$` alone is the answer itself.

| `mapping` field | meaning |
|---|---|
| `node_type` | the node type of every record's entity |
| `id` | path to the record's stable id |
| `name` | path to the entity's name |
| `aliases` | paths to further names; each holds a value or a list of values. A mapped alias resolves across records and sources (below) |
| `properties` | `property` and `path` pairs: the value at the path is asserted for the property |
| `relations` | `relation`, `target_type` and `target_name`: a relation from the record's entity to the entity of `target_type` named by the value at the `target_name` path, one per value when it holds a list |

A record without an id or a name is skipped, and a record whose id was already read in the run
counts once.

**Identity.** A record's identity is `<adapter>:<operation>:<id>`. When masking or a `redaction`
rule with a `replacement` would change the id, the identity carries the first 16 hex digits of the
id's SHA-256 in its place, so the id is never stored and two such ids stay two records. The
identity is the record's document key, and its evidence reads `Source: record:<identity>`. EKR treats things of one node
type that share any alias as one thing, so the entity's aliases are, in order:

1. `<name> (<identity>)`, which names the node, for example `Ada Lovelace (directory:people.list:P-1)`;
2. the identity;
3. the mapped `aliases`, as the spec lists them.

The bare name and the bare id are not aliases. Two records of one name, or one record's id equal
to another's alias, are two nodes, and a record whose name changes stays the same node. A mapped
alias resolves on purpose: records, and nodes from other sources, that share one are one node. To
merge a record with nodes known by its plain name, map the name as an alias too
(`aliases: ["$.name"]`).

**Values.** A property's value is stored as text (a number or a boolean as written in JSON); a
property whose value is absent, empty, null, a list or an object is not set. A changed value adds
an assertion of the new value, and the old one stays active beside it: EKR 0.0.30 does not
supersede an assertion, and cortex will once EKR does. Until then a property can hold every value
a record has had.

**Relations.** A relation target names a record of the same run by that record's identity (the
target taken as an id) or, when exactly one record of the run has it, by its name or a mapped
alias; the relation then names the target by all that record's aliases. Any other target of the
mapping's own node type is named by the target text and its identity as an id, so it reaches a
record of an earlier run by its id, or a node by a mapped alias; a target of another node type is
named by the target text. The mapping's node type, its properties, the target types and the
relations are added to the store's ontology where it does not hold them yet.

**Evidence and rejections.** Each record is stored as the evidence every fact from it cites: a
`Source:` header and the record's JSON. Before that, every string in the record has credential
shapes masked and the `redaction` policy's rules with a `replacement` applied; no model sees a
record, so nothing is replaced by a placeholder. `max_chars_per_document` does not cut a record,
and EKR rejects a text value over 65,536 bytes (EKR 0.0.30's string limit). A record with any part
EKR rejects is counted in `parts_rejected`, is not counted in `documents_applied` and is not marked
seen; the run is then not successful, so later runs read the record and try it again until it
applies. Shorten or unmap the value at the source to clear it. Change detection,
`refresh_after_days` and `max_documents_per_run` work as for any source.

### `policy`

| field | meaning |
|---|---|
| `refresh_after_days` | a document whose text changed is extracted again only when it was last applied at least this many days ago; 0 or more |
| `change` | how a change is detected; `ContentHash` is the only value |
| `max_documents_per_run` | the most documents one run extracts; above 0 |
| `max_chars_per_document` | longer text is cut to this many characters; above 0. A change is detected on the whole text, before the cut, and the run counts the cut documents it shows the model as `truncated` |

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

## `serve`

| field | meaning |
|---|---|
| `view_port` | optional port for `ekr view` on 127.0.0.1; without it, `cortex create` takes the first free port from 18900 |
