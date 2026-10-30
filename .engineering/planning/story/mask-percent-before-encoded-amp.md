---
format: aep.planning-md/3
id: story:mask-percent-before-encoded-amp
kind: story
status: active
title: A raw % before an encoded & keeps two query keys apart in the mask
tags:
- release-check-0-7-0
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T15:47:08Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-30T15:47:08Z", actor: "human:timo", revision: 3}
---
## Outcome

The always-on credential mask keeps two query keys apart when a raw `%` stands before an encoded
`&` (`%26`), so the mask covers only the credential's value and the rest of the query is kept.

## Why

Release check of 0.7.0 by the chef session (2026-10-30): `tests/redact_mask_adv_w30a.rs:272`
(`a_raw_percent_before_an_encoded_ampersand_keeps_two_keys_apart`) was merged `#[ignore]`d in wave
20261030a. It is a credential-masking defect and ships in 0.7.1. The same file holds four more
ignored masking cases (`:286`, `:300`, `:310`, `:320`), each fixed here if the fix is small, else
named with a story id in its ignore reason and a known-limits line in the CHANGELOG.

## Acceptance

- `:272` runs without `#[ignore]` and passes; it fails with the fix reverted.
- Every case still ignored in that file names a story id in its reason.
