---
format: aep.planning-md/3
id: review-result:acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, round 2
relations:
- reviews: epic:first-web-instance
- reviews: story:ci-runs-task-check
- reviews: story:seen-documents-modelled
- reviews: story:extraction-links-facts
- reviews: story:timer-runs-unattended
- reviews: story:first-web-instance
- reviews: story:web-pages-cited-as-urls
revision: 1
---
needs-revision
story:first-web-instance — the three conditions all pass on an instance that never ran: item 1 is vacuous with zero runs, item 2 reads "at least as many edges as documents applied" (0 ≥ 0), and item 3's "counts as answered when the store holds no node claiming one" passes an empty store. The acceptance names no minimum number of timer-started runs and no minimum number of documents applied, and it gives no after-state for the "7 days" transition from the 0-run, 0-document start. — .engineering/planning/story/first-web-instance.md:34-44

What I read: 8 of 8 ids (epic:first-web-instance, story:ci-runs-task-check, story:seen-documents-modelled, story:extraction-links-facts, story:timer-runs-unattended, story:first-web-instance, story:web-pages-cited-as-urls, upstream-blocker:ekr-url-evidence). I ran `aep plan artifact show <id>` for each, plus `aep plan artifact show review-result:acceptance-round-1`.

Round-1 findings:
- **`story:timer-runs-unattended`:** fixed. The acceptance is now the check at line 29: every run exits 0, and `runs` equals the journal entry count. The failure-counter outcomes moved out to the conformance suite.
- **`story:first-web-instance` (named questions, edge count):** fixed on those two points. The three questions are named and the edge count is stated against the 0 baseline. The vacuous pass above is a new finding on the same acceptance, not a repeat.

What I could not establish: none.

Out of my lane, not counted toward the verdict:
- `story:timer-runs-unattended` records that `src/ports.rs:175` disables a source on its second consecutive failure. Whether that is intended is a design question.
- `upstream-blocker:ekr-url-evidence` says the upstream story is "not yet committed there". That is a fact about another repository that I did not check.

```findings
[
  {
    "file": ".engineering/planning/story/first-web-instance.md",
    "line": 34,
    "category": "acceptance",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "all three acceptance conditions pass on an instance that never ran (no runs, 0 edges >= 0 documents applied, empty store counts as answered); the acceptance names no minimum number of timer-started runs and no minimum number of documents applied, so it does not distinguish a week of work from a week of nothing"
  }
]
```
