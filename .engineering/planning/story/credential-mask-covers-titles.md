---
format: aep.planning-md/3
id: story:credential-mask-covers-titles
kind: story
status: implemented
title: A credential in a document's title or description is masked
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
scope:
- confidence: cited
  path: src/evidence.rs
- confidence: cited
  path: src/mask.rs
- confidence: cited
  path: src/run.rs
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T14:13:37Z", actor: "agent:claude", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-05T14:13:37Z", actor: "agent:claude", revision: 6}
- {from: "active", to: "implemented", at: "2026-10-05T14:48:54Z", actor: "agent:claude", revision: 8, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

A credential in a fetched document's title or description is masked before it reaches the model, the evidence or the store, the same way a credential in its text is.

## Why

Found 2026-10-05 by the implementor of `story:redaction-before-model` (wave 20261005c): `mask::mask` is applied to `text` only, while `title` and `description` also reach the prompt and the evidence payload (`src/run.rs`). Redaction (that story) now covers all three fields; credential masking does not. Pre-existing on `main` at `b0078ce`.

## Work

Apply the credential mask to `title` and `description` wherever it is applied to `text`, and count those masks in the run's `masked` total.

## Acceptance

An e2e case plants a credential-shaped token in a document title and in its description; the stand-in model's prompt, every file under the instance and the evidence hold neither, and `cortex.log` counts both masks.

## The document key too

Also the document key (URL): `mask` runs on `d.text` only (`src/run.rs:158`; the same on `main` at `6352006`), so an `access_token=` in a search-result URL reaches the prompt and the stored evidence identity. Found by `review-result:adversary-redaction-before-model-pass-2`. Masking the key changes the evidence identity of such documents only; keys without a credential are unchanged. The Acceptance extends to a credential-shaped token in a document URL.

## Scope

Landed 2026-10-05 in `ae4b75f` (wave 20261005e).

- **Files:** `src/mask.rs` (`mask_key`, url-password, prefixed names, percent-encoded names), `src/run.rs`, `src/state.rs`, `src/sources.rs` (two lines: child-failure keys masked), `tests/credential_mask.rs` (new, 12 cases), `AGENTS.md`, `website/docs/limits.md`
- **Inferred line, not needed:** `src/evidence.rs` reads the already masked key
- **Review:** `review-result:adversary-credential-mask-covers-titles-pass-1` (7), fixed; coordinator check of the correction
