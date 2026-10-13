---
format: aep.planning-md/3
id: story:dependency-pins-current
kind: story
status: implemented
title: Every pin outside spec/ names the current release
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
scope:
- confidence: inferred
  path: .github/workflows/check.yml
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: examples
- confidence: inferred
  path: src/main.rs
- confidence: inferred
  path: tests
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T19:21:16Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-13T19:21:17Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-13T19:53:17Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

Every version cortex pins outside `spec/` names the current release: EKR 0.0.33, aep 0.71.2, and
the CI tools and actions at their newest releases, with checksums and commit SHAs from the
releases themselves.

## Why

`.github/workflows/check.yml` pins `EKR_VERSION` 0.0.32 and `AEP_VERSION` 0.68.0;
`cortex setup` defaults to EKR 0.0.32 (`src/main.rs`); `examples/*.yaml` name 0.0.32; `AGENTS.md`
names the 0.0.32 test binary. EKR 0.0.33 and aep 0.71.2 are released (2026-10-13).

## Work

1. EKR 0.0.33: `check.yml` `EKR_VERSION`, `cortex setup`'s default, `examples/*.yaml`,
   `AGENTS.md`'s end-to-end test binary and its build command; any test fixture that holds the
   pinned default.
2. aep 0.71.2: `check.yml` `AEP_VERSION` and `AEP_SHA256`; `aep plan artifact validate` with it.
3. The CI tool pins (`task` runner, Rust toolchain, actions pinned by SHA): each checked against
   its newest release and moved when one is newer, except where `AGENTS.md` ties a pin to the
   sibling repositories (the docs-site pins).
4. Connectors: nothing in the repository pins a `connectors` version; recorded as checked.
   cortex has no eventlog dependency; recorded as checked.

## Acceptance

- `grep -rn 0.0.32` over `src/ tests/ examples/ .github/ AGENTS.md README.md` finds no pin left
  (history in `CHANGELOG.md` and upstream-facts rows that cite the version a fact was measured on
  stay).
- `cargo test -p cortex-cli` passes with `CORTEX_TEST_EKR` set to an `ekr` 0.0.33 binary.
- The `Check` workflow on the wave's pull request builds `ekr` 0.0.33 and passes.
