---
format: aep.planning-md/3
id: story:spec-compat-gate
kind: story
status: implemented
title: The gate fails on an unacknowledged incompatible specification change
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T13:30:19Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-30T13:30:20Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-30T15:30:14Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Outcome

`task check` fails when the specification changes incompatibly against the last release and the
change is not acknowledged.

## Why

Wave 20261030a's hardening ran technique 7 by hand: `ess verify diff` from `main` to the wave
reported 14 changes, 2 of them breaking for readers (`AnswerOutcome` gains `client-closed` and
`cut-off`), and `--fail-on breaking-or-unknown` exited 4. Nothing in the gate would have said so.

## Work

1. `task compat`: `ess verify diff --from <spec at the newest release tag> --to spec --fail-on
   breaking-or-unknown --acknowledgements spec/compat-ack.json`, part of `task check`.
2. `spec/compat-ack.json` acknowledges the reviewed changes of the release being prepared, each
   with its reason.
3. Plant checks, recorded in the commit body: the spec against itself exits 0; a removed command
   input, unacknowledged, exits 4.

## Acceptance

- `task compat` exits 0 on the wave head with the acknowledgements, and exits non-zero with one of
  them removed.
- `task check` runs `task compat`.
- AGENTS.md names the task and when the acknowledgements file is reset (after each release).
