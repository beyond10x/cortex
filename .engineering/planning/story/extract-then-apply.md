---
format: aep.planning-md/3
id: story:extract-then-apply
kind: story
status: implemented
title: Extract now, apply later
summary: run --extract-only keeps the extraction without writing; cortex apply writes a kept extraction once
relations:
- serves: vision:self-updating-instances
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T10:28:55Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T10:28:55Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-24T00:20:47Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

A caller can put its own check between the facts a run extracted and their write to the store.

## Work

Consumer: the runbook refresh of `timo/dev-operator` (session `bf-runbook`), 2026-11-23. Its
protocol wants a gate (a check of the extracted claims) between "facts extracted" and "facts
applied"; today the check can only run after the write.

- `cortex run <instance>/<source> --extract-only` fetches, extracts and keeps the extraction in the
  run directory, and writes nothing to the store; the seen state is not advanced.
- `cortex apply <instance> <run>` applies a kept extraction, once, with the same reconcile and
  refusal rules as a normal run, and advances the seen state then.
- A kept extraction that the store moved past (a newer run applied the same documents) is refused
  with a named outcome.

## Acceptance

With a stand-in model, `run --extract-only` leaves the store's assertion count unchanged and a kept
extraction in the run directory; `apply` of that run adds the facts and a second `apply` of it is
refused; a normal run after `--extract-only` and before `apply` still sees the documents as new.
