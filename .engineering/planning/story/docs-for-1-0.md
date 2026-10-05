---
format: aep.planning-md/3
id: story:docs-for-1-0
kind: story
status: draft
title: The public documentation describes every 1.0 feature
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
- depends_on: story:store-backend-per-instance
- depends_on: story:redaction-before-model
- depends_on: story:structured-source
- depends_on: story:codex-model-backend
- depends_on: story:run-snapshots
- depends_on: story:adopt-existing-store
- depends_on: story:release-pipeline
scope:
- confidence: cited
  path: README.md
- confidence: cited
  path: website/docs/commands.md
- confidence: cited
  path: website/docs/limits.md
- confidence: cited
  path: website/docs/operating.md
- confidence: cited
  path: website/docs/quickstart.md
- confidence: cited
  path: website/docs/spec-file.md
revision: 8
---
## Outcome

The public documentation describes every 1.0 feature as it behaves, nothing that does not exist,
and how to install a released binary.

## Work

- `website/docs/quickstart.md`: install from the GitHub Release tarball and check `SHA256SUMS`
  (today step 1 is `git clone` plus `cargo install --locked --path .`, `website/docs/quickstart.md:19-24`);
  building from source moves to a second section.
- `website/docs/spec-file.md`: store backends, model backends, redaction, structured sources,
  snapshots.
- `website/docs/commands.md`: `restore`, `adopt`.
- `website/docs/operating.md`: PostgreSQL provisioning, snapshots and restore, Codex cost reporting.
- `website/docs/limits.md`: redaction is pattern-based; the open EKR limits (URL evidence,
  relations not walked by graph reads) with their upstream story ids.
- README.md feature list.

## Acceptance

The PR body carries a table with one row per statement added to the six files, each naming the test
or command output that shows it, and no row without one.

## Depends on

`story:store-backend-per-instance`, `story:redaction-before-model`, `story:structured-source`,
`story:codex-model-backend`, `story:run-snapshots`, `story:adopt-existing-store`,
`story:release-pipeline` (the install section names the release asset names it defines).

## Scope

`website/docs/quickstart.md`, `website/docs/spec-file.md`, `website/docs/commands.md`,
`website/docs/operating.md`, `website/docs/limits.md`, `README.md`.
