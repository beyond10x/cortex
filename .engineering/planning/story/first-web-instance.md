---
format: aep.planning-md/3
id: story:first-web-instance
kind: story
status: active
title: The agent-tooling web instance runs for 7 days
relations:
- decomposes: epic:first-web-instance
- serves: vision:self-updating-instances
- depends_on: story:extraction-links-facts
- depends_on: story:timer-runs-unattended
- depends_on: story:seen-documents-modelled
scope:
- confidence: inferred
  path: examples/agent-tooling.yaml
- confidence: inferred
  path: examples/seed/agent-tooling.yaml
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T16:13:32Z", actor: "agent:claude", revision: 10, decided_on: {"recorded":{"review_outcome":6}}}
- {from: "proposed", to: "active", at: "2026-10-05T16:13:32Z", actor: "agent:claude", revision: 11, decided_on: {"recorded":{"review_outcome":6}}}
---
## Outcome

The first real instance, `agent-tooling`, runs from the web for 7 days with no manual step, and its
store answers questions about its subject.

## Instance (default chosen 2026-10-05; the operator may replace it)

| part | value |
|---|---|
| search source, daily | queries: "Model Context Protocol specification change", "Claude Code release", "Rust AI agent framework release"; `topic: news`, `time_range: week`, `max_results: 5` |
| crawl source, weekly | `https://modelcontextprotocol.io/specification`, `limit: 20`, `max_depth: 2` |
| seed schema | node types for specification, release, project and organisation; edge types for `releases`, `specifies`, `depends_on`, taken from the seed schema `story:extraction-links-facts` leaves in `examples/seed/schema.yaml` |
| model budget | $0.50 per run |
| spec file | `examples/agent-tooling.yaml` |

## Acceptance

The 7-day run passes when all of these hold, read on day 8:

0. The instance did work: at least 7 timer-started search runs and 1 crawl run completed with
   exit 0, and at least 20 documents were applied (`cortex sources`; the start is 0 runs and
   0 documents).
1. No manual step was taken: no `cortex run` by hand and no timer restarted (`journalctl --user`
   shows only timer-started runs).
2. The store holds at least as many `Relation` assertions as documents applied, and at least 20
   (`ekr snapshot`; the smoke baseline is 4 relations for 8 documents).
3. Each of these three questions is answered through `ekr mcp` `search` by a node whose evidence
   cites a page the instance fetched, and opening that page confirms the answer:
   - Which Model Context Protocol specification revision is the newest, and what is its date?
   - Which Claude Code release in the 7 days mentions MCP?
   - Which Rust AI agent framework published a release in the 7 days?
   A question with no such page in the 7 days counts as answered when the store holds no node
   claiming one.

Tavily credits and model cost for the 7 days are recorded beside the result in a
`verification-report`.

## Concurrency

The instance runs one cortex binary for the whole 7 days: built once from `main` on day 1, installed
to `~/.local/bin/cortex`, and not rebuilt or replaced until day 8. Every story that lands on `main`
during the 7 days, in whichever wave `aep plan artifact waves` places it, does not reach the
running instance. The EKR binary and the evidence kind stay as they were on day 1.

## Depends on

`story:seen-documents-modelled`, `story:extraction-links-facts`, `story:timer-runs-unattended`.

## Scope

`examples/agent-tooling.yaml`, `examples/seed/agent-tooling.yaml`. No source code.
