---
format: aep.planning-md/3
id: story:credential-key-spellings
kind: story
status: implemented
title: A credential under a common key spelling is masked or refused
tags:
- downstream-request
- security
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T15:57:08Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-30T16:50:18Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-10-30T18:35:38Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Outcome

A credential written under a common key spelling is masked before it is stored and before a model
sees it, or `refuse_if_left: [Credential]` refuses the batch.

## Why

Reported by a downstream instance on 2026-10-30 against v0.6.1, and measured again against v0.7.0
with a policy of `classes: [Credential]`, `rules: []`, `refuse_if_left: [Credential]`, run as
mask, scrub, replace, left. Of 25 spellings, 13 keep their value in the store and in the model's
view, and `left` is empty for each, so `refuse_if_left` refuses none of them:

- a keyword followed by `_KEY`: `SECRET_KEY=`, `DJANGO_SECRET_KEY=`, `AWS_SECRET_KEY=`;
- a keyword glued to a prefix or cut short: `PGPASSWORD=`, `DB_PASS=`;
- keys inside escaped JSON: `{\"db_password\":\"…\"}`, the escaped AWS secret access key;
- a key wrapped in markup: `` `DB_PASSWORD`: ``, `*AWS_SECRET_ACCESS_KEY*: `, `**api_key**: `;
- an XML element: `<db_password>…</db_password>`;
- a space-separated flag: `--db-password …`;
- a value that starts with `%2B` after `aws_secret_access_key=`.

Masked as expected: lower, upper and mixed case keys, `key: `, spaced `=`, quoted values, JSON
`"k": "v"`, URL query strings, `%3F`/`%3D` encoding, a YAML value on the next line.

## Acceptance

- Each of the 13 spellings above, with a fake value assembled at run time, keeps no part of the
  value in the scrubbed text or in the batch's replacement.
- Each of the 12 spellings already masked stays masked.
- A spelling the masker still misses is named by `left` when `refuse_if_left` lists `Credential`.

## Measured against the wave 20261030b mask fix

The same probe against `unit/w30b-mask` (b39be845): 12 of 25 still leak. The escaped-JSON
`db_password` is masked there; the escaped AWS secret access key and the `%2B` value still leak.

After `access[_-]?key` joined the always-on name list (`unit/w30b-mask` at bb4835c7), 10 of 25
still leak: the three markup-wrapped keys, the XML element, `--db-password`, `SECRET_KEY=`,
`DJANGO_SECRET_KEY=`, `AWS_SECRET_KEY=`, `PGPASSWORD=` and `DB_PASS=`. Both escaped-JSON keys and
the `%2B` value are masked there.

## Scope in wave 20261030c

Taken up first after 0.7.1 at the chef session's request (its decision D286, 2026-10-30): fix the
10 spellings that still leak in 0.7.1 and the two cases 0.7.1 left ignored under this story in
`tests/w30b_mask_adversary.rs` (`secret_key = <v>`, and a URL password holding `,` or `;` after a
user name with a raw `@`), remove those two ignores, and release. One integration branch, one pull
request, one release.

The 25 spellings become a committed test, `tests/credential_key_spellings.rs`, each with a fake
value assembled at run time, checked both through `mask::mask` alone and through a policy of
`classes: [Credential]`, `refuse_if_left: [Credential]`.
