---
format: aep.planning-md/3
id: review-result:cb3-gaps-parallel-round-1
kind: review-result
status: archived
title: 'Plan critic (parallel), round 1: cb3 gap stories'
relations:
- reviews: story:structured-children
- reviews: story:extraction-supersedes
- reviews: story:postgres-run-undo
- reviews: story:record-field-lookups
revision: 2
transitions:
- {from: "active", to: "archived", at: "2026-10-08T13:07:38Z", actor: "human:timo", revision: 2}
---
needs-revision
structured-children — its `src/structured.rs` and the Structured-input arm and `fetch_structured` of `src/sources.rs` are also edited by structured-from-files-and-drops (cited, both bodies), and neither says so; add an ordering edge naming those files or split the surface — .engineering/planning/story/structured-children.md:34
structured-children — it adds spec fields, so it must regenerate `generated/`, `spec/suite.json` and edit `src/model_map.rs` (inferred, `structured()` at `src/model_map.rs:205` builds the struct literal), none listed and all shared with record-field-lookups and postgres-credential-from-connectors; add an edge or accept the conflict in generated files — .engineering/planning/story/structured-children.md:34
structured-children — its acceptance "each event's valid time equals its record time" goes through the evidence observed-time path that story:document-time-as-valid-time owns (inferred, `src/evidence.rs:55`) and so waits on upstream-blocker:ekr-extraction-valid-time, and the body names neither — .engineering/planning/story/structured-children.md:28
record-field-lookups — it edits `spec/domains/instance.yaml` and `src/sources.rs` beside structured-children and `src/sources.rs` beside structured-from-files-and-drops (cited, bodies; different functions, same file) and names no edge to either; add edges or accept — .engineering/planning/story/record-field-lookups.md:32
extraction-supersedes — it edits the prompt and facts in `src/extract.rs` (`SYSTEM_PROMPT`, `admitted_facts`, `merge`) as do story:document-time-as-valid-time, story:quality-judge, story:web-pages-cited-as-urls and the active story:codex-model-backend (cited, their Scope sections), and the body mentions none of them; add edges or split — .engineering/planning/story/extraction-supersedes.md:34
extraction-supersedes — "cortex pins it" moves the EKR pin (inferred sites: `src/main.rs:143`, `examples/*.yaml`, `.github/workflows/check.yml:21`, `AGENTS.md:149-166`) but lists none, and story:web-pages-cited-as-urls (cited) names the same pin sites — .engineering/planning/story/extraction-supersedes.md:32
extraction-supersedes — the EKR release it needs is recorded as prose only, with no edge to upstream-blocker:ekr-extraction-supersession, so `aep plan artifact waves` places it in wave 1 beside record-field-lookups as workable now — .engineering/planning/story/extraction-supersedes.md:30
postgres-run-undo — its acceptance is the same docker case of `tests/store_backend.rs` that story:postgres-credential-from-connectors rewrites (cited, its line 49), and an undo that calls `ekr` goes through `Store::cmd()` in `src/ekr.rs`, which that story changes (inferred); neither says so; add an edge or split — .engineering/planning/story/postgres-run-undo.md:36
read-answers-name-types — it shares `crates/ekr/src/cli/mcp.rs` and `docs/cli.md` with story:explain-bounds-documents (cited, both Files sections), and putting names into `explain` answers lands in `crates/ekr/src/cli/explain.rs` (inferred, unlisted), which that story owns; neither names the other — .engineering/planning/story/read-answers-name-types.md:37
read-answers-name-types — the overlap with story:read-surface-specified is a prose remark ("one owner should take both") with no edge or decision, and the new type read is a 12th operation in `systems/ekr/domains/views.yaml`, `mcp.rs` and `view.rs` beyond that story's list of 11 and its dispatcher rewrite in story:read-surface-served-over-http (cited) — .engineering/planning/story/read-answers-name-types.md:37
explain-bounds-documents — a bound argument and `truncated`/full-length fields change the `explain` tool's input schema, which story:read-surface-specified models as a view command and checks against `tools/list` (cited, its Build and Acceptance), and the "way to read the rest" may be the `describe_evidence` tool that story:read-surface-served-over-http adds; the body names neither — .engineering/planning/story/explain-bounds-documents.md:31
read-answers-name-types — it and story:explain-bounds-documents edit `docs/cli.md` and the `[Unreleased]` changelog (neither lists `CHANGELOG.md`) beside the open wave 20261005b stories, but the wave's Selection records that hunk-sharing only for ocel-process-map — .engineering/planning/specification/wave-20261005b-extraction-parity.md:30
secrets-scan-without-redact — it and story:retire-local-redaction claim one outcome, `task secrets` calling a working detector once `src/redact.rs` is gone (cited, retire-local-redaction line 45), and story:retire-ingestion also edits `Taskfile.yml` (line 26); none says so, and the typed scope is empty so `waves` lists it unassessed; add an edge or split — .engineering/planning/story/secrets-scan-without-redact.md:24
served-channels-by-audience — `charts/company-brain/values.yaml` lies inside story:cortex-cutover's scope `charts/company-brain/` (line 29), and the `deploy/cortex/` spec it edits is a new file (it does not exist yet) that story:cortex-instance-spec creates (line 26); "(with story:cortex-instance-spec)" records no edge, and the typed scope is empty so `waves` lists it unassessed — .engineering/planning/story/served-channels-by-audience.md:28
cortex-host-reaches-staff-db — `charts/company-brain/templates/database.yaml` and `network-policy.yaml` lie inside story:cortex-cutover's scope `charts/company-brain/` (cited, its line 29) with no edge, and the typed scope is empty so `waves` lists it unassessed — .engineering/planning/story/cortex-host-reaches-staff-db.md:24

What I read:
- 9 stories, plus the 5 named cortex drafts, 6 EKR wave and read-surface stories, and 3 cb3 stories' neighbours (retire-ingestion, retire-local-redaction, cortex-cutover, cortex-instance-spec).
- The other open stories of each store, by scope.
- The commands were `aep plan artifact show/list/waves` in all three stores and `git grep` on the cortex and EKR trees.
- The cortex stories were read from `~/beyond10x/cortex` `main`, because PR #22 merged while I worked (883256d).
- Surfaces: 9 of 9 placed from body Files sections (cited, all marked "unverified" by the drafters; I confirmed the cortex symbols exist). Every typed scope on the cortex and EKR stories is `inferred`. 3 of 9 (the cb3 stories) have an empty typed scope. 0 unplaceable by body.

What I could not establish:
- Whether `postgres-run-undo` needs an EKR change: the body says "unknown", so its surface may extend into the EKR store.
- Which `explain`/`timeline` code carries the type ids that `read-answers-name-types` must name. I assumed `crates/ekr/src/cli/explain.rs` and `crates/ekr-views/src/timeline.rs`.
- Out of my lane, not counted in the verdict:
  - `served-channels-by-audience` needs a per-channel audience class that no cortex story provides.
  - `cortex-host-reaches-staff-db` depends on the cortex credential story and the Connectors launch, with no edge.
  - `postgres-run-undo` can make a statement in `docs-for-1-0` false.

```findings
[
 {
  "file": ".engineering/planning/story/structured-children.md",
  "line": 34,
  "category": "parallel-safety",
  "severity": "blocker",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "src/structured.rs and the Structured-input arm and fetch_structured of src/sources.rs are also edited by story:structured-from-files-and-drops (cited, both bodies) and neither body says so; add an ordering edge naming those files or split the surface"
 },
 {
  "file": ".engineering/planning/story/structured-children.md",
  "line": 34,
  "category": "parallel-safety",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "inferred - adding spec fields regenerates generated/ and spec/suite.json and edits src/model_map.rs, none listed, all shared with story:record-field-lookups and story:postgres-credential-from-connectors; add an edge or accept the conflict in generated files"
 },
 {
  "file": ".engineering/planning/story/structured-children.md",
  "line": 28,
  "category": "parallel-safety",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "inferred - the acceptance that an event's valid time equals its record time goes through the evidence observed-time path (src/evidence.rs) owned by story:document-time-as-valid-time and blocked on upstream-blocker:ekr-extraction-valid-time; the body names neither"
 },
 {
  "file": ".engineering/planning/story/record-field-lookups.md",
  "line": 32,
  "category": "parallel-safety",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "shares spec/domains/instance.yaml and src/sources.rs with story:structured-children and src/sources.rs with story:structured-from-files-and-drops (cited, bodies; different functions, same file) with no edge to either; add edges or accept"
 },
 {
  "file": ".engineering/planning/story/extraction-supersedes.md",
  "line": 34,
  "category": "parallel-safety",
  "severity": "blocker",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "edits SYSTEM_PROMPT, admitted_facts and merge in src/extract.rs, also edited by story:document-time-as-valid-time, story:quality-judge, story:web-pages-cited-as-urls and the active story:codex-model-backend (cited, their Scope sections); the body mentions none; add edges or split"
 },
 {
  "file": ".engineering/planning/story/extraction-supersedes.md",
  "line": 32,
  "category": "parallel-safety",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "inferred - \"cortex pins it\" moves the EKR pin (src/main.rs:143, examples/*.yaml, .github/workflows/check.yml:21, AGENTS.md:149-166), none listed, and story:web-pages-cited-as-urls cites the same pin sites"
 },
 {
  "file": ".engineering/planning/story/extraction-supersedes.md",
  "line": 30,
  "category": "parallel-safety",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "the dependency on the unreleased EKR supersession is prose only, with no edge to upstream-blocker:ekr-extraction-supersession, so aep plan artifact waves places it in wave 1 as workable now"
 },
 {
  "file": ".engineering/planning/story/postgres-run-undo.md",
  "line": 36,
  "category": "parallel-safety",
  "severity": "blocker",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "its acceptance is the same docker case of tests/store_backend.rs that story:postgres-credential-from-connectors rewrites (cited), and (inferred) an undo that calls ekr goes through Store::cmd() in src/ekr.rs, which that story changes; neither body says so; add an edge or split"
 },
 {
  "file": ".engineering/planning/story/read-answers-name-types.md",
  "line": 37,
  "category": "parallel-safety",
  "severity": "blocker",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "shares crates/ekr/src/cli/mcp.rs and docs/cli.md with story:explain-bounds-documents (cited, both Files sections); naming types in explain answers lands in crates/ekr/src/cli/explain.rs (inferred, unlisted), which that story owns; neither body names the other"
 },
 {
  "file": ".engineering/planning/story/read-answers-name-types.md",
  "line": 37,
  "category": "parallel-safety",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "the overlap with story:read-surface-specified is a prose remark with no edge or decision; the new type read adds a 12th operation to views.yaml, mcp.rs and view.rs beyond that story's 11 and the dispatcher rewrite of story:read-surface-served-over-http (cited)"
 },
 {
  "file": ".engineering/planning/story/explain-bounds-documents.md",
  "line": 31,
  "category": "parallel-safety",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "a bound argument and truncated/full-length fields change the explain tool's input schema that story:read-surface-specified models and checks against tools/list (cited), and the way to read the rest may be the describe_evidence tool story:read-surface-served-over-http adds; the body names neither"
 },
 {
  "file": ".engineering/planning/specification/wave-20261005b-extraction-parity.md",
  "line": 30,
  "category": "parallel-safety",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "story:read-answers-name-types and story:explain-bounds-documents edit docs/cli.md and the [Unreleased] changelog beside the open wave 20261005b stories, but the wave Selection records that hunk-sharing only for ocel-process-map"
 },
 {
  "file": ".engineering/planning/story/secrets-scan-without-redact.md",
  "line": 24,
  "category": "parallel-safety",
  "severity": "blocker",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "claims the same outcome as story:retire-local-redaction (task secrets calling a working detector without src/redact.rs; cited, its line 45) and shares Taskfile.yml with story:retire-ingestion (line 26); none says so, and the empty typed scope leaves it unassessed in waves; add an edge or split"
 },
 {
  "file": ".engineering/planning/story/served-channels-by-audience.md",
  "line": 28,
  "category": "parallel-safety",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "charts/company-brain/values.yaml lies inside story:cortex-cutover's scope (line 29) and deploy/cortex/ is a file that does not exist yet, created by story:cortex-instance-spec (line 26); the \"(with story:cortex-instance-spec)\" remark records no edge, and the typed scope is empty so waves lists it unassessed"
 },
 {
  "file": ".engineering/planning/story/cortex-host-reaches-staff-db.md",
  "line": 24,
  "category": "parallel-safety",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "charts/company-brain/templates/database.yaml and network-policy.yaml lie inside story:cortex-cutover's scope charts/company-brain/ (cited, line 29) with no edge, and the typed scope is empty so waves lists it unassessed"
 }
]
```