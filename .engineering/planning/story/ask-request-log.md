---
format: aep.planning-md/3
id: story:ask-request-log
kind: story
status: draft
title: The ask service logs one line per request with its request and trace ids
summary: request_id, trace_id, verified identity, status and latency per request; never the question text or a token
relations:
- decomposes: epic:instance-answers-questions
- serves: vision:self-updating-instances
- depends_on: story:ask-service
revision: 1
---
## Outcome

`cortex ask serve` writes one structured log line per request: its `request_id` (the incoming
`X-Request-Id`, or a new one it then returns), the `trace_id` of an incoming `traceparent`, the
verified identity it was handed, the endpoint, status, latency and the answer's token counts, and
never the question's text or any token. The request id is passed on to the MCP reader and the
model gateway on every call the request makes. It ships with the service, so a deployment needs no
logging code of its own.

No implementation starts until the operator accepts
`architecture-decision-record:answer-service-in-cortex`, through `story:ask-service`.

## Why

A downstream deployment carries its own structured-log and request-id module for its answer
service. Verified in this tree: cortex's only log is `cortex.log`, one JSON line per run
(`src/run.rs`, `log_line`; `website/docs/operating.md`), with no request or trace id; nothing in
cortex reads `X-Request-Id` or `traceparent`, since cortex serves no port
(`spec/domains/instance.yaml`, `cortex.instance.MonitoringSpec`).

## Domain relations

No new entity. New value type `cortex.ask.RequestLogLine` (`format` `cortex.request-log/1`, the
fields above) in `spec/domains/ask.yaml`, with a comment stating what is never written.

## Work

- Spec first: `RequestLogLine`; validate, conformance, `task generate`. Then the code.
- One line per request on standard error (or a file the spec names), the id propagated as
  `X-Request-Id` on outgoing calls.
- Docs: the service page.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- A request carrying `X-Request-Id: r1` and a `traceparent` logs one line with `request_id: r1`
  and the trace id from the header, and the stand-in model and MCP reader each saw `X-Request-Id:
  r1`.
- A request without `X-Request-Id` gets one, returned in the response header and equal to the
  logged one.
- No log line contains the question's text or the bearer token of the request (both chosen as
  unique strings in the test and searched for).

## Not in this story

- The service (`story:ask-service`).
- Request metrics on EKR's readers or the gateway: other components' requests.
