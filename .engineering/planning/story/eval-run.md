---
format: aep.planning-md/3
id: story:eval-run
kind: story
status: implemented
title: An operator runs an eval set end to end and gets a deterministic score
summary: cortex eval run over a question set with mcp, ask or replay runners, scored by expected facts and cited nodes
relations:
- decomposes: epic:instance-answers-questions
- serves: vision:self-updating-instances
- depends_on: architecture-decision-record:answer-service-in-cortex
- informed_by: story:quality-judge
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-17T03:36:58Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-17T03:36:58Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-17T16:10:56Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

`cortex eval run <instance> --set <questions.json> --runner mcp|ask|replay` asks each question of
an eval set end to end and scores each answer deterministically: the expected fact strings it must
contain and the node ids it must cite. The `mcp` runner asks through the store's MCP reader with
the instance's model, `ask` through a running `cortex ask serve`, and `replay` re-scores answers an
earlier run recorded, with no model call. Each run writes `eval/<UTC stamp>/` with one result per
case and a summary (cases, passed, per-check counts, tokens, cost), and its detail names that
directory.

**No implementation starts until the operator accepts
`architecture-decision-record:answer-service-in-cortex`.**

## Why

A downstream deployment keeps an eval harness with three runners and exposure checks in code of
its own. Verified in this tree: cortex's only evaluation verb is `cortex quality`, which judges
whether sampled facts are supported by their evidence (`src/quality.rs`,
`website/docs/commands.md`, "Measuring fact quality"; `story:quality-judge`). Nothing in cortex asks
the store a question or scores an answer. `quality` writes `quality/<UTC stamp>/` in the instance
directory; this story follows the same shape.

## Domain relations

New in `spec/domains/ask.yaml` (or `cortex.instance`, as the record's domain split settles):

- value `cortex.ask.EvalCase` (`format` `cortex.eval-case/1`: `id`, `question`,
  `expect_facts: List<String>`, `expect_nodes: List<String>`, optional `source`), and
  `cortex.ask.EvalSet` (a list of them).
- entity `cortex.ask.EvalRun` (identity: the stamp): `instance_name`, `runner`, `cases`, `passed`,
  `cost_usd`. Relation: `cortex.instance.Instance` owns many `EvalRun` via `instance_name`.
- value `cortex.ask.EvalResult`, one line of `results.jsonl` per case.

## Work

- Spec first: the types above and the command `RunEval` with its outcomes (`evaluated`,
  `set-invalid`, `runner-failed`, `no-such-instance`); validate, conformance, `task generate`.
  Then the code.
- `src/eval.rs`: set loading, the three runners, the scorer; `cortex eval run` in `src/main.rs`.
- The `mcp` runner counts against `model.daily_ceiling_usd` as `quality` does.
- Docs: `commands.md` gains the verb and the result files.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- A set of two cases against a stand-in that answers the first with its expected fact and node and
  the second without: the summary says 2 cases, 1 passed, and the second result names the missing
  fact string.
- `--runner replay` over that run's directory gives the same per-case verdicts with no model call
  (stand-in model never started).
- A set that is not `cortex.eval-case/1` answers `set-invalid` and asks no model.
- Two runs of `replay` over one directory write byte-identical results files.

## Not in this story

- Cases from reviewed answers, and any improvement loop (`story:eval-cases-from-review`).
- Judging fact support (`cortex quality`, already there).
- The ask service itself (`story:ask-service`); without it the `ask` runner answers `runner-failed`.
