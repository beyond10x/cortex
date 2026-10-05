---
format: aep.planning-md/3
id: review-result:standalone-design-round-2
kind: review-result
status: active
title: Design critic, standalone 1.0, round 2
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
revision: 1
---
needs-revision
story:spec-standalone-types — its Work makes only the model answer's cost optional and claims a no-cost backend then "needs no change to `src/run.rs`", but the run report's `cost_usd` is a summed `f64` (`Report.cost_usd`, `SourceRan.cost_usd: Decimal`, `src/ports.rs:266`, `src/main.rs:483,677`), so `story:codex-model-backend`'s acceptance `"cost_usd": null` has no owner. `story:structured-source` expects "cost 0" for the same no-cost case, and no body states which rule applies. Either make the report and `SourceRan` cost optional here, with the rule for an all-`None` run versus a no-model run and `src/ports.rs` and `src/main.rs` added to scope, or change Codex's acceptance to `0` — `.engineering/planning/story/spec-standalone-types.md:56-59` vs `.engineering/planning/story/codex-model-backend.md:38`, `src/run.rs:35,198,253` and `spec/domains/instance.yaml:750`

**What you read:** 11 stories and 1 epic in the standalone set, plus the 6 `first-web-instance` stories and its epic. I ran `aep plan artifact show` on each, plus `relations`, `graph`, `waves`, `validate` ("valid"), the working-tree diff of `spec/domains/instance.yaml` and `src/run.rs`, `src/ports.rs` and `src/extract.rs`. I followed all 150 or so declared edges, including those outside the set. I found no cycle in `depends_on` or `blocks`.

**Round-1 findings:**
- Fixed: the four refusal-lift findings. The spec story now adds no refusals, so no feature story has anything to lift.
- Fixed: the Codex budget limit. It is now "no new spec field", bounded by existing limits.
- Fixed: the `web-pages-cited-as-urls` edge to `codex-model-backend` is now in that story's Depends-on section.

**What I could not establish:**
- The spec-first split (types in `story:spec-standalone-types`, behaviour in each feature story) leaves `main` silently ignoring `store.backend: postgres` and `redaction` until those stories land. The epic names this and rules out a release in between, so I treated it as a stated trade-off, not a defect.
- The spine spec → redaction → structured → snapshots → adopt → docs → release-1-0 is still 7 deep, and its reasons (shared files) are written beside each edge. I did not report it, as in round 1.
- Out of my lane: the `aep plan artifact waves` collisions are all between stories that already have an ordering edge. The `src/run.rs` gap for `story:adopt-existing-store` was already noted in round 1 and is unchanged.

```findings
[
  {"file": ".engineering/planning/story/spec-standalone-types.md", "line": 59, "category": "design", "severity": "blocker", "verdict": "needs-revision", "origin": "introduced", "message": "its Work makes only the model answer's cost optional and claims a no-cost backend then needs no change to `src/run.rs`, but the run report's `cost_usd` is a summed `f64` (`Report.cost_usd`, `SourceRan.cost_usd: Decimal`, `src/ports.rs:266`, `src/main.rs:483,677`), so `story:codex-model-backend`'s acceptance `\"cost_usd\": null` has no owner and `story:structured-source` expects \"cost 0\" for the same no-cost case; either make the report and `SourceRan` cost optional here, with the rule for an all-`None` run versus a no-model run and `src/ports.rs` and `src/main.rs` added to scope, or change Codex's acceptance to `0`"}
]
```
