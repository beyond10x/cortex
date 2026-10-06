---
format: aep.planning-md/3
id: story:test-stand-ins-never-busy
kind: story
status: implemented
title: The test harness never fails with Text file busy
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
scope:
- confidence: cited
  path: tests/common/mod.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T01:11:25Z", actor: "agent:claude", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-06T01:11:25Z", actor: "agent:claude", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-06T01:53:06Z", actor: "agent:claude", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

The test harness never fails with `Text file busy` (ETXTBSY, os error 26) when it runs a stand-in executable it just wrote.

## Why

Wave 20261005h's gate failed once in `tests/seed_change.rs` (`a_renamed_seed_file_is_a_seed_change`): `cortex: connectors failed: cannot run connectors: Text file busy (os error 26)`, from `tests/common/mod.rs:181`. A test writes a stand-in script and executes it while another thread's forked child still holds the write descriptor; the re-run passed. Log: `~/.cache/cortex-wave-20261005h/int/gate-1-etxtbsy.log`.

## Work

Write each stand-in to a temporary name, close it, then rename it into place before any test executes it; or retry the spawn on ETXTBSY.

## Acceptance

`cargo test --workspace` run 20 times in a loop under load shows no ETXTBSY.

## Scope

Landed 2026-10-06 in `335f41c` (wave 20261006b, merged `d7ace25`).

- **Files:** `tests/common/mod.rs` (a child `/bin/sh` writes and chmods each stand-in), `tests/stand_ins.rs` (new: a 16-thread probe and a check that every stand-in goes through `executable`)
- **Measured:** the base harness failed the probe in 3 of 3 runs (9, 7, 7 of 2400 busy); the story's suggested rename failed 3 of 3 (4, 4, 2); the fix passed 20 of 20. `cargo test --workspace` under `stress --cpu 16`: 20 of 20 runs exit 0, 339 passed each.
- **Review:** a test-harness fix; coordinator review, no adversary pass
