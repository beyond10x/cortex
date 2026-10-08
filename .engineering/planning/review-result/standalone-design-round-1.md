---
format: aep.planning-md/3
id: review-result:standalone-design-round-1
kind: review-result
status: archived
title: Design critic, standalone 1.0, round 1
relations:
- reviews: epic:standalone-1-0
- reviews: story:spec-standalone-types
- reviews: story:store-backend-per-instance
- reviews: story:redaction-before-model
- reviews: story:structured-source
- reviews: story:codex-model-backend
- reviews: story:run-snapshots
- reviews: story:adopt-existing-store
- reviews: story:release-pipeline
- reviews: story:docs-for-1-0
- reviews: story:release-1-0
- reviews: story:web-pages-cited-as-urls
revision: 2
transitions:
- {from: "active", to: "archived", at: "2026-10-08T13:07:39Z", actor: "human:timo", revision: 2}
---
needs-revision
story:redaction-before-model — its Work never says it lifts the `redaction` refusal that `story:spec-standalone-types` adds, and its e2e acceptance cannot create an instance with `redaction` while that refusal stands; add the lift to Work (store-backend-per-instance already does this for postgres) — .engineering/planning/story/redaction-before-model.md:29 vs .engineering/planning/story/spec-standalone-types.md:47
story:structured-source — its Work never says it lifts the `structured` source-kind refusal, so its e2e acceptance cannot create the instance; add the lift to Work — .engineering/planning/story/structured-source.md:30 vs .engineering/planning/story/spec-standalone-types.md:47
story:codex-model-backend — its Work never says it lifts the `model.backend: Codex` refusal, so its e2e acceptance cannot create the instance; add the lift to Work — .engineering/planning/story/codex-model-backend.md:22 vs .engineering/planning/story/spec-standalone-types.md:47
story:run-snapshots — its Work never says it lifts the `snapshots` refusal, so its e2e acceptance (`snapshots.keep: 2`) cannot create the instance; add the lift to Work — .engineering/planning/story/run-snapshots.md:36 vs .engineering/planning/story/spec-standalone-types.md:47
story:codex-model-backend — it enforces the budget by a "per-run call and token limit", but `ModelSpec` carries only `budget_usd` and `timeout_s` and no story adds a limit field, so half of the Codex abstraction is unowned; either add the field to the types in `story:spec-standalone-types` (with an edge) or state a fixed default limit in this body — .engineering/planning/story/codex-model-backend.md:27 and spec/domains/instance.yaml:180
story:web-pages-cited-as-urls — the graph has `depends_on story:codex-model-backend` but the body's "Depends on" section and the epic order table name no reason for it, so the edge and its cause (shared `src/extract.rs`) are unrecorded in prose; add it to the section — .engineering/planning/story/web-pages-cited-as-urls.md:46 and .engineering/planning/story/web-pages-cited-as-urls.md:12

**What you read:** 11 stories in the drafted set plus the epic, the 5 related `first-web-instance` stories and epic, and the `instance.yaml` diff. Commands: `aep plan artifact show` on each, `relations`, `graph`, `validate`. I walked all 100+ declared edges, including those outside the set. There is no `needs-first` cycle (`depends_on` and `blocks`), and `validate` printed "valid".

**What I could not establish:**
- Whether the `cortex create` refusal for non-default settings lives in `src/spec.rs` or `src/instance.rs`. The feature-story scopes mostly omit both files, which belongs to parallel-safety.
- The spine `spec` → `redaction` → `structured` → `snapshots` → `adopt` → `docs` → `release-1-0` runs 7 deep, and its middle edges exist only because of shared `src/run.rs`, spec and `src/main.rs` edits. The reasons are written beside each edge, so I did not report it. Splitting that surface into e.g. `src/snapshot.rs` would shorten it.
- Out of my lane: `story:extraction-links-facts`, `story:codex-model-backend` and `story:spec-standalone-types` all edit `src/extract.rs` with no edge between the first two (parallel-safety). `story:adopt-existing-store` describes a "first run applies only documents EKR does not already cite" behaviour that needs `src/run.rs`, which is outside its scope (scope and acceptance).
- `story:spec-standalone-types` bundles a test-fixture move and the optional-cost change into the types story. Both are recorded and ordered by an edge, so I treated them as a stated trade-off and not a defect.

```findings
[
  {"file": ".engineering/planning/story/redaction-before-model.md", "line": 29, "category": "design", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "its Work never says it lifts the `redaction` refusal that `story:spec-standalone-types` adds, and its e2e acceptance cannot create an instance with `redaction` while that refusal stands; add the lift to Work (store-backend-per-instance already does this for postgres)"},
  {"file": ".engineering/planning/story/structured-source.md", "line": 30, "category": "design", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "its Work never says it lifts the `structured` source-kind refusal, so its e2e acceptance cannot create the instance; add the lift to Work"},
  {"file": ".engineering/planning/story/codex-model-backend.md", "line": 22, "category": "design", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "its Work never says it lifts the `model.backend: Codex` refusal, so its e2e acceptance cannot create the instance; add the lift to Work"},
  {"file": ".engineering/planning/story/run-snapshots.md", "line": 36, "category": "design", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "its Work never says it lifts the `snapshots` refusal, so its e2e acceptance (`snapshots.keep: 2`) cannot create the instance; add the lift to Work"},
  {"file": ".engineering/planning/story/codex-model-backend.md", "line": 27, "category": "design", "severity": "blocker", "verdict": "needs-revision", "origin": "introduced", "message": "it enforces the budget by a \"per-run call and token limit\", but `ModelSpec` carries only `budget_usd` and `timeout_s` and no story adds a limit field, so half of the Codex abstraction is unowned; either add the field to the types in `story:spec-standalone-types` (with an edge) or state a fixed default limit in this body"},
  {"file": ".engineering/planning/story/web-pages-cited-as-urls.md", "line": 46, "category": "design", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the graph has `depends_on story:codex-model-backend` but the body's \"Depends on\" section names no reason for it, so the edge and its cause (shared `src/extract.rs`) are unrecorded in prose; add it to the section"}
]
```
