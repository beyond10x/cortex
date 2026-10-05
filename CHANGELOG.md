# Changelog

Every user-visible change to cortex. Versions are the `version` in `Cargo.toml`; a release is the
tag `v<version>`. The process is in [AGENTS.md](AGENTS.md#cutting-a-release).

## Unreleased

### Added

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
