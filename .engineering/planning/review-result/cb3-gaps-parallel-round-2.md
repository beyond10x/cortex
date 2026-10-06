---
format: aep.planning-md/3
id: review-result:cb3-gaps-parallel-round-2
kind: review-result
status: active
title: 'Plan critic (parallel), round 2: cb3 gap stories'
relations:
- reviews: story:structured-children
- reviews: story:extraction-supersedes
- reviews: story:postgres-run-undo
- reviews: story:record-field-lookups
revision: 1
---
needs-revision
structured-children — still shares `generated/`, `spec/suite.json`, `src/model_map.rs` and `spec/domains/instance.yaml` with record-field-lookups and postgres-credential-from-connectors (cited, their Files sections), and none of the three bodies names an edge; add an ordering edge that records those files or split the surface — .engineering/planning/story/structured-children.md:54
record-field-lookups — still edits `src/sources.rs` beside structured-from-files-and-drops and `generated/`, `spec/domains/instance.yaml`, `src/model_map.rs` and `src/sources.rs` beside structured-children (cited, bodies), and its Depends names only story:file-records; add edges or accept the overlap in the body — .engineering/planning/story/record-field-lookups.md:43
extraction-supersedes — only the document-time-as-valid-time overlap on `src/extract.rs` is now ordered; it is still unordered against story:quality-judge (`src/extract.rs`, `src/main.rs`), the active story:codex-model-backend (`src/extract.rs`) and story:postgres-credential-from-connectors (`src/main.rs`), and its body names none of them (cited, their Scope sections); add edges or split — .engineering/planning/story/extraction-supersedes.md:37
extraction-supersedes — it now lists the EKR pin sites (`src/main.rs`, `examples/*.yaml`, `.github/workflows/check.yml`, `AGENTS.md`), but story:web-pages-cited-as-urls (cited, its Scope) edits the same sites and extract.rs hunks and neither has an edge to the other; add an edge naming the pin sites — .engineering/planning/story/extraction-supersedes.md:41
postgres-run-undo — it and story:structured-from-files-and-drops both edit `src/ekr.rs` (cited, both Files/Scope sections; the other story adds a read of a source's active assertions) and neither names the other; add an edge or split the surface — .engineering/planning/story/postgres-run-undo.md:46
read-answers-name-types — it edits `crates/ekr/src/cli/mcp.rs` and `crates/ekr/src/cli/view.rs`, which story:read-surface-served-over-http rewrites into one dispatcher keyed by wire name (cited, its Context and Build), and the body orders itself after read-surface-specified only; add an edge to story:read-surface-served-over-http or say which lands first — .engineering/planning/story/read-answers-name-types.md:49
explain-bounds-documents — it names story:read-surface-specified only as "when it lands it carries `offset` and `limit`", with no edge and no order, although both change the `explain` input in `systems/ekr/domains/views.yaml` and `crates/ekr/src/cli/mcp.rs` (cited, read-surface-specified Build and Acceptance), and story:read-surface-served-over-http (same files) is not named; add an ordering edge or split — .engineering/planning/story/explain-bounds-documents.md:38
specification:wave-20261005b-extraction-parity — the in-flight stories (extraction-partial-apply, extraction-valid-time, extracted-relations-visible-to-graph-reads, extraction-supersession, ocel-process-map) share `docs/cli.md` and `CHANGELOG.md` with read-answers-name-types and explain-bounds-documents, and ocel-process-map also shares `systems/ekr/domains/views.yaml` (inferred, its Scope), yet Selection records only ocel-process-map's hunk-sharing and neither story body mentions the wave; record it in Selection or in both story bodies (round-1 outcome was no-op) — .engineering/planning/specification/wave-20261005b-extraction-parity.md:30
served-channels-by-audience — `charts/company-brain/values.yaml` lies inside story:cortex-cutover's scope `charts/company-brain/` (cited, its line 30; "point the Helm values at it") and nothing orders the two, since cortex-cutover depends only on cortex-shadow-run and cortex-host-reaches-staff-db; add an ordering edge or split — .engineering/planning/story/served-channels-by-audience.md:37
postgres-store-backup — its scope `charts/company-brain/templates` (a whole directory, inferred) contains `database.yaml` and `network-policy.yaml` owned by story:cortex-host-reaches-staff-db and lies inside story:cortex-cutover's `charts/company-brain/` (cited, its line 30), with no edge to either; narrow it to the new job file or add edges — .engineering/planning/story/postgres-store-backup.md:30

What I read: 18 stories, 3 stores. I ran `aep plan artifact show --format json`, `list`, `waves` and `graph`. I checked unordered pairs that share a scope path with a script over the `depends_on` closure. I also read both stores' round-1 records.

Round-1 status (15 findings):

| round-1 finding | state |
|---|---|
| structured-children vs structured-from-files-and-drops | resolved (edge, `structured-children.md:50`) |
| structured-children generated, model_map, suite.json | open (finding 1) |
| structured-children valid time | resolved (edge to document-time-as-valid-time) |
| record-field-lookups | open (finding 2) |
| extraction-supersedes `src/extract.rs` | partly: document-time ordered; quality-judge, codex-model-backend, web-pages open (finding 3) |
| extraction-supersedes pin sites | partly: sites now listed; web-pages overlap open (finding 4) |
| extraction-supersedes upstream edge | resolved (`upstream-blocker:ekr-extraction-supersession` blocks it) |
| postgres-run-undo vs postgres-credential-from-connectors | resolved (edge); new overlap with structured-from-files-and-drops (finding 5) |
| read-answers-name-types vs explain-bounds-documents | resolved (edge, `read-answers-name-types.md:51`) |
| read-answers-name-types vs read-surface-specified | partly: edge and "twelfth operation" recorded; read-surface-served-over-http open (finding 6) |
| explain-bounds-documents vs read-surface-* | partly: describe_evidence no longer needed; order and served-over-http open (finding 7) |
| wave 20261005b docs/cli.md and changelog | open, recorded no-op (finding 8) |
| secrets-scan-without-redact | resolved (archived; retire-ingestion depends on retire-local-redaction) |
| served-channels-by-audience | partly: edge to cortex-instance-spec and typed scope added; cortex-cutover overlap open (finding 9) |
| cortex-host-reaches-staff-db | resolved (cortex-cutover depends on it; typed scope filled) |

Surfaces: all 9 revised stories placed from body Files/Scope sections (cited); their typed scopes are inferred. 0 unplaceable.

What I could not establish:
- Whether postgres-run-undo needs an EKR change. Its Open section says "unknown", so its surface may reach the EKR store.
- Whether document-time-as-valid-time moves the EKR pin first, as extraction-supersedes says. Its Scope lists no pin sites. The edge orders them either way.
- document-time-as-valid-time's body says it depends on codex-model-backend (both edit `src/extract.rs`), but there is no edge. This is pre-existing and not in the revised set.
- Typed scope is empty for read-surface-specified, read-surface-served-over-http, cortex-cutover, cortex-instance-spec and retire-ingestion. `waves` lists them unassessed, so I placed them from their bodies only.
- Out of my lane, not counted in the verdict:
  - served-channels-by-audience needs a cortex per-channel audience class that no cortex story provides.
  - cortex-host-reaches-staff-db has no edge to the cortex credential and Connectors launch stories.
  - postgres-run-undo can make a statement in docs-for-1-0 false.

```findings
[
 {"file": ".engineering/planning/story/structured-children.md", "line": 54, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "shares generated/, spec/suite.json, src/model_map.rs and spec/domains/instance.yaml with story:record-field-lookups and story:postgres-credential-from-connectors (cited, their Files sections) and no body names an edge; add an ordering edge recording those files or split the surface"},
 {"file": ".engineering/planning/story/record-field-lookups.md", "line": 43, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "edits src/sources.rs beside story:structured-from-files-and-drops and generated/, spec/domains/instance.yaml, src/model_map.rs, src/sources.rs beside story:structured-children (cited, bodies) while its Depends names only story:file-records; add edges or accept the overlap in the body"},
 {"file": ".engineering/planning/story/extraction-supersedes.md", "line": 37, "category": "parallel-safety", "severity": "blocker", "verdict": "needs-revision", "origin": "introduced", "message": "only the document-time-as-valid-time overlap on src/extract.rs is ordered; still unordered against story:quality-judge (src/extract.rs, src/main.rs), the active story:codex-model-backend (src/extract.rs) and story:postgres-credential-from-connectors (src/main.rs), none named in the body (cited, their Scope sections); add edges or split"},
 {"file": ".engineering/planning/story/extraction-supersedes.md", "line": 41, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "now lists the EKR pin sites (src/main.rs, examples/*.yaml, .github/workflows/check.yml, AGENTS.md) but story:web-pages-cited-as-urls (cited, its Scope) edits the same sites and src/extract.rs and neither has an edge to the other; add an edge naming the pin sites"},
 {"file": ".engineering/planning/story/postgres-run-undo.md", "line": 46, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "it and story:structured-from-files-and-drops both edit src/ekr.rs (cited, both Files/Scope sections) and neither names the other; add an edge or split the surface"},
 {"file": ".engineering/planning/story/read-answers-name-types.md", "line": 49, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "edits crates/ekr/src/cli/mcp.rs and crates/ekr/src/cli/view.rs, which story:read-surface-served-over-http rewrites into one dispatcher keyed by wire name (cited, its Context and Build); the body orders itself after read-surface-specified only; add an edge to read-surface-served-over-http or say which lands first"},
 {"file": ".engineering/planning/story/explain-bounds-documents.md", "line": 38, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "names story:read-surface-specified only as 'when it lands it carries offset and limit' with no edge or order, although both change the explain input in systems/ekr/domains/views.yaml and crates/ekr/src/cli/mcp.rs (cited, its Build and Acceptance); story:read-surface-served-over-http (same files) is not named; add an ordering edge or split"},
 {"file": ".engineering/planning/specification/wave-20261005b-extraction-parity.md", "line": 30, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "in-flight wave stories share docs/cli.md and CHANGELOG.md with story:read-answers-name-types and story:explain-bounds-documents, and ocel-process-map also shares systems/ekr/domains/views.yaml (inferred, its Scope), but Selection records hunk-sharing only for ocel-process-map and neither story body mentions the wave (round-1 outcome was no-op); record it in Selection or both bodies"},
 {"file": ".engineering/planning/story/served-channels-by-audience.md", "line": 37, "category": "parallel-safety", "severity": "blocker", "verdict": "needs-revision", "origin": "introduced", "message": "charts/company-brain/values.yaml lies inside story:cortex-cutover's scope charts/company-brain/ (cited, its line 30, 'point the Helm values at it') and nothing orders the two, since cortex-cutover depends only on cortex-shadow-run and cortex-host-reaches-staff-db; add an ordering edge or split"},
 {"file": ".engineering/planning/story/postgres-store-backup.md", "line": 30, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "inferred surface: scope charts/company-brain/templates (a whole directory) contains database.yaml and network-policy.yaml owned by story:cortex-host-reaches-staff-db and lies inside story:cortex-cutover's charts/company-brain/ (cited, its line 30), with no edge to either; narrow it to the new job file or add edges"}
]
```