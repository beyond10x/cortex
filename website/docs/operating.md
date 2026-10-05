---
title: Operating an instance
sidebar_position: 5
description: Where instances live, what a run does, the systemd units, failures and the model call.
---

# Operating an instance

## Where things live

Everything lives under the home, `$CORTEX_HOME` or `~/.local/share/cortex`:

| path | holds |
|---|---|
| `registry.json` | every instance and source with its state and counters |
| `cortex.lock` | the lock every writing command holds, so two commands on one home never interleave |
| `bin/cortex` | the copy of cortex the systemd units run |
| `instances/<name>/instance.yaml` | the frozen spec file, beside copies of the seed files and instructions it names |
| `instances/<name>/store.sqlite` | the instance's EKR store, unless its spec names `store.backend: postgres`; then the store is in PostgreSQL, opened through the spec's `ekr.postgres/1` file |
| `instances/<name>/host.json` | the EKR host document the store is opened with |
| `instances/<name>/meta.json` | the viewer's port and the directory the spec was created from |
| `instances/<name>/state/<source>.json` | what the source has already applied: each document's key, text hash and time |
| `instances/<name>/state/entities.json` | entity names from earlier runs, offered to the model so it reuses them |
| `instances/<name>/runs/<time>-<source>/batch-<n>/` | per model call: the prompt, the model's answer, the extraction document applied and EKR's report |
| `instances/<name>/cortex.log` | one JSON line per run: counts, cost, seconds, why it stopped |

## systemd units

`cortex create` installs systemd user units, in `$XDG_CONFIG_HOME/systemd/user` or
`~/.config/systemd/user`:

- `cortex-<instance>-<source>.timer` and `.service` per source. The timer has
  `OnCalendar=<schedule>` and `Persistent=true`; the service runs
  `<home>/bin/cortex --home <home> run <instance>/<source> --record-failure`.
- `cortex-<instance>-view.service`, which runs `ekr view` on the store at the instance's port.

The source services carry `PATH`, the `XDG_*` directories and every `CONNECTORS_*` variable of the
shell that ran `cortex create`, so `connectors` and `claude` find their configuration.

The units run the copy at `<home>/bin/cortex`. Rebuilding cortex changes nothing for existing
timers until a `create` or `update` installs the units again and copies the new binary.

## What one run does

1. **Fetch** the source's documents (web pages or records through `connectors`, or local files).
2. **Mask** credential-shaped text in each document, then cut it to `max_chars_per_document`.
3. **Keep** documents whose key was never applied, and documents whose text hash changed once
   their last application is `refresh_after_days` old, at most `max_documents_per_run`.
4. **Issue evidence.** cortex mints one evidence item per document; the model never does.
5. **Extract** in batches of up to 60,000 characters. Each batch is one `claude -p` call that
   answers in the JSON form of `ekr schema ekr.extraction-document/1`. The prompt carries the
   instance's description, its instructions, the store's node and edge types, and up to 50 known
   entity names per type. When the spec has a `redaction` policy, every value it finds in the
   documents and the known names is shown to the model as a placeholder (`[Email-1]`), and each
   placeholder in the answer is put back before merging; the mapping exists for that one call.
6. **Merge.** A fact citing an evidence id cortex did not issue for that batch is refused; cortex
   adds the evidence items itself and records each web page as a `WebPage` node.
7. **Apply** the document with `ekr apply-extraction`, then record the batch's documents as seen.

A `structured` source skips steps 5 and 6: each batch of records is mapped to an extraction
document by the source's `mapping` and applied without a model call, at a cost of 0 (see
[the spec file](./spec-file.md#kind-structured)). It masks every string in a record rather than
the text as a whole, and never cuts a record.

The run stops starting batches when its `budget_usd` is spent. When a batch fails after an earlier
one was applied, the run ends as `ran` with `stopped` set; when the first batch fails, nothing was
applied and the run fails.

## Failures

A run that fails (`fetch-failed`, `extraction-failed`, `apply-refused`) applies nothing and leaves
the source's seen state where it was. The timers pass `--record-failure`, so a failure is counted;
the second failure in a row disables the source and stops its timer. A successful run resets the
count. `cortex source enable <instance>/<source>` switches it back on.

If Connectors refuses a read because a connection's validation lapsed, cortex revalidates the
connection once and retries.

## The model call

Extraction runs `claude -p` in an empty directory under the run's directory, isolated from the
user's configuration:

- `ANTHROPIC_API_KEY` is removed from its environment, so the signed-in account is used;
- `--tools ""`, `--setting-sources ""`, `--strict-mcp-config`, `--disable-slash-commands` and
  `--no-session-persistence`: no tools, no settings, no MCP servers, no slash commands;
- `--json-schema` with EKR's extraction schema, and `--max-budget-usd` with what is left of the
  run's budget;
- the call is stopped after `timeout_s` seconds.

The model sees fetched text and nothing else. Fetched text is untrusted: it is masked for
credential shapes, stored as evidence and sent to a model that cannot act on it, with the
personal data a `redaction` policy names replaced by placeholders (see [Limits](./limits.md)).

## Serving a store

`cortex list` prints each instance's viewer address, `http://127.0.0.1:<port>/`.
`cortex mcp-line <name>` prints a `claude mcp add --transport stdio cortex-<name> -- … ekr mcp`
line with the instance's host document and store, which registers the store as an MCP server.
