---
format: aep.planning-md/3
id: story:extraction-supersedes
kind: story
status: active
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
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T07:36:01Z", actor: "agent:claude", revision: 6, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "proposed", to: "active", at: "2026-10-06T07:36:01Z", actor: "agent:claude", revision: 7, decided_on: {"recorded":{"review_outcome":3}}}
---
## Outcome

The model may mark a property fact as replacing the active value of the same subject and property; cortex passes it to EKR's extraction supersession (`replaces: true`).

## Why

Found 2026-10-06 by a gap inventory for running a downstream company deployment entirely on cortex and EKR; no story covered it.
EKR wave 20261005b adds supersession to extraction; cortex's prompt and answer schema do not use it yet, so a changed fact (an owner, a status) stays beside the old one.

## Acceptance

Two documents a day apart give one property two values; after both runs exactly one assertion is active (the newer), and the older is superseded, citing the newer document.

## Depends on

The EKR release that ships `story:extraction-supersession` (recorded as `upstream-blocker:ekr-extraction-supersession`), and `story:document-time-as-valid-time`, which edits the same prompt and fact merge in `src/extract.rs` and moves the EKR pin first.

## Shared files

- `story:quality-judge` (`src/extract.rs` `Model::ask_with`, `src/main.rs`) and `story:web-pages-cited-as-urls` (`src/extract.rs`, `src/evidence.rs`, and the EKR pin sites `src/main.rs`, `examples/*.yaml`, `.github/workflows/check.yml`, `AGENTS.md`) land after this story; their edges record it.
- `story:postgres-credential-from-connectors` edits `src/main.rs` in the store-open path, not the EKR pin constant; the overlap is accepted, and the second to land rebases.
- `story:codex-model-backend` (active, blocked by `decision-blocker:codex-exec-keeps-shell`, its work archived) edits `src/extract.rs`; if it resumes it rebases on this story.

## Files (from the inventory, unverified)

`src/extract.rs` (`SYSTEM_PROMPT`, `admitted_facts`, `merge`), `tests/supersession.rs`; the EKR pin sites `src/main.rs`, `examples/*.yaml`, `.github/workflows/check.yml`, `AGENTS.md`.
