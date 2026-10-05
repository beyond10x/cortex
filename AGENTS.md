# AGENTS.md — cortex

Instructions for agents changing this repository. Humans start at [README.md](README.md).

## What this is

`cortex` spins up EKR knowledge brains from an instance spec. One instance is one EKR SQLite store
plus the data sources that feed it on a schedule. The engine is EKR (`ekr` binary); integrations
and every secret belong to Connectors (`connectors` binary); extraction is a tool-less
`claude -p` call whose output is checked against EKR's own schema.

## The specification comes first

`spec/` is an ESS specification (`ess/19`, pinned to ESS 0.52.0 in `spec/ess-inputs.yaml`). It is
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

## Hard rules

- **No secret enters cortex.** A source names a Connectors connection id; cortex only invokes
  operations through `connectors`. Never read, print, store or pass a credential, and never add a
  spec field that carries one.
- **No instance data in Git.** Instances live under `$CORTEX_HOME` (default
  `~/.local/share/cortex`). Tests build their own homes in temp directories.
- **Committed code is Rust.** Probes in other languages stay outside the repository.
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

## Build and verify

- `task check`: spec validity, generated-code drift, fmt, clippy (`-D warnings`), all tests.
- Format with `cargo fmt -p cortex-cli`. `cargo fmt --all` also rewrites the generated crates,
  which are path dependencies, and `task drift` then fails.
- `tests/conformance.rs` runs every scenario of `spec/suite.json` (35) in process; `tests/e2e.rs`
  drives the binary end to end; the clap ⇔ spec test in `src/main.rs` holds the command line to
  the commands, inputs and views `spec/` puts on it.
- Builds use `CARGO_TARGET_DIR=$HOME/.cache/b10x-target/cortex`. Check `df -h /` first; do not
  start a build with less than 10 GB free.
- The end-to-end tests run a real `ekr` (`CORTEX_TEST_EKR`, default the 0.0.30 binary under
  `~/.cache/company-brain-v3/bin/0.0.30/bin/ekr`) with stand-in `connectors`, `claude` and
  `systemctl`. A missing `ekr` fails them; it never skips them.
