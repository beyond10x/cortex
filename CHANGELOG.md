# Changelog

Every user-visible change to cortex. Versions are the `version` in `Cargo.toml`; a release is the
tag `v<version>`. The process is in [AGENTS.md](AGENTS.md#cutting-a-release).

## Unreleased

### Added

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
