---
format: aep.planning-md/3
id: upstream-blocker:ekr-extraction-valid-time
kind: upstream-blocker
status: cleared
title: EKR extraction does not take a fact's valid time from its evidence
relations:
- blocks: story:document-time-as-valid-time
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-06T07:08:44Z", actor: "agent:claude", revision: 3}
---
## What would clear it

See the title: an EKR release whose extraction path does it. The EKR story is named in this
blocker's relations once filed, and drafted 2026-10-05 in the EKR planning store.

## Cleared 2026-10-06

EKR 0.0.31 (released 2026-10-06, tag `0.0.31` at `91746f3`) ships wave 20261005b: extraction applies in parts, takes a fact's valid time from its evidence, supersedes with `replaces: true`, and writes relations as graph edges.
