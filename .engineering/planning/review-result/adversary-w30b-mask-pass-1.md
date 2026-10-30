---
format: aep.planning-md/3
id: review-result:adversary-w30b-mask-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the masking fix
relations:
- reviews: story:mask-percent-before-encoded-amp
revision: 1
---
unit: mask (wave 20261030b), unit/w30b-mask at b39be845, adversary pass 1 (opus), cases in tests/w30b_mask_adversary.rs (40ee6d8b)
verdict: NEEDS-CHANGE
needs-coordinator: none

1. `a_percent_and_one_hex_digit_before_a_percent_stays_in_a_key_value`: red. A raw `%` plus one
   hex digit before another `%` (`%2%41`, `%a%41`, `%2%x`, `%252%41`) ends the key value; the rest
   of the credential is stored in the key and kept in a log line. Base 7a94960c masked it whole.
2. `a_url_with_a_user_and_a_port_before_a_comma_and_an_address_is_kept`: red. A URL with a user
   and a port before `,admin@…` has its port and the address's local part masked as a password.
3. Re-key: a key stored by 0.7.0 for a `%%26` URL matches no new key, so each such document is
   read once more after the upgrade (measured with `SeenState::load` and `wants`).
4. Idempotence over 20k generated inputs (400k in a probe): green.
5. Linear time up to 4 MB: green; two shapes 25-50 times slower than base.
No new case found where two documents share a key.

```findings
- file: src/mask.rs
  line: 42
  category: spec-gap
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: key_unit! ends a key value at a raw % plus one hex digit before another %, so the rest of the credential is stored and logged in the clear
- file: src/mask.rs
  line: 107
  category: spec-gap
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a raw @ in the url-password user name masks the port and the next address's local part of scheme://user@host:port,addr@host
```
