---
format: aep.planning-md/3
id: story:ask-body-limit-test-race
kind: story
status: active
title: The over-limit body case of the ask service passes on every run
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

`tests/ask_adversary.rs` `a_body_above_the_body_limit_is_refused_inside_the_event_contract`
passes on every run, and a client that sends a body above the limit reads the refusal.

## Why

`main`'s `Check` run 41082799472 on 7a94960c (the v0.7.0 commit) failed there:
`tests/ask_adversary.rs:981` unwrapped `"read: error decoding response body"`. The same test
passed in the pull request's run and in the local full gate, so it depends on timing: the service
answers before the client has sent the whole body.

## Acceptance

- The service's answer to an over-limit body reaches a client still sending it (or the test asserts
  the contract the service actually keeps, with the reason in a comment).
- The test passes 50 runs in a row under `cargo test --test ask_adversary -- --test-threads 8`.
