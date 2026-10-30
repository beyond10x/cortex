---
format: aep.planning-md/3
id: story:mask-escaped-and-encoded-credentials
kind: story
status: draft
title: The mask catches escaped JSON-quoted, twice-encoded and user-with-@ URL credentials
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
revision: 2
---
## Outcome

The always-on mask catches the three credential shapes wave 20261030a left as ignored cases in
`tests/redact_mask_adv_w30a.rs`, and the two INFEASIBLE notes are either fixed or recorded as
accepted with the reason.

## Acceptance

- `an_escaped_json_quoted_password_is_masked` (`tests/redact_mask_adv_w30a.rs:300`) passes without
  `#[ignore]`: `Log: {\"password\": \"…\"}` leaves no secret in the masked text.
- `a_twice_encoded_separator_is_masked` (`:310`) passes without `#[ignore]`:
  `?next=%252Fapi%253Faccess_token%253D…` leaves no secret.
- `a_url_password_after_a_user_with_an_at_sign_is_masked` (`:320`) passes without `#[ignore]`:
  `ftp://jane@corp.example:<secret>@files.example/x` leaves no secret.
- `a_raw_percent_before_an_encoded_ampersand_keeps_two_keys_apart` (`:272`, `%%26` key merge) and
  `a_masked_key_is_not_masked_again` (`:286`, `%3A` re-mask) either pass without `#[ignore]`, or
  their ignore reason names this story and the INFEASIBLE verdict, and `CHANGELOG.md` says nothing
  about them as fixed.
- `cargo test --test redact_mask_adv_w30a` reports 0 ignored, or only the two INFEASIBLE cases.

## Source

- `review-result:adversary-w30a-redact-pass-1` § 2 (the three pre-existing follow-ups and the two
  INFEASIBLE notes, `src/mask.rs:36`); `review-result:adversary-w30a-redact-pass-2` § 5 ("the known
  follow-ups were not re-reported").
- Wave 20261030a coordinator review of the unit reports.
