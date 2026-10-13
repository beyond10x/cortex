---
format: aep.planning-md/3
id: story:container-image
kind: story
status: draft
title: Each release publishes a container image with cortex and its default EKR
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
revision: 1
---
## Outcome

Each cortex release also publishes a container image that holds the `cortex` binary of that release
and the EKR binary of the version `cortex setup` installs by default, so a cluster runs cortex
without a Rust toolchain or a build step.

## Why

Verified in this tree:

- The README requires "Linux on x86-64 with systemd user units" and "Rust and Cargo, for
  `cortex setup`" (`README.md:39-41`); its Quickstart installs from the release tarball and then runs
  `cortex setup` (`README.md:47-58`).
- A release carries a binary for `x86_64-unknown-linux-gnu` only, and `cortex setup` installs EKR
  with `cargo install`, so it needs a Rust toolchain (`website/docs/limits.md:17-20`;
  `src/ekr.rs:64`, `:74`). The default EKR version of `cortex setup` is `0.0.32`
  (`src/main.rs:165`).
- "Cutting a release": `.github/workflows/release.yml` attaches
  `cortex-<version>-x86_64-unknown-linux-gnu.tar.gz` and `SHA256SUMS` and nothing else
  (`AGENTS.md:193-203`); only its upload job holds `contents: write` (`release.yml:103-104`).
- No `Dockerfile`, `Containerfile` or image step exists in the tree (searched the root and
  `.github/workflows/`).

## Work

- A container build in the repository that installs the release binary and builds or fetches the
  pinned EKR, reproducibly, with no secret in any layer (AGENTS.md "No secret enters cortex").
- `release.yml` builds and pushes the image tagged with the release version, from the same commit as
  the tarball; the job that pushes holds only the permission it needs. The registry is not decided
  here; the repository package registry is the candidate, and the choice is the operator one when
  the story is proposed.
- Spec first: no new noun, command or field is expected. If the image needs cortex to take an EKR
  binary by path instead of `cortex setup`, that option goes into `spec/` first.
- Update "Cutting a release" in `AGENTS.md` (the verification list gains the image),
  `README.md` "Requirements" and `website/docs/limits.md` "Platform".

## Acceptance

- After a release is published, pulling the image tagged with that version succeeds.
- Running `cortex --version` in that image prints `cortex <version>`.
- Running `ekr --version` in that image prints the EKR version `cortex setup` defaults to.

## Not in this story

`connectors` and `claude` in the image, and their credentials (`story:headless-connectors-credentials`,
`story:unattended-model-credential-design`); where the home lives (`story:checkpoints-volume-contract`);
non-x86-64 images; any deployment manifest.
