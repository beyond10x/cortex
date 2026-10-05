---
format: aep.planning-md/3
id: review-result:org-design-round-2
kind: review-result
status: active
title: Design critic, organisation scale, round 2
relations:
- reviews: epic:organisation-scale-instance
- reviews: story:connectors-source-walks
- reviews: story:file-records
- reviews: story:redaction-names-and-gate
- reviews: story:structured-from-files-and-drops
- reviews: story:document-time-as-valid-time
- reviews: story:run-gate
- reviews: story:quality-judge
- reviews: story:spec-standalone-types
revision: 1
---
approve

I found nothing in the revised set that would change what was drafted. All three round-1 findings are closed:

- **`story:spec-organisation-types` reshape:** the `StructuredSource` reshape now lives in `story:spec-standalone-types`, under "Organisation-scale types". `story:structured-source` depends on that story, so it implements the `input` union form. The order is recorded (`depends_on`).
- **Paging claim:** `story:structured-from-files-and-drops` now carries `depends_on story:connectors-source-walks`. Its body says it "adds no second walk", and its Depends-on section names the shared `src/sources.rs`.
- **In-process restore:** `story:run-gate` names `snapshot::restore_held`, which `story:run-snapshots` provides at its line 45. Both stories edit `src/run.rs`, and `depends_on story:run-snapshots` records the order.

Nothing new:
- **Cycles:** none in `depends_on` or `blocks`.
- **Serialising chain:** none. There are two independent chains, `connectors-source-walks` → `file-records` → `structured-from-files-and-drops` and `redaction-names-and-gate` → `run-gate` → `quality-judge`. `document-time-as-valid-time` hangs off `file-records`. Each edge has its shared-file or data reason written in its story's Depends-on section.
- **Split abstraction / horizontal slice:** none. Folding the types into `story:spec-standalone-types` puts a schema layer ahead of the feature stories, but each feature story still has its own end-to-end acceptance test. No type is half-implemented in one story and half in another.
- **Hidden dependency:** none. `document-time-as-valid-time`'s files-source acceptance is covered by its edge to `file-records`.

**What I read:** the 7 stories, the epic, `story:spec-standalone-types`, `story:structured-source`, `story:run-snapshots` and `review-result:org-design-round-1`, all through `aep plan artifact show`. I also ran `relations` and `graph` (edges walked: all in the store, outside the set included, 51 artifacts) and `validate`, which printed "valid".

**What I could not establish:** none in my lane. Outside it:
- **Parallel safety:** `story:connectors-source-walks`, `story:file-records` and `story:structured-from-files-and-drops` all edit `src/sources.rs`. This round they are ordered by edges.
- **Acceptance:** `story:redaction-names-and-gate` speaks of "known_names files" while the type is `List<String>`. This is for the acceptance critic.

```findings
[]
```
