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
| `instances/<name>/snapshots/<time>-<source>.sqlite` | copies of a `sqlite` store, each taken before a run applied anything; the newest `snapshots.keep` (default 3) stay |
| `instances/<name>/snapshots-state/<time>-<source>.json` | each snapshot's copy of `state/` |
| `instances/<name>/cortex.log` | one JSON line per run: counts, cost, seconds, why it stopped |

## systemd units

`cortex create` installs systemd user units, in `$XDG_CONFIG_HOME/systemd/user` or
`~/.config/systemd/user`:

- `cortex-<instance>-<source>.timer` and `.service` per source. The timer has
  `OnCalendar=<schedule>` and `Persistent=true`; the service runs
  `<home>/bin/cortex --home <home> run <instance>/<source> --record-failure`.
- `cortex-<instance>-view.service`, which runs `ekr view` on the store at the instance's port.

A unit runs with the systemd user manager's environment, not your shell's: its `PATH` holds no
user tool directory such as `~/.local/bin` or `~/.cargo/bin`, and its working directory is your
home directory. So the source services carry, from the shell that ran `cortex create` or
`cortex update`:

- `PATH`, `HOME`, the `XDG_*` directories and every `CONNECTORS_*` variable, so the tools are
  found on the same `PATH`, and `claude`'s sign-in and the default `ekr` under
  `~/.cache/cortex` under the same `HOME`;
- `CORTEX_CONNECTORS`, `CORTEX_CLAUDE` and `CORTEX_CODEX`: the binaries that command ran, from
  `--connectors`, `--claude` and `--codex` or their defaults. A path is made absolute; a bare name
  is looked up on the unit's `PATH`.

A relative `--home` or `$CORTEX_HOME` is made absolute, so the units name the same home from any
directory. Run `cortex create` from a shell where `connectors` and `claude` work; after changing
either, `cortex update` writes the units again.

The timers run only while your systemd user manager runs. To have them run while you are logged
out, enable lingering once: `loginctl enable-linger`.

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
   Before the run's first apply, a `sqlite` store and `state/` are copied, and kept as a
   snapshot once that apply commits (see [Undoing a run](#undoing-a-run)); a copy that cannot be
   taken fails the run with nothing applied.
8. **Check** the run against the spec's [`gate`](./spec-file.md#gate), when it applied
   something. A failed check undoes the run (see [Failures](#failures)).

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
store was not), and `gate.reason`. A run with no snapshot (a `postgres` store,
`snapshots.keep: 0`) cannot be undone: it still fails, and its `reason` says it was not undone.

If Connectors refuses a read because a connection's validation lapsed, cortex revalidates the
connection once and retries.

## Undoing a run

Before a run's first `apply-extraction` on a `sqlite` store, cortex copies the store, through
SQLite's online backup, and every file of `state/` (each source's seen state and the known entity
names) to hidden temporary files. Once that apply commits, the copies become the snapshot
`<time>-<source>`, where `<time>` is the run's start in Unix milliseconds, the same time that
names the run's directory under `runs/` and its `at` in `cortex.log`:
`instances/<name>/snapshots/<time>-<source>.sqlite` and
`instances/<name>/snapshots-state/<time>-<source>.json`. A run that applies nothing, or whose
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

To undo a restore, restore its `before_restore` snapshot.

A reader attached during a restore keeps a consistent view: a read it has started finishes on
the store as it was, and its next read sees the restored store. After the copy, cortex waits up
to 5 seconds for such reads to finish and writes SQLite's write-ahead log back into the store
file, because until then EKR 0.0.30 refuses to open the store (`store-replaced`). A connection that
keeps one read transaction open for longer delays that: the restore still succeeds, cortex says
so on stderr, and EKR opens the store again once that reader lets go and the next connection to
close the store writes the log back.

A `postgres` store has no snapshot; its backup is the operator's database backup, and
`cortex restore` answers `backend-unsupported`.

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
