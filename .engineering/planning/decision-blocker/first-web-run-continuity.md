---
format: aep.planning-md/3
id: decision-blocker:first-web-run-continuity
kind: decision-blocker
status: cleared
title: 'The 7-day run can reach 6 of the 7 search runs its acceptance needs: keep, patch or restart'
relations:
- blocks: story:first-web-instance
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-08T13:09:17Z", actor: "human:timo", revision: 3}
---
## What stops the story

The run can complete at most 6 timer-started search runs inside days 1 to 7 (2026-10-07 to 2026-10-13): the news run of 2026-10-07 exited 0, the one of 2026-10-08 exited 1 (`tavily.websearch.search: timeout at admission`), and 5 remain. Acceptance 0 needs 7. Whether the 2026-10-14 00:30 run counts is not stated: the Acceptance says "at least 7 timer-started search runs" and "read on day 8: 2026-10-14"; the Outcome says "for 7 days".

The 2026-10-07 news run stopped at 7 of 15 documents on `extraction-value-mismatch … Release.release_date`; its cause and fix are in `story:prompt-names-value-kinds` and shipped in 0.2.0 with the EKR 0.0.32 pin. The running instance is cortex 0.1.0 with EKR 0.0.30.

## Options

| option | does | acceptance |
|---|---|---|
| A | keep the run as is | passes only if the day-8 run counts and no further search run fails |
| B | install a newer cortex and EKR into the running instance | breaks the one-binary rule (Concurrency) and acceptance 1 |
| C | release a cortex that retries `timeout at admission` once, install it with EKR 0.0.32, and start the 7 days again | day 1 moves to the first run on the new binary |

Recommended: C.

## What would clear it

A choice of A, B or C, written into the story's run section.

## Decided (2026-10-08)

Option C. cortex 0.2.1 retries a Connectors invocation once when it answers `timeout` at `admission`, and retries nothing else. It is installed with EKR 0.0.32, and the `connectors` binary is copied to a fixed path the units name, its version and sha256 recorded in the story, so the 7 days run on one fixed set of binaries. Day 1 is the first timer run on them. The first run's evidence stays in the story.
