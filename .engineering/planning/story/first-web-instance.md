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
revision: 19
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

## Restart (2026-10-13)

`agent-tooling-2` stopped on day 2. Its news run applied 15 documents on 2026-10-09 00:30 CEST (`cortex.log`), then failed on 2026-10-10 and 2026-10-11 00:30 CEST with `connections list: metadata_unavailable at observation` (outcome `fetch-failed`); the second failure disabled the source and removed its timer (`journalctl --user -u cortex-agent-tooling-2-news.service`). Its crawl never ran. The day-8 read could not pass.

Cause, outside cortex. Reproduced on 2026-10-13 the way the unit runs it (`env -i`, the pinned binary): `connectors-0.33.0 connections list --adapter tavily --output json` answers `metadata_unavailable` at `observation`, exit 1, while `connectors 0.45.0` and `0.47.0` list the connection against the same configuration and state. The shared Connectors state (`~/.local/state/connectors/metadata.sqlite3`) was last rewritten 2026-10-09 07:08–07:09 UTC, between the last success and the first failure, by a session running a newer `connectors`; `connectors-0.33.0 setup check` still reports the metadata `ready`. Which change in that state the older binary refuses is Connectors' to find; that request went to Connectors with the reproduction. cortex's code did what it should: the error is not a lapse or a missing connection, so the run fails and `--record-failure` counts it.

How a stop is seen the same day: every failed run leaves its service `failed`, which `systemctl --user --failed` lists until the next run of that source succeeds (checked: `cortex-agent-tooling-2-news.service` was listed from 2026-10-10). The `failed` states of the removed instances' units were reset on 2026-10-13, so the list holds only live failures.

`agent-tooling-2` was removed with `cortex remove` on 2026-10-13; its directory and store stay under `$CORTEX_HOME/instances/agent-tooling-2`. The run starts again as `agent-tooling-3` (spec `~/.config/cortex/agent-tooling-3/agent-tooling.yaml`: the `agent-tooling-2` spec with the name changed and `ekr.version` set to `0.0.33`; not committed), created 2026-10-13 with `cortex create`:

| binary | version | sha256 |
|---|---|---|
| `$CORTEX_HOME/bin/cortex` (from the v0.2.2 Release tarball, `sha256sum --check` OK) | cortex 0.2.2 | `5466b5f4e224be77085c52fe681d6c2bf583e37a4a6a0679f1fbaecf87a39ae8` |
| `ekr` under `~/.cache/cortex/bin/0.0.33/`, built from the tag (`cargo install --locked --git … --tag 0.0.33 ekr`; the release has no assets) | 0.0.33 | `7e4b7d8f2ce88f1bb1ffdfeb0b6bf65c0ac1d6263b52061f36c84f910b5684f6` |
| `connectors`, built from the tag `v0.47.0` (the release has no assets) and copied to `~/.local/share/cortex-pins/connectors/connectors-0.47.0` | 0.47.0, the newest release | `dff436afd48295fa0cf331a8607bdf93a65100523b5a5df180a550231f475f92` |
| `claude` at `~/.local/share/cortex-pins/claude/claude-2.1.294`, unchanged | 2.1.294 | `27122ca7b624f537546fbef35b80c66370d974ff258f3d9b10ac50bb8771f262` |

Before the pin, `connectors 0.47.0` read the live state (`setup check`, `connections list`) and `schema_migrations` stayed at versions 1 and 9; `connectors 0.45.0` still listed the connection afterwards.

- Viewer: `http://127.0.0.1:18900/`, HTTP 200 at creation
- Day 1: 2026-10-14 (news 00:30 CEST, crawl 01:30 CEST, a Wednesday); days 1 to 7 hold 7 news runs and 1 crawl
- Read on day 8: 2026-10-21, against the Acceptance
- Risk: a `connectors` newer than 0.47.0 that rewrites the shared state can stop the pinned binary again; a stop shows in `systemctl --user --failed`
- At the restart: news and crawl `runs: 0`, `consecutive_failures: 0`

## Restart (2026-10-14)

`agent-tooling-3`'s news run applied 15 documents on 2026-10-14 00:30 CEST (cost 0.3573 USD). Its crawl, started by its timer at 01:30 CEST, ended `extraction-failed` with `claude answered an error (success), cost 0.0000 USD`, and `--record-failure` counted it (`journalctl --user -u cortex-agent-tooling-3-mcp-specification.service`; the unit was listed by `systemctl --user --failed`). cortex 0.2.2 keeps only the answer's subtype and cost, so the run directory holds no reason; a zero-cost error answer is the shape of a sign-in or usage-limit answer from `claude`, inferred, not verified. `story:model-error-text-and-retry` keeps the text and retries once.

The crawl was then started by hand (`systemctl --user start`, 2026-10-14 02:48 CEST): it ran, applied 18 documents, cost 0.2406 USD, and the failure count went back to 0. A manual start breaks Acceptance 1, so the run starts again.

`agent-tooling-3` was removed with `cortex remove` on 2026-10-14 00:50 UTC; its directory, store and `cortex.log` stay under `$CORTEX_HOME/instances/agent-tooling-3`. The run starts again as `agent-tooling-4` (spec `~/.config/cortex/agent-tooling-4/agent-tooling.yaml`: the `agent-tooling-3` spec and seed with the name changed; not committed), created 2026-10-14 00:51 UTC with `cortex create` and no manual start of any unit. Its binaries are the four of the 2026-10-13 restart, by the same sha256 (`sha256sum`, 2026-10-14): cortex 0.2.2, ekr 0.0.33, connectors 0.47.0, claude 2.1.294.

- Viewer: `http://127.0.0.1:18900/`, HTTP 200 at creation
- First timer: news 2026-10-15 00:30 CEST; crawl 2026-10-21 01:30 CEST (`systemctl --user list-timers 'cortex-agent-tooling-4-*'`)
- Day 1: 2026-10-15; days 1 to 7 (2026-10-15 to 2026-10-21) hold 7 news runs and 1 crawl
- Read on day 8: 2026-10-22, after the 00:30 CEST news run, against the Acceptance
- At the restart: news and crawl `runs: 0`, `consecutive_failures: 0` (`cortex sources`)

## Restart (2026-10-16)

`agent-tooling-4` stopped on day 1. Its news run failed on 2026-10-15 and 2026-10-16 00:30 CEST with
`connections list: metadata_unavailable at observation` (outcome `fetch-failed`); the second failure
disabled the source (`journalctl --user -u cortex-agent-tooling-4-news.service`). Cause, outside
cortex, as recorded by the session that ran it: the shared Connectors state was rewritten by
`connectors` 0.50.0, which the pinned 0.47.0 cannot read, as in the 2026-10-13
restart. Its crawl, started by its timer on 2026-10-21 01:30 CEST, failed with `connections list: invalid_configuration at configuration`.

The restart gives the instance Connectors state of its own, so a newer `connectors` run elsewhere
on the machine cannot rewrite it. The run starts again as `agent-tooling-5`, in its own home
`~/.local/share/cortex-at5`, with `XDG_CONFIG_HOME=~/.config/cortex-at5` and
`XDG_STATE_HOME=~/.local/state/cortex-at5` copied into its units, a Tavily connection registered in
that state, and spec `~/.config/cortex/agent-tooling-5/agent-tooling.yaml` (the `agent-tooling-4`
spec with the name, the connection id, `ekr.version` `0.0.34` and `view_port` 18910 changed; not
committed). Created 2026-10-16 13:12 UTC with `cortex create`; no unit was started by hand.

| binary | version | sha256 |
|---|---|---|
| `bin/cortex` in the instance's home | cortex 0.5.0 | `4c4c368609092917a6f9bb20b082fd39c0c77d48b485ec523e22837f529897cb` |
| `ekr` under `~/.cache/cortex/bin/0.0.34/` | 0.0.34 | `e10f2f0a3b2594b56f5f4738653cfbc13b452c0dd34f16a4128d84d272526e4a` |
| `connectors` at `~/.local/share/cortex-pins/connectors/connectors-0.51.0`, named by the units' `CORTEX_CONNECTORS` | 0.51.0 | `a2afc83f6c80f3b35e9ad81cdc157e858b8b9ea34b3d88772be7c54b39c7af93` |
| `claude` at `~/.local/share/cortex-pins/claude/claude-2.1.294`, unchanged | 2.1.294 | `27122ca7b624f537546fbef35b80c66370d974ff258f3d9b10ac50bb8771f262` |

- Viewer: `http://127.0.0.1:18910/`
- Day 1: 2026-10-17 (news 00:30 CEST); the crawl ran on 2026-10-21 01:30 CEST (day 5)
- Read on day 8: 2026-10-24, after the 00:30 CEST news run, against the Acceptance
- At 2026-10-21 23:00 CEST (`journalctl --user -u 'cortex-agent-tooling-5-*'`): 5 timer-started
  news runs and 1 crawl, each `outcome: ran`, exit 0; 66 documents applied (15, 7, 9, 6, 10 and 19)

`agent-tooling-4` was removed with `cortex remove` on 2026-10-21 21:09 UTC; its directory and store
stay under `~/.local/share/cortex/instances/agent-tooling-4`.
