---
format: aep.planning-md/3
id: specification:wave-20261030a-review-fixes
kind: specification
status: approved
title: 'Wave 20261030a: review fixes, downstream defects, performance'
tags:
- review-20261030
relations:
- specifies: story:connectors-invoke-0-60
- specifies: story:one-store-read-per-run
- specifies: story:idle-supersede-source-cheap
- specifies: story:run-cost-counted-on-refusal
- specifies: story:replacement-applied-as-assertion
- specifies: story:ask-outcome-truthful
- specifies: story:ask-service-bounds
- specifies: story:ask-error-text
- specifies: story:redaction-patterns-hold
- specifies: story:mask-quoted-credentials
- specifies: story:update-refuses-rekey
- specifies: story:schema-apply-outcome-unknown
- specifies: story:unit-names-unique
- specifies: story:spec-integer-bounds
- specifies: story:durable-home-writes
- specifies: story:ask-records-deadline
- specifies: story:eval-proxy-bounds
- specifies: story:poison-document-skipped
- specifies: story:window-advances-past-applied
- specifies: story:record-key-template
- specifies: story:child-text-and-filter
- specifies: story:ekr-session-per-run
- specifies: story:connectors-call-deadline
- specifies: story:window-relative-placeholders
- specifies: story:runs-retention
- specifies: story:small-robustness
revision: 10
transitions:
- {from: "draft", to: "in_review", at: "2026-10-30T10:11:00Z", actor: "human:timo", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-30T10:11:00Z", actor: "human:timo", revision: 3}
---
## Wave 20261030a: review fixes, downstream defects, performance

Opened 2026-10-30, `aep:implementing` wave mode, skill version 0.25.0. Based on `origin/main`
`a870a628` (release 0.6.1). Integration branch `wave/20261030a`, managed tree
`cortex-plan-reconcile-20261030`. One PR to `main`, then release 0.7.0 through AGENTS.md
"Cutting a release". The operator asked for the wave on 2026-10-30 ("dispatch it in a wave"); the
coordinating session `b10x-chef-orchestrator` approved the branch, the PR and the release.

Sources: five code reviews and one performance review of `main` (2026-10-30), verified by an
independent reviewer; eleven defects measured downstream on 0.6.1 (C-01..C-11). Every story is
tagged `review-20261030`.

## Folded in: wave 20261018a

`wave/20261018a` (13 commits, 2026-10-18) merged its four units and never reached `main`:
`story:structured-lookup-across-types`, `story:ontology-names-masked`, `story:fetch-stop-point`,
`story:partial-run-counts`. It is merged into this wave first. `git merge-tree` against
`wave/20261030a` conflicts only in generated output (`generated/`, `spec/suite.json`,
`website/docs/reference/ess/`, `website/static/schemas/`), which is regenerated on merge. Its
drafted `story:structured-children-follow-rule` joins unit `shapes`.

## Units

Each unit is one implementor (`aep:implementor`) working its stories in the order given, then one
adversary (`aep:adversary`). At most 4 units run at once. A unit forks from `wave/20261030a` when
its stage opens, so it starts from every unit merged before it.

| unit | stories, in order | after | stage |
|---|---|---|---|
| w18a | the four stories of wave 20261018a | — | merged `7a217da7` |
| conn | `connectors-invoke-0-60` | w18a | merged |
| read | `one-store-read-per-run`, `idle-supersede-source-cheap` | w18a | merged |
| cost | `run-cost-counted-on-refusal`, `replacement-applied-as-assertion` | w18a | merged |
| ask | `ask-outcome-truthful`, `ask-service-bounds`, `ask-error-text` | w18a | merged |
| redact | `redaction-patterns-hold`, `mask-quoted-credentials` | w18a | merged |
| outcome | `update-refuses-rekey`, `schema-apply-outcome-unknown` | w18a | implementing |
| cli | `unit-names-unique`, `spec-integer-bounds`, `durable-home-writes` | w18a | merged |
| askio | `ask-records-deadline`, `eval-proxy-bounds` | ask | implementing |
| fail | `poison-document-skipped`, `window-advances-past-applied` | cost | implementing |
| shapes | `structured-children-follow-rule`, `record-key-template`, `child-text-and-filter` | conn, outcome | planned |
| session | `ekr-session-per-run` | read, fail | planned |
| tail | `connectors-call-deadline`, `window-relative-placeholders`, `runs-retention`, `small-robustness` | conn, shapes | planned |
| ess | move the `ess` pin in `spec/ess-inputs.yaml` from 0.60.0 to the newest ESS release and regenerate | every unit above | planned |

Every story serves `vision:self-updating-instances`. Each story's `## Scope` was written by
`aep:story-scoper` on 2026-10-30 and its entries are recorded with `aep plan artifact scope`, cited
and inferred kept apart; the open design questions the scopers named are answered in each story's
`## Decisions`.

Why these units: `src/run.rs` `process_in` is edited by read, cost, fail, session and tail, so
those run in the order above and never two on the same lines at once. `src/sources.rs`
`fetch_records` is edited by conn, shapes and tail, in that order. The `cortex.ask` domain and
`src/ask/{mod,serve,model}.rs` are one unit. Every unit that changes `spec/domains/*.yaml`
regenerates `generated/`, `spec/suite.json` and the website projections in its own tree; a merge
conflict there is resolved by `task generate` and `task docs-generate` on the integration branch,
never by hand.

## Selection

`aep plan artifact waves --kind story --status draft` on 2026-10-30 returned 15 waves of 1 to 5
stories and 528 collisions, almost all on generated output and `CHANGELOG.md`. Selection path:
units by code region, as above, instead of the verb's file-level waves. Its lists:

- waves: 1 ask-outcome-truthful, connectors-invoke-0-60, eval-proxy-bounds,
  redaction-patterns-hold, spec-integer-bounds; 2 ask-records-deadline, feed-source,
  mask-quoted-credentials; 3 ask-service-bounds, durable-home-writes; 4 ask-error-text,
  one-store-read-per-run, web-pages-cited-as-urls; 5 poison-document-skipped; 6
  child-text-and-filter, replacement-applied-as-assertion; 7 run-cost-counted-on-refusal; 8
  runs-retention; 9 schema-apply-outcome-unknown, small-robustness; 10
  idle-supersede-source-cheap, unit-names-unique; 11 update-refuses-rekey; 12
  ekr-session-per-run, window-relative-placeholders; 13 connectors-call-deadline; 14
  record-key-template; 15 window-advances-past-applied
- cycles: none
- unassessed: ask-canvas-blocks, ask-figure-event, ask-figure-tool, ask-report-download,
  ask-request-log, checkpoints-volume-contract, container-image, cronjob-from-spec,
  derivation-run, eval-gated-improve, extraction-cache-by-content, fetch-stop-point,
  headless-connectors-credentials, instance-schema-from-another, ontology-names-masked,
  parallel-sources, partial-run-counts, question-entity-linking,
  reconcile-changed-and-deleted-documents, repository-documents-source, research-run, run-report,
  scheduler-one-shot-run, structured-lookup-across-types, unattended-model-credential-design,
  vitals-and-topics

## Left out

- `story:feed-source`, `story:web-pages-cited-as-urls`: drafts from earlier plans, not review
  findings.
- `story:extraction-cache-by-content` (C-05): size L, a store of its own; follows this wave.
- `story:parallel-sources` (C-11, P-05, R5-13): blocked by
  `decision-blocker:parallel-sources-shape`, an architecture decision for the operator.
- C-02 is `story:reconcile-changed-and-deleted-documents`, an EKR question.
- P-12, P-13, P-15 and the documented-behaviour findings R1-14, R2-01, R2-05, R4-06, R4-15, R5-04:
  not filed in this wave.

## Pre-flight (2026-10-30)

- Main checkout `cortex`: on `main`, clean.
- Free disk: 89 GB on `/`; floor 25 GB; a watch stops builds under 25 GB.
- One measured build: `cargo test --workspace --no-run` in one tree, 64 s, 2.8 GB `target/`.
- Builds: `CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0`, each tree its own
  `target/`.
- Implementors run only the targeted tests of the code they change. The full gate (`task check`)
  runs once, on the integration branch, before the PR.
- Leftover trees of wave 20261018a (`cortex-w18a-{int,lookup,onto,fetch,counts}`): clean, no
  `target/`; finished once w18a is merged.

## Trees

Worktree, build directory and scratch root per unit. Trees under
`~/.local/state/worktree/trees/b10x/cortex/`, scratch under the coordinating session's scratch
directory `w30a/<unit>/`.

| unit | branch | head | worktree | build | scratch |
|---|---|---|---|---|---|
| int | `wave/20261030a` | 7a217da7 | `cortex-plan-reconcile-20261030` | `<tree>/target` | `w30a/int` |
| conn | `unit/w30a-conn` | 7a217da7 | `cortex-w30a-conn` | `<tree>/target` | `w30a/conn` |
| read | `unit/w30a-read` | 7a217da7 | `cortex-w30a-read` | `<tree>/target` | `w30a/read` |
| cost | `unit/w30a-cost` | 7a217da7 | `cortex-w30a-cost` | `<tree>/target` | `w30a/cost` |
| ask | `unit/w30a-ask` | 7a217da7 | `cortex-w30a-ask` | `<tree>/target` | `w30a/ask` |
| redact | `unit/w30a-redact` | 7119552e | `cortex-w30a-redact` | `<tree>/target` | `w30a/redact` |
| fail | `unit/w30a-fail` | 7119552e | `cortex-w30a-fail` | `<tree>/target` | `w30a/fail` |
| outcome | `unit/w30a-outcome` | 97573d67 | `cortex-w30a-outcome` | `<tree>/target` | `w30a/outcome` |
| cli | `unit/w30a-cli` | 97573d67 | `cortex-w30a-cli` | `<tree>/target` | `w30a/cli` |
| askio | `unit/w30a-askio` | 3c7066a4 | `cortex-w30a-askio` | `<tree>/target` | `w30a/askio` |

Unit rows are added as each tree is created.

## Commits this wave makes

All as `b10x-bot[bot]` through `b10x-gates bot`: this opening store commit; the merge of
`wave/20261018a` and its regeneration; each unit's commits on its branch; each unit's merge into
`wave/20261030a` and any regeneration after it; the closing store commit; one PR to `main`,
auto-merged when CI is green; the release commits of 0.7.0 (release branch, version, CHANGELOG),
the tag `v0.7.0` and its GitHub Release. Nothing else.
