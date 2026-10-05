---
format: aep.planning-md/3
id: review-result:org-scope-round-2
kind: review-result
status: active
title: Scope critic, organisation scale, round 2
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
needs-revision
story:connectors-source-walks — the epic promises "paged and nested Connectors reads", and the types story gives `ChildCall` an `optional paging` field (`cortex.instance.ChildCall`: `paging: Optional<Paging>`). The ChildCall bullet names only `operation` and `input` and never says whether the child's pages are read, and no other story claims it. Under the types story's own rule, `main` parses a setting and does not act on it until a feature story gives it behaviour, so this field has no feature story. Say in the bullet that the child answer's pages are walked, or state that a child call reads one page and drop the field — .engineering/planning/story/connectors-source-walks.md:32 (promise: .engineering/planning/epic/organisation-scale-instance.md:14; type: .engineering/planning/story/spec-standalone-types.md:102)

Both round-1 findings are fixed.
- **Paged structured reads:** `story:structured-from-files-and-drops` now has a Work bullet that wires `paging` into the structured path and reuses the walk from `connectors-source-walks`. No other story claims it.
- **Partial apply and graph edges:** the epic now says these two EKR stories block no story in this epic and that the consumer's repository tracks them. That is a named omission, not a gap.

I extracted 7 promises from the epic outcome plus the types-land-first sentence. All 8 trace to an item and 7 are claimed exactly once. The eighth, nested reads, is claimed only partly, as in the finding above. Nothing reaches beyond the epic's exclusions, and no two items claim the same outcome.

Out of my lane: no acceptance covers paged structured reads (acceptance critic), and whether `known_names: List<String>` is meant to hold file paths (design critic). The earlier question about connector-sourced documents in `document-time-as-valid-time` also stays with the acceptance critic.

```findings
[
  {
    "file": ".engineering/planning/story/connectors-source-walks.md",
    "line": 32,
    "category": "scope",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "the epic promises paged and nested Connectors reads and the types story gives ChildCall an optional paging field, but the ChildCall bullet says nothing about walking the child's pages and no other story claims it, so the field has no feature story that gives it behaviour; state that child pages are read, or drop the field"
  }
]
```
