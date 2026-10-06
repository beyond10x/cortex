---
format: aep.planning-md/3
id: review-result:cb3-gaps-scope-round-2
kind: review-result
status: active
title: 'Plan critic (scope), round 2: cb3 gap stories'
relations:
- reviews: story:structured-children
- reviews: story:extraction-supersedes
- reviews: story:postgres-run-undo
- reviews: story:record-field-lookups
revision: 1
---
needs-revision

epic:run-on-cortex — the old "Filed for the missing rows", "Critical path to cutover" and "Pilot" sections still follow the revised ones, so the body carries two contradictory paths; the stale step 1 still runs the three cb3 stories and `structured-children`, `postgres-run-undo` and `record-field-lookups` in parallel, and the stale table still lists the archived `story:secrets-scan-without-redact`. Round-1 finding 3 therefore holds at the old lines — .engineering/planning/epic/run-on-cortex.md:94-111 (stale step 1 at :104)
epic:run-on-cortex — path step 5 puts `retire-local-redaction` before `retire-ingestion` without naming that `dependency-blocker:ekr-type-packs` blocks it (through `redaction-policy-per-class`). Done-criterion 5 now waits on EKR and ESS type packs, and no step lists that prerequisite — .engineering/planning/epic/run-on-cortex.md:88 (`aep plan artifact graph`: "dependency-blocker:ekr-type-packs" -> "story:retire-local-redaction" blocks)
story:cortex-cutover — path step 4 places `served-channels-by-audience` before the shadow run and cutover, but cutover has no `depends_on` edge to it, so nothing stops the cutover from serving restricted channels (row P3) — .engineering/planning/story/cortex-cutover.md:10-11
upstream-blocker:cortex-org-features — the clearing list names none of the new cortex stories (`structured-children`, `record-field-lookups`, `extraction-supersedes`), though `cortex-instance-spec` needs them for `gitlab_repos` and Slack, so the spec is not gated on them — .engineering/planning/upstream-blocker/cortex-org-features.md:13
upstream-blocker:cortex-release — "every story above" lists no story, and `postgres-run-undo` and `postgres-credential-from-connectors` are named in no cb3 blocker or edge, so cutover is not gated on the PostgreSQL undo that done-criterion 4 relies on — .engineering/planning/upstream-blocker/cortex-release.md:13
story:structured-children — "Left out" sends dropping of refused records to `story:structured-from-files-and-drops`, but that story only supersedes values a source stopped listing (`dropped: Supersede`). cb3's "refused pipelines" are projects GitLab refused (`src/structured.rs:149-153`), so that part of row 17 still has no claimant (round-1 finding 6 only partly resolved) — .engineering/planning/story/structured-children.md:48
story:structured-children — the derived merge-to-first-tag link is "not part of this story" with no owner and no "dropped" decision. cb3 already ships it as `story:merges-linked-to-shipping-tag` (implemented), and the epic table does not record the loss — .engineering/planning/story/structured-children.md:48

**What I read:** 18 artifacts: the initiative, the epic, 6 cb3 stories plus the decision-blocker, 4 cortex stories, 2 EKR stories, and 3 round-1 records (the cortex one in full). I also read the inventory, `structured-from-files-and-drops`, `structured-source`, the cb3 upstream-blockers, `epic:organisation-scale-instance`, and the `aep plan artifact graph` output in all three stores. I read `src/structured.rs:1-12,149-153` as evidence.

**Promise count:** I extracted 15 promises (10 missing rows and 5 done-criteria). All 15 trace to a story or path step, but rows 11, 17 and 6 are narrowed and the narrowing is not fully recorded.

**Round-1 findings:**

| # | finding | state |
|---|---|---|
| 1 | row 29 unclaimed | resolved by `postgres-store-backup` and the epic row |
| 2 | epic capability table | resolved at :20 and :24 |
| 3 | critical path order | open: the new path (:84-88) is right, the old one (:104) remains |
| 4 | served-channels acceptance against revision 547 | resolved by the new acceptance and `pilot-store-restricted-text`, which blocks cutover |
| 5 | row 6 narrowing | resolved by "Left out" |
| 6 | row 17 "refused pipelines" | partial, see the structured-children finding above |

**Not mine to set the verdict:**
- The cortex `release-1-0` and `docs-for-1-0` have no `depends_on` edge to the new stories, so a 1.0 can ship without them (design).
- The new cortex stories still decompose `epic:organisation-scale-instance`, whose table does not list them (design).
- `read-answers-name-types` depends on `read-surface-specified` (blocked on ESS#423), and the epic does not say P1 and P2 are off the cutover path (design).
- The archived `story:secrets-scan-without-redact` has a doubled "Archived" heading (style).

**Could not establish:** whether the `retire-local-redaction` acceptance checks the staged-credential case that stub B1 specified. That is the acceptance critic's lane.

```findings
[
 {
  "file": ".engineering/planning/epic/run-on-cortex.md",
  "line": 94,
  "category": "scope",
  "severity": "blocker",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "the old 'Filed for the missing rows', 'Critical path to cutover' and 'Pilot' sections still follow the revised ones, so the body carries two contradictory paths (the stale step 1 at :104 runs the three cb3 stories and the cortex stories in parallel, the stale table lists the archived secrets-scan-without-redact); round-1 finding 3 still holds at the old lines"
 },
 {
  "file": ".engineering/planning/epic/run-on-cortex.md",
  "line": 88,
  "category": "scope",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "path step 5 puts retire-local-redaction before retire-ingestion without naming that dependency-blocker:ekr-type-packs blocks it through redaction-policy-per-class, so done-criterion 5 now waits on EKR and ESS type packs that no step lists"
 },
 {
  "file": ".engineering/planning/story/cortex-cutover.md",
  "line": 10,
  "category": "scope",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "path step 4 places served-channels-by-audience before the shadow run and cutover, but cortex-cutover has no depends_on edge to it, so nothing gates the cutover on the restricted-channel exclusion (row P3)"
 },
 {
  "file": ".engineering/planning/upstream-blocker/cortex-org-features.md",
  "line": 13,
  "category": "scope",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "the clearing list names none of the new cortex stories (structured-children, record-field-lookups, extraction-supersedes), so cortex-instance-spec is not gated on the stories it needs for gitlab_repos and Slack"
 },
 {
  "file": ".engineering/planning/upstream-blocker/cortex-release.md",
  "line": 13,
  "category": "scope",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "'every story above' lists no story, and postgres-run-undo and postgres-credential-from-connectors are named in no cb3 blocker or edge, so cutover is not gated on the PostgreSQL undo that done-criterion 4 relies on"
 },
 {
  "file": ".engineering/planning/story/structured-children.md",
  "line": 48,
  "category": "scope",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "'Left out' sends dropping of refused records to structured-from-files-and-drops, which only supersedes values a source stopped listing (dropped: Supersede); the upstream importer's refused pipelines are projects the API refused, so that part of row 17 has no claimant (round-1 finding 6 only partly resolved)"
 },
 {
  "file": ".engineering/planning/story/structured-children.md",
  "line": 48,
  "category": "scope",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "the derived merge-to-first-tag link is 'not part of this story' with no owner and no dropped decision, although the downstream deployment ships it today as an implemented story and the epic table does not record the loss"
 }
]
```