# AGENTS.md — cortex

Instructions for agents changing this repository. Humans start at [README.md](README.md); the
user documentation is the site under `website/`, published at <https://beyond10x.github.io/cortex/>.

## What this is

`cortex` spins up EKR knowledge brains from an instance spec. One instance is one EKR SQLite store
plus the data sources that feed them on a schedule. The engine is EKR (`ekr` binary); integrations
and every secret belong to Connectors (`connectors` binary); extraction is a tool-less
`claude -p` call whose output is checked against EKR's own schema.

| path | holds |
|---|---|
| `spec/` | the ESS specification, the source of truth |
| `generated/` | everything `ess` generates from `spec/`; never edited by hand |
| `src/` | the `cortex-cli` crate: the `cortex` binary and its library |
| `tests/` | `conformance.rs` (the suite of `spec/suite.json`) and `e2e.rs` (the binary end to end) |
| `crates/cortex-docs/` | generates and checks the derived files of the documentation site |
| `website/` | the Docusaurus documentation site |
| `examples/` | a complete spec file and its seed schema |
| `.engineering/` | the AEP planning store |

## Commands

| command | does |
|---|---|
| `task check` | the gate: spec validity, generated-code drift, fmt, clippy (`-D warnings`), every test, documentation drift |
| `task generate` | regenerate `generated/`, `spec/suite.json` and the derived site files from `spec/` |
| `task drift` | fail when `generated/` or `spec/suite.json` differ from what `spec/` generates |
| `task docs-generate` | regenerate the derived site files (`cortex-docs generate`) |
| `task docs-check` | fail when a derived site file is missing, stale or edited by hand |
| `task website` | build the site (`npm ci`, `npm run build` in `website/`), as CI does |
| `task build`, `task test` | release build; every test of the workspace |
| `aep plan artifact validate` | validate the planning store |

## The specification comes first

`spec/` is an ESS specification (`ess/20`, pinned to ESS 0.52.0 in `spec/ess-inputs.yaml`). It is
the source of truth for every noun, command, outcome, error and event.

- Change `spec/`, then run `task generate`. Never edit anything under `generated/` or
  `spec/suite.json` by hand; `task drift` fails when they differ from what `spec/` generates.
- `generated/cortex-model` (`ess generate synthesize --target rust --layout crate`) holds the
  model types and the command behaviours. Two behaviours are obligations the code implements:
  `RunSource` and `RecordFailure` (`generated/cortex-model/PLAN.md`).
- `generated/spec-types` (`ess generate types`) holds the serde types the spec file is read with.
- `src/model_map.rs` maps the serde types onto the model types. The compiler checks it field by
  field; keep it a mapping and nothing else.
- The command line is clap derive (`src/main.rs`). `ess generate synthesize --target clap` is not
  used: it emits the builder API (`clap::ArgMatches`).
- Every `UNMAPPED:` marker in `spec/domains/instance.yaml` is a rule the specification cannot
  state; the Rust tests hold it instead. Do not delete a marker without the construct that
  replaces it.

## Documentation site

The site follows the independent project-site pattern of the sibling repositories canon and loom:
Docusaurus with `@beyond10x/docs-system` (pinned by commit in `website/package.json`), built in
this repository and published by the organisation's reusable workflow.

- **Generated, never hand-edited:** `website/docs/reference/ess/` (from `ess generate --kind docs`),
  `website/data/ess/` (domain graphs from `ess specify compile`) and `website/static/schemas/`
  (the spec file's JSON Schema from `ess generate --kind schema`). `task docs-check`, part of
  `task check`, fails when they drift.
- **Hand-written:** every other page under `website/docs/`, the landing page `website/product.json`
  and the status record `website/data/status.json`. Every claim on them must be checkable against
  the code, `spec/`, `examples/` or the planning store. When a change alters behaviour a page
  describes, update the page in the same change. `shipped` in `status.json` means code on `main`;
  `planned` means a story in `.engineering/`.
- **Admonitions** carry their title in brackets (`:::note[Title]`); `cortex-docs` refuses a raw
  title.
- **Publication:** `.github/workflows/pages.yml` ("Documentation validation") builds the site on
  every pull request and push, and on a push to `main` binds it to its commit
  (`cortex-docs provenance` writes `.well-known/b10x-site.json`) and uploads it.
  `.github/workflows/b10x-docs-site.yml` hands that artifact to
  `beyond10x/website/.github/workflows/project-site.yml`, which deploys it to GitHub Pages at
  `/cortex/`; it runs only for bot-authored commits on `main`. Actions and the reusable workflow
  are pinned by commit SHA, at the same pins as canon and loom.
- `website/build` and `website/node_modules` are build output; they are ignored and never committed.

## Hard rules

- **No secret enters cortex.** A source names a Connectors connection id; cortex only invokes
  operations through `connectors`. Never read, print, store or pass a credential, and never add a
  spec field that carries one.
- **No instance data in Git.** Instances live under `$CORTEX_HOME` (default
  `~/.local/share/cortex`). Tests build their own homes in temp directories.
- **The repository is public.** No secrets, no machine-specific absolute paths, nothing private in
  code, docs, examples or the planning store.
- **Every commit and every GitHub write goes through the bot.** Commit with
  `b10x-gates bot --repository beyond10x/cortex -- commit`, push with `b10x-gates bot ... -- push`,
  and open or merge pull requests with `b10x-gates api`. The shared `Security and privacy` gate
  refuses a commit after the enrolled baseline that is not authored by `b10x-bot[bot]`
  ("inadmissible authorship"), and a `/home/<name>/` path in any commit, which admits no exception.
  Run `b10x-gates scan-text` on every staged file first and check its own exit status.
- **Committed code is Rust.** Probes in other languages stay outside the repository. The one
  exception is `website/`, whose Docusaurus configuration is TypeScript as in canon and loom.
- **Fetched text is untrusted.** It is masked for credential shapes (`src/mask.rs`), stored as
  evidence and sent to a model that has no tools. There is **no PII redaction**: do not point an
  instance at personal data.
- **The model never issues evidence.** cortex mints every evidence id and payload; a fact citing
  an id cortex did not issue for that batch is refused.
- **The model call is isolated.** `claude -p` runs with `ANTHROPIC_API_KEY` removed from its
  environment (the key otherwise overrides the OAuth sign-in), `--setting-sources ""`,
  `--strict-mcp-config`, `--disable-slash-commands` and `--tools ""`, in an empty directory.
  Measured on 2026-10-05: 423 input tokens with these flags against 10,837 with the user's
  setting sources, which carry the user's global instructions. Keep the flags.

## Upstream facts the code works around

| fact | where | effect here |
|---|---|---|
| `ekr apply-extraction` admits only `!HumanStatement` evidence (`extraction-evidence-kind-unsupported`) | EKR 0.0.30 `crates/ekr-integrate/src/extraction.rs:61`, `crates/ekr-kernel/src/validate/provenance.rs:129` | a web page is evidence `!HumanStatement {identity: <url>}` instead of `!Url` |
| EKR's ESS spec models `ExtractedFact` as a `kind`-tagged union, its YAML reader wants `!Property` tags, and `ekr schema` writes `"!Property"` keys | EKR 0.0.30 `systems/ekr/domains/integrate.yaml:203`, `ekr schema ekr.extraction-document/1` | the model answers in `ekr schema`'s form; `src/extract.rs` turns `"!X"` keys into YAML tags |
| Claude's `--json-schema` refuses a schema that names draft 2020-12 in `$schema` | `claude` 2.1.289 | the `$schema` key is removed before the call |
| `ess generate --kind docs` titles the domain page with the domain's display name (`Instances`), which equals a view's name, and Docusaurus then moves that view's anchor | ESS 0.52.0, `spec/domains/instance.yaml` `naming.display` | `cortex-docs` drops the page's level-one heading; the front-matter title heads the page |

## Build and verify

- `task check`: spec validity, generated-code drift, fmt, clippy (`-D warnings`), all tests,
  documentation drift.
- Format with `cargo fmt -p cortex-cli -p cortex-docs`. `cargo fmt --all` also rewrites the
  generated crates, which are path dependencies, and `task drift` then fails.
- `tests/conformance.rs` runs every scenario of `spec/suite.json` (35) in process; `tests/e2e.rs`
  drives the binary end to end; the clap ⇔ spec test in `src/main.rs` holds the command line to
  the commands, inputs and views `spec/` puts on it.
- Builds use `CARGO_TARGET_DIR=$HOME/.cache/b10x-target/cortex`. Check `df -h /` first; do not
  start a build with less than 10 GB free.
- The end-to-end tests run a real `ekr` (`CORTEX_TEST_EKR`, default the 0.0.30 binary under
  `~/.cache/company-brain-v3/bin/0.0.30/bin/ekr`) with stand-in `connectors`, `claude` and
  `systemctl`. A missing `ekr` fails them; it never skips them.
- After a site change, `task website` must build; it fails on a broken link or anchor.
