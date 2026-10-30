---
format: aep.planning-md/3
id: story:unit-takeover-across-homes
kind: story
status: implemented
title: An instance never takes over another home's unit of the same name
tags:
- release-check-0-7-0
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T15:47:08Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-30T15:47:08Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-30T16:33:15Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Outcome

An instance never takes over a systemd unit of an instance in another home whose name maps to the
same unit name (`a-b/c` and `a/b-c` both give `cortex-a-b-c`).

## Why

Release check of 0.7.0 by the chef session (2026-10-30): `tests/w30a_cli_adversary.rs:221` was
merged `#[ignore]`d in wave 20261030a. Across two homes, `Systemd::foreign` asks about the
requesting instance, so instance `a-b/c` takes over the live unit of instance `a/b-c`.

## Acceptance

- `:221` runs without `#[ignore]` and passes; it fails with the fix reverted.
- `create`, `update` and `adopt` refuse, naming the unit and the home that holds it.
