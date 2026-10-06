---
format: aep.planning-md/3
id: review-result:cb3-gaps-scope-round-1
kind: review-result
status: active
title: 'Plan critic (scope), round 1: cb3 gap stories'
relations:
- reviews: story:structured-children
- reviews: story:extraction-supersedes
- reviews: story:postgres-run-undo
- reviews: story:record-field-lookups
revision: 1
---
needs-revision

1. epic:run-on-cortex — the epic says 10 missing rows were filed, but its filed table lists 9 stories; inventory row 29 (verified off-instance backup of the PostgreSQL store, "no story anywhere") has no claimant, and the epic does not say it is deliberately left open; add a cb3 story or an explicit "not covered, owner undecided" line — .engineering/planning/epic/run-on-cortex.md:62 (also :71-73); cb3-gap-inventory.md:48
2. epic:run-on-cortex — the capability table at :23 and :27 still routes `gitlab_repos` to the flat `structured-source` and backup/rollback to `run-snapshots`/`run-gate`, which the inventory's "Corrections to the epic's table" say is wrong for both (flat records only; snapshots and undo are SQLite-only); fix the two rows — .engineering/planning/epic/run-on-cortex.md:23,27; cb3-gap-inventory.md:180-184
3. epic:run-on-cortex — critical path step 1 runs `served-channels-by-audience` and `cortex-host-reaches-staff-db` in parallel with everything else, but the first has its output in the `cortex-instance-spec` file (step 4) and the second's acceptance needs the Connectors launch release and `postgres-credential-from-connectors` (step 2). They cannot finish in step 1, so the path and done-criteria 4 and 5 do not line up — .engineering/planning/epic/run-on-cortex.md:77; story:served-channels-by-audience Files line 30; story:cortex-host-reaches-staff-db Acceptance line 22
4. story:served-channels-by-audience — the acceptance "a query for a known message from an excluded channel answers nothing on the deployment" cannot hold while done-criterion 2 requires the head to descend from 547 "without a reseed", because the 547 store already holds that text (the story's own Open section says so); restate the acceptance for newly written revisions, or record the conflict with criterion 2 — .engineering/planning/story/served-channels-by-audience.md:22,26; initiative:run-on-cortex:24-25
5. story:record-field-lookups — row 6 lists five Slack rendering behaviours; the story claims name lookup, `<@id>` mentions and first-non-empty text, and nothing says `text_blocks`/attachment text, `[file: …]` markers and `--min-chars` are dropped or covered elsewhere — .engineering/planning/story/record-field-lookups.md:21; cb3-gap-inventory.md:25
6. story:structured-children — row 17 lists "refused pipelines dropped" among the `gitlab_repos` behaviours; the outcome and acceptance cover child nodes, derived link and alias only, and nothing records that the drop is left out or covered by `structured-from-files-and-drops` — .engineering/planning/story/structured-children.md:23,30; cb3-gap-inventory.md:36

**What I read:** 11 artifacts (initiative, epic, 9 stories) plus the inventory and the critic rubric. I ran `aep plan artifact show` for each, and `aep plan artifact graph` in the cb3 store. The cortex stories came from branch `plan/cb3-gaps` in worktree cortex-plan-gaps (7378cac), because PR #22 is not on origin/main. The EKR stories came from ekr-wx-int at 095479e9e.

**Promise count:** I extracted 15 promises: the 10 missing rows (6, 11, 17, 20, 29, 30, 47, P1, P2, P3) and the 5 done-criteria. I traced 14 to an item or a path step; row 29 is the one untraced. Rows 6 and 17 are traced but narrowed (findings 5 and 6).

**Mapping of the 9 stories:**

| story | claims | traceable to the goal |
|---|---|---|
| `structured-children` | row 17 | yes |
| `extraction-supersedes` | row 20 | yes |
| `postgres-run-undo` | row 30 | yes |
| `record-field-lookups` | row 6 | yes |
| `read-answers-name-types` | P1 | yes (inventory; epic names it) |
| `explain-bounds-documents` | P2 | yes (inventory; epic names it) |
| `secrets-scan-without-redact` | row 11 | yes |
| `served-channels-by-audience` | P3 | yes (epic "Pilot" paragraph) |
| `cortex-host-reaches-staff-db` | row 47 | yes |

No two stories claim the same outcome. Nothing in the nine reaches past the goal.

**Could not establish:**
- I did not see the drafter's report, so I could not check whether row 29 was named as deliberately left open. Finding 1 assumes it was not.
- Three things are out of my lane and did not set the verdict:
  - The new cb3 stories have no `depends_on` or `blocks` edge to `cortex-cutover` or `retire-ingestion`, so nothing gates those two on them (design).
  - `secrets-scan-without-redact` can only be tested once `redact.rs` is deleted, which is the last step (acceptance, design).
  - The cortex stories decompose `epic:organisation-scale-instance`, which does not list them, and they name Company Brain v3 although that epic says the consumer "is named only in its own repository" (design).

```findings
[
 {
  "file": ".engineering/planning/epic/run-on-cortex.md",
  "line": 62,
  "category": "scope",
  "severity": "blocker",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "the epic says 10 missing rows were filed but lists 9 stories; inventory row 29 (verified off-instance backup of the PostgreSQL store, no story anywhere) has no claimant and no stated omission, and done-criterion 5 deletes backup code"
 },
 {
  "file": ".engineering/planning/epic/run-on-cortex.md",
  "line": 23,
  "category": "scope",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "pre-existing",
  "message": "the capability table still routes gitlab_repos to flat structured-source (line 23) and backup/rollback to run-snapshots and run-gate (line 27), which the inventory's corrections say is wrong for both"
 },
 {
  "file": ".engineering/planning/epic/run-on-cortex.md",
  "line": 77,
  "category": "scope",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "critical path step 1 runs served-channels-by-audience and cortex-host-reaches-staff-db in parallel, but the first lands in the cortex-instance-spec file (step 4) and the second needs the Connectors launch and postgres-credential-from-connectors (step 2), so they cannot finish in step 1"
 },
 {
  "file": ".engineering/planning/story/served-channels-by-audience.md",
  "line": 22,
  "category": "scope",
  "severity": "blocker",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "the acceptance that an excluded channel's known message answers nothing on the deployment cannot hold under done-criterion 2 (head descends from 547 without reseed) because the 547 store already holds that text; restate for new revisions or record the conflict"
 },
 {
  "file": ".engineering/planning/story/record-field-lookups.md",
  "line": 21,
  "category": "scope",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "inventory row 6 lists text_blocks/attachment text, [file: \u2026] markers and --min-chars besides name lookup and mentions; the story claims only lookup, mentions and first-non-empty text and does not say the rest is dropped or covered elsewhere"
 },
 {
  "file": ".engineering/planning/story/structured-children.md",
  "line": 23,
  "category": "scope",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "inventory row 17 lists refused pipelines dropped among the gitlab_repos behaviours; the story's outcome and acceptance omit it and do not say it is covered by structured-from-files-and-drops or left out"
 }
]
```