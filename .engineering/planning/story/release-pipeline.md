---
format: aep.planning-md/3
id: story:release-pipeline
kind: story
status: implemented
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
revision: 18
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T11:10:14Z", actor: "agent:claude", revision: 15, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-10-05T11:10:14Z", actor: "agent:claude", revision: 16, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-10-05T11:47:27Z", actor: "agent:claude", revision: 18, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
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

Landed 2026-10-05 in `2a047ee` (wave 20261005a, merged `1e02578`); read from `git show --stat 2a047ee`.

- **Files:** `.github/workflows/release.yml` (new, 150 lines), `CHANGELOG.md` (new, 33), `AGENTS.md` (+50, a "Cutting a release" section only)
- **Not changed, confirmed:** `Cargo.toml`, `Cargo.lock`, `Taskfile.yml`, `README.md`, `website/`, `src/`
- **Open questions, answered:** the bot creates the Release (`b10x-gates gh -- release create`) and the workflow attaches to it on `release: published`; the version comes from `Cargo.toml` and a tag other than `v<version>` fails before upload; uploads use `gh release upload` with `GITHUB_TOKEN` in the publish job only (`contents: write`); `actions/upload-artifact` for `workflow_dispatch`
- **Safety fact, changed:** a tag push alone starts no release build (the scoper read `v*` tag pushes as the trigger); the publish job refuses a prerelease and a tag whose commit is not on `origin/main`
- **Review:** `review-result:adversary-release-pipeline-pass-1` (5 findings, all fixed) and `-pass-2` (3 notes, fixed by the coordinator)
