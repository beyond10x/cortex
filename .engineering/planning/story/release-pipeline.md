---
format: aep.planning-md/3
id: story:release-pipeline
kind: story
status: active
title: cortex releases through a tag, a changelog and a binary
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
scope:
- confidence: cited
  path: .github/workflows/release.yml
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: CHANGELOG.md
revision: 16
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T11:10:14Z", actor: "agent:claude", revision: 15, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-10-05T11:10:14Z", actor: "agent:claude", revision: 16, decided_on: {"recorded":{"review_outcome":2}}}
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

Derived 2026-10-05 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `.github/workflows/release.yml` (new) — cited, story Work
- **Files:** `CHANGELOG.md` (new; no `CHANGELOG*` exists in the tree) — cited, story Work
- **Files:** `AGENTS.md`, a new "Cutting a release" section (today's sections: What this is, Commands, The specification comes first, Documentation site, Hard rules, Upstream facts, Build and verify) — cited, story Work
- **Symbols:** `#[command(name = "cortex", version)]` at `src/main.rs:23`, read only and not changed: `cortex --version` already prints `cortex <CARGO_PKG_VERSION>` — cited
- **Not changed:** `Cargo.toml` and `Cargo.lock`. The story cuts no release and edits neither. The workflow and the acceptance only read `version = "0.1.0"` (`Cargo.toml:3`). The bump is `story:release-1-0`'s work, and its scope lists both files — inferred
- **Not changed:** `website/docs/quickstart.md` and `README.md`. Installing from the release tarball is `story:docs-for-1-0`'s work, which depends on this story — cited, that story's Work
- **Not changed:** `website/data/status.json`. Its rows describe features and none mentions releases. Whether a "released binary" row belongs there is for `docs-for-1-0` or `release-1-0` — inferred
- **Not changed:** `Taskfile.yml`. Its `build` task (`cargo build --locked --release`) is the build the workflow can reuse, so it is read, not edited — inferred
- **Documents:** `AGENTS.md`, `CHANGELOG.md`
- **Confidence:** high — the story's Work names every file it writes, and the tree confirms which are new
- **Would collide with:** any unit editing `AGENTS.md`. `story:redaction-before-model` edits its "Hard rules" section, a different section, and already `depends_on` this story. Any unit adding or editing files under `.github/workflows/` could conflict only on the same new filename, which is unlikely — inferred
- **Safety fact:** `release.yml` is new and runs only on a `v*` tag push or `workflow_dispatch`. Merging it runs nothing on pull requests or `main` pushes, and it cuts no release. Its tag job needs `contents: write`, while every existing workflow declares `contents: read` (`.github/workflows/check.yml`, `shared-gates.yml`), so that write scope is the new privilege. `shared-gates.yml` already runs on `tags: ['*']`, so a tag push will also start the shared gates. Level 1–2, unproven.
- **Open for the implementor:** whether `GITHUB_TOKEN` may write releases here; whether the workflow creates the Release or attaches to one `b10x-gates gh` created; how the tarball name gets the version (tag vs `Cargo.toml`, and whether a mismatch fails); which pinned action uploads assets.
