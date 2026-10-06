---
format: aep.planning-md/3
id: review-result:feeds-design-round-1
kind: review-result
status: active
title: 'Plan critic (design), round 1: datasource feeds'
relations:
- reviews: story:feed-source
revision: 1
---
needs-revision

epic:generic-datasource-feeds — nothing in the decomposition owns realizing a feed binding in the catalog engine, yet the four binding stories all land under `adapters/catalog/contracts/feed/`, and the catalog contract says the engine is unary `generic-http` and `generic-http-page` only and serves no other datasource; add a story the four bindings `depends_on`, as `story:read-post-capability` was for websearch — `.engineering/planning/epic/generic-datasource-feeds.md:27-31`, `contracts/catalog/v1alpha1/semantics.md:139`
story:feed-contract — the body names two operations but not how a binding marks them as the family's (fixed operation ids, as `websearch.search` is, or a descriptor marker), so `story:feed-bindings-discoverable` defers to "whichever the CLI contract settles" and a consumer cannot know which operations to call; state the marker here — `.engineering/planning/story/feed-contract.md:23`, `.engineering/planning/story/feed-bindings-discoverable.md:19`
story:feed-source — `dropped: Supersede` is described as reuse of the `story:structured-from-files-and-drops` machinery, but that machinery is `structured`-only (`end_dropped`) and spec validation refuses Supersede where the model extracts values from the same records, so the body does not say how a deleted feed item's extracted values end; decide that and add `depends_on story:structured-from-files-and-drops` — `.engineering/planning/story/feed-source.md:38`, `src/run.rs:924`, `src/spec.rs:140-162`
story:feed-source — the `lookups` option and the seen-state watermark build on `story:record-field-lookups` and `story:connectors-source-walks` (`SeenState`), and the graph shows only `decomposes` and `serves` for this story; add `depends_on` to both — `.engineering/planning/story/feed-source.md:35-37`, `aep plan artifact graph` (cortex store)
story:feed-source — the requirement is quoted as "any datasource contract" but only the feed family is delivered, so every provider without a feed binding stays readable only through the existing `connectors` kind, and the body states neither that this kind is kept nor what happens when a feed source and a `connectors` source read one connection (they would cite the same content under different identities); state "keep, coexisting" and add a coexistence refusal beside `src/spec.rs:140` — `.engineering/planning/story/feed-source.md:31`, `src/spec.rs:140-162`
story:feed-source — acceptance runs only against a stand-in `connectors`, while the blocker clears on a release that has a real binding, so no item shows cortex reading a real binding and the stand-in's discovery and container answers can drift from the real CLI unobserved; add a step against the first released binding, or a recorded real-CLI transcript — `.engineering/planning/story/feed-source.md:40-46`, `.engineering/planning/upstream-blocker/connectors-feed-family.md:13`

**Verdict on the questions you asked**
- **Delivers the requirement:** partly. It meets "no cortex rebuild for a new feed-capable connector" (acceptance 4). It does not meet "any datasource contract": records, logs and series providers are not covered, and the narrowing is unrecorded.
- **Overlap:**
  - `story:catalog-slack-reads` is raw provider operations, and the binding layers on it with an edge. That is clean.
  - `datasource.records` is the item body envelope. Its `document` profile is proposed and unimplemented (the first finding above).
  - `story:structured-children` and `story:structured-from-files-and-drops` produce typed nodes. Feed produces documents. They do not overlap if the story states which one it produces. The `dropped` finding above is the one seam.
- **Retire or keep the per-operation `connectors` source:** keep it. `story:structured-children` builds on its `ChildCall` and `StructuredConnectors`, and it is the only path for providers with no feed binding.
- **Ordering:** no cycle, and no serialising chain. The shape is contract, then discoverability, then four parallel bindings, and Slack alone is gated on `story:catalog-slack-reads`.

**What I read**
- 8 Connectors artifacts and 3 cortex artifacts, plus `story:record-field-lookups`, `story:connectors-source-walks` and `story:read-post-capability`.
- Commands: `aep plan artifact show`, `relations`, `graph` and `validate` in both stores. Both validators were clean.
- The Connectors store was read from an `origin/main` export, because the local checkout is detached at `2086402`, one commit behind. I deleted the export afterwards.

**Edges walked**
- About 45 in the Connectors graph, including outside the set (`story:catalog-confluence-reads`, `story:catalog-grafana-reads`).
- 3 in the cortex graph. No cycle in either.

**What I could not establish**
- Whether a feed binding could be realized as data in the engine, or needs per-provider code. That is the open question in the first finding.
- Out of my lane, and they did not set the verdict:
  - **Parallel safety:** `story:feed-source` shares `spec/domains/instance.yaml`, `src/sources.rs` and `generated/` with `story:structured-children` and `story:postgres-credential-from-connectors`, with no ordering edge. The four bindings probably share `adapters/catalog/generated/bundles/index.json`.
  - **Acceptance:** the "consumer" in `story:feed-bindings-discoverable`'s fixture-adapter test is not named.

```findings
[
 {
  "file": ".engineering/planning/epic/generic-datasource-feeds.md",
  "line": 27,
  "category": "design",
  "severity": "blocker",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "nothing in the decomposition owns realizing a feed binding in the catalog engine, which serves only unary generic-http and generic-http-page, so the four binding stories each need work no story owns; add a story the four bindings depend_on"
 },
 {
  "file": ".engineering/planning/story/feed-contract.md",
  "line": 23,
  "category": "design",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "the body names two operations but not how a binding marks them as the family's (operation ids or descriptor marker), so story:feed-bindings-discoverable defers it to whichever the CLI contract settles; state the marker in the contract"
 },
 {
  "file": ".engineering/planning/story/feed-source.md",
  "line": 38,
  "category": "design",
  "severity": "blocker",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "dropped Supersede is described as reuse of the structured-only drop machinery (end_dropped), which spec validation refuses where the model extracts values from the same records, so the body does not say how a deleted feed item's values end; decide that and add depends_on story:structured-from-files-and-drops"
 },
 {
  "file": ".engineering/planning/story/feed-source.md",
  "line": 35,
  "category": "design",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "the lookups option and the seen-state watermark build on story:record-field-lookups and story:connectors-source-walks, and the graph records only decomposes and serves; add depends_on to both"
 },
 {
  "file": ".engineering/planning/story/feed-source.md",
  "line": 31,
  "category": "design",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "the requirement is quoted as any datasource contract but only the feed family is delivered, and the body states neither that the existing connectors source kind is kept nor what happens when a feed source and a connectors source read one connection; state keep and coexisting, and add a coexistence refusal beside the existing spec check"
 },
 {
  "file": ".engineering/planning/story/feed-source.md",
  "line": 40,
  "category": "design",
  "severity": "warning",
  "verdict": "needs-revision",
  "origin": "introduced",
  "message": "acceptance runs only against a stand-in connectors while the blocker clears on a real binding, so no item shows cortex reading a real binding; add a step against the first released binding or a recorded real-CLI transcript"
 }
]
```