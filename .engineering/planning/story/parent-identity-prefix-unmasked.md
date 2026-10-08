---
format: aep.planning-md/3
id: story:parent-identity-prefix-unmasked
kind: story
status: implemented
title: No structured parent identity prefix reads as a credential to masking
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
scope:
- confidence: inferred
  path: src/spec.rs
- confidence: cited
  path: src/structured.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T13:07:55Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-08T13:07:55Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-10-08T13:59:01Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Outcome

No part of a structured parent's identity prefix (the adapter, the parent operation, a `files` path or glob) makes credential masking read the identity as an assigned value, so the seen state keeps every parent identity and an unchanged record is not applied again.

## Why

Found 2026-10-07 while fixing adversary finding A1 of `story:structured-children` (wave 20261007c), `pre-existing`: masking reads a credential name followed by `:` and six or more characters as an assignment. A parent identity is `<adapter>:<operation>:<id>` (or a `files` path), so an operation or path ending in such a name (for example `dir:vault.secret:<id>`) is masked, `SeenState::load` drops the key, and unchanged records are applied again on every run. A1 fixed the same class for a child's own parts (`%x` and hex, `src/structured.rs`), not for the parent prefix.

## Decided (2026-10-08)

A part of the parent prefix (the operation, a `files` path, the glob) is written in hex only when masking would change `<part>:<digest>`, the test `child_prefix` already applies to a child's operation (`src/structured.rs`). Identities of sources no masking touches do not change, so no migration; a source whose prefix masking changed gets new identities once (its records are applied again on every run today). Hex for every part was not taken: it changes every working identity. Refusing such a spec at load was not taken: existing instances must keep loading (`decision-blocker:parent-identity-prefix-encoding`).

## Acceptance

A structured source whose operation, `files` path or glob ends in a credential name masking reads (for example `vault.secret`) keeps every parent identity in its seen state, and a second run over unchanged records applies 0 documents; a source with no such part keeps the identities it has on `main` today, byte for byte (a test pins one).
