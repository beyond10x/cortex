---
format: aep.planning-md/3
id: story:run-path-override
kind: story
status: active
title: A files source reads a per-run directory with stable keys
summary: cortex run --path for the runbook refresh
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T03:32:07Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T03:32:07Z", actor: "human:timo", revision: 3}
---
## Outcome

A step of an outside orchestrator runs a `files` source over a directory it names per run, and a
record keeps one key across runs whatever directory it was read from.

## Work

Consumer: the runbook refresh of `timo/dev-operator` (session `bf-runbook`, 2026-11-23). Each runbook
run writes its slices under its own `work/` directory and calls `cortex run` once per step; the
spec's `paths` is fixed, and a document's key is `<file path>#<id>`, so a new directory per run
makes every record a new document. Today the caller links a fixed directory to each run's `work/`.

- `cortex run <instance>/<source> --path <dir>` reads a `kind: files` source from `<dir>` in place
  of its `paths` for that run only; the spec and its `glob`, `records` and `lookup` stay as they
  are.
- With `--path`, a document's key and its evidence citation use the file's path relative to
  `<dir>`: `<relative path>#<id>` and `file:<relative path>#<id>`. The same record read from two
  run directories is one document; unchanged, it is not extracted again.
- `--path` on a source of another kind is refused with its own outcome; a missing or unreadable
  `<dir>` fails the fetch and names the directory.
- `--fetch-only` takes `--path` too.

## Acceptance

Two runs with `--path` over two directories holding the same JSONL record apply it once, and the
second run reports `documents_new` 0; a changed record in the second directory is applied again;
`--path` on a `web` source is refused; `website/docs/commands.md` and `spec-file.md` describe the
flag and the key rule.
