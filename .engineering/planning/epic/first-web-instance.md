---
format: aep.planning-md/3
id: epic:first-web-instance
kind: epic
status: active
title: The first real web instance runs unattended for a week
relations:
- serves: vision:self-updating-instances
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T14:07:50Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-08T14:07:50Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":1}}}
---
## Outcome

The first real cortex instance runs from the web: search queries and a crawl of named sites through
the Connectors `tavily` adapter (Connectors v0.27.0), on systemd timers, for 7 days with no manual
step, into its own EKR store, with the result inspected through `ekr view` and the MCP server.

## Starting point (2026-10-05)

| fact | source |
|---|---|
| One manual smoke run: search applied 3 documents ($0.0722), the repeat applied 0, a crawl applied 5 ($0.0955) | `~/.cache/cortex-smoke-8c9138d1`, live run 2026-10-05 |
| That store holds 14 nodes and 22 assertions, of which 4 are relations (`Relation` predicate); EKR leaves the separate `edges` section empty for extraction | `ekr snapshot` on the smoke store |
| No timer has ever run | `systemctl --user list-timers` lists no cortex unit |
| No CI | no `.github/workflows/` in the repository |
| Web pages are filed as `!HumanStatement` evidence because EKR 0.0.30 admits no other kind | `AGENTS.md` upstream table; EKR `crates/ekr-integrate/src/extraction.rs:722` |
| Three `UNMAPPED:` markers | `spec/domains/instance.yaml:213`, `:328`, `:382` |

## Stories

1. `story:ci-runs-task-check` — CI holds the gate.
2. `story:seen-documents-modelled` — the three `UNMAPPED:` markers settled in the spec.
3. `story:extraction-links-facts` — extraction states the relations its documents carry.
4. `story:timer-runs-unattended` — scheduled runs, observed for 24 h.
5. `story:first-web-instance` — the real instance, 7 days.
6. `story:web-pages-cited-as-urls` — evidence kind, blocked on EKR.

## Order

The `depends_on` edges carry it; this table restates them with the reason.

| story | after | why |
|---|---|---|
| 2 | 1 | its spec and code changes land behind the CI gate |
| 3 | 1 | same; it edits `src/extract.rs` and the seed schema |
| 4 | 1, 2 | 2 changes `src/state.rs` / `src/home.rs`, which hold the run state 4 observes |
| 5 | 2, 3, 4 | a 7-day run must not have the seen-state format or generated types change under it |
| 6 | 1, 3, blocker | it moves the EKR pin the CI workflow carries, and edits `src/extract.rs` after 3 |

2 and 3 can run in parallel: they share no file.

## Not in this epic

Connectors and files sources in a real instance; a hosted (non-systemd) runner; multi-user access to
the MCP server.
