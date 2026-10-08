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
ekr: {version: "0.0.32"}
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
| `documents` | directories whose files are extracted once when the instance is created, through the same pipeline as a source run (recorded as the source `seed`), unless `cortex create --no-extract` is given. A seed that stops before every document is extracted (its budget spent, a model call or an apply failed) makes `create` answer `partial`; `cortex run <name>/seed` extracts the documents it left |

The seed of an existing store cannot change: `cortex update` with a different `seed` section or
different seed files answers `seed-change-refused`.

## `model`

| field | meaning |
|---|---|
| `model` | the model name passed to `claude --model` |
| `backend` | optional: `Claude` (the default) runs `claude -p`. `Codex` is accepted by `create` and `update` but extracts nothing in this release: every run fails as `extraction-failed`, naming `story:codex-model-backend` (see [Commands](./commands.md#global-options)) |
| `budget_usd` | the spending limit of one run, in US dollars, as a decimal string; above 0. A run stops starting new batches once it is spent |
| `timeout_s` | the time limit of one model call, in seconds; above 0 |
| `instructions` | optional text file whose content is added to every extraction prompt |

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
| `input` | `from: connectors` with its `value`: `adapter`, `connection`, `operation`, `inputs` and an optional `paging`, as for [`kind: connectors`](#kind-connectors), window included, and optional `children` (below). Or `from: files` with its `value`: `paths` and `glob`, as for [`kind: files`](#kind-files); each matching file is one JSON document (a leading byte-order mark is ignored) |
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
| `properties` | `property` and `path` pairs: the value at the path is asserted for the property |
| `relations` | `relation`, `target_type` and `target_name`: a relation from the record's entity to the entity of `target_type` named by the value at the `target_name` path, one per value when it holds a list |

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
3. the mapped `aliases`, as the spec lists them.

The bare name and the bare id are not aliases. Two records of one name, or one record's id equal
to another's alias, are two nodes, and a record whose name changes stays the same node. A mapped
alias resolves on purpose: records, and nodes from other sources, that share one are one node. To
merge a record with nodes known by its plain name, map the name as an alias too
(`aliases: ["$.name"]`). EKR adds no alias to a node an extraction matches, so a record whose
mapped alias changes (a renamed path) keeps its node and its old alias, and the new value is not
added as an alias; the property holding it is set as any changed value is.

**Values.** A property's value is stored as text (a number or a boolean as written in JSON); a
property whose value is absent, empty, null, a list or an object is not set. A changed value adds
an assertion of the new value. With `dropped: Keep` the old one stays active beside it, so a
property can hold every value a record has had.

**Dropped values.** With `dropped: Supersede`, after each run cortex reads the store
(`ekr snapshot`) for the active assertions this source made: those whose evidence is all
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
parent changed. A child call that fails for one parent (Connectors answers `forbidden`, say, or the
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
| `refresh_after_days` | a document whose text changed is extracted again only when it was last applied at least this many days ago; 0 or more. A record read from a file ([`kind: files`](#kind-files)) and a `structured` source's change whose text changed only in its links ([`kind: structured`](#kind-structured)) wait for nothing |
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
undone from the snapshot taken before it and fails as `apply-refused`. See
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
| `min`, `max` | decimals, written as strings; the check fails when the measure is below `min` or above `max`. Both are compared at 4 decimal places, the precision `cortex run` prints a cost at, so a cost printed as `0.3000` is within `max: "0.3"`. A measure that cannot be read (a misspelt name, a `cost_usd` no answer carried, an `ekr quality` that fails) fails its check |

## `serve`

| field | meaning |
|---|---|
| `view_port` | optional port for `ekr view` on 127.0.0.1; without it, `cortex create` takes the first free port from 18900 |

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
    schema_connection: {adapter: postgres, connection: <connection id>}
```

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
a spec whose connection Connectors does not list (`connection-missing`). A `config` without that
`password_file`, or with a `password` key, is refused by `cortex create` (`seed-refused`),
`cortex update` (`seed-change-refused`) and `cortex adopt` (exit 2), and with `schema_connection`
`cortex create` refuses such a `--postgres-schema-config` file (`seed-refused`) before any `ekr`
runs. Each refusal names the fields, never their values. cortex does not read the connection file
a `config` names; an `ekr` refuses one that carries a password while `password_file` is set.
