---
format: aep.planning-md/3
id: architecture-decision-record:answer-service-in-cortex
kind: architecture-decision-record
status: proposed
title: An ask service, its answer records and an eval loop move into cortex
summary: cortex gains a long-running ask service with its own domain and tables beside the store, and eval as a batch verb
relations:
- decides: epic:instance-answers-questions
revision: 1
---
## Context

A downstream deployment asks cortex to take on three responsibilities it carries in its own code
today: an ask/answer service over an instance's store with a server-sent-event contract
(`story:ask-service`), answer records with votes and a review queue kept in tables of their own
(`story:answer-records-and-review`), and an eval loop that runs question sets end to end and feeds
reviewed answers back into them (`story:eval-run`, `story:eval-cases-from-review`). None of the
three is specific to that deployment. Each changes what cortex is, so the change is decided here
once, before any of the four stories is implemented.

What cortex is today, verified in this tree:

- A batch command line. Every verb in `src/main.rs` (`enum Command`: `create`, `adopt`, `update`,
  `remove`, `run`, `restore`, `quality`, `source`, `list`, `sources`, `mcp-line`, `setup`,
  `schema`) runs, prints one JSON line and exits; the timers run `cortex run` once per schedule
  (`src/schedule.rs`).
- It serves no port. The spec says so in words (`spec/domains/instance.yaml`, the comment on
  `cortex.instance.MonitoringSpec`: "cortex serves no port"); metrics are a file,
  `metrics.prom`, for a textfile collector or a sidecar (`src/status.rs`). The viewer is
  `ekr view` (`src/schedule.rs`, the viewer unit's `["view", "--port", …]`) and the MCP reader is
  `ekr mcp` (`src/main.rs`, `mcp_line`), both EKR's processes.
- It owns no database tables. Its state is files under the instance directory
  (`website/docs/operating.md`, the instance directory table: `state/`, `runs/`, `snapshots/`,
  `spend.json`, `status/`, `cortex.log`). A PostgreSQL store is EKR's and is opened only through
  `ekr` (`spec/domains/instance.yaml`, `cortex.instance.PostgresStore`; `website/docs/spec-file.md`,
  `connection`). `Cargo.toml` has `rusqlite` and no PostgreSQL client, HTTP server or async
  runtime.
- Its model calls are `claude -p` (or `codex`) subprocesses with no tools
  (`src/extract.rs`, the `timeout … -p --tools "" --setting-sources ""` command). There is no
  HTTP model client and no tool loop.
- Its one evaluation verb is `cortex quality` (`story:quality-judge`): it samples facts and has the
  model judge whether their evidence supports them (`src/quality.rs`). Nothing in cortex asks the
  store a question or scores an answer.

Two stories already in the plan touch the same ground and are not repeated here:
`story:model-through-gateway` (an instance's spec names a model gateway, for extraction) and
`story:token-budget` (token limits where a model's cost is unknown).

## Decision (proposed)

1. **The ask service ships in the cortex repository and release, as a long-running process of its
   own**, `cortex ask serve --spec <instance spec>`, under an ESS domain of its own
   (`spec/domains/ask.yaml`) so the batch domain `cortex.instance` keeps its meaning. It reads the
   instance's store only through the EKR MCP reader (`ekr mcp`, as `mcp-line` starts it today) and
   writes nothing to the store. Its model route is the instance's gateway, as
   `story:model-through-gateway` names it, spoken to as an OpenAI-compatible endpoint.
2. **Answer records, votes and review items are cortex's own tables, never EKR's.** For an
   instance on a `sqlite` store they live in a separate `ask.sqlite` in the instance directory; for
   a `postgres` store, in a schema of their own (not EKR's) reached through a Connectors connection
   of their own, as `store.connection` is reached today. Retention is a spec value, enforced by the
   service.
3. **Eval is a batch verb of the `cortex` binary**, `cortex eval run`, with a deterministic scorer.
   Its `mcp` and `replay` runners need no service; its `ask` runner calls a running service. An
   improvement loop (`cortex eval improve`) is planned only behind a gated-change protocol adopted
   upstream (`eval.gated_change/1`); until that protocol is published, cortex records eval cases and
   runs, and keeps no change on its own say-so.

## Options considered

| option | what it costs | why not (or why) |
|---|---|---|
| A. Leave all three outside cortex | nothing in cortex; every deployment writes and maintains its own service, tables and eval harness | the request exists because the deployment's code is generic; a second deployment would copy it |
| B. A separate component in its own repository | a second release pipeline, a second spec repository and pin set, and a contract between the two that both must keep | the service is configured from the same instance spec and reads the same store; splitting the spec across repositories is the larger cost |
| C. Fold everything into the batch `cortex` binary and the `cortex.instance` domain, tables in the store's database | an async runtime, an HTTP server and a PostgreSQL client in every `cortex run`; cortex writing beside EKR in EKR's database; the domain's "serves no port" statement reversed | couples the scheduled batch path to a service's dependencies and failure modes, and makes cortex a second writer in EKR's schema |
| D. (proposed) Same repository and release, a service subcommand with its own domain, its own tables beside the store, eval as a batch verb | new dependencies (HTTP server, async runtime, HTTP model client, PostgreSQL client) behind the service path; a second ESS domain; a long-running process to operate; a retention job | keeps the batch path as it is, gives the service one spec and one release with the instance it serves, and keeps EKR the store's only writer |

## Consequences

- cortex gains a long-running process. Its health, its limits (steps, bytes, time, per identity)
  and its request log (`story:ask-request-log`) become cortex's to specify, test and document.
- The statement "cortex serves no port" stays true of `cortex.instance` and becomes false of
  `cortex ask serve`; the operating page says which process serves what.
- cortex owns a second set of data: answer records hold questions and answers, which may carry
  personal data. Retention, purge and what is never logged are part of the spec, not an option.
- The release carries more dependencies; build time and the container image
  (`story:container-image`) grow.
- Eval cases and runs become files cortex writes and reads; scoring stays deterministic so a run is
  reproducible.
- No implementation of `story:ask-service`, `story:answer-records-and-review`, `story:eval-run` or
  `story:eval-cases-from-review` starts until the operator accepts this record.

## What stays outside cortex

- Verifying a bearer token, admitting identities and limiting requests at the edge: a guard in
  front of the service (another component's request). cortex reads a verified identity it is
  handed; it verifies no token in code beyond what the spec names.
- The identity provider, its clients and domains.
- The model backend itself: renting, waking and stopping GPU capacity, spend caps of a backend, and
  the gateway's own metrics.
- Any chat front-end and posting into a chat thread.
- Claim submission and decision in the store (an EKR request); research runs depend on it
  (`story:research-run`).
- The gated-change protocol `eval.gated_change/1` and its conformance cases: cortex consumes it once
  published, and does not define it.
- Rendering answers, vitals or topics in a user interface.
