---
format: aep.planning-md/3
id: story:build-into-tree-target
kind: story
status: active
title: Every cortex worktree builds into its own target/
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Taskfile.yml
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T20:07:12Z", actor: "agent:claude", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-06T20:07:12Z", actor: "agent:claude", revision: 4}
---
## Outcome

Every cortex worktree builds into its own `target/`, and a finished tree's build cache is discarded with the tree, so no shared build directory outlives the work that made it.

## Why

Conductor decision DEC-20261006-02 (dispatch DSP-20261006-04, 2026-10-06): a shared `~/.cache/b10x-target/cortex` let one tree's gate run another tree's test binaries, and its size was invisible to worktree cleanup. Building into the tree's own `target/` ties the cache to the tree and lets `worktree finish --discard-cache --archive` remove it.

## Work

- `Taskfile.yml`: drop the `CARGO_TARGET_DIR` default (the `TARGET` variable), so cargo builds into the tree's `target/` unless the caller sets the variable (CI still sets its own).
- `AGENTS.md`: the build rule reads: build into this tree's `target/`; end each tree with `worktree finish --discard-cache --archive <tree>`; check `df -h /` first.

## Acceptance

- `task check` in a fresh tree with no `CARGO_TARGET_DIR` set writes its build output under that tree's `target/` (the pull request's CI runs the gate).
- `AGENTS.md` states the rule above and no longer names `~/.cache/b10x-target/cortex`.

## Not modelled

Build tooling only: cortex's ESS specification models the instance, not how the repository builds, so there is nothing to model.
