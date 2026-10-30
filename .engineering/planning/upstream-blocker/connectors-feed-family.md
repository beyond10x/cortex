---
format: aep.planning-md/3
id: upstream-blocker:connectors-feed-family
kind: upstream-blocker
status: cleared
title: Connectors has no datasource.feed family yet
relations:
- blocks: story:feed-source
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-30T09:39:33Z", actor: "human:timo", revision: 3}
---
## What would clear it

A Connectors release with `story:feed-contract` and `story:feed-bindings-discoverable` (Connectors `epic:generic-datasource-feeds`), and at least one binding to read.

## Cleared (2026-10-30)

Both stories are `implemented` in the Connectors planning store (`story:feed-contract` and
`story:feed-bindings-discoverable`, moved 2026-10-06 on a recorded `test_result`); the commit that
closed the second is first contained in Connectors tag `v0.31.0`. Bindings to read exist:
`connectors-definitions` `connectors/jira.yaml:143`, `connectors/gitlab.yaml:314` and
`connectors/confluence-v1.yaml:37` each declare `kind: feed`. Read from `origin/main` of both
repositories during the store reconcile of 2026-10-30.
