---
format: aep.planning-md/3
id: story:document-time-as-valid-time
kind: story
status: implemented
title: A fact is dated by when its source said it
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:file-records
- depends_on: story:spec-standalone-types
scope:
- confidence: cited
  path: .github/workflows/check.yml
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: examples/agent-tooling.yaml
- confidence: cited
  path: examples/example.yaml
- confidence: cited
  path: src/evidence.rs
- confidence: cited
  path: src/main.rs
- confidence: cited
  path: tests/agent_tooling.rs
- confidence: cited
  path: tests/common/mod.rs
- confidence: cited
  path: tests/conformance.rs
- confidence: cited
  path: tests/document_time.rs
- confidence: cited
  path: tests/fixtures/spec_compat/example.json
- confidence: cited
  path: tests/spec_compat.rs
- confidence: cited
  path: website/docs/commands.md
- confidence: cited
  path: website/docs/quickstart.md
- confidence: cited
  path: website/docs/spec-file.md
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T07:13:58Z", actor: "agent:claude", revision: 6, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-06T07:13:59Z", actor: "agent:claude", revision: 7, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-10-06T07:33:13Z", actor: "agent:claude", revision: 12, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Outcome

A fact is dated by when its source said it: the document's time is the evidence's observed time and
the fact's valid-from time.

## Work

- `src/evidence.rs`: the evidence entry for a document carries the document's `published` /
  record `time` as its observed time.
- `src/extract.rs`: the extraction document asks EKR to take valid time from evidence time, once
  `upstream-blocker:ekr-extraction-valid-time` clears.

## Acceptance

`tests/document_time.rs`: a `files` source with two JSON-lines records dated 2026-01-02 and
2026-03-04 yields facts whose `valid_time.from` in `ekr snapshot` equal those instants.

## Depends on

`story:spec-standalone-types` (the types), `upstream-blocker:ekr-extraction-valid-time`,
`story:file-records` (record times come from it), `story:codex-model-backend` (both edit
`src/extract.rs`).

## Scope

Landed 2026-10-06 in `70dbdd9` (wave 20261006d).

- **Files:** `src/evidence.rs` (`observed_at`, the date parser), `tests/document_time.rs` (new), `tests/adopt.rs`; the EKR 0.0.31 pin in `src/main.rs`, `examples/*.yaml`, `.github/workflows/check.yml`, `AGENTS.md`, `tests/common/mod.rs`, `tests/agent_tooling.rs`, `tests/spec_compat.rs`, `tests/fixtures/spec_compat/example.json`, `tests/conformance.rs`; docs `operating.md`, `spec-file.md`, `commands.md`, `quickstart.md`
- **Not changed:** `src/extract.rs` (EKR 0.0.31 dates facts from evidence)
- **Review:** coordinator review; one correction (epoch seconds need 9 to 11 digits; nothing before 1970)

## Design decisions (2026-10-06)

Decided 2026-10-06 by the coordinating session (operator delegation), after scoping against EKR 0.0.31:

- **The EKR pin moves to 0.0.31 in this story**, at every pin site: `src/main.rs` (`cortex setup` default), `examples/*.yaml`, `.github/workflows/check.yml`, `AGENTS.md`, `tests/common/mod.rs`, `tests/agent_tooling.rs`, `tests/spec_compat.rs`, `tests/fixtures/spec_compat/example.json`, `tests/conformance.rs`, and the three docs pages that name the default.
- **No change to `src/extract.rs`.** EKR 0.0.31 gives an extracted fact valid time from the earliest `observed_at` of the evidence it cites.
- **Evidence time comes from the document.** `src/evidence.rs` sets `observed_at` from `Document.published` when it parses. It accepts RFC 3339, a date alone (`YYYY-MM-DD`, taken as 00:00 UTC) and epoch seconds with an optional fraction (a chat export's `ts`). A value that does not parse, or lies in the future, falls back to the run start. Nothing else changes: seen state and refresh windows keep running on the run's own times.
- **Pin side effects are accepted and tested.** Partial apply means a refused fact is now listed in `rejected` instead of failing the batch. A `!Relation` now also writes a graph edge.
