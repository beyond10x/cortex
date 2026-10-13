---
format: aep.planning-md/3
id: story:docs-match-code
kind: story
status: active
title: AGENTS.md, README.md and STATUS.md match the code
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
- depends_on: story:ess-pin-0-59
- depends_on: story:dependency-pins-current
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: README.md
- confidence: inferred
  path: STATUS.md
- confidence: inferred
  path: website/docs
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T19:21:17Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-13T19:21:17Z", actor: "human:timo", revision: 4}
---
## Outcome

`AGENTS.md` (for agents) and `README.md` (for humans) match the code: each links the other, no
content is duplicated between them, and every command either names runs as written. `STATUS.md`
holds the repository's state against named targets, every row dated and naming the command it was
read from.

## Why

There is no `STATUS.md`. `AGENTS.md` and `README.md` were last checked against the code before
0.2.2 and before the pins of this wave moved.

## Work

1. Run every command `AGENTS.md` and `README.md` name (or, for one that changes a machine, check
   its arguments against `cortex --help` and the Taskfile); fix what does not run as written.
2. Remove what one file duplicates from the other; keep the cross-links.
3. `STATUS.md`: one row per target (gate, CI on `main`, open issues, pins, release, docs site,
   the 7-day run's acceptance), each with its date and the command it was read from.

## Acceptance

- Each command in both files, run as written in a tree of the wave's head, exits 0, or is listed
  in the unit's report with why it was not run.
- `STATUS.md` exists and every row carries a UTC date and a command.
