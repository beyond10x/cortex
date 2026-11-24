---
format: aep.planning-md/3
id: story:ess-pin-0-98
kind: story
status: implemented
title: Pin ESS 0.98.0
summary: Cortex specifies and checks with ESS 0.98.0; conformance runs real scenario documents
relations:
- serves: vision:self-updating-instances
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T23:16:50Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T23:16:51Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-24T00:20:46Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

Cortex specifies, generates and checks with ESS 0.98.0, and its conformance steps run the real
scenario documents through cortex's own loader instead of a placeholder spec.

## Work

- Move the ESS pin 0.77.0 to 0.98.0: the CI workflow (`ESS_VERSION`, `ESS_SHA256`), `AGENTS.md`, and
  wherever else the version is named. Read the ESS CHANGELOG from 0.78 to 0.98 for breaking changes
  first.
- ESS 0.93 or later refuses `spec/components.yaml`; bring the specification to the form it accepts.
- Regenerate `generated/` and `spec/suite.json` (`task generate`); `task drift` and `task compat`
  pass. A spec change the new toolchain forces is acknowledged in `spec/compat-ack.json` only when it
  is a real contract change; a change of generated shape alone is not.
- Review finding 26 (the rest after #102): 88 generated conformance steps still pass a fixed
  placeholder instance spec to the generated behaviour. Give the scenarios real instance documents
  (ESS `example:`, available from 0.78), fix Decimal values written as numbers in scenarios, and
  run each scenario document through cortex's real spec loader so a format the CLI refuses fails the
  suite.

## Acceptance

`task check` is green with ESS 0.98.0 locally and on CI; no conformance step uses a placeholder
spec (a test counts them: 0); a scenario whose document the CLI refuses fails the suite (a test
shows it red with a refused format and green with the real one).
