---
format: aep.planning-md/3
id: story:test-stand-ins-never-busy
kind: story
status: draft
title: The test harness never fails with Text file busy
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
scope:
- confidence: cited
  path: tests/common/mod.rs
revision: 2
---
## Outcome

The test harness never fails with `Text file busy` (ETXTBSY, os error 26) when it runs a stand-in executable it just wrote.

## Why

Wave 20261005h's gate failed once in `tests/seed_change.rs` (`a_renamed_seed_file_is_a_seed_change`): `cortex: connectors failed: cannot run connectors: Text file busy (os error 26)`, from `tests/common/mod.rs:181`. A test writes a stand-in script and executes it while another thread's forked child still holds the write descriptor; the re-run passed. Log: `~/.cache/cortex-wave-20261005h/int/gate-1-etxtbsy.log`.

## Work

Write each stand-in to a temporary name, close it, then rename it into place before any test executes it; or retry the spawn on ETXTBSY.

## Acceptance

`cargo test --workspace` run 20 times in a loop under load shows no ETXTBSY.
