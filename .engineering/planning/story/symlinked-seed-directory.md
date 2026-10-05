---
format: aep.planning-md/3
id: story:symlinked-seed-directory
kind: story
status: active
title: A seed directory that is a symlink is copied, not refused
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
scope:
- confidence: cited
  path: src/instance.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T13:10:50Z", actor: "agent:claude", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-05T13:10:51Z", actor: "agent:claude", revision: 5}
---
## Outcome

A seed directory that is a symbolic link is copied like a real directory: `cortex create` freezes its contents instead of refusing.

## Why

Found 2026-10-05 by the adversary of `story:seen-documents-modelled` (wave 20261005c, `review-result:adversary-seen-documents-modelled-pass-2`, finding 5). `copy_tree` (`src/instance.rs:117`) never creates the destination for a symlinked seed directory, so `create` answers `seed-refused` with `No such file or directory`. The seed digest follows symlinks since that wave, so the copy and the digest disagree only here. Not checked against the base.

## Work

`copy_tree` follows a symlinked root and symlinked subdirectories, copying their contents as regular files and directories.

## Acceptance

`create` with `seed: {documents: [docs]}` where `docs` is a symlink to a directory answers `created`. The frozen copy holds the files as regular files, and an unchanged `update` answers `updated`.

## Symlinked files inside a seed directory

Also a symlinked **file** inside a seed directory: `copy_tree` does not copy it, while the seed digest (since `story:seen-documents-modelled`) reads it as its target, so such an instance answers `seed-change-refused` to an unchanged update. Reported by that unit's implementor, 2026-10-05. The Acceptance covers both: a symlinked directory and a symlinked file inside one are frozen as regular files and an unchanged update answers `updated`.
