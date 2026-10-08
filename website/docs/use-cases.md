---
title: Use cases
sidebar_position: 3
description: Five problems one cortex instance solves, with the sources, the question and what each needs.
---

# Use cases

Each use case below is one cortex instance: a [spec file](./spec-file.md) naming the sources, a
seed schema and a model budget. The seed types and edge types are suggestions; write the ones
your questions need. Every instance is served to agents through `ekr mcp`, with the line
`cortex mcp-line <name>` prints, and every answer cites the document it came from.

**Ready** says what this release of cortex and the
[Connectors](https://github.com/beyond10x/connectors) catalog (v0.33.0) provide for it today.

:::note[What an instance cannot do yet]
A web page is cited as a statement naming its URL, not as URL evidence, and the Codex model
backend does not extract. See [Limits](./limits.md).
:::

## 1. Dependency and vendor watch for an engineering team

**Problem.** A team depends on dozens of libraries, managed services and APIs. Releases,
deprecations, security advisories and pricing changes land on changelogs, advisories and news;
nobody reads all of them, and the one that matters is found after it breaks something.

**Instance.** A [`web`](./spec-file.md#kind-web) source with `input: search`, one query per
dependency, `policy.topic: news` and `policy.time_range: week`, plus a weekly `web` source with
`input: sites` and `mode: Crawl` over each vendor's changelog page. Seed: `Product`, `Release`,
`Vulnerability`, `Organization`; `AFFECTS`, `RELEASES`, `DEPRECATES`, `REPLACES`.

**Question.** "Did anything we depend on ship a breaking change or a security fix this week?"

**Ready:** yes. Web sources run through the `tavily` adapter by default.

## 2. Engineering memory across code, tickets and docs

**Problem.** Why a service is built the way it is, who owns it and which incident changed it lives
in merge requests, issues, tickets and wiki pages across three tools. New people and coding agents
ask the same questions and work the answer out again each time.

**Instance.** [`connectors`](./spec-file.md#kind-connectors) sources over GitLab (merge requests,
issues), Jira (issues) and Confluence (pages changed since the last run, through `{since}` in the
inputs). Records whose fields already say who owns what can go through a
[`structured`](./spec-file.md#kind-structured) source instead, which maps them to nodes and
relations with no model call. Seed: `Service`, `Team`, `Person`, `Decision`, `Incident`; `OWNS`,
`DECIDED`, `CAUSED`, `CHANGED`.

**Question.** "Who owns the billing service, and what was the last decision about its retry
policy?"

**Ready:** GitLab, Jira and Confluence are providers of the Connectors catalog. Chat threads are
not: the catalog has no Slack provider.

## 3. Account knowledge for support and sales

**Problem.** What a customer runs, what they reported, who their contacts are and what was promised
is split between the CRM and the support desk. People answering a ticket do not see the sales
history, and the account owner does not see the open tickets.

**Instance.** `connectors` or `structured` sources over HubSpot (companies, deals, contacts) and
Zendesk (tickets, users, organizations). A [`redaction`](./spec-file.md#redaction) policy keeps
email addresses, phone numbers and the names it lists out of what the model is shown, and
`refuse_if_left` fails a run that would still show one. Seed: `Account`, `Contact`, `Product`,
`Ticket`, `Deal`; `USES`, `REPORTED`, `OWNS_ACCOUNT`, `BLOCKS_DEAL`.

**Question.** "Which open tickets does this account have, and is any of them tied to a deal in
progress?"

**Ready:** HubSpot and Zendesk are providers of the Connectors catalog. Redaction finds personal
data by pattern, so check what it misses on your own records first (see [Limits](./limits.md#data)).

## 4. Standards and regulation tracking

**Problem.** A product must follow a moving specification: a protocol, an API version policy, a
regulation. Changes are published as new revisions of long documents, and the question is always
what changed since the version the product implements, and whether it matters.

**Instance.** A weekly `web` source with `input: sites` and `mode: Crawl` over the specification
site, plus a `web` search for announcements. Seed: `Specification`, `Revision`, `Requirement`,
`Organization`; `SUPERSEDES`, `REQUIRES`, `PUBLISHED`. The repository's
`examples/agent-tooling.yaml` is an instance of this kind for the Model Context Protocol
specification.

**Question.** "What changed between the revision we implement and the newest one?"

**Ready:** yes.

## 5. Grounding for coding and support agents

**Problem.** Agents read the same files and docs again on every task and still miss facts that live
outside the repository. Their answers come without provenance, so a person cannot check them.

**Instance.** A [`files`](./spec-file.md#kind-files) source over a repository's docs and decision
records, plus a `connectors` source over the issue tracker, served to the agent through `ekr mcp`.
Every fact carries the file or record it came from.

**Question.** Any "where is X decided" or "what does Y mean here" question, answered with a
citation.

**Ready:** yes. With the pinned EKR 0.0.32, extracted relations are graph edges, so `search`
counts them in a node's degree and `expand` walks them.
