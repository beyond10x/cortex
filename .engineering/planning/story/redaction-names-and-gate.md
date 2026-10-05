---
format: aep.planning-md/3
id: story:redaction-names-and-gate
kind: story
status: draft
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
revision: 6
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

`src/redact.rs`, `src/run.rs`, `tests/redaction_gate.rs` (new).
