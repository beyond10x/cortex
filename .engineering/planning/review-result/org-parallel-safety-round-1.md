---
format: aep.planning-md/3
id: review-result:org-parallel-safety-round-1
kind: review-result
status: archived
title: Parallel-safety critic, organisation scale, round 1
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
story:spec-organisation-types — edits `spec/domains/instance.yaml`, `generated/` and `spec/suite.json` (cited), the same three files as story:seen-documents-modelled, story:run-snapshots and story:adopt-existing-store, and its Depends on names only story:spec-standalone-types. Add ordering edges that record the shared spec files, or split the surface — .engineering/planning/story/spec-organisation-types.md:206
story:spec-organisation-types — its diff reshapes `StructuredSource` into `StructuredConnectors` plus `StructuredInput` and adds `RedactionClass` variants (cited, diff lines 111-152). That breaks the consumers story:structured-source and story:redaction-before-model are building (inferred: structured-source consumes the type via `src/sources.rs` and `src/structured.rs`; redact.rs consumes the class enum), and no edge records it. Add an ordering edge or split the surface — .engineering/planning/story/spec-organisation-types.md:111
story:connectors-source-walks — `src/state.rs` (the `{since}` last-run state, cited) is also in story:seen-documents-modelled, story:adopt-existing-store and story:timer-runs-unattended (inferred), and the body names none of them. Add an ordering edge recording the file, or split the surface — .engineering/planning/story/connectors-source-walks.md:49
story:file-records — `src/state.rs` (per-record seen state, cited) is also in story:seen-documents-modelled (which re-models seen state), story:adopt-existing-store and story:timer-runs-unattended (inferred), with no edge or mention. Add an ordering edge recording the file, or split the surface — .engineering/planning/story/file-records.md:49
story:structured-from-files-and-drops — `src/sources.rs` (cited) is also edited by story:connectors-source-walks, story:file-records and story:structured-source. It depends on structured-source only, yet takes paging from connectors-source-walks. Add edges to connectors-source-walks and file-records recording the file, or split the surface — .engineering/planning/story/structured-from-files-and-drops.md:48
story:redaction-names-and-gate — `src/run.rs` (cited) is also edited by story:run-snapshots, story:structured-source and story:timer-runs-unattended (inferred), with no edge between this story and them. The body says "both edit" only for redaction-before-model. Add ordering edges or split the surface — .engineering/planning/story/redaction-names-and-gate.md:45
story:run-gate — `src/run.rs` (cited) is also in story:redaction-names-and-gate. Its pre-model refusal and this story's post-apply restore sit in one run path, and there is no edge between them. The body names only run-snapshots. Add an ordering edge or split the surface — .engineering/planning/story/run-gate.md:44
story:quality-judge — `src/lib.rs` and `src/main.rs` (cited) are shared with story:run-gate, story:run-snapshots, story:structured-source, story:redaction-before-model, story:store-backend-per-instance, story:adopt-existing-store and story:web-pages-cited-as-urls. Its only edge is to spec-standalone-types and the body mentions none of them. Add ordering edges or split the surface — .engineering/planning/story/quality-judge.md:44
story:document-time-as-valid-time — `src/extract.rs` (cited) is also in story:codex-model-backend and story:web-pages-cited-as-urls, and `src/evidence.rs` (cited) is also in web-pages-cited-as-urls. It has no edge to either and names neither. Add ordering edges or split the surface — .engineering/planning/story/document-time-as-valid-time.md:45

What I read: 8 new stories, the new epic, `upstream-blocker:ekr-extraction-supersession` and the scope of 14 other open stories, via `aep plan artifact show` or `cat` of each body, `aep plan artifact graph` and `aep plan artifact waves`. I checked the 51 `waves` collisions that involve a new story against the `depends_on` closure; 26 are already ordered by edges and the rest are the findings above. `aep plan artifact waves` does place every colliding pair in a different wave, but that order is derived from scope and no edge records it.

Surface counts for the 8 new stories: 8 cited, 0 inferred, 0 unplaceable. Each has at least one cited existing file; the new files `src/gate.rs`, `src/quality.rs`, `src/redact.rs` and the `tests/*` files are inferred and do not exist yet. Of the findings above, the `RedactionClass` and `StructuredSource` coupling in the second finding rests on an inferred consumer surface.

Not established or out of my lane:
- story:ci-runs-task-check and story:extraction-links-facts are reported `unassessed` by `waves`; both are implemented and not part of any open collision.
- Whether spec-organisation-types should fold into spec-standalone-types is a split question for the design critic.

```findings
[
  {
    "file": ".engineering/planning/story/spec-organisation-types.md",
    "line": 206,
    "category": "parallel-safety",
    "severity": "blocker",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "edits spec/domains/instance.yaml, generated/ and spec/suite.json (cited) like story:seen-documents-modelled, story:run-snapshots and story:adopt-existing-store; Depends on names only spec-standalone-types; add ordering edges recording the shared spec files or split the surface"
  },
  {
    "file": ".engineering/planning/story/spec-organisation-types.md",
    "line": 111,
    "category": "parallel-safety",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "diff reshapes StructuredSource into StructuredConnectors/StructuredInput and extends RedactionClass (cited), which story:structured-source and story:redaction-before-model consume (inferred consumer surface); no edge records it; add an ordering edge or split the surface"
  },
  {
    "file": ".engineering/planning/story/connectors-source-walks.md",
    "line": 49,
    "category": "parallel-safety",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "src/state.rs (cited) is also in story:seen-documents-modelled, story:adopt-existing-store and story:timer-runs-unattended (inferred) and none is named; add an ordering edge recording the file or split the surface"
  },
  {
    "file": ".engineering/planning/story/file-records.md",
    "line": 49,
    "category": "parallel-safety",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "src/state.rs (cited) is also in story:seen-documents-modelled, story:adopt-existing-store and story:timer-runs-unattended (inferred); no edge or mention; add an ordering edge recording the file or split the surface"
  },
  {
    "file": ".engineering/planning/story/structured-from-files-and-drops.md",
    "line": 48,
    "category": "parallel-safety",
    "severity": "blocker",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "src/sources.rs (cited) is also edited by story:connectors-source-walks, story:file-records and story:structured-source; edge only to structured-source though it takes paging from connectors-source-walks; add edges recording the file or split the surface"
  },
  {
    "file": ".engineering/planning/story/redaction-names-and-gate.md",
    "line": 45,
    "category": "parallel-safety",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "src/run.rs (cited) is also edited by story:run-snapshots, story:structured-source and story:timer-runs-unattended (inferred) with no edge to this story; add ordering edges recording the file or split the surface"
  },
  {
    "file": ".engineering/planning/story/run-gate.md",
    "line": 44,
    "category": "parallel-safety",
    "severity": "blocker",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "src/run.rs (cited) is also in story:redaction-names-and-gate (pre-model refusal vs post-apply restore in one run path) with no edge and no mention; add an ordering edge recording the file or split the surface"
  },
  {
    "file": ".engineering/planning/story/quality-judge.md",
    "line": 44,
    "category": "parallel-safety",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "src/lib.rs and src/main.rs (cited) are shared with story:run-gate, run-snapshots, structured-source, redaction-before-model, store-backend-per-instance, adopt-existing-store and web-pages-cited-as-urls; only edge is to spec-standalone-types; add ordering edges or split the surface"
  },
  {
    "file": ".engineering/planning/story/document-time-as-valid-time.md",
    "line": 45,
    "category": "parallel-safety",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "src/extract.rs and src/evidence.rs (cited) are also in story:codex-model-backend and story:web-pages-cited-as-urls; no edge or mention; add ordering edges recording the files or split the surface"
  }
]
```
