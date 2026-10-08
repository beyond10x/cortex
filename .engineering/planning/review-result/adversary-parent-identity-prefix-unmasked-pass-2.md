---
format: aep.planning-md/3
id: review-result:adversary-parent-identity-prefix-unmasked-pass-2
kind: review-result
status: active
title: 'Adversary pass 2: parent-identity-prefix-unmasked (wave 20261008b)'
relations:
- reviews: story:parent-identity-prefix-unmasked
revision: 1
---
## Adversary pass 2: parent-identity-prefix-unmasked (wave 20261008b)

Against `4e71a41`. Verdict: NEEDS-CHANGE. Cases in `tests/parent_prefix_adv2.rs`; suite 536 passed, 4 failed (the four cases). Pass 1 had 3 findings (all fixed in `4e71a41`); pass 2 has 3, none carried.

| # | file:line | verdict | origin | finding |
|---|---|---|---|---|
| G1 | src/structured.rs:89 | NEEDS-CHANGE | pre-existing | when two prefix parts each end in a credential name, no single hex lowers the mask count, so the greedy loop stops and the identity stays masked |
| G2 | src/structured.rs:928 | NEEDS-CHANGE | introduced | `earlier` makes old identities owned but `unsettled` and `held` name only new ones, so the first run after the prefix turns hex retracts values of records it did not apply |
| G3 | src/structured.rs:796 | INFEASIBLE | introduced | `owns` accepts anything under an earlier child prefix with no parent part, so a hexed source claims the current identities of a source whose operation is `<op>/<child op>` |

Could not break: loop termination, hex parts matching with neighbours, unmasked prefixes unchanged from base, `earlier` overlaps, plain connectors spec check, child prefixes on a hexed prefix.

```findings
- file: src/structured.rs
  line: 89
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: when two prefix parts each end in a credential name, no single hex lowers the mask count, so the greedy loop stops and the identity stays masked
- file: src/structured.rs
  line: 928
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: earlier makes old identities owned but unsettled and held name only new ones, so the first run after the prefix turns hex retracts values of records it did not apply
- file: src/structured.rs
  line: 796
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: owns accepts anything under an earlier child prefix with no parent part, so a hexed source claims the current identities of a source whose operation is <op>/<child op>
```
