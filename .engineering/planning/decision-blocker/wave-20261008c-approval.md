---
format: aep.planning-md/3
id: decision-blocker:wave-20261008c-approval
kind: decision-blocker
status: cleared
title: Wave 20261008c toward 1.0 waits for approval
relations:
- blocks: story:release-1-0
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-08T14:08:18Z", actor: "human:timo", revision: 3}
---
## Question

Approve wave 20261008c toward 1.0: plan-store moves, the 1.0 changelog summary, and an install-and-quickstart rehearsal from a released tarball in an empty home. The 1.0.0 version bump, tag and Release wait for the 7-day run of `story:first-web-instance`.

## Options

- A: the wave as above. One integration branch, one pull request.
- B: A plus the 1.0.0 version bump merged to `main` now. `main` then prints `cortex 1.0.0` for a week before the tag.
- C: store moves only.

Recommendation: A.

## Decided (2026-10-08)

Option A: the wave runs as proposed, on one integration branch with one pull request. The rehearsal
runs only in an empty home with its own instance name and touches no running instance.
