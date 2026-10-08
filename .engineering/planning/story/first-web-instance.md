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
revision: 16
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

Landed 2026-10-05 in `dbc0f4f` (wave 20261005h, merged `5b6249a`): the instance definition only.

- **Files:** `examples/agent-tooling.yaml`, `examples/seed/agent-tooling.yaml`, `tests/agent_tooling.rs` (new, 3 cases), `website/docs/spec-file.md`
- **Inferred scope line wrong:** `examples/seed/schema.yaml` still holds only `DEVELOPS`
- **Not yet done:** the 7-day run. It starts from `main` after this wave merges, with model cost bounded by `budget_usd: "0.50"` per run (at most about $4.00 over 7 days). The story stays `active` until the day-8 verification report.

## The run

Created 2026-10-05 ~20:55 CEST by the coordinating session from `main` at `712488a` (cortex 0.1.0 built `--release --locked`, installed to `~/.local/bin/cortex`; `ekr` 0.0.30 at `~/.cache/cortex/bin/0.0.30/bin/ekr`).

- Spec: `~/.config/cortex/agent-tooling/agent-tooling.yaml` (the example with the machine's Tavily connection id; not committed)
- Timers: `cortex-agent-tooling-news.timer` daily 00:30, `cortex-agent-tooling-mcp-specification.timer` Wednesdays 01:30; linger on
- Units carry no `ANTHROPIC_API_KEY`; `claude` uses its signed-in session

Restarted window. The first timer run (2026-10-06 00:30) failed before any search: `connections revalidate: outcome_unknown at publication` (Connectors applied the revalidation and could not confirm it; `next_action: retry_status`). cortex PR #19 (`4c0eb7d`) settles that case by reading `connections status`; the binary was rebuilt from `4c0eb7d` and installed 2026-10-06 01:16. The story wants one binary for all 7 days, so the window counts from the first run on that binary:

- Day 1: 2026-10-07 (news 00:30, crawl 01:30)
- Read on day 8: 2026-10-14, against the Acceptance; Tavily credits and model cost go into the `verification-report`
- At the restart: news `runs: 0`, `consecutive_failures: 1` (a success resets it); crawl `runs: 0`

## Restart (2026-10-08)

The first window could reach at most 6 of the 7 search runs Acceptance 0 needs inside days 1 to 7 (`decision-blocker:first-web-run-continuity`):

| date (CEST) | source | result (journalctl) |
|---|---|---|
| 2026-10-07 00:30 | news | exit 0; 7 of 15 new documents applied, then `stopped`: `extraction-value-mismatch … Release.release_date` (EKR 0.0.30 refused the batch; see `story:prompt-names-value-kinds`) |
| 2026-10-07 01:30 | crawl | exit 0; 4 documents applied |
| 2026-10-08 00:30 | news | exit 1; `tavily.websearch.search: timeout at admission` (Connectors' 30 s CLI deadline; `story:connectors-admission-timeout-retry`) |

The first instance `agent-tooling` was removed with `cortex remove` on 2026-10-08 13:44 UTC; its directory and store stay under `$CORTEX_HOME/instances/agent-tooling` as the record of that window.

The run starts again as the instance `agent-tooling-2` (spec `~/.config/cortex/agent-tooling-2/agent-tooling.yaml`, the example with the Tavily connection id and the name changed; not committed), created 2026-10-08 13:45 UTC on fixed binaries:

| binary | version | sha256 |
|---|---|---|
| `$CORTEX_HOME/bin/cortex` (from the v0.2.1 Release tarball, checksum OK) | cortex 0.2.1 | `a47ce7d8cf4f3051d9ef7f06a0470bdbe6ef53d99554675b251634f0d23c608e` |
| `ekr` under `~/.cache/cortex/bin/0.0.32/` | 0.0.32 | `49196b5dfd5d8b4f3604e4b6d81b7741d7aa844436bf7e62a6d3c911083fb2cc` |
| `connectors`, copied to `~/.local/share/cortex-pins/connectors/connectors-0.33.0` and named by the units' `CORTEX_CONNECTORS` | 0.33.0, chosen; v0.34.0 existed (published 13:21 UTC): 0.33.0 carries the page-scan fix and the run needs a fixed binary | `0a175af28d5ab7d0f06b9630ad3232f510a1c9f9c4932a7915db4cc2dee944cd` |
| `claude`, copied to `~/.local/share/cortex-pins/claude/claude-2.1.294` and named by the units' `CORTEX_CLAUDE` (`cortex update --claude`, 2026-10-08 13:55 UTC) | 2.1.294 | `27122ca7b624f537546fbef35b80c66370d974ff258f3d9b10ac50bb8771f262` |

- Day 1: 2026-10-09 (news 00:30); the first crawl is 2026-10-14 01:30 (day 6)
- Read on day 8: 2026-10-16, against the Acceptance
- At the restart: news and crawl `runs: 0`, `consecutive_failures: 0`
