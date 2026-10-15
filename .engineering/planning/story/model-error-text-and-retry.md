---
format: aep.planning-md/3
id: story:model-error-text-and-retry
kind: story
status: implemented
title: A model error answer keeps its reason and is retried once
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-15T03:29:15Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-15T03:29:15Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-15T03:50:55Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

When `claude -p` answers an error, the run keeps the reason it gave: the run directory and the
run's `reason` hold claude's error text, masked as fetched text is. A zero-cost error answer is
retried once before the run fails, so a sign-in or usage-limit answer that clears within the retry
does not count a failure against the source.

## Why

Observed on 2026-10-14 (`journalctl --user -u cortex-agent-tooling-3-mcp-specification.service`):
a timer-started crawl of a real instance ended `extraction-failed` with the reason
`claude answered an error (success), cost 0.0000 USD`, and `--record-failure` counted it. The run
directory's `batch-0/claude/` was empty, so nothing recorded what claude said.

- `src/extract.rs:304-314` builds the reason from `subtype` and `total_cost_usd` only and drops the
  answer's `result` text.
- An error answer with zero cost is the shape of an answer claude gives before any model call (a
  sign-in or a usage limit); this is inferred from the cost, not verified for that run.

## Work

- Spec first: state the reason's content and the retry in `spec/domains/instance.yaml` (the
  `RunSource` outcome `extraction-failed` and its reason), validate with `ess specify validate`,
  regenerate with `task generate`.
- Keep claude's `result` text, masked (`src/mask.rs`) and cut to a bound, in the reason and in
  `batch-<n>/claude/error.txt`.
- On an error answer with zero cost, wait a fixed interval and ask once more; a second error fails
  the run as now.
- Document both on the operating page "Failures".

## Acceptance

- With a stand-in `claude` that answers `is_error: true` with a text and zero cost, the run's
  reason and `batch-0/claude/error.txt` hold that text, masked.
- With a stand-in that answers an error with zero cost once and then a valid document, the run
  ends `ran` and the source's `consecutive_failures` stays 0.
- With a stand-in that answers an error with a cost above zero, no retry is made.

## Not in this story

Rotating or choosing the model account; the unattended model credential
(`story:unattended-model-credential` if drafted). It changes
no binary of a 7-day run in progress.
