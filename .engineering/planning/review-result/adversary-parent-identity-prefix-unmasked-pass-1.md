---
format: aep.planning-md/3
id: review-result:adversary-parent-identity-prefix-unmasked-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: parent-identity-prefix-unmasked (wave 20261008b)'
relations:
- reviews: story:parent-identity-prefix-unmasked
revision: 1
---
## Adversary pass 1: parent-identity-prefix-unmasked (wave 20261008b)

Against `ea8b58e` (base `413de0e`). Verdict: NEEDS-CHANGE. Cases in `tests/parent_prefix_adv.rs`; suite 533 passed, 2 failed (the two cases).

| # | file:line | verdict | origin | finding |
|---|---|---|---|---|
| F1 | src/structured.rs:741 (`owns`) via :79 | NEEDS-CHANGE | introduced | once a prefix turns hex, `owns`/`owner` no longer match evidence written under the base prefix, so those assertions and nodes are never ended |
| F2 | src/structured.rs:94 | NEEDS-CHANGE | introduced | a files path that is not last is followed by `,`, which masking never reads, yet it is written in hex, so a source that works on the base changes identity |
| F3 | website/docs/spec-file.md:317 | CONFIRMED | introduced | the Identity paragraph does not mention the `%x` hex form |

Could not break: other separators and cases, `%3A` after a name, url-password across parts, token rules; `child_prefix`, `child_scope`, `child_of`, `failed_child`, `held`; collisions between hex and literal parts.

```findings
- file: src/structured.rs
  line: 741
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: once a prefix turns hex, owns() no longer matches evidence written under the base prefix, so those assertions and nodes are never ended
- file: src/structured.rs
  line: 94
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a files path that is not last is followed by a comma, which masking never reads, yet it is written in hex, so a source that works on the base changes identity
- file: website/docs/spec-file.md
  line: 317
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the Identity paragraph does not mention the %x hex form a prefix part now takes
```
