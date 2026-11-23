---
format: aep.planning-md/3
id: story:placeholder-restore-and-retry-error
kind: story
status: active
title: Restore bare placeholders, explain retry failures, keep known names
summary: Bare Name-N placeholders are restored in values and aliases, a batch ending on exhausted structured-output retries leaves a diagnosable error.txt, and RareName does not mask a name the store knows (runbook 0.8.0 findings 1, 3, 4)
relations:
- serves: vision:self-updating-instances
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T16:52:48Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T16:52:48Z", actor: "human:timo", revision: 3}
---
## Outcome

Every placeholder a model writes back is restored, in brackets or bare, in values and in aliases; a
batch that ends on exhausted structured-output retries leaves a diagnosable error; and a name the
store already knows is not masked as rare.

## Work

From the runbook consumer's run of 0.8.0 (synthetic examples here):

- A model answers `"value": "Name-10 Name-11"` and `"aliases": ["Name-10"]` without the brackets
  the masked text used (`[Name-10]`). 0.8.0 restores the bare form only where a type name goes, so
  these count as unrestored placeholders and the redaction gate undoes the run (8 against a limit
  of 5). Restore a bare placeholder that the batch's mapping holds, wherever a bracketed one is
  restored; a bare token the mapping does not hold stays as written.
- A batch whose model call ends with `error_max_structured_output_retries` writes an empty
  `claude/error.txt` and no answer. Write the result subtype, the number of turns and the last
  structured-output validation message (masked like every other error text) into `error.txt` and
  the batch's reason.
- `RareName` masks a customer name said once, and its facts are lost (recall 20 covered of 34
  lines with it on, 26 off). A word that is already a node name or alias in the store at the
  run's head (or in the spec's `known_names`) is not a rare name. If that needs more than this
  change can carry, the PR adds a draft story with the design instead.

## Acceptance

A batch answer with bare `Name-N` placeholders in a value and an alias is stored with the
originals and the gate counts 0 unrestored; a bare `Name-99` the mapping does not hold stays as
written and is counted; a stubbed model call ending in `error_max_structured_output_retries`
leaves a non-empty `error.txt` naming the subtype; and a capitalised word said once that is a
known node name is not replaced (or the draft story exists).
