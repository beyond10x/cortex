---
format: aep.planning-md/3
id: story:ignored-adversary-limits
kind: story
status: active
title: Ignored adversary cases of wave 20261030a are fixed or documented limits
tags:
- release-check-0-7-0
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T15:47:08Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-30T15:47:09Z", actor: "human:timo", revision: 3}
---
## Outcome

Each limit below, merged in wave 20261030a as an ignored adversary case, is either fixed with its
case un-ignored, or stays documented in the CHANGELOG's known limits.

## Why

Release check of 0.7.0 by the chef session (2026-10-30): each ignored case needs a story id.

- `tests/rekey_adversary_pass2.rs:146`: a redaction rule that rewrites structured ids of a source
  with seen records is not refused, though it re-keys them.
- `tests/rekey_adversary_pass2.rs:363`: the whole record id `$` and a member literally named `$`
  are not told apart (no known provider sends such a member).
- `tests/partial_counts_adversary.rs:184`: a run that failed before it applied a batch drops the
  tokens its failed answers reported (`website/docs/limits.md`).

## Acceptance

- Each case runs without `#[ignore]` and passes, or names this story in its reason and has a
  known-limits line in the CHANGELOG.
