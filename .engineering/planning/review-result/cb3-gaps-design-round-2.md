---
format: aep.planning-md/3
id: review-result:cb3-gaps-design-round-2
kind: review-result
status: active
title: 'Plan critic (design), round 2: cb3 gap stories'
relations:
- reviews: story:structured-children
- reviews: story:extraction-supersedes
- reviews: story:postgres-run-undo
- reviews: story:record-field-lookups
revision: 1
---
needs-revision

- `story:structured-children` — acceptance 3 (a renamed project keeps one node and its old path as an alias) is a second outcome that the Outcome and title no longer claim, and it holds for flat records with no children, so it belongs with `story:structured-source` or in its own story, or the Outcome must state it — `.engineering/planning/story/structured-children.md:44` against `:31-33`
- `story:structured-children` — the `depends_on story:structured-from-files-and-drops` edge, recorded for the shared `src/structured.rs` and `fetch_structured`, also makes this story wait for `upstream-blocker:ekr-extraction-supersession`, which its own outcome does not need. The choice is to keep the order and accept the wait, or to split children out of `fetch_structured` so the edge can go — `.engineering/planning/story/structured-children.md:52` and `aep plan artifact graph` (cortex store: that blocker `blocks` `story:structured-from-files-and-drops`)
- `story:read-answers-name-types` — it holds two things with different dependencies. Names on existing reads (acceptance 1-5) need no new read surface, and `describe_type` (acceptance 6) does. The single `depends_on story:read-surface-specified` puts all six behind `dependency-blocker:ess-openapi-command-response`; move `describe_type` into its own story that depends on `story:read-surface-specified` — `.engineering/planning/story/read-answers-name-types.md:39-51` and `aep plan artifact graph` (EKR store: `dependency-blocker:ess-openapi-command-response -> story:read-surface-specified`)
- `story:explain-bounds-documents` — the Note says `story:read-surface-specified` "when it lands … carries `offset` and `limit`", but that story's body never mentions them and no edge orders the two. `ekr.views` has no explain command today (`systems/ekr/domains/views.yaml` has no `Explain`), so each story assumes the other models `explain`'s input. Add the `depends_on` in the direction chosen and put `offset` and `limit` into `story:read-surface-specified`'s Build — `.engineering/planning/story/explain-bounds-documents.md:40` against `read-surface-specified.md` (no match for `offset` or `limit`)
- `story:cortex-cutover` — cutover makes cortex write the store the deployment serves, which `story:served-channels-by-audience` exists to keep clear of restricted text. That order is stated only in the epic's critical path, with no edge; add `depends_on story:served-channels-by-audience` — `.engineering/planning/story/cortex-cutover.md:14` and `.engineering/planning/epic/run-on-cortex.md:87-88` (cb3 store), `aep plan artifact graph`
- `epic:run-on-cortex` — the body carries two copies of "Filed for the missing rows", "Critical path to cutover" and "Pilot" that contradict each other. The second copy (lines 94-109) lists the archived `story:secrets-scan-without-redact` as filed and runs `structured-children` and `postgres-run-undo` in parallel in step 1, against the edges now recorded; delete it — `.engineering/planning/epic/run-on-cortex.md:67-92` against `:94-109`

**Round-1 findings (cortex, EKR and cb3)**

| Round-1 finding | State |
|---|---|
| `story:structured-children` missing edges | resolved (3 `depends_on` edges, walk reuse stated) |
| `story:extraction-supersedes` blocker edge | resolved (`upstream-blocker:ekr-extraction-supersession` now `blocks` it) |
| `story:postgres-run-undo` missing edges | resolved (3 `depends_on` edges) |
| `story:record-field-lookups` missing edge | resolved (`depends_on story:file-records`) |
| `story:explain-bounds-documents` read-the-rest operation | resolved (`offset` and `limit` on `explain`); a new ordering gap is reported above |
| `story:read-answers-name-types` split or fold | part resolved: the edge was added, the two-things seam remains (finding above) |
| `story:secrets-scan-without-redact` and `story:retire-ingestion` | resolved (story archived, `story:retire-ingestion` depends on `story:retire-local-redaction`) |
| `story:served-channels-by-audience` | resolved (depends on `story:cortex-instance-spec`; 547-store text goes to `decision-blocker:pilot-store-restricted-text`) |
| `story:cortex-cutover` | resolved (depends on `story:cortex-host-reaches-staff-db`) |

**What I read:** 16 artifacts shown whole, plus neighbours (`story:structured-source`, `story:structured-from-files-and-drops`, `story:document-time-as-valid-time`, `story:connectors-source-walks`, `story:read-surface-served-over-http`, `story:explain-reads-an-index`, `story:cortex-instance-spec`, `story:retire-local-redaction`, `story:retire-ingestion`) and round 1's `review-result:cb3-gaps-design-round-1` in each store. I ran `aep plan artifact relations`, `show` and `graph` in all three stores, and `validate` (all `valid`). I walked about 190 edges in the cortex store, and the cb3 and EKR graphs filtered to the neighbours of the set, including outside it. I found no cycle and no serialising chain. Cross-store dependencies are prose only, so cycles across stores cannot be declared.

**What I could not establish:**
- Out of lane (acceptance): `story:retire-local-redaction` no longer holds the archived story's check that `task secrets` fails on a staged credential once `src/redact.rs` is gone.
- Out of lane (acceptance): `story:served-channels-by-audience` does not say what turns a `restricted` class into an excluded channel (a cortex filter or a generator).
- Out of lane (parallel-safety): `story:cortex-host-reaches-staff-db` and `story:postgres-store-backup` both touch `charts/company-brain/templates/`.
- Out of lane (parallel-safety): `story:read-answers-name-types` edits `crates/ekr/src/cli/mcp.rs` and `view.rs`, which `story:read-surface-served-over-http` replaces with one dispatcher.
- Out of lane (scope): the epic's "Stories here" list omits the new cb3 stories.
- Pre-existing, not set against this round: `story:cortex-host-reaches-staff-db` names cortex `story:postgres-credential-from-connectors` in prose, with no `upstream-blocker` record, unlike the epic's stated convention.
- Pre-existing, not set against this round: `story:document-time-as-valid-time` names `story:codex-model-backend` in prose with no edge, so `story:extraction-supersedes` is transitively affected.
- `story:postgres-run-undo` Open (does EKR need a revert point?) is still unknown, so it is not a finding.

```findings
[
  {
    "file": ".engineering/planning/story/structured-children.md",
    "line": 44,
    "category": "design",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "acceptance 3 (a renamed project keeps one node and its old path as an alias) is a second outcome that the Outcome and title no longer claim, and it holds for flat records with no children, so it belongs with story:structured-source or in its own story, or the Outcome must state it"
  },
  {
    "file": ".engineering/planning/story/structured-children.md",
    "line": 52,
    "category": "design",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "the depends_on story:structured-from-files-and-drops edge, recorded for the shared src/structured.rs and fetch_structured, also makes this story wait for upstream-blocker:ekr-extraction-supersession, which its own outcome does not need; the choice is to keep the order and accept the wait, or to split children out of fetch_structured so the edge can go"
  },
  {
    "file": ".engineering/planning/story/read-answers-name-types.md",
    "line": 47,
    "category": "design",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "two things in one story: names on existing reads (acceptance 1-5) need no new read surface, and describe_type (acceptance 6) does, yet the single depends_on story:read-surface-specified puts all six behind dependency-blocker:ess-openapi-command-response; move describe_type into its own story that depends on story:read-surface-specified"
  },
  {
    "file": ".engineering/planning/story/explain-bounds-documents.md",
    "line": 40,
    "category": "design",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "the Note says story:read-surface-specified will carry offset and limit, but that story's body never mentions them and no edge orders the two, while ekr.views has no explain command today; add the depends_on in the chosen direction and put offset and limit into story:read-surface-specified's Build"
  },
  {
    "file": ".engineering/planning/story/cortex-cutover.md",
    "line": 14,
    "category": "design",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "cutover makes cortex write the store the deployment serves, which story:served-channels-by-audience exists to keep clear of restricted text, and that order is stated only in the epic's critical path with no edge; add depends_on story:served-channels-by-audience"
  },
  {
    "file": ".engineering/planning/epic/run-on-cortex.md",
    "line": 94,
    "category": "design",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "the body carries two copies of 'Filed for the missing rows', 'Critical path to cutover' and 'Pilot' that contradict each other; the second copy (lines 94-109) lists the archived story:secrets-scan-without-redact as filed and runs structured-children and postgres-run-undo in parallel in step 1 against the edges now recorded; delete it"
  }
]
```