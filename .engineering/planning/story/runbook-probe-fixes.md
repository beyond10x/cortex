---
format: aep.planning-md/3
id: story:runbook-probe-fixes
kind: story
status: implemented
title: RareName keeps type names, run folders keep prompt and answer, no phantom viewer
summary: Defects 1, 3 and 5 of the runbook refresh probe
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T05:01:55Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T05:01:56Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-24T00:21:10Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

A runbook refresh round with a `RareName` redaction policy applies its facts, its run folders show
what the model was sent and answered, and `create --no-units` reports no viewer it did not start.

## Work

Consumer: the runbook refresh of `timo/dev-operator` (session `bf-runbook`). Its first probe on
2026-11-23 (cortex 0.7.2, EKR 0.0.34, one `files` source, 145 records, 4 extraction calls) found:

- `RareName` redaction also replaces the ontology's type names in the prompt. The model then
  writes the placeholder as a type, EKR refuses every entity (`extraction-type-undeclared:
  Name-32`), 38 placeholders stay unrestored, 0 facts, and the run is undone. Node type, property
  and edge type names of the store's ontology are not counted or replaced by `RareName`; a
  placeholder that still reaches a type, property or edge position of the answer is restored
  before the answer is applied, or the batch fails naming it. Explicit `known_names` and `rules`
  still apply to them, as `story:ontology-names-masked` says.
- Run folders (`runs/<run>/batch-N/claude/`) keep only the extraction document. Keep the prompt as
  sent (redacted, never the placeholder mapping) and the model's raw answer beside it, under the
  run's retention (`runs.keep_days`).
- `cortex create --no-units` prints a viewer address although no viewer started. Print none, and
  list none in `cortex list`, for an instance without units.

## Acceptance

With a `RareName` policy and an ontology whose type names are rare in the batch, a stand-in model
that echoes the prompt's type names applies its facts and the prompt holds the type names
unchanged; a batch folder holds `prompt.txt` and `answer.json`, and neither holds an unredacted
value of a redacted class; `create --no-units` output and `cortex list` name no viewer address.
