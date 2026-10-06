---
format: aep.planning-md/3
id: story:extraction-supersedes
kind: story
status: draft
title: Extraction supersedes the value a newer document replaces
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:document-time-as-valid-time
scope:
- confidence: inferred
  path: .github/workflows/check.yml
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: src/extract.rs
- confidence: inferred
  path: src/main.rs
- confidence: inferred
  path: tests/supersession.rs
revision: 4
---
## Outcome

The model may mark a property fact as replacing the active value of the same subject and property; cortex passes it to EKR's extraction supersession (`replaces: true`).

## Why

Found 2026-10-06 by the gap inventory for running Company Brain v3 entirely on cortex and EKR (cb3 `initiative:run-on-cortex`); no story covered it.
EKR wave 20261005b adds supersession to extraction; cortex's prompt and answer schema do not use it yet, so a changed fact (an owner, a status) stays beside the old one.

## Acceptance

Two documents a day apart give one property two values; after both runs exactly one assertion is active (the newer), and the older is superseded, citing the newer document.

## Depends on

The EKR release that ships `story:extraction-supersession` (recorded as `upstream-blocker:ekr-extraction-supersession`), and `story:document-time-as-valid-time`, which edits the same prompt and fact merge in `src/extract.rs` and moves the EKR pin first.

## Files (from the inventory, unverified)

`src/extract.rs` (`SYSTEM_PROMPT`, `admitted_facts`, `merge`), `tests/supersession.rs`; the EKR pin sites `src/main.rs`, `examples/*.yaml`, `.github/workflows/check.yml`, `AGENTS.md`.
