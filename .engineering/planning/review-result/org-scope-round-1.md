---
format: aep.planning-md/3
id: review-result:org-scope-round-1
kind: review-result
status: active
title: Scope critic, organisation scale, round 1
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
revision: 1
---
needs-revision
story:structured-from-files-and-drops — the epic promises "paged and nested Connectors reads" and the spec adds `paging` to `StructuredConnectors`; this body says "now with `paging`", but its Work and Acceptance have no paging step and `story:connectors-source-walks` claims paging only for a `connectors` source, so paged structured imports are claimed by no one — .engineering/planning/story/structured-from-files-and-drops.md:28 (promise: .engineering/planning/epic/organisation-scale-instance.md:14; connectors-source-walks.md:24)
epic:organisation-scale-instance — the epic names "partial apply in extraction" and "relations as graph edges" as upstream EKR stories, "`upstream-blocker:*` records below", but only `ekr-extraction-supersession` and `ekr-extraction-valid-time` exist and no drafted story depends on or claims the other two (the nearest home is `story:run-gate`, which counts `facts_refused` and `documents_applied`) — .engineering/planning/epic/organisation-scale-instance.md:38

What I read: 12 artifacts (the epic, the 8 stories, the vision, `story:structured-source` and `story:run-snapshots`), plus the `aep plan artifact list` and `graph` output and `.engineering/drafts/org/blockers.md`. I extracted 7 promises from the epic's outcome and traced all 7 to an item. Six are claimed exactly once: file records, redaction and gate, structured imports, document time, run gate, quality judge. The seventh, paged and nested Connectors reads, is claimed for the `connectors` source only (first finding). Nothing in the set reaches beyond the epic's exclusions, and I found no two items claiming the same outcome.

Could not establish: I did not see the drafter's report, so I can't tell whether partial apply and graph edges were left out on purpose. Out of my lane: whether `known_names: List<String>` is meant to hold file paths (the redaction story says "`known_names` files"), and whether the `document-time-as-valid-time` acceptance covers connector-sourced documents. Both are for the design and acceptance critics.

```findings
[
  {
    "file": ".engineering/planning/story/structured-from-files-and-drops.md",
    "line": 28,
    "category": "scope",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "the epic promises paged Connectors reads and the spec adds paging to StructuredConnectors, but this body says \"now with paging\" while its Work and Acceptance have no paging step and story:connectors-source-walks covers only a connectors source, so paged structured imports are claimed by no one"
  },
  {
    "file": ".engineering/planning/epic/organisation-scale-instance.md",
    "line": 38,
    "category": "scope",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "the epic names partial apply in extraction and relations as graph edges as upstream EKR stories with upstream-blocker records, but no such blocker exists and no drafted story depends on or claims either"
  }
]
```
