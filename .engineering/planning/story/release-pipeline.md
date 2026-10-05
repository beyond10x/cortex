---
format: aep.planning-md/3
id: story:release-pipeline
kind: story
status: draft
title: cortex releases through a tag, a changelog and a binary
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
scope:
- confidence: inferred
  path: .github/workflows/release.yml
- confidence: cited
  path: AGENTS.md
- confidence: inferred
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
revision: 11
---
## Outcome

cortex has a release workflow that builds a Linux x86_64 binary and its SHA-256, and a written
release process, ready for `story:release-1-0` to use.

## Work

- `CHANGELOG.md` with the unreleased changes so far. (`cortex --version` exists: `src/main.rs:23`.)
- `.github/workflows/release.yml`: on a `v*` tag, build `--release --locked`, attach
  `cortex-<version>-x86_64-unknown-linux-gnu.tar.gz` and `SHA256SUMS` to the release; on
  `workflow_dispatch`, build the same files and upload them as a workflow artifact only.
- AGENTS.md "Cutting a release": bump, changelog, PR through the bot, annotated tag through the bot,
  release page through `b10x-gates gh`, verify author, tag and assets.
- No release is cut by this story.

## Acceptance

A `workflow_dispatch` run of `release.yml` on `main` uploads the tarball and `SHA256SUMS` as an
artifact, the checksum matches the tarball, and the binary inside prints `cortex <Cargo.toml version>`.

## Scope

`Cargo.toml`, `Cargo.lock`, `CHANGELOG.md` (new), `.github/workflows/release.yml` (new), `AGENTS.md`
(the "Cutting a release" section; `story:redaction-before-model` edits a different section of the
same file and comes after this story, its edge records it).
