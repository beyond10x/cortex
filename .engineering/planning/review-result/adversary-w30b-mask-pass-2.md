---
format: aep.planning-md/3
id: review-result:adversary-w30b-mask-pass-2
kind: review-result
status: active
title: Adversary pass 2 on the masking fix
relations:
- reviews: story:mask-percent-before-encoded-amp
revision: 1
---
unit: mask (wave 20261030b), unit/w30b-mask at bb4835c7, adversary pass 2 (opus), cases added to tests/w30b_mask_adversary.rs (d4e6bfb8)
verdict: CONFIRMED with limits
needs-coordinator: none

10 cases, 5 green, 5 red and ignored; origins from the same cases run at base 7a94960c.

- `%2%26` (and `%6`, `%a`, `%4`, `%1` before `%26`): the `%` and one hex digit of the credential
  stay in the key; introduced; no source shown to write such a value. Known limit.
- `%2&amp;` before a raw separator: same, pre-existing. Known limit.
- `(scheme://user@host:port)addr@host`: the port, `)` and the next local part are masked as a
  password; introduced; no text shown that writes it. Known limit.
- A user name with a raw `@` and a password with `,` or `;`: the password is kept in clear;
  pre-existing. Moved to story:credential-key-spellings.
- `secret_key = <v>`: kept in clear, while the new `access_key` name masks the (not secret) key
  ID; pre-existing. Moved to story:credential-key-spellings.
- Green: 222,220 stop sequences without a separator; each url-password rule masks once;
  `access_key_id`, `has_access_keys`, `access-key-rotation`, `accessKeyId` untouched; idempotent
  over 20k inputs; linear on `%2`, `@` and `access_key` runs.

```findings
- file: src/mask.rs
  line: 44
  category: spec-gap
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a raw percent and one hex digit before %26 leave that percent and digit of the credential in the key
- file: src/mask.rs
  line: 44
  category: spec-gap
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: a raw percent and one hex digit before a raw & or # leave that percent and digit of the credential in the key
- file: src/mask.rs
  line: 116
  category: spec-gap
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: a user name with a raw @ and a password holding , or ; matches neither url-password rule, so the password is kept in clear
- file: src/mask.rs
  line: 116
  category: spec-gap
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the second url-password rule reads a port, a ) and the next address's local part as a password
- file: src/mask.rs
  line: 29
  category: spec-gap
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: secret must be the last word of a name, so secret_key values are kept in clear
```
