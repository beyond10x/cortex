---
format: aep.planning-md/3
id: story:ask-service
kind: story
status: active
title: An instance answers a question as a stream of events over its store
summary: 'cortex ask serve: POST /ask streams status, tool, delta, discard, done and error within the spec''s limits'
relations:
- decomposes: epic:instance-answers-questions
- serves: vision:self-updating-instances
- depends_on: architecture-decision-record:answer-service-in-cortex
- informed_by: story:model-through-gateway
- informed_by: story:token-budget
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-17T03:36:58Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-17T03:36:58Z", actor: "human:timo", revision: 3}
---
## Outcome

`cortex ask serve --spec <instance spec>` answers questions over an instance's store. `POST /ask
{question}` answers `text/event-stream` with the events `status`, `tool`, `delta`, `discard`,
`done` and `error`, in a versioned contract (`cortex.ask-events/1`): `done` carries the answer, the
cited node and evidence ids, token and cache counts and the context size; `error` carries a reason
and, on a 429, `retry_after`. The service runs a tool loop over the EKR MCP reader against the
instance's model gateway, rewrites tool results into compact text under a byte cap, turns citation
markers into links from the spec's link templates, and stops at the spec's step, byte, time and
per-identity limits.

**No implementation starts until the operator accepts
`architecture-decision-record:answer-service-in-cortex`.**

## Why

A downstream deployment runs this as a service of its own over the EKR MCP reader, with a v1 event
contract being written as a red test; none of it is specific to that deployment. Verified in this
tree:

- cortex serves no port (`spec/domains/instance.yaml`, comment on `cortex.instance.MonitoringSpec`)
  and has no HTTP server, async runtime or HTTP model client in `Cargo.toml`.
- Its model calls are `claude -p` subprocesses with no tools (`src/extract.rs`, the `timeout … -p
  --tools "" --setting-sources ""` command); there is no tool loop.
- The store is read by agents through `ekr mcp`, started from the line `cortex mcp-line` prints
  (`src/main.rs`, `mcp_line`; `website/docs/commands.md`).

Related, not repeated: `story:model-through-gateway` gives the spec its `model.gateway` (base URL
and token variable), which this service reuses as its model route; `story:token-budget` bounds
extraction by tokens, while this story's limits bound one answer.

## Domain relations

A new ESS domain `spec/domains/ask.yaml` (`cortex.ask`), per the record. No new entity: an answer
is streamed, not stored (storage is `story:answer-records-and-review`). New value types: `AskSpec`
(an optional `ask` section of `cortex.instance.InstanceSpec`: listen address, limits
`max_steps`, `max_result_bytes`, `max_seconds`, `per_identity_per_minute`, `link_templates`,
`guidance`), `AskEvent` (a union tagged by event name, one variant per event above), and the
command `cortex.ask.Ask`. The model route is a reference to `cortex.instance.Model`'s `gateway`
(added by `story:model-through-gateway`), not a second copy.

## Work

- Spec first: `spec/domains/ask.yaml` with `AskSpec`, `AskEvent` and `Ask`; the optional `ask`
  field on `cortex.instance.InstanceSpec`; validate with the pinned `ess`, conformance, `task
  generate`. Then the code.
- A `cortex ask serve` subcommand: HTTP endpoint, the identity it is handed by the guard in front
  of it (header named in the spec), the tool loop over `ekr mcp`, result rewriting with the byte
  cap, citation links, limits, the event stream.
- Tests: a stand-in model that answers a scripted tool loop, a stand-in MCP reader, and the event
  contract as a conformance-style test of each event's fields.
- Docs: a new page for the service and its event contract; `operating.md` on which process serves
  what.

## Acceptance

- `ess specify validate` passes for both domains and `task drift` is clean after `task generate`.
- With a stand-in model scripted to call one MCP tool and then answer citing one node, `POST /ask`
  streams `status`, `tool`, at least one `delta`, then `done` whose cited node ids equal the
  scripted one and whose token counts equal the stand-in's.
- A scripted loop longer than `max_steps` ends with `error` naming the step limit, and no `done`.
- A tool result larger than `max_result_bytes` reaches the model cut to that many bytes.
- The `per_identity_per_minute`+1-th request of one identity in a minute answers 429 with an
  `error` event carrying `retry_after`.
- A request without the identity header the spec names answers 401 and asks no model.

## Not in this story

- Keeping answers, votes and reviews (`story:answer-records-and-review`).
- The request log (`story:ask-request-log`).
- Entity matches before the answer (`story:question-entity-linking`).
- Verifying a bearer token, the identity provider, the model backend: outside cortex, per the record.
- The gateway itself (`story:model-through-gateway`).
