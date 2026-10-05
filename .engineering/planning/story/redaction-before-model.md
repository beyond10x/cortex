---
format: aep.planning-md/3
id: story:redaction-before-model
kind: story
status: implemented
title: Personal data is replaced before the model sees a document
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
- depends_on: story:spec-standalone-types
- depends_on: story:release-pipeline
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: src/lib.rs
- confidence: inferred
  path: src/redact.rs
- confidence: cited
  path: src/run.rs
- confidence: inferred
  path: tests/redaction.rs
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T11:54:37Z", actor: "agent:claude", revision: 10, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-05T11:54:37Z", actor: "agent:claude", revision: 11, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-10-05T13:01:18Z", actor: "agent:claude", revision: 14, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Outcome

Personal data in a fetched document is replaced before the model sees it, by the classes and rules
the instance's `redaction` policy names.

## Work

- `src/redact.rs`: built-in classes `Email`, `Phone`, `IpAddress`, `PaymentCard` and the spec's
  regex rules, each replaced by its `replacement` (default `[REDACTED:<class>]`).
- `src/run.rs`: apply redaction after credential masking (`mask::mask`) and before the batch is
  built, so the stored evidence payload and the prompt carry the redacted text, never the original.
- A run's report counts redactions per class.
- AGENTS.md's "no PII redaction" hard rule changes to name the policy and its limits (pattern-based;
  names of people are not detected).

## Acceptance

A unit test per class and for a custom rule, and an e2e test whose stand-in model receives a prompt
in which a planted email address, phone number and card number are absent and the run report counts
3 redactions.

## Depends on

`story:spec-standalone-types`.

## Scope

Landed 2026-10-05 in `5868e4d` (wave 20261005c, merged `601ba5f`).

- **Files:** `src/redact.rs` (new), `src/run.rs`, `src/lib.rs`, `tests/redaction.rs` (new), `AGENTS.md`, `website/docs/limits.md`; `src/main.rs` (run and seed report counts) applied at merge
- **Design changed by the operator, 2026-10-05:** placeholders in the prompt, restored before apply; the store keeps the real values (section "Operator decision")
- **Not built here:** credential masking of titles, descriptions and document keys → `story:credential-mask-covers-titles` (pre-existing)
- **Review:** `review-result:adversary-redaction-before-model-pass-1` (14) and `-pass-2` (10), fixed or documented; coordinator check of the final diff

## Operator decision: PII stays out of the model, not out of the store

2026-10-05, operator: "It would be nice if something like an email or otherwise PII would not reach the LLM models - but we still would kind of need to find the data later, so storing it in the brain and being able to search for it is actually fine".

What follows from it (coordinator, same day):

- Redaction applies to what the model sees: text, title, description and the `Source:` line built from the document key. Each distinct value becomes a stable placeholder within the batch (`[Email-1]`, `[Phone-2]`, `[<rule>-1]`).
- Before the extraction document is applied, its placeholders are replaced by the original values. The store, the evidence text and the evidence identity hold the real values, and the brain stays searchable by them.
- A placeholder that cannot be restored stays as written and is counted as `unrestored`.
- The placeholder-to-value mapping lives in memory for one batch only. It is never written to disk or to the log.
- Credentials are different: they are masked irreversibly, as before, and never stored.
- The Acceptance's check "no planted value reaches the model" holds. Checks that a value is absent from the store are replaced by checks that it is present there in its original form.
