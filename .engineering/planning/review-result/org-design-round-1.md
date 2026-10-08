---
format: aep.planning-md/3
id: review-result:org-design-round-1
kind: review-result
status: archived
title: Design critic, organisation scale, round 1
relations:
- reviews: epic:organisation-scale-instance
- reviews: story:spec-organisation-types
- reviews: story:connectors-source-walks
- reviews: story:file-records
- reviews: story:redaction-names-and-gate
- reviews: story:structured-from-files-and-drops
- reviews: story:document-time-as-valid-time
- reviews: story:run-gate
- reviews: story:quality-judge
revision: 2
transitions:
- {from: "active", to: "archived", at: "2026-10-08T13:07:38Z", actor: "human:timo", revision: 2}
---
needs-revision
story:spec-organisation-types — its diff replaces `StructuredSource`'s flat `adapter/connection/operation/inputs` fields with `input: StructuredInput`, which `story:structured-source` reads in `src/sources.rs` and `src/structured.rs`, yet no `depends_on` orders the two and this story's scope lists neither file; add `depends_on story:structured-source` and put those two files in scope, or move the reshape into `story:structured-from-files-and-drops` — .engineering/planning/story/spec-organisation-types.md:111-141
story:structured-from-files-and-drops — it claims `connectors` input "now with `paging`" but has no work item, no acceptance and no edge to `story:connectors-source-walks`, which owns the paging walk; add `depends_on story:connectors-source-walks` (it also edits `src/sources.rs`) or drop the claim — .engineering/planning/story/structured-from-files-and-drops.md:28
story:run-gate — it restores the pre-run snapshot from inside the run, but `story:run-snapshots` only offers `cortex restore`, which refuses while the instance lock is held, so the body must name an in-process restore entry point that `story:run-snapshots` provides — .engineering/planning/story/run-gate.md:32 (against .engineering/planning/story/run-snapshots.md:57)

**What I read:** all 8 stories and the epic, plus `story:spec-standalone-types`, `story:structured-source`, `story:redaction-before-model` and `story:run-snapshots`. Commands were `aep plan artifact show`, `relations`, `graph` and `validate`. I walked all declared edges in the store, including those outside the set (47 artifacts), and found no cycle in the `depends_on` edges. `validate` printed "valid".

**What I could not establish:**
- Parallel safety, out of my lane: `story:connectors-source-walks`, `story:file-records` and `story:structured-from-files-and-drops` all edit `src/sources.rs`, and only the first two are ordered by an edge.
- The fan-out from `story:spec-organisation-types`, and the chain `story:connectors-source-walks` → `story:file-records` → `story:document-time-as-valid-time`, both have their reasons written in the bodies, so I did not flag them.
- I did not check whether `ekr quality` and `ekr fact-quality` measures exist upstream (out of lane).

```findings
[
  {
    "file": ".engineering/planning/story/spec-organisation-types.md",
    "line": 111,
    "category": "design",
    "severity": "blocker",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "the diff replaces StructuredSource's flat fields with input: StructuredInput, which story:structured-source reads in src/sources.rs and src/structured.rs, yet no depends_on orders the two and this story's scope lists neither file; add depends_on story:structured-source and put those files in scope, or move the reshape into story:structured-from-files-and-drops"
  },
  {
    "file": ".engineering/planning/story/structured-from-files-and-drops.md",
    "line": 28,
    "category": "design",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "it claims connectors input \"now with paging\" but has no work item, acceptance or edge to story:connectors-source-walks, which owns the paging walk; add depends_on story:connectors-source-walks or drop the claim"
  },
  {
    "file": ".engineering/planning/story/run-gate.md",
    "line": 32,
    "category": "design",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "it restores the pre-run snapshot from inside the run, but story:run-snapshots only offers cortex restore, which refuses while the instance lock is held, so the body must name an in-process restore entry point that story:run-snapshots provides"
  }
]
```
