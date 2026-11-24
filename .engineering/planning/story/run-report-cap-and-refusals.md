---
format: aep.planning-md/3
id: story:run-report-cap-and-refusals
kind: story
status: implemented
title: A run says when it was capped and why EKR refused a part
summary: A capped run reports stopped document-cap with the count left; each batch directory keeps EKR's refusal reasons
relations:
- serves: vision:self-updating-instances
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T10:26:53Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T10:26:53Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-24T00:21:09Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

A caller can tell from a run's report alone whether the run read everything it was given, and can
read why EKR refused any part of it, from the run directory.

## Work

Consumer: the runbook refresh of `timo/dev-operator` (session `bf-runbook`), 2026-11-23, running
cortex 0.7.2 as a runbook step (1012 records, 4167 facts), synthetic wording:

- When `max_documents_per_run` caps a run, the run reports `ran` with `stopped: null`, the same as
  a run that read every wanted document. The caller counts what is left with a second
  `--fetch-only` call. A capped run reports `stopped: document-cap` and the number of wanted
  documents left for the next run.
- A part EKR refuses leaves no reason in the run directory. Beside the prompt and the raw answer
  each batch directory keeps (PR #96), the batch directory keeps each refusal: the part, the
  outcome EKR named and its reason text, masked under a redaction policy like the rest of the
  directory.

## Acceptance

With a stand-in model, a source with 5 wanted documents and `max_documents_per_run: 2` reports
`stopped: document-cap` with 3 left, and a run of all 5 reports `stopped: null` with 0 left; a batch
whose apply EKR refuses for one part keeps a refusal file in its batch directory naming the part,
the outcome and the reason, and under a redaction policy that file holds placeholders only.
