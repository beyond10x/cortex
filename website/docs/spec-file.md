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

`settings` is `kind` plus `value`, where `kind` is `web`, `connectors` or `files`.

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
| `inputs` | a list of JSON inputs; the operation is invoked once per input |
| `records` | dotted path to the array of records in the answer, for example `issues` |
| `id` | dotted path to a record's stable id |
| `time` | optional dotted path to the record's publication time |
| `text` | templates whose `{a.b}` placeholders are filled from the record; the non-empty results, joined, are the document's text |

A record without an id, or whose text is empty, is skipped. A document's key is
`<adapter>:<operation>:<id>`.

### `kind: files`

| field | meaning |
|---|---|
| `paths` | directories to read; relative paths are read against the directory the spec file was created from, and `~/` is expanded |
| `glob` | matched against each file's path relative to its directory, for example `**/*.md` |

Files that are not UTF-8 text, or are empty, are skipped. A document's key is the file's path.

### `policy`

| field | meaning |
|---|---|
| `refresh_after_days` | a document whose text changed is extracted again only when it was last applied at least this many days ago; 0 or more |
| `change` | how a change is detected; `ContentHash` is the only value |
| `max_documents_per_run` | the most documents one run extracts; above 0 |
| `max_chars_per_document` | longer text is cut to this many characters; above 0 |

## `serve`

| field | meaning |
|---|---|
| `view_port` | optional port for `ekr view` on 127.0.0.1; without it, `cortex create` takes the first free port from 18900 |
