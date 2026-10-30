---
format: aep.planning-md/3
id: review-result:adversary-w30c-keys-pass-2
kind: review-result
status: active
title: Adversary pass 2 on the credential key spellings fix
relations:
- reviews: story:credential-key-spellings
revision: 1
---
unit: keys (wave 20261030c), unit/w30c-keys at dc3e89db, adversary pass 2 (opus), cases added to tests/w30c_keys_adversary.rs (5189e281)
verdict: NEEDS-CHANGE
needs-coordinator: none

10 cases, 4 green, 6 red and ignored.

- Re-key: 0 of 49 stored keys and 0 of 57 prefix parts change against 6aaa4800. Of 4,352
  generated inputs only contrived ones with a value after `=`, `: `, a flag or an element change
  (280 keys, 15 joins, 5 parts); plain paths, searches, globs and operations: 0.
- The pass-1 precondition changed from assert_ne to assert_eq pins the 6aaa4800 identity.
- Every spelling 6aaa4800 masked is masked as it was; the two assignment rules mask a value once;
  linear time at 1 MiB.
- Red: `DB_PASS: v` and `{"DB_PASS":"v"}` stay in clear; a quoted flag value before punctuation is
  masked again by a second mask; bracketed metavars, `(string)` types, a title-case word and a
  quoted `$VAR` after a flag are masked; a JSON-escaped quoted flag value stays in clear and
  `--flag="a b c"` keeps every word after the first (the latter also at 6aaa4800).

```findings
- file: src/mask.rs
  line: 47
  category: spec-gap
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: ENV_PASS reads only =, so DB_PASS in a YAML map, JSON or escaped JSON is left in clear
- file: src/mask.rs
  line: 98
  category: spec-gap
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: bracketed metavars, (string) types, title-case words and quoted $VARs after a flag are masked
- file: src/mask.rs
  line: 7
  category: spec-gap
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a quoted flag value masked before punctuation is masked again by a second mask, so masking is not idempotent
- file: src/mask.rs
  line: 77
  category: spec-gap
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a JSON-escaped quoted flag value is left in clear, and a quoted value after --flag= keeps every word after the first
```
