---
format: aep.planning-md/3
id: review-result:scope-round-1
kind: review-result
status: active
title: Scope critic, round 1
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
story:first-web-instance — the epic promises the result is "inspected through `ekr view` and the MCP server", but the acceptance names only `ekr mcp` `search`, so the `ekr view` half is narrowed away; the acceptance should require an `ekr view` inspection of the 7-day store — .engineering/planning/epic/first-web-instance.md:15
story:first-web-instance — the epic title promises a run "unattended for a week", but neither the outcome nor the acceptance says no manual step was taken during the 7 days, so a hand-restarted run would still pass; the body should state that — .engineering/planning/epic/first-web-instance.md:6

**What you read**
- 8 artifacts: the epic, 6 stories, 1 upstream-blocker and the vision.
- Commands: `aep plan artifact show <id>` for each, `aep plan artifact graph`, and a grep of the store files for line numbers.
- I extracted 8 promises from the epic and traced 6 to an item. Two are narrowed: `ekr view` inspection and "unattended".
- The 8 promises:
  1. Web search and crawl through the Connectors `tavily` adapter: `story:first-web-instance`.
  2. Systemd timers: `story:timer-runs-unattended`.
  3. 7 days: `story:first-web-instance`.
  4. Own EKR store: `story:first-web-instance`.
  5. `ekr view` inspection: narrowed.
  6. MCP inspection: `story:first-web-instance`.
  7. "Unattended": narrowed.
  8. The listed stories 1, 2, 3 and 6, each claimed exactly once.
- No item claims anything outside the epic's story list, and none lands in the "Not in this epic" exclusions.

**What you could not establish**
- I could not tell whether the Connectors v0.27.0 `tavily` adapter pin is claimed by any story. Nothing in the set checks it, but the epic states it as a given, so I did not count it as a gap.
- Out of my lane (parallel-safety/design): the epic's order says "1, then 2 and 3 in parallel, then 4, then 5", but `story:timer-runs-unattended` depends only on `story:ci-runs-task-check`. `story:first-web-instance` does not depend on `story:seen-documents-modelled` at all.
- Out of my lane (acceptance): the 24 h timer acceptance relies on a "forced failure (connection revoked)" whose setup the story does not describe.

```findings
- file: .engineering/planning/epic/first-web-instance.md
  line: 15
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: the epic promises the result is "inspected through `ekr view` and the MCP server", but story:first-web-instance's acceptance names only `ekr mcp` `search`, so the `ekr view` half is narrowed away; the acceptance should require an `ekr view` inspection of the 7-day store
- file: .engineering/planning/epic/first-web-instance.md
  line: 6
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: the epic title promises a run "unattended for a week", but neither the outcome nor the acceptance of story:first-web-instance says no manual step was taken during the 7 days, so a hand-restarted run would still pass; the body should state that
```
