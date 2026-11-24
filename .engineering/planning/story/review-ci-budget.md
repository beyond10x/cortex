---
format: aep.planning-md/3
id: story:review-ci-budget
kind: story
status: implemented
title: The test step ends within 120 s and planning-only pull requests skip the build
summary: Findings 27 and 28 of the 0.7.2 review
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T01:53:58Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T01:53:59Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-24T00:21:02Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

The test step of `task check` and of CI ends within 120 s, no test runs longer than 60 s, and a
pull request that changes only planning or prose files passes its required check without building.

## Work

Findings 27 and 28 of the external review of cortex 0.7.2 (2026-11-23):

- 27: the release test step ran `cargo test --locked --workspace` for 304.67 s (30.31 s of it
  compilation) and launched 128 integration-test binaries; no 60 s per-test bound and no 120 s step
  bound is enforced (`Taskfile.yml`, `.github/workflows/check.yml`).
- 28: pull request 80 changed two files under `.engineering/` and its `task check` step ran 3 min 13 s.

- Consolidate the integration tests into one binary per crate (`tests/main.rs` with modules).
- No debug info in test builds, set in `.cargo/config.toml`; `rust-toolchain.toml` pins rustc for
  the local gate and CI.
- The runner kills a test over 60 s, fails the step over 120 s and prints the ten slowest tests,
  locally and in CI.
- `check.yml` keeps the required check and exits 0 early when every changed path is planning or
  prose (`.engineering/`, `*.md` outside `website/`), with a test case for both sides of the list.

## Acceptance

`task check` on this change prints the test step's wall time under 120 s and its ten slowest
tests; one test binary per crate under `tests/`; a planning-only pull request's `Check` run
skips build and test and is green.
