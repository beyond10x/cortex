---
format: aep.planning-md/3
id: review-result:scope-round-2
kind: review-result
status: active
title: Scope critic, round 2
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
approve

**What I read:** 9 artifacts (the epic, 6 stories, the upstream-blocker, the vision, plus `review-result:scope-round-1`). Commands: `aep plan artifact show <id>` for each, and `aep plan artifact graph`. I wrote the epic's promise list before opening any story.

- **Promises:** I extracted 9 from the epic and traced all 9 to an item.
- **Round-1 finding 1 (`ekr view`):** fixed. `story:first-web-instance` acceptance 2 now reads "(`ekr view` counts; …)" and acceptance 3 uses `ekr mcp` `search`, so both inspection routes are claimed.
- **Round-1 finding 2 ("unattended"):** fixed. Acceptance 1 now says "No manual step was taken: no `cortex run` by hand and no timer restarted (`journalctl --user` shows only timer-started runs)". The outcome says "with no manual step".

| # | Promise | Claimed by |
|---|---|---|
| 1 | Search queries and a crawl of named sites through the `tavily` adapter | `story:first-web-instance` (Instance table) |
| 2 | Systemd timers | `story:timer-runs-unattended` and `story:first-web-instance` |
| 3 | 7 days | `story:first-web-instance` |
| 4 | No manual step | `story:first-web-instance` acceptance 1, plus `story:timer-runs-unattended` for 24 h |
| 5 | Own EKR store | `story:first-web-instance` ("its store") |
| 6 | Inspection through `ekr view` | `story:first-web-instance` acceptance 2 |
| 7 | Inspection through the MCP server | `story:first-web-instance` acceptance 3 |
| 8 | The three `UNMAPPED:` markers | `story:seen-documents-modelled` |
| 9 | No CI, evidence kind, and edges: stories 1, 3 and 6, each once | `story:ci-runs-task-check`, `story:extraction-links-facts`, `story:web-pages-cited-as-urls` |

- **No double claims:** each outcome is claimed by exactly one story.
- **No reach beyond the epic:** every story traces to a row of the epic's starting-point table or its story list, and none lands in "Not in this epic".
- **Blocked story:** `story:web-pages-cited-as-urls` is the drafter's honest named omission, held by `upstream-blocker:ekr-url-evidence` (`blocks` edge).
- **Order table:** the `depends_on` edges match it.

**What I could not establish:**
- Whether the Connectors v0.27.0 `tavily` pin is checked by any story. The epic states it as given, so it is not a gap.
- The epic says "a crawl of named sites" (plural) and the instance has one crawl site. I did not count this as narrowing, since the epic names no number.
- Out of my lane (acceptance): `story:timer-runs-unattended` does not say how it checks that `connectors` and `claude` credentials are reachable from the unit, beyond "every run exits 0".

```findings
[]
```
