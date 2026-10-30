---
format: aep.planning-md/3
id: review-result:adversary-w30c-keys-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the credential key spellings fix
relations:
- reviews: story:credential-key-spellings
revision: 1
---
unit: keys (wave 20261030c), unit/w30c-keys at e66d0af8, adversary pass 1 (opus), cases in tests/w30c_keys_adversary.rs (f0c7167c)
verdict: NEEDS-CHANGE
needs-coordinator: none

11 cases, 2 green, 9 red and ignored.

- Structured re-key: 14 of 57 plausible structured prefix parts change their hex form
  (`get_secret_key`, `onepassword`, `**/*secret*`, `…_pass`); `earlier` misses a child operation
  newly in hex and a prefix partly in hex at 6aaa4800, so values applied then are never ended and
  stand beside the re-read ones.
- Stored keys: 3 of 49 plausible keys change (`render_pass%3D…`, `boarding_pass=`, `season_pass=`),
  none a credential; 3 keys mask to 2 distinct, so two documents merge.
- `…_pass` names in code and settings are masked (`let shadow_pass = …`, `first_pass:`).
- A namespaced or CDATA password element is left in clear; prose after a `<password>` placeholder
  is masked up to a later closing tag.
- A quoted flag value with spaces is masked to its first space only; metavars, `$VARS`,
  `--secret id=…,src=…` and an accented lower-case word after a flag are masked.
- Green: idempotence over 20,000 seeded mixes; linear time to 4 MiB.

```findings
- file: src/structured.rs
  line: 827
  category: spec-gap
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a structured prefix part newly masked changes the record id while earlier misses the old form, so values applied before are never ended and are duplicated by the re-read
- file: src/mask.rs
  line: 33
  category: spec-gap
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the lower-case _pass suffix masks non-credential names and through state.rs:107 drops stored keys and merges two documents into one
- file: src/mask.rs
  line: 48
  category: spec-gap
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a namespaced or CDATA password element stays in clear, and prose after a password placeholder is masked up to a later closing tag
- file: src/mask.rs
  line: 51
  category: spec-gap
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a quoted flag value is masked to its first space only, and metavars, variables, --secret mounts and accented words after a flag are masked
```
