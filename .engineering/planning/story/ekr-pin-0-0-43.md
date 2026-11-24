---
format: aep.planning-md/3
id: story:ekr-pin-0-0-43
kind: story
status: implemented
title: Pin EKR 0.0.43
summary: Cortex runs against EKR 0.0.43 and reads store outcomes by name
relations:
- serves: vision:self-updating-instances
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T21:01:39Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T21:01:39Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-24T00:20:45Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

Cortex runs against EKR 0.0.43 and tells a store outcome by EKR's own name and exit status, not by
its English text.

## Work

- Move the EKR pin 0.0.34 to 0.0.43: the CI workflow's `EKR_VERSION`, the `cortex setup` default
  (`src/main.rs`), `AGENTS.md`, the docs pages and the examples that name it, and every test or
  stand-in that names the version.
- An unseeded store: read `ekr.views.NotSeeded` (exit 2) instead of matching "has no seed".
- An unknown publication outcome: read `publication-outcome-unknown` (exit 3) instead of the
  message `publication outcome is unknown` (`src/ekr.rs` `outcome_unknown`).
- A web page's evidence is `!Url`, which EKR 0.0.43 retains (finding 12 of the 0.0.42 review),
  instead of `!HumanStatement {identity: <url>}`; the `AGENTS.md` upstream-limit row goes.
- Every other change between EKR 0.0.34 and 0.0.43 that breaks a cortex test is fixed in cortex.

## Acceptance

`task check` is green with `CORTEX_TEST_EKR` set to an `ekr` 0.0.43 built from tag `0.0.43`; a test
shows each named outcome is read from the exit status and refusal name with the English message
changed; a fetched web page is stored as `!Url` evidence.
