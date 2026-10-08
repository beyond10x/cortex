---
format: aep.planning-md/3
id: story:release-1-0
kind: story
status: active
title: cortex 1.0.0 is released and installable
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
- depends_on: story:release-pipeline
- depends_on: story:docs-for-1-0
- depends_on: story:timer-runs-unattended
- depends_on: story:first-web-instance
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T14:08:18Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-08T14:08:18Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":1}}}
---
## Outcome

cortex 1.0.0 is released and installable from its GitHub Release.

## Work

- Version 1.0.0, CHANGELOG entry listing the 1.0 features and the open limits.
- Release through the process `story:release-pipeline` wrote.
- On a user account with no cortex checkout, install the released binary and follow the site's
  quickstart (which `story:docs-for-1-0` points at the release).

## Acceptance

The `v1.0.0` Release carries the tarball and `SHA256SUMS`, and on that account the released binary
completes the quickstart with `cortex create` exit 0, `cortex run` exit 0 with
`documents_applied` greater than 0, and the viewer URL answering HTTP 200; each recorded as a
`test_result` evidence record on this story with the command as its source.

## Depends on

`story:release-pipeline`, `story:docs-for-1-0`, `story:timer-runs-unattended` (1.0 claims unattended
scheduled runs, which that story proves), and `story:first-web-instance`: the `v1.0.0` tag waits
until its 7-day run passes, read on day 8 (2026-10-16).

Before then, wave 20261008c writes the 1.0 summary into `CHANGELOG.md` under `## Unreleased` and
rehearses this story's acceptance from the `v0.2.2` tarball in an empty home. The version bump, the
tag and the Release are cut after the run passes, through the process in `AGENTS.md`.

## Scope

`Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`.
