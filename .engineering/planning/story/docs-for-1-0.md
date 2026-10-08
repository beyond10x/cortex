---
format: aep.planning-md/3
id: story:docs-for-1-0
kind: story
status: active
title: The public documentation describes every 1.0 feature
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
- depends_on: story:store-backend-per-instance
- depends_on: story:redaction-before-model
- depends_on: story:structured-source
- depends_on: story:run-snapshots
- depends_on: story:adopt-existing-store
- depends_on: story:release-pipeline
scope:
- confidence: cited
  path: README.md
- confidence: cited
  path: website/docs/commands.md
- confidence: cited
  path: website/docs/limits.md
- confidence: cited
  path: website/docs/operating.md
- confidence: cited
  path: website/docs/quickstart.md
- confidence: cited
  path: website/docs/spec-file.md
- confidence: inferred
  path: website/docs/use-cases.md
revision: 15
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T13:07:55Z", actor: "human:timo", revision: 14, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-08T13:07:55Z", actor: "human:timo", revision: 15, decided_on: {"recorded":{"review_outcome":1}}}
---
## Outcome

The public documentation describes every 1.0 feature as it behaves, nothing that does not exist,
and how to install a released binary.

## Work

- `website/docs/quickstart.md`: install from the GitHub Release tarball and check `SHA256SUMS`
  (today step 1 is `git clone` plus `cargo install --locked --path .`, `website/docs/quickstart.md:19-24`);
  building from source moves to a second section.
- `website/docs/spec-file.md`: store backends, model backends, redaction, structured sources,
  snapshots.
- `website/docs/commands.md`: `restore`, `adopt`.
- `website/docs/operating.md`: PostgreSQL provisioning, snapshots and restore, Codex cost reporting.
- `website/docs/limits.md`: redaction is pattern-based; the open EKR limits (URL evidence,
  relations not walked by graph reads) with their upstream story ids.
- README.md feature list.
- `website/docs/use-cases.md` (new): five problems cortex solves, from the draft below,
  each with its sources, the question it answers and what it needs; every "Ready" claim
  re-checked against the code at the time of writing.

## Acceptance

The PR body carries a table with one row per statement added to the seven files, each naming the test
or command output that shows it, and no row without one.

## Depends on

`story:store-backend-per-instance`, `story:redaction-before-model`, `story:structured-source`,
`story:codex-model-backend`, `story:run-snapshots`, `story:adopt-existing-store`,
`story:release-pipeline` (the install section names the release asset names it defines).

## Scope

`website/docs/quickstart.md`, `website/docs/use-cases.md` (new), `website/docs/spec-file.md`, `website/docs/commands.md`,
`website/docs/operating.md`, `website/docs/limits.md`, `README.md`.

## Use cases page draft (2026-10-05)

Each is one cortex instance: a spec file naming the sources, a seed schema, a model budget. "Ready"
means the Connectors adapter or cortex source kind exists today (Connectors v0.27.0, cortex `main`).

#### 1. Dependency and vendor watch for an engineering team

**Problem.** A team depends on dozens of libraries, managed services and APIs. Releases,
deprecations, CVEs and pricing changes land on changelogs, advisories and news; nobody reads all of
them, and the one that matters is found after it breaks something.

**Instance.** Web search per dependency (`topic: news`, `time_range: week`) plus a weekly crawl of
each vendor's changelog page. Seed: Product, Release, Vulnerability, Organization;
`AFFECTS`, `RELEASES`, `DEPRECATES`, `REPLACES`.

**Question it answers over MCP.** "Did anything we depend on ship a breaking change or a CVE this
week?" Each answer cites the page.

**Ready:** yes (web source, `tavily`).
database by accident.

#### 2. Engineering memory across code, tickets and docs

**Problem.** Why a service is built the way it is, who owns it and which incident changed it lives in
merge requests, issues, tickets and wiki pages across three tools. New people and coding agents ask
the same questions and re-derive the answer from scratch.

**Instance.** Connectors sources over GitLab (merge requests, issues), Jira (issues, changelogs) and
Confluence (pages changed since a cutoff). Seed: Service, Team, Person, Decision, Incident;
`OWNS`, `DECIDED`, `CAUSED`, `CHANGED`.

**Question.** "Who owns the billing service and what was the last decision about its retry policy?"

**Ready:** GitLab, Jira and Confluence through the Connectors catalog. Slack threads are not: the
Connectors Slack story is a draft.

#### 3. Account knowledge for support and sales

**Problem.** What a customer runs, what they reported, who their contacts are and what was promised is
split between the CRM and the support desk. Agents answering a ticket do not see the sales history,
and the account owner does not see the open tickets.

**Instance.** Connectors sources over HubSpot (companies, deals, contacts) and Zendesk (tickets,
users, organizations, incremental exports). Seed: Account, Contact, Product, Ticket, Deal;
`USES`, `REPORTED`, `OWNS_ACCOUNT`, `BLOCKS_DEAL`.

**Question.** "Which open tickets does this account have, and is any of them tied to a deal in
progress?"

**Ready:** both adapters ship (HubSpot by 0.25.1, Zendesk since 0.26.0). Zendesk has run only against
a local fixture, not a live account. Customer data needs a redaction policy before it reaches a model,
which cortex does not have yet.

#### 4. Standards and regulation tracking

**Problem.** A product must follow a moving specification (a protocol spec, an API version policy, a
regulation). Changes are published as new revisions of long documents, and the question is always
"what changed since the version we implemented, and does it touch us?".

**Instance.** A weekly crawl of the specification site plus web search for announcements. Seed:
Specification, Revision, Requirement, Organization; `SUPERSEDES`, `REQUIRES`, `PUBLISHED`.

**Question.** "What changed between the revision we implement and the newest one?"

**Ready:** yes. 

#### 5. Grounding for coding and support agents

**Problem.** Agents re-read the same files and docs on every task and still miss facts that live
outside the repository. Answers come without provenance, so a person cannot check them.

**Instance.** A files source over a repository's docs and ADRs, plus a Connectors source over the
issue tracker, served to the agent through `cortex mcp` (`ekr mcp`). Every fact carries the file or
record it came from.

**Question.** Any "where is X decided / what does Y mean here" question, answered with a citation.

**Ready:** yes, with one EKR limit: relations are stored but graph walks (`expand`, `degree`) do not
see them yet (EKR story `extracted-relations-visible-to-graph-reads`). Search and explain work.

#### What these share, and what cortex lacks for them

| gap | needed by | where it belongs |
|---|---|---|
| Redaction of personal data before the model sees a document | 2, 3 | cortex (a spec-level policy), generic |
| Relations visible to graph reads | 2, 3, 5 | EKR |
| Slack reads | 2 | Connectors (`story:catalog-slack-reads`, draft) |
| A deterministic source kind: map structured records straight to entities without a model call | 2, 3 | cortex |

## Carried from wave 20261005a

From wave 20261005a (`story:spec-standalone-types`, landed `b628e4d`): `website/docs/commands.md` must document `--codex` beside `--claude` and say `cost_usd` may be `null` (an uncosted answer). The implementor left this patch for this story, against `b628e4d`:

```diff
--- a/website/docs/commands.md	2026-10-05 13:19:33.430301006 +0200
+++ b/website/docs/commands.md	2026-10-05 13:19:33.432128747 +0200
@@ -23,6 +23,7 @@
 | `--home <dir>` | `CORTEX_HOME` | `~/.local/share/cortex` |
 | `--connectors <bin>` | `CORTEX_CONNECTORS` | `connectors` |
 | `--claude <bin>` | `CORTEX_CLAUDE` | `claude` |
+| `--codex <bin>` | `CORTEX_CODEX` | `codex` |
 
 ## Instances
 
@@ -45,12 +46,14 @@
 | `cortex run <instance>/<source> [--record-failure]` | run one source once. With `--record-failure`, which the timers pass, a failed run is also counted | `ran`, `fetch-failed`, `extraction-failed`, `apply-refused`, `disabled`, `no-such-source` |
 | `cortex source enable <instance>/<source>` | enable a source that two failed runs in a row disabled, and its timer | `enabled`, `wrong-state`, `no-such-source` |
 | `cortex source record-failure <instance>/<source> --reason <text>` | count one failed run; the second in a row disables the source and its timer | `counted`, `disabled`, `already-disabled`, `no-such-source` |
-| `cortex source add --instance-name <name> --source-id <id> --name <name> --kind <Web\|Connectors\|Files> --schedule <calendar>` | register a source in the registry; `create` and `update` do this for the spec's sources | `added` |
+| `cortex source add --instance-name <name> --source-id <id> --name <name> --kind <Web\|Connectors\|Files\|Structured> --schedule <calendar>` | register a source in the registry; `create` and `update` do this for the spec's sources | `added` |
 | `cortex sources` | sources: id, kind, state, schedule, runs, consecutive failures | |
 
 A `ran` line's detail carries `documents_new`, `documents_applied`, `cost_usd`, `facts_refused`
 (facts the model cited unissued evidence for), `parts_rejected` (parts EKR rejected), `masked`
-(credential shapes replaced) and `stopped` (why the run ended early, if it did).
+(credential shapes replaced) and `stopped` (why the run ended early, if it did). `cost_usd` is `"0.0000"`
+when no model was asked, the sum when every model answer carried a cost, and `null` when any
+answer carried none; a missing cost is never written as `0`.
 
 ## Setup
 
```

## Codex is not in 1.0

Decided 2026-10-06 by the coordinating session under the operator's standing wave approval: `story:codex-model-backend` stays open behind `decision-blocker:codex-exec-keeps-shell` (codex exec keeps a shell tool), and no longer holds back 1.0. `story:docs-for-1-0` documents the Codex backend as unavailable until that blocker clears; `story:document-time-as-valid-time` never needed it (it waits on EKR valid time).

## Remaining (2026-10-08)

Checked against `main` at `ca393d0` by the store audit of 2026-10-08. `restore`, `adopt`, quality and schema commands, snapshots and PostgreSQL provisioning are already documented (`commands.md`, `operating.md`, `spec-file.md`). What is left:

- `quickstart.md` step 1 installs from a `git clone` (`quickstart.md:22-24`): install from the GitHub Release tarball and check `SHA256SUMS`; building from source becomes a second section.
- `use-cases.md` does not exist: write it from the draft above, re-checking every "Ready" line against the code (redaction and structured sources are implemented; the draft's gap table predates them). Drop the stray fragment after use case 1.
- `limits.md:88` names EKR 0.0.30; the pin is 0.0.32. Re-check each EKR limit against 0.0.32.
- Codex: the Codex backend is not in 1.0 (section above). No page may document `--codex` as a working model backend; the carried patch's `--codex` row is not applied. `operating.md:65` names `CORTEX_CODEX`; say the backend is unavailable.
- The carried patch's `cost_usd` sentence and the `Structured` kind in `source add` are checked against `src/main.rs` and applied if still missing.
- README feature list against the shipped features.
