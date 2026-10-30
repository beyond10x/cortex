---
format: aep.planning-md/3
id: review-result:release-check-v0-7-0
kind: review-result
status: active
title: Release check of v0.7.0 by the chef session
relations:
- reviews: story:mask-quoted-credentials
- reviews: story:unit-names-unique
- reviews: story:ask-service-bounds
revision: 1
---
unit: release v0.7.0 of cortex (wave 20261030a), checked by the chef session's verify-release.sh on 2026-10-30
verdict: NEEDS-CHANGE
needs-coordinator: none

1. `main`'s `Check` run 41082799472 on 7a94960c (the v0.7.0 commit) concluded failure in
   `task check`: `tests/ask_adversary.rs:981`
   `a_body_above_the_body_limit_is_refused_inside_the_event_contract` unwrapped
   `"read: error decoding response body"` (read from the run's failed log by b10x-cortex). The
   pull request's run and the local full gate passed the same test.
2. The release diff adds `#[ignore]` to 9 adversary cases:
   `tests/partial_counts_adversary.rs:184`, `tests/redact_mask_adv_w30a.rs:272`, `:286`, `:300`,
   `:310`, `:320`, `tests/rekey_adversary_pass2.rs:146`, `:363`,
   `tests/w30a_cli_adversary.rs:221`.
   - `redact_mask_adv_w30a.rs:272` is a credential-masking defect: fixed in 0.7.1.
   - `w30a_cli_adversary.rs:221` is an instance taking over another instance's live unit: fixed
     in 0.7.1.
   - Each remaining ignore names a filed story id in its reason and has a known-limits line in
     the CHANGELOG.

```findings
- file: tests/ask_adversary.rs
  line: 981
  category: false-positive
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: main's Check failed on a timing-dependent body-limit case the pull request's run passed
- file: tests/redact_mask_adv_w30a.rs
  line: 272
  category: spec-gap
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: a raw % before %26 swallows the rest of the query into the masked value; merged ignored
- file: tests/w30a_cli_adversary.rs
  line: 221
  category: spec-gap
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: across two homes an instance takes over another instance's live systemd unit; merged ignored
- file: tests/rekey_adversary_pass2.rs
  line: 146
  category: missing-scenario
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: ignored case without a story id or a known-limits line
- file: tests/partial_counts_adversary.rs
  line: 184
  category: missing-scenario
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: ignored case without a story id or a known-limits line
```
