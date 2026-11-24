---
format: aep.planning-md/3
id: story:aep-pin-0-80
kind: story
status: active
title: Pin AEP 0.80.2
summary: The planning store is in AEP 0.80.2's format and CI validates it with 0.80.2
relations:
- serves: vision:self-updating-instances
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T00:17:41Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T00:17:41Z", actor: "human:timo", revision: 3}
---
## Outcome

Cortex's planning store is in the format AEP 0.80.2 writes and validates, and CI validates it with
AEP 0.80.2.

## Work

- Move the CI pin `AEP_VERSION` 0.75.0 to 0.80.2 with the sha256 from the release's `SHA256SUMS`
  (`.github/workflows/check.yml:79-80`), and every other place that names the AEP version.
- Read the AEP CHANGELOG 0.76 to 0.80.2 for planning-store format changes and migrate the store with
  AEP's own migration command if one exists (never by hand where a command exists); `aep plan
  artifact validate` passes with 0.80.2.
- Mark the stories already merged as implemented, with evidence: rare-name-keeps-store-types (#114),
  postgres-tests-one-container (#113), review-ci-budget (#95), quality-missing-verdicts (#112),
  extract-then-apply (#115), ekr-pin-0-0-43 (#116), ess-pin-0-98 (#117), and any other story whose
  PR has merged but whose status is still active (list them from the store).

## Acceptance

CI validates the store with AEP 0.80.2; `aep plan artifact validate` exits 0 locally with 0.80.2; no
story whose PR has merged is still active.
