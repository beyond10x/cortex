---
format: aep.planning-md/3
id: epic:organisation-scale-instance
kind: epic
status: active
title: An organisation's own sources run through cortex at production volume
relations:
- serves: vision:self-updating-instances
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T14:07:50Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-08T14:07:50Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":1}}}
---
## Outcome

An organisation's own sources (issue trackers, code hosting, chat exports, curated documents,
registries) run through one cortex instance every day at production volume: paged and nested
Connectors reads, file exports split into records, personal data masked with a refusing gate,
deterministic imports for structured registries, facts dated by their source, a post-run gate that
restores the store when the run is bad, and a model judge of fact quality.

## Why now

A company-scale instance built outside cortex does all of this in its own code today (≈27k lines,
its own write path to EKR). Moving it onto cortex needs these capabilities in cortex, generically.
The consumer is named only in its own repository.

## Stories

| story | adds |
|---|---|
| `connectors-source-walks` | paging, a child call per record, `{since}`/`{until}` windows |
| `file-records` | JSON-lines and markdown-section records from files, with filters and thread context |
| `redaction-names-and-gate` | URL and credential classes, known-name lists, rare-name masking, a refusing gate |
| `structured-from-files-and-drops` | structured imports from files, paged structured reads; superseding values a source dropped |
| `document-time-as-valid-time` | a document's time becomes its evidence time and its facts' valid time |
| `run-gate` | post-run checks; a failure restores the pre-run snapshot |
| `quality-judge` | `cortex quality`: sample facts, judge them with the model, report fact quality |

The types for all of these land first, in `story:spec-standalone-types` (its "Organisation-scale
types" section); the separate types story drafted for this epic was folded into it and archived.

Upstream, two EKR stories block stories here: `upstream-blocker:ekr-extraction-supersession` and
`upstream-blocker:ekr-extraction-valid-time`. Two more EKR stories (partial apply of an extraction
document, relations written as graph edges) block no story in this epic; the consumer's cutover
needs them and its own repository tracks that.

## Not in this epic

The consumer's own spec file, its seen-state conversion and its cutover (they live in its
repository); process-map views (an EKR story).
