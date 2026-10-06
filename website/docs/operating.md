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
2. If the viewer (`cortex-<name>-view.service`) is running, it stops it. It refuses as `busy` if
   another process still holds the store open, such as an `ekr mcp` session, and then starts the
   viewer again.
3. It snapshots the store and `state/` as they are, as `<now>-before-restore`, which counts
   toward `keep`, and reports the name as `before_restore`.
4. It copies the snapshot over the store and puts `state/` back to the snapshot's copy, then
   starts the viewer if it was running. The snapshot stays.

Because `state/` is put back, the next runs apply again what the undone runs applied, at the
cost of their model calls. A snapshot with no copy of `state/` in `snapshots-state/` (one you
copied without it) puts back the store only, and the restore reports `state_restored: false`.

To undo a restore, restore its `before_restore` snapshot.

The check for another process holding the store reads `/proc`: it does not see a process of
another user or one `/proc` hides, and a process can open the store after the check. SQLite's
locking keeps such a reader consistent, but it reads the restored store from then on.

A `postgres` store has no snapshot; its backup is the operator's database backup, and
`cortex restore` answers `backend-unsupported`.

## Adopting an existing store

`cortex adopt --spec <file> --store <file>` makes an EKR store that cortex did not create an
instance, with its whole revision history. Nothing is seeded, no seed document is extracted and
nothing is written to the store named. The spec's sources are added, and the units installed, as
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

`adopted` reports the store's head as `revision`. `backend-mismatch`, `store-unreadable`,
`store-held` and `seed-types-missing` leave no instance directory and register nothing.

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
