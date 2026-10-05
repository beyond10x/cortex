---
format: aep.planning-md/3
id: story:ci-runs-task-check
kind: story
status: implemented
title: CI runs task check on every pull request
relations:
- decomposes: epic:first-web-instance
- serves: vision:self-updating-instances
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T08:30:18Z", actor: "agent:claude", revision: 2, decided_on: {"recorded":{"test_result":2}}}
- {from: "proposed", to: "active", at: "2026-10-05T08:30:18Z", actor: "agent:claude", revision: 3, decided_on: {"recorded":{"test_result":2}}}
- {from: "active", to: "implemented", at: "2026-10-05T08:30:18Z", actor: "agent:claude", revision: 4, decided_on: {"recorded":{"test_result":2}}}
---
## Outcome

A pull request to `main` runs `task check` on GitHub Actions and is green or red on its own.

## Work

- `.github/workflows/check.yml`: Rust stable, `go-task`, `ess` 0.52.0 (the pin in
  `spec/ess-inputs.yaml`), and the EKR 0.0.30 release binary from
  `beyond10x/epistemic-knowledge-runtime` (public, release `0.0.30`), passed as `CORTEX_TEST_EKR`.
- The e2e tests already use stand-in `connectors`, `claude` and `systemctl`, so CI needs no secret.

## Acceptance

A PR that breaks `cargo fmt -p cortex-cli` fails the job, and the same PR reverted passes it; both
run URLs recorded as `test_result` evidence.

## Scope

`.github/workflows/check.yml` only (new file).
