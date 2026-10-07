# AGENTS.md — cortex

Instructions for agents changing this repository. Humans start at [README.md](README.md); the
user documentation is the site under `website/`, published at <https://beyond10x.github.io/cortex/>.

## What this is

`cortex` spins up EKR knowledge brains from an instance spec. One instance is one EKR store, SQLite
or PostgreSQL as its spec names, plus the data sources that feed it on a schedule. The engine is EKR (`ekr` binary); integrations
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
  evidence and sent to a model that has no tools. Masking covers a document's text, title,
  description and key (its URL, file path or record key), and the key wherever cortex writes it:
  the run report's `stopped` and `skipped` messages, `cortex.log` and `state/<source>.json`.
  It is irreversible: a credential in a listed shape is never stored, never sent and never
  restored. In a key, a masked value ends at `&` or `#` (raw or percent-encoded), so the rest of
  the URL is kept; the masked key is the evidence identity, and documents whose keys differ only in
  a credential are one document. Masking is by shape, so a credential in no listed shape (a
  token of another form with no name before it, or an assigned value shorter than six
  characters) is not masked.
- **Personal data stays out of the prompt, not out of the store.** The instance's `redaction`
  policy (`src/redact.rs`) replaces each value it finds with a per-batch placeholder (`[Email-1]`,
  `[Phone-2]`, `[Card-1]`, `[IpAddress-1]`, `[Url-1]`, `[Name-1]`, `[<rule>-1]`) in everything
  the model is shown: each document's text, title, description and `Source:` key, and the known
  entity names. Every
  placeholder in the answer is restored before it is applied, so the store, the evidence and the
  extracted facts hold the originals and stay searchable. The mapping lives in memory for one batch
  and is never written to disk or to the log; with a policy, a batch directory under `runs/` keeps
  only the restored `extraction.yaml`. A placeholder with no value is left as written and counted
  as `unrestored`. Limits: detection is by pattern (the classes `Url`, `Email`, `Phone`,
  `IpAddress`, `PaymentCard` and `RareName`, the `known_names` list, and the policy's regex
  `rules`), so four-part version numbers and long digit ids that look like an IP address or a card
  are hidden from the model too (and restored); a person's name is found only when listed in
  `known_names` or rare in the batch (`rare_limit`); the instance's own description and
  instructions are sent as written; and a spec without a `redaction` policy sends text unchanged.
  Rarity counts only the batch's document texts outside what another class hides (for a record
  read from a file, its own text only, not its thread context, key or title); known entity
  names do not count. Known names match in any case and either Unicode normal form.
  `Credential` is irreversible (`[masked:<shape>]`, also in the stored key). `refuse_if_left`:
  every batch is checked as the model would be shown it before the first model call, and a class
  still detected then fails the run with nothing sent or stored; each batch is checked again just
  before it is sent (the known entity names grow), and a class found then stops the run before
  that batch, with the batches before it applied.
- **A rule with a `replacement` is irreversible, like a credential.** A `rules` entry with a
  non-empty `replacement` replaces its matches by that text in a document's text, title and
  description before the document is stored (`Redactor::scrub`), and in its URL before the model
  is shown it; nothing is ever restored to them. The URL is stored as it is, as for credentials.
  A rule with `replacement: ""` is a reversible `[<rule>-N]` placeholder.
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
| `ekr apply-extraction` takes an evidence payload of at most 16,384 bytes: it reads the payload as a YAML sequence of one element per byte and rejects every fact citing a longer one (`transaction document limit: sequence_elements (at most 16384)`) | EKR 0.0.30 `ekr` help on `payload`; measured on 2026-10-06 (`tests/evidence_bound.rs`: 16,384 bytes apply) | `src/evidence.rs` cuts each payload at a character boundary to `PAYLOAD_MAX_BYTES`, and a run cuts the text it shows the model to what the payload holds. The cut keeps whole characters, not grapheme clusters (a letter can lose a following combining mark); the model text is cut at the same byte, so a citation still matches its evidence. A structured record's evidence leads with its mapped values; a record whose mapped values alone exceed the bound is skipped |
| Claude's `--json-schema` refuses a schema that names draft 2020-12 in `$schema` | `claude` 2.1.289 | the `$schema` key is removed before the call |
| A schema transaction cites no evidence: EKR holds `transaction.evidence` to the evidence its assertions cite, and a schema change travels alone, so a non-empty set is `evidence-set-mismatch` | EKR 0.0.31 `docs/cli.md` ("Evolve the schema"); measured on 2026-10-06 | `cortex schema` records each change's facts and evidence in `schema/<stamp>/proposals.jsonl` beside its transaction |
| `ess generate --kind docs` titles the domain page with the domain's display name (`Instances`), which equals a view's name, and Docusaurus then moves that view's anchor | ESS 0.52.0, `spec/domains/instance.yaml` `naming.display` | `cortex-docs` drops the page's level-one heading; the front-matter title heads the page |

## Build and verify

- `task check`: spec validity, generated-code drift, fmt, clippy (`-D warnings`), all tests,
  documentation drift.
- Format with `cargo fmt -p cortex-cli -p cortex-docs`. `cargo fmt --all` also rewrites the
  generated crates, which are path dependencies, and `task drift` then fails.
- `tests/conformance.rs` runs every scenario of `spec/suite.json` (55) in process; `tests/e2e.rs`
  drives the binary end to end; the clap ⇔ spec test in `src/main.rs` holds the command line to
  the commands, inputs and views `spec/` puts on it.
- Build into this tree's own `target/` (leave `CARGO_TARGET_DIR` unset), and end each tree with
  `worktree finish --discard-cache --archive <tree>`, so its build cache goes with it. Check
  `df -h /` first; do not start a build with less than 10 GB free. Before a test run counts as
  evidence for a tree, check that the test names it printed exist there (`cargo test -- --list`).
- The end-to-end tests run a real `ekr` (`CORTEX_TEST_EKR`, default the 0.0.32 binary under
  `~/.cache/company-brain-v3/bin/0.0.32/bin/ekr`) with stand-in `connectors`, `claude` and
  `systemctl`. A missing `ekr` fails them; it never skips them.
- After a site change, `task website` must build; it fails on a broken link or anchor.

## Cutting a release

A release is the tag `v<version>`, where `<version>` is `version` in `Cargo.toml`, and a GitHub
Release on that tag. `.github/workflows/release.yml` runs when the Release is published: it reads
the version from `Cargo.toml` and fails before uploading anything when the tag is not
`v<version>`, runs `cargo build --release --locked`, checks that the binary prints
`cortex <version>`, and attaches `cortex-<version>-x86_64-unknown-linux-gnu.tar.gz` (the `cortex`
binary, `LICENSE`, `README.md`) and `SHA256SUMS` to the Release. Only its upload job holds
`contents: write`. A tag push alone starts no release build. A manual run (`workflow_dispatch`)
builds the same two files as a workflow artifact and attaches nothing. Repackaging the same binary
gives the same tarball: entries sorted, dated at the commit time, owned by 0/0, setuid and setgid
bits cleared, and gzip without a name or time. An independent rebuild does not: the binary embeds
the build host's Cargo registry paths.

- The upload job refuses a prerelease before uploading anything; publish a full release.
- The upload job refuses a tag whose commit is not on `origin/main` before uploading anything; tag
  only a commit merged to `main`.

1. On a `release/<version>` branch from current `origin/main`: set `version` in `Cargo.toml` and
   let a build update the `cortex-cli` entry in `Cargo.lock` (`--locked` refuses a lock file that
   disagrees). In `CHANGELOG.md`, turn `## Unreleased` into `## <version> — <YYYY-MM-DD>` and put
   a new, empty `## Unreleased` above it.
2. `task check`, exit 0.
3. Commit and push through the bot (`b10x-gates bot --repository beyond10x/cortex -- commit`,
   `... -- push`), open the pull request with `b10x-gates api`, and merge it once `Check` and the
   shared gates are green.
4. Tag the merge commit on `origin/main` through the bot:
   `b10x-gates bot --repository beyond10x/cortex -- tag -a v<version> <commit> -m "cortex <version>"`
   and `b10x-gates bot --repository beyond10x/cortex -- push origin v<version>`.
5. Write the version's `CHANGELOG.md` section to a notes file outside the checkout and publish the
   Release page through the bot:
   `b10x-gates gh --repository beyond10x/cortex -- release create v<version> --verify-tag --title "cortex <version>" --notes-file <file>`.
   Publishing it starts the `Release` workflow.
6. Verify, and report the release only when every line holds:
   - the `Release` run for the tag succeeded (`gh run list --repo beyond10x/cortex --workflow release.yml`);
   - the Release carries exactly the tarball and `SHA256SUMS`
     (`gh release view v<version> --repo beyond10x/cortex --json assets,author`), and its author
     is `b10x-bot[bot]`;
   - the checksum: `gh release download v<version> --repo beyond10x/cortex --dir <dir>`, then
     `sha256sum --check --strict SHA256SUMS` in `<dir>`, and the unpacked `cortex --version`
     prints `cortex <version>`;
   - the tagger: `git for-each-ref refs/tags/v<version> --format='%(objecttype) %(taggername)'`
     prints `tag b10x-bot[bot]`;
   - the author: `git log -1 --format='%an' 'v<version>^{commit}'` prints `b10x-bot[bot]`.

The workflow never overwrites an asset (`gh release upload` without `--clobber`). After a partial
upload, delete the stray asset through the bot
(`b10x-gates gh --repository beyond10x/cortex -- release delete-asset v<version> <asset>`) and
re-run the workflow.
