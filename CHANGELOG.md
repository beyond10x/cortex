# Changelog

Every user-visible change to cortex. Versions are the `version` in `Cargo.toml`; a release is the
tag `v<version>`. The process is in [AGENTS.md](AGENTS.md#cutting-a-release).

## Unreleased

### Added

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

### Fixed

- A second home no longer takes over or deletes another home's systemd units for an instance of
  the same name. Each unit records its home as `X-CortexHome=`; `create`, `update` and `adopt`
  refuse a unit another home wrote, naming that home, and `remove`, `restore`, `source enable` and
  `--record-failure` leave it alone. Units written by an earlier cortex are read as belonging to
  the home their service runs.
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
