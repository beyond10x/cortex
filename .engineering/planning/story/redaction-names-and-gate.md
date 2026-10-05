---
format: aep.planning-md/3
id: story:redaction-names-and-gate
kind: story
status: implemented
title: Names, links and credentials are masked and a run refuses what survives
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:redaction-before-model
- depends_on: story:spec-standalone-types
- depends_on: story:structured-source
scope:
- confidence: inferred
  path: src/redact.rs
- confidence: cited
  path: src/run.rs
- confidence: inferred
  path: tests/redaction_gate.rs
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T15:00:26Z", actor: "agent:claude", revision: 8, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-10-05T15:00:26Z", actor: "agent:claude", revision: 9, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-10-05T15:38:30Z", actor: "agent:claude", revision: 11, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Outcome

Personal data is masked to the level an organisation's chat and ticket text needs, and a run that
would still send a protected class to the model fails and writes nothing.

## Work

- Classes added to `story:redaction-before-model`'s: `Url` → `[link]`; `Credential`, a rule set
  covering private keys, chat and code-hosting tokens, cloud keys, model-provider keys, JWTs, bearer
  headers, basic-auth URLs and `secret-word = value` assignments, with an allow-list for
  placeholders and digests; `RareName`: a capitalised word not in the `known_names` files and seen at
  most `rare_limit` times in the batch becomes `[name]`.
- `refuse_if_left`: after masking, a class still detected (for example a phone number with
  person context) fails the run with a named error before any model call or store write.

## Acceptance

`tests/redaction_gate.rs`: a batch containing a phone number with person context that masking does
not remove, under `refuse_if_left: [Phone]`, fails with the class named, makes no model call, and
leaves `ekr head` at the revision from before the run.

Masking of each added class (`Url`, `Credential`, `RareName`) is held by unit tests in
`src/redact.rs`, one per class, each asserting the planted value is absent from the output.

## Depends on

`story:spec-standalone-types` (the types), `story:redaction-before-model` (both edit
`src/redact.rs`), `story:structured-source` (both edit `src/run.rs`).

## Scope

Landed 2026-10-05 in `f3bbbf1` (wave 20261005f, merged `25a8cb9`).

- **Files:** `src/redact.rs`, `src/run.rs`, `tests/redaction_gate.rs` (new, 5 cases), `Cargo.toml` and `Cargo.lock` (`unicode-normalization` 0.1.25), `AGENTS.md`, `website/docs/limits.md`, `website/docs/spec-file.md`
- **Decided during the wave:** placeholders `[Url-n]` and `[Name-n]`, restored before storage (operator decision); `known_names` are names in the spec, not file paths; a refusal is the existing `extraction-failed` outcome; rarity counts only the batch's document texts; known names match case-insensitively after NFC normalisation
- **Documented limits:** a bare host with no path is not a link; a rare name in capitals only is not caught; a refusal in a later batch leaves earlier batches applied
- **Review:** `review-result:adversary-redaction-names-and-gate-pass-1` (9), fixed or documented; coordinator check of the correction

## Operator decision: PII stays out of the model, not out of the store

2026-10-05, operator: "It would be nice if something like an email or otherwise PII would not reach the LLM models - but we still would kind of need to find the data later, so storing it in the brain and being able to search for it is actually fine".

What follows from it (coordinator, same day):

- Redaction applies to what the model sees: text, title, description and the `Source:` line built from the document key. Each distinct value becomes a stable placeholder within the batch (`[Email-1]`, `[Phone-2]`, `[<rule>-1]`).
- Before the extraction document is applied, its placeholders are replaced by the original values. The store, the evidence text and the evidence identity hold the real values, and the brain stays searchable by them.
- A placeholder that cannot be restored stays as written and is counted as `unrestored`.
- The placeholder-to-value mapping lives in memory for one batch only. It is never written to disk or to the log.
- Credentials are different: they are masked irreversibly, as before, and never stored.
- The Acceptance's check "no planted value reaches the model" holds. Checks that a value is absent from the store are replaced by checks that it is present there in its original form.
