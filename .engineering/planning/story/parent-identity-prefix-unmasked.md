---
format: aep.planning-md/3
id: story:parent-identity-prefix-unmasked
kind: story
status: draft
title: No structured parent identity prefix reads as a credential to masking
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
scope:
- confidence: inferred
  path: src/spec.rs
- confidence: cited
  path: src/structured.rs
revision: 3
---
## Outcome

No part of a structured parent's identity prefix (the adapter, the parent operation, a `files` path or glob) makes credential masking read the identity as an assigned value, so the seen state keeps every parent identity and an unchanged record is not applied again.

## Why

Found 2026-10-07 while fixing adversary finding A1 of `story:structured-children` (wave 20261007c), `pre-existing`: masking reads a credential name followed by `:` and six or more characters as an assignment. A parent identity is `<adapter>:<operation>:<id>` (or a `files` path), so an operation or path ending in such a name (for example `dir:vault.secret:<id>`) is masked, `SeenState::load` drops the key, and unchanged records are applied again on every run. A1 fixed the same class for a child's own parts (`%x` and hex, `src/structured.rs`), not for the parent prefix.

## Open

Two ways, a decision for the story: write the prefix in hex as children do (changes existing stores' identities, so a migration is needed), or refuse such a spec in `spec::load_given` (existing instances must keep loading).
