---
format: aep.planning-md/3
id: review-result:cb3-gaps-acceptance-round-1
kind: review-result
status: active
title: 'Plan critic (acceptance), round 1: cb3 gap stories'
relations:
- reviews: story:structured-children
- reviews: story:extraction-supersedes
- reviews: story:postgres-run-undo
- reviews: story:record-field-lookups
revision: 1
---
needs-revision

- story:structured-children — the clause "a renamed project keeps one node with the old path as an alias" names no before state or trigger (a second import after the rename), so it reads the same on a fixture where nothing was renamed, and it is the fourth independent outcome in one sentence — .engineering/planning/story/structured-children.md:32 (cortex store, branch plan/cb3-gaps)
- story:record-field-lookups — the Outcome promises "pick the first non-empty of several text fields", but the acceptance checks only the id-to-name lookup and the `<@id>` mention, so the story can close with that capability untested — .engineering/planning/story/record-field-lookups.md:21 against :30 (cortex store, branch plan/cb3-gaps)
- story:read-answers-name-types — the Outcome has `search`, `describe_node`, `expand`, `timeline` and `explain` carry names for node types, edge types and property ids, but the acceptance checks only type ids in a `search` answer, and "a type read" names no operation — .engineering/planning/story/read-answers-name-types.md:24 against :33 (EKR store, ekr-wx-int)
- story:explain-bounds-documents — the Outcome promises "a way to read the rest" and the cited span or a window around it, but the acceptance checks only `truncated: true` and the full length, so the story can close with no way to read the remainder and without the cited fact inside the returned bytes — .engineering/planning/story/explain-bounds-documents.md:20 against :29 (EKR store, ekr-wx-int)
- story:served-channels-by-audience — the acceptance demands that a query for an excluded channel's message "answers nothing on the deployment", while the Open section leaves removal of the text already in the store at revision 547 as "a separate operator decision", so the story cannot honestly close until that decision is made — .engineering/planning/story/served-channels-by-audience.md:22 against :26
- story:served-channels-by-audience — the acceptance joins two independent outcomes with a semicolon (the spec lists each channel with its class; an excluded channel's message is not answered), so one can pass while the other fails, and neither "class", "known message" nor the query tool is named — .engineering/planning/story/served-channels-by-audience.md:22

Read: 9 of 9 ids, via `aep plan artifact show story:<id>` in each store (cortex through the worktree `cortex-plan-gaps` on plan/cb3-gaps, because PR #22 is not yet merged and the main checkout holds none of the four). I also read `aep plan artifact kinds` and `aep plan artifact lifecycle story`, and checked the tree with grep and sed (`tests/store_backend.rs`, `tests/e2e.rs`, `Taskfile.yml:117-121`, `story:retire-ingestion`).

Approved without findings:
- story:extraction-supersedes — states a before and an after, and cortex has a stand-in model to run it with.
- story:postgres-run-undo — `tests/store_backend.rs` has the docker PostgreSQL case, and `src/gate.rs:21` has the `facts_refused` gate.
- story:secrets-scan-without-redact — the exit status is observable, and `src/redact.rs` absent gives the before and after.
- story:cortex-host-reaches-staff-db — the acceptance is a command whose output can be checked.

Could not establish:
- Whether "answers the deployed head" in story:cortex-host-reaches-staff-db names the head to compare against, and whether a read proves the role can write, since the Why says cortex will write the store. This is an unease, not a finding.
- Whether the EKR 0.0.30 file:line claims in the two EKR stories are accurate. I did not open the EKR tree, and that is not an acceptance question.
- Out of my lane: story:secrets-scan-without-redact, story:served-channels-by-audience and story:cortex-host-reaches-staff-db carry no `serves` relation and no `scope`, unlike the six others. The overlap of story:read-answers-name-types with story:read-surface-specified is a parallel-safety matter.

```findings
[
 {
  "file": ".engineering/planning/story/structured-children.md",
  "line": 32,
  "category": "acceptance",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "the clause \"a renamed project keeps one node with the old path as an alias\" names no before state or trigger (a second import after the rename), so it reads the same on a fixture where nothing was renamed, and it is the fourth independent outcome in one sentence"
 },
 {
  "file": ".engineering/planning/story/record-field-lookups.md",
  "line": 21,
  "category": "acceptance",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "the Outcome promises \"pick the first non-empty of several text fields\", but the acceptance checks only the id-to-name lookup and the `<@id>` mention, so the story can close with that capability untested"
 },
 {
  "file": ".engineering/planning/story/read-answers-name-types.md",
  "line": 24,
  "category": "acceptance",
  "severity": "blocker",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "the Outcome has search, describe_node, expand, timeline and explain carry names for node types, edge types and property ids, but the acceptance checks only type ids in a search answer, and \"a type read\" names no operation"
 },
 {
  "file": ".engineering/planning/story/explain-bounds-documents.md",
  "line": 20,
  "category": "acceptance",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "the Outcome promises \"a way to read the rest\" and the cited span or a window around it, but the acceptance checks only truncated true and the full length, so the story can close with no way to read the remainder and without the cited fact inside the returned bytes"
 },
 {
  "file": ".engineering/planning/story/served-channels-by-audience.md",
  "line": 22,
  "category": "acceptance",
  "severity": "blocker",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "the acceptance demands that a query for an excluded channel's message \"answers nothing on the deployment\", while the Open section leaves removal of the text already in the store at revision 547 as \"a separate operator decision\", so the story cannot honestly close until that decision is made"
 },
 {
  "file": ".engineering/planning/story/served-channels-by-audience.md",
  "line": 22,
  "category": "acceptance",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "the acceptance joins two independent outcomes with a semicolon (the spec lists each channel with its class; an excluded channel's message is not answered), so one can pass while the other fails, and neither \"class\", \"known message\" nor the query tool is named"
 }
]
```