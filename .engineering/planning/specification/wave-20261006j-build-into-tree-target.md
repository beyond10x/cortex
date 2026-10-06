---
format: aep.planning-md/3
id: specification:wave-20261006j-build-into-tree-target
kind: specification
status: implemented
title: 'Wave 20261006j: build into the tree''s own target/'
revision: 5
transitions:
- {from: "draft", to: "in_review", at: "2026-10-06T20:07:13Z", actor: "agent:claude", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-06T20:07:13Z", actor: "agent:claude", revision: 3}
- {from: "approved", to: "implemented", at: "2026-10-06T20:35:52Z", actor: "agent:claude", revision: 5}
---
## Wave 20261006j: build into the tree's own target/

Opened 2026-10-06 by the coordinating session for conductor dispatch DSP-20261006-04 (DEC-20261006-02).

**Approval:** the operator approved every wave up front on 2026-10-05 and again on 2026-10-06 ("do it. dispatch next waves. i approved them").

## Units

| unit | story |
|---|---|
| a | `story:build-into-tree-target` (coordinator-written: two lines of build configuration) |

## Commits approval authorises

The unit commit through `b10x-gates bot`, the closing store commit, the pull request and its merge. No release.

## Outcome

Closed 2026-10-06. The unit (`01a4931`) passed `task check` in the pull request CI, run 37524133336 (PR #48), building into the job tree's own `target/`.
