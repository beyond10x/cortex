---
format: aep.planning-md/3
id: story:review-answers-eval
kind: story
status: implemented
title: Answers and evaluations keep refusals, verified citations and spent usage
summary: Findings 19 to 24 of the 0.7.2 review
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T01:54:01Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T01:54:01Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-24T00:20:59Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

An answer, its citations and its evaluation report what happened: a refusal stays a refusal, a
citation names only what the reader returned, and spent usage is kept.

## Work

Findings 19 to 24 of the external review of cortex 0.7.2 (2026-11-23):

- 19: `eval run --runner ask` exits 1 with "no service" while `ask serve` runs (`src/eval.rs`,
  `website/docs/commands.md`, `website/data/status.json`). Wire the runner to the instance's
  configured answer endpoint.
- 20: a Chat Completions refusal (no content, non-empty `refusal`) becomes `done` with an empty
  answer recorded as answered (`src/ask/model.rs`, `src/ask/mod.rs`). Keep the refusal in the
  answer record.
- 21: a node id the model names but the reader never returned is published in `done.nodes` and as
  a link (`src/ask/mod.rs`, `src/ask/cite.rs`). Track returned ids; mark unsupported references.
- 22: replay passes a refused eval case whose saved reason contains the expected word
  (`src/eval.rs`). Keep the non-answer outcome when rescoring.
- 23: the quality judge gets byte-identical prompts for one fact valid in January and in February
  (`src/quality.rs`). Include the assertion's validity interval.
- 24: a step-limited answer records zero steps and zero tokens after two model calls
  (`spec/domains/ask.yaml`, `src/ask/mod.rs`, `src/ask/records.rs`). Keep accumulated usage on
  terminal outcomes after a model call.

## Acceptance

A test per finding (local gateway and reader stand-ins, no model call) reproduces the reviewed
scenario, fails on 0.7.2's code and passes on this change; spec changes pass `task compat`.
