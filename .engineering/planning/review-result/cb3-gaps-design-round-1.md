---
format: aep.planning-md/3
id: review-result:cb3-gaps-design-round-1
kind: review-result
status: active
title: 'Plan critic (design), round 1: cb3 gap stories'
relations:
- reviews: story:structured-children
- reviews: story:extraction-supersedes
- reviews: story:postgres-run-undo
- reviews: story:record-field-lookups
revision: 1
---
needs-revision

- `story:read-answers-name-types` — its type-read operation is half of the read surface that `story:read-surface-specified` defines as exactly 11 operations, with no `depends_on` edge, so both claim the same surface and the 11-operation acceptance breaks; split the type read into its own story that `depends_on` `story:read-surface-specified`, or fold it in — `~/.local/state/worktree/trees/b10x/epistemic-knowledge-runtime/ekr-wx-int/.engineering/planning/story/read-answers-name-types.md:24,29,35` against `read-surface-specified.md:43,59`
- `story:explain-bounds-documents` — "a way to read the rest" names no operation, and over MCP the only candidate, `describe_evidence`, exists only after `story:read-surface-served-over-http` adds it, with no edge. Either name a self-contained window parameter on `explain` or add `depends_on story:read-surface-served-over-http` — `~/.local/state/worktree/trees/b10x/epistemic-knowledge-runtime/ekr-wx-int/.engineering/planning/story/explain-bounds-documents.md:20` against `read-surface-served-over-http.md:27` and `grep -n describe_evidence crates/ekr/src/cli/mcp.rs` (no match)
- `story:secrets-scan-without-redact` — `story:retire-local-redaction` already owns "`task secrets` … call the shared detector, and `cb3 redact` … removed or calls it", while this story leaves the scanner open between a minimal `cb3 redact --scan` and gitleaks, so two stories own one replacement; fold it into `story:retire-local-redaction` or state which one owns the scanner and add `depends_on` — `<the company-brain store>/.engineering/planning/story/secrets-scan-without-redact.md:24-26` against `retire-local-redaction.md:45-46`
- `story:retire-ingestion` — it deletes `src/redact.rs`, which `task secrets` runs (`Taskfile.yml:119-121`), and no edge orders it after `story:secrets-scan-without-redact`; add `depends_on story:secrets-scan-without-redact` — `<the company-brain store>/.engineering/planning/story/retire-ingestion.md:12-22` and `secrets-scan-without-redact.md:13,18`
- `story:served-channels-by-audience` — its acceptance queries "the deployment", which serves the revision-547 store that already holds the excluded text, and `story:cortex-cutover` is what adopts that store, with no edge and no item for the removal; add `depends_on story:cortex-cutover` or a decision-blocker, or limit the outcome to what cortex writes — `<the company-brain store>/.engineering/planning/story/served-channels-by-audience.md:22,26` against `cortex-cutover.md:19,25`
- `story:served-channels-by-audience` — the channel policy as data is already part of `story:cortex-instance-spec`'s outcome and both edit `deploy/cortex/`, so the audience class is half of one abstraction with no edge; add `depends_on story:cortex-instance-spec` — `<the company-brain store>/.engineering/planning/story/served-channels-by-audience.md:13,30` against `cortex-instance-spec.md:16`
- `story:cortex-cutover` — "adopt the promoted store … on the PostgreSQL backend the deployment reads" cannot be shown until the timer host reaches that database, which `story:cortex-host-reaches-staff-db` delivers, and no edge records it; add `depends_on story:cortex-host-reaches-staff-db` — `<the company-brain store>/.engineering/planning/story/cortex-cutover.md:10,19` against `cortex-host-reaches-staff-db.md:13,18`
- `story:structured-children` — "each event's valid time equals its record time" needs the evidence-time and valid-time work in `story:document-time-as-valid-time` and the open `upstream-blocker:ekr-extraction-valid-time`, which blocks only that story; add `depends_on story:document-time-as-valid-time` — `~/beyond10x/cortex/.engineering/planning/story/structured-children.md:32` against `document-time-as-valid-time.md:28` and `aep plan artifact graph` (`upstream-blocker:ekr-extraction-valid-time` blocks one story)
- `story:structured-children` — "calls child operations per parent record" restates `ChildCall` from `story:connectors-source-walks`, whose reuse `story:structured-from-files-and-drops` states as "adds no second walk", and it shares `src/structured.rs` and `StructuredSource` with that story; add `depends_on story:structured-source` and `depends_on story:connectors-source-walks` and say it reuses the walk — `~/beyond10x/cortex/.engineering/planning/story/structured-children.md:23` against `connectors-source-walks.md:39` and `structured-from-files-and-drops.md:36`
- `story:extraction-supersedes` — its dependency on the EKR supersession release is prose only, although `upstream-blocker:ekr-extraction-supersession` exists and blocks only `story:structured-from-files-and-drops`; record `upstream-blocker:ekr-extraction-supersession blocks story:extraction-supersedes` — `~/beyond10x/cortex/.engineering/planning/story/extraction-supersedes.md:30-32` and `aep plan artifact graph` (the `blocks` edges from `upstream-blocker:*`)
- `story:postgres-run-undo` — it extends the snapshot and gate undo (`story:run-snapshots`, `story:run-gate`) and the docker case of `story:store-backend-per-instance`, and declares no `depends_on` to any of them, unlike `story:postgres-credential-from-connectors`; add the three edges — `~/beyond10x/cortex/.engineering/planning/story/postgres-run-undo.md:26,30` and `aep plan artifact show story:postgres-run-undo` (relations: decomposes, serves)
- `story:record-field-lookups` — it adds `FileRecords.lookups` and edits `tests/file_records.rs`, both owned by `story:file-records`, with no `depends_on story:file-records` — `~/beyond10x/cortex/.engineering/planning/story/record-field-lookups.md:21,30` and `aep plan artifact show story:record-field-lookups` (relations: decomposes, serves)

**What I read:** all 9 stories whole, plus `story:read-surface-specified`, `story:read-surface-served-over-http`, `story:resolve-by-type-name`, `story:overview-sections`, `story:extraction-supersession`, and the cortex and cb3 neighbours they touch. I ran `aep plan artifact show`, `relations`, `graph` and `validate` in each store.

**What I could not establish:**
- Edges walked: the full cortex graph. In the EKR and cb3 stores I filtered the graph to the neighbours of the 9 ids. I walked outside the set in all three stores and found no cycle. Nor is there a serialising chain: the new stories carry almost no `depends_on` edges, so the problem is missing edges, not too many.
- cortex store source: PR #22 merged during the run, so I read the cortex stories from `~/beyond10x/cortex` on `main` (883256d), not from `plan/cb3-gaps`.
- Validators: cortex and cb3 report `valid`; the EKR warning about `story:seed-if-absent` and the cortex prose-only review are not findings.
- `story:postgres-run-undo` Open: it asks whether EKR needs a revert or restore point. If it does, that is an upstream dependency with no artifact yet. I could not tell, so it is not a finding.
- Out of lane:
  - parallel-safety: `src/extract.rs` (`story:extraction-supersedes`, `story:document-time-as-valid-time`, `story:codex-model-backend`) and `src/sources.rs` are shared.
  - parallel-safety: `tests/store_backend.rs` is shared between `story:postgres-run-undo` and `story:postgres-credential-from-connectors`.
  - parallel-safety: `charts/company-brain/` is shared by `story:cortex-cutover`, `story:cortex-host-reaches-staff-db` and `story:served-channels-by-audience`.
  - acceptance: `story:structured-children` states an optional compare-derived link that its acceptance does not test.
  - scope: `epic:organisation-scale-instance`'s story table does not list the four new cortex stories.

```findings
[
 {
  "file": "~/.local/state/worktree/trees/b10x/epistemic-knowledge-runtime/ekr-wx-int/.engineering/planning/story/read-answers-name-types.md",
  "line": 24,
  "category": "design",
  "severity": "blocker",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "its type-read operation is half of the read surface that story:read-surface-specified defines as exactly 11 operations, with no depends_on edge, so both claim the same surface and the 11-operation acceptance breaks; split the type read into its own story that depends_on story:read-surface-specified, or fold it in"
 },
 {
  "file": "~/.local/state/worktree/trees/b10x/epistemic-knowledge-runtime/ekr-wx-int/.engineering/planning/story/explain-bounds-documents.md",
  "line": 20,
  "category": "design",
  "severity": "blocker",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "\"a way to read the rest\" names no operation, and over MCP the only candidate, describe_evidence, exists only after story:read-surface-served-over-http adds it, with no edge; either name a self-contained window parameter on explain or add depends_on story:read-surface-served-over-http"
 },
 {
  "file": "<the company-brain store>/.engineering/planning/story/secrets-scan-without-redact.md",
  "line": 24,
  "category": "design",
  "severity": "blocker",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "story:retire-local-redaction already owns \"task secrets calls the shared detector, and cb3 redact removed or calls it\" while this story leaves the scanner open between a minimal cb3 redact --scan and gitleaks, so two stories own one replacement; fold it into story:retire-local-redaction or state which one owns the scanner and add depends_on"
 },
 {
  "file": "<the company-brain store>/.engineering/planning/story/retire-ingestion.md",
  "line": 12,
  "category": "design",
  "severity": "blocker",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "it deletes src/redact.rs, which task secrets runs (Taskfile.yml:119-121), and no edge orders it after story:secrets-scan-without-redact; add depends_on story:secrets-scan-without-redact"
 },
 {
  "file": "<the company-brain store>/.engineering/planning/story/served-channels-by-audience.md",
  "line": 22,
  "category": "design",
  "severity": "blocker",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "its acceptance queries the deployment, which serves the revision-547 store that already holds the excluded text, and story:cortex-cutover is what adopts that store, with no edge and no item for the removal; add depends_on story:cortex-cutover or a decision-blocker, or limit the outcome to what cortex writes"
 },
 {
  "file": "<the company-brain store>/.engineering/planning/story/served-channels-by-audience.md",
  "line": 13,
  "category": "design",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "the channel policy as data is already part of story:cortex-instance-spec's outcome and both edit deploy/cortex/, so the audience class is half of one abstraction with no edge; add depends_on story:cortex-instance-spec"
 },
 {
  "file": "<the company-brain store>/.engineering/planning/story/cortex-cutover.md",
  "line": 19,
  "category": "design",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "adopting the promoted store on the PostgreSQL backend the deployment reads cannot be shown until the timer host reaches that database, which story:cortex-host-reaches-staff-db delivers, and no edge records it; add depends_on story:cortex-host-reaches-staff-db"
 },
 {
  "file": "~/beyond10x/cortex/.engineering/planning/story/structured-children.md",
  "line": 32,
  "category": "design",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "\"each event's valid time equals its record time\" needs the evidence-time and valid-time work in story:document-time-as-valid-time and the open upstream-blocker:ekr-extraction-valid-time, which blocks only that story; add depends_on story:document-time-as-valid-time"
 },
 {
  "file": "~/beyond10x/cortex/.engineering/planning/story/structured-children.md",
  "line": 23,
  "category": "design",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "\"calls child operations per parent record\" restates ChildCall from story:connectors-source-walks, whose reuse story:structured-from-files-and-drops states as \"adds no second walk\", and it shares src/structured.rs and StructuredSource with that story; add depends_on story:structured-source and story:connectors-source-walks and say it reuses the walk"
 },
 {
  "file": "~/beyond10x/cortex/.engineering/planning/story/extraction-supersedes.md",
  "line": 30,
  "category": "design",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "its dependency on the EKR supersession release is prose only, although upstream-blocker:ekr-extraction-supersession exists and blocks only story:structured-from-files-and-drops; record upstream-blocker:ekr-extraction-supersession blocks story:extraction-supersedes"
 },
 {
  "file": "~/beyond10x/cortex/.engineering/planning/story/postgres-run-undo.md",
  "line": 26,
  "category": "design",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "it extends the snapshot and gate undo (story:run-snapshots, story:run-gate) and the docker case of story:store-backend-per-instance and declares no depends_on to any of them, unlike story:postgres-credential-from-connectors; add the three edges"
 },
 {
  "file": "~/beyond10x/cortex/.engineering/planning/story/record-field-lookups.md",
  "line": 21,
  "category": "design",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "it adds FileRecords.lookups and edits tests/file_records.rs, both owned by story:file-records, with no depends_on story:file-records"
 }
]
```