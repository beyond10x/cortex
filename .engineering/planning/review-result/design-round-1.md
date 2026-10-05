---
format: aep.planning-md/3
id: review-result:design-round-1
kind: review-result
status: active
title: Design critic, round 1
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
epic:first-web-instance — the Order line (1, then 2 and 3, then 4, then 5) disagrees with the declared edges: story:seen-documents-modelled and story:extraction-links-facts carry no edge to story:ci-runs-task-check, and story:timer-runs-unattended has no edge to either of them; revise the line or add the `depends_on` edges, whichever the drafter means — .engineering/planning/epic/first-web-instance.md:37; `aep plan artifact graph` (timer-runs-unattended depends_on only ci-runs-task-check)
story:timer-runs-unattended — its acceptance asserts on `runs` and `consecutive_failures` state, and story:seen-documents-modelled regenerates `generated/` and edits `src/state.rs` / `src/home.rs`; no edge records which lands first, so add `depends_on story:seen-documents-modelled` or drop 2 from the epic's pre-4 order — .engineering/planning/story/seen-documents-modelled.md:35; .engineering/planning/story/timer-runs-unattended.md:29

What I read: 8 artifacts via `aep plan artifact show`, plus `relations`, `graph` and `validate`. The graph run includes the edges out to vision:self-updating-instances, and I found no cycle. Validate printed "valid".
Not established: why story:timer-runs-unattended needs story:ci-runs-task-check. Its body gives no reason and the edge may be only the epic's ordering; if so, it should be stated. Unease, not a finding: the edge types in story:first-web-instance's seed (`examples/seed/agent-tooling.yaml`) and the seed schema story:extraction-links-facts may change (`examples/seed/schema.yaml`) are two declarations of one concern. Out of my lane (parallel safety): story:seen-documents-modelled and story:timer-runs-unattended may both touch `src/`.

```findings
[
  {
    "file": ".engineering/planning/epic/first-web-instance.md",
    "line": 37,
    "category": "design",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "the Order line (1, then 2 and 3, then 4, then 5) disagrees with the declared edges: stories 2 and 3 have no edge to story 1, and story 4 has no edge to 2 or 3"
  },
  {
    "file": ".engineering/planning/story/seen-documents-modelled.md",
    "line": 35,
    "category": "design",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "story:timer-runs-unattended asserts on runs and consecutive_failures state that story:seen-documents-modelled may regenerate in src/state.rs, and no depends_on edge records the order"
  }
]
```
