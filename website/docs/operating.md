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
| `bin/cortex.version` | the version of that copy, recorded when it is placed |
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
   keeps, and its window, still go by the run's own times.
5. **Extract** in batches of up to 60,000 characters. Each batch is one `claude -p` call that
   answers in the JSON form of `ekr schema ekr.extraction-document/1`. The prompt carries the
   instance's description, its instructions, the store's node types with each property's value
   kind (`Release(version: String, release_date: String)`), its edge types, and up to 50 known
   entity names per type. When the spec has a `redaction` policy, every value it finds in the
   documents and the known names is shown to the model as a placeholder (`[Email-1]`), and each
   placeholder in the answer is put back before merging; the mapping exists for that one call.
   When a document says a property's value changed (a new owner, a new status), the model marks
   that fact `replaces: true`, and EKR supersedes every active value of the same entity and
   property from the new one's time on, so one value stays current and the older one is kept,
   superseded by it. EKR rejects such a fact when there is no current value to replace
   (`replacement-without-active-assertion`), and the value is then not stored; and when the
   current value is dated later than the replacement (`invalid-supersession`), as when an older
   document is read after a newer one, and the newer value then stays current. Either rejection is
   listed in the run's `rejected` and the rest of the document applies.
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

## Serving a store

`cortex list` prints each instance's viewer address, `http://127.0.0.1:<port>/`.
`cortex mcp-line <name>` prints a `claude mcp add --transport stdio cortex-<name> -- … ekr mcp`
line with the instance's host document and store, which registers the store as an MCP server.
For a `postgres` store that names a `connection`, the line, like the viewer unit, starts
`ekr mcp` through `connectors connections launch … --args '["mcp"]'`, so the server gets the
database password from Connectors (see [Spec file](./spec-file.md#store)).
