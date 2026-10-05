---
format: aep.planning-md/3
id: review-result:parallel-safety-round-2
kind: review-result
status: active
title: Parallel-safety critic, round 2
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
story:first-web-instance — it has no edge to or from `story:web-pages-cited-as-urls`, which changes the EKR pin (`src/main.rs:89`) and the evidence kind written into the store, so that story can land mid-way through the 7-day run and its day-8 reads of `ekr view` and `ekr mcp` `search` can mix `!HumanStatement` and `!Url` evidence. Neither body says whether it may land during the 7 days (inferred, from the tree) — .engineering/planning/story/first-web-instance.md:52

**What I read:** all 6 stories with `aep plan artifact show`, plus `review-result:parallel-safety-round-1`, `aep plan artifact graph`, `aep plan artifact waves`, and `src/run.rs`, `src/extract.rs`, `src/evidence.rs`, `tests/`, `examples/` and `spec/domains/instance.yaml` in the tree.

**Round-1 outcome:** the 4 round-1 findings are fixed.
- `story:web-pages-cited-as-urls` now has `depends_on` edges to `story:ci-runs-task-check` and `story:extraction-links-facts`, and its scope names `src/extract.rs` and the pin files.
- `story:timer-runs-unattended` now has `depends_on` an edge to `story:seen-documents-modelled`, and its scope names `src/state.rs` and `src/home.rs`.
- `story:first-web-instance` now has `depends_on` an edge to `story:seen-documents-modelled`.

**Surfaces:** 6 cited, 0 inferred, 0 unplaced. The one finding above rests on inferred reasoning about the running instance, not on a shared file. The other unordered pairs have disjoint cited scopes: `story:seen-documents-modelled` / `story:extraction-links-facts`, `story:timer-runs-unattended` / `story:extraction-links-facts`, and `story:web-pages-cited-as-urls` against both `story:seen-documents-modelled` and `story:timer-runs-unattended`.

Both remedies are open: add an ordering edge recording the running instance as the reason, or state in the body that the two may overlap. I choose neither.

**Not established:**
- `aep plan artifact waves` still prints "0 wave(s), 6 unassessed" because no story has a scope recorded with `aep artifact scope`. The CLI reports this itself, so I do not count it as a finding.
- `src/run.rs:128-232` is a possible shared file between `story:timer-runs-unattended` (forced fixes), `story:seen-documents-modelled` (it uses `SeenState`) and `story:extraction-links-facts` (it calls `extract::prompt`, `extract::merge` and `extract::entity_names`). Neither of the last two lists it, and I could not tell whether either would actually need to edit it, so I do not count it as a finding.
- `story:web-pages-cited-as-urls` is blocked by `upstream-blocker:ekr-url-evidence`, so it is not concurrent today. That is why the one finding is a warning.

```findings
[
  {
    "file": ".engineering/planning/story/first-web-instance.md",
    "line": 52,
    "category": "parallel-safety",
    "severity": "warning",
    "verdict": "needs-revision",
    "origin": "introduced",
    "message": "no edge or sentence orders story:first-web-instance against story:web-pages-cited-as-urls, which changes the EKR pin (src/main.rs:89) and the evidence kind written into the store, so it can land under the 7-day run and mix evidence kinds in the store the day-8 acceptance reads (inferred, from the tree)"
  }
]
```
