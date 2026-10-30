---
format: aep.planning-md/3
id: story:ask-error-text
kind: story
status: active
title: HTTP clients of the ask service get no internal paths or SQL text
summary: Store and reader failures answer a stable code and a request id; the detail goes to the log only
tags:
- review-20261030
relations:
- decomposes: epic:instance-answers-questions
- serves: vision:self-updating-instances
- depends_on: story:ask-service-bounds
scope:
- confidence: inferred
  path: generated/cortex-model/src/ask.rs
- confidence: inferred
  path: generated/spec-types/types.rs
- confidence: inferred
  path: spec/domains/ask.yaml
- confidence: cited
  path: src/ask/mcp.rs
- confidence: cited
  path: src/ask/mod.rs
- confidence: cited
  path: src/ask/model.rs
- confidence: cited
  path: src/ask/records.rs
- confidence: cited
  path: src/ask/serve.rs
- confidence: inferred
  path: src/model_map.rs
- confidence: inferred
  path: tests/answer_records.rs
- confidence: inferred
  path: website/docs/ask-service.md
revision: 16
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T10:11:02Z", actor: "human:timo", revision: 15}
- {from: "proposed", to: "active", at: "2026-10-30T10:11:02Z", actor: "human:timo", revision: 16}
---
## Outcome

Internal error text reaches HTTP clients, including the database path and the reader program path
(`src/ask/serve.rs:78-81`; `src/ask/records.rs:155-157`; `src/ask/mcp.rs:54-58`;
`src/ask/model.rs:119-122`; R1-10).

## Acceptance

A store failure on a vote answers 500 with `store-failed` and a request id and no path; the log
line with the same request id carries the detail (new test).

## Findings it closes

R1-10.

## Scope

Derived 2026-10-30 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `src/ask/serve.rs`, the record routes' 500 path, plus `final_event` in `src/ask/mod.rs` — cited
- **Files (verified against a870a628):**
  - `src/ask/serve.rs:78-81`: `store_failed(reason)` writes `reason` to stderr and also returns it as the JSON `message` of the 500 — cited, confirmed
  - `src/ask/records.rs:155-157`: `sql(at)` formats every SQLite error as `"<db path>: <error>"` (12 `map_err(sql(..))` sites) — cited, confirmed
  - `src/ask/mcp.rs:54-58`: `Reader::start` failure `"cannot start the reader <program path>: <e>"` — cited, confirmed
  - `src/ask/model.rs:119-122`: `Gateway::chat` non-2xx is `"the gateway answered <status>: <first 300 chars of body>"` — cited, confirmed. Not in the store's current scope entries; add it
- **Symbols and line ranges the change edits:**
  - `serve.rs` `store_failed` 78-81: stable message plus request id, detail to the log only — cited (story outcome and the code)
  - `serve.rs` `blocking` 84-90, whose join error goes through `store_failed` — cited
  - `serve.rs` callers of `store_failed`: `vote` 232-294 (line 292), `review` 297-320 (line 317), `close` 341-412 (lines 379 and 397-400) — cited
  - `serve.rs` `json_error` 63-65, if the request id becomes a JSON field of the refusal body — inferred
  - `mod.rs` `final_event` 355-389: the `ModelFailed` arm (386) and `ReaderFailed` arm (387) put `e.reason` (gateway body, reader path) into the client's SSE `error.message` — cited (code read; this is the path by which the mcp.rs and model.rs text reaches clients)
  - `serve.rs` `ask` 198-226: the spot to log the detail with `answer_id` before `final_event` (line 225); `answer_id` is created at 199 — inferred
- **Not necessarily edited:** `records.rs` `sql` 155-157, `mcp.rs` 54-58, `model.rs` 119-122 produce the detail. The fix can leave them as log text and filter at `serve.rs`/`final_event` — inferred
- **Spec/generated impact:** none if the request id rides in `message` or only in the record-route JSON, which is hand-built in `serve.rs` and not typed in ESS — inferred. If `/ask` errors gain a `request_id` field, the change reaches `spec/domains/ask.yaml:226-234` (`cortex.ask.AskError`; the comment at 226-228 lists the reasons), `generated/cortex-model/src/ask.rs:76-80`, `generated/spec-types/types.rs:102` (`CortexAskAskError`), `generated/docs/docs/domains/cortex-ask.md`, `website/docs/reference/ess/cortex-ask.md` and `src/model_map.rs:245-255` — inferred
- **Tests:**
  - new test, inferred to go in `tests/answer_records.rs`: a vote with a failing SQLite store answers 500 `store-failed` with a request id and no path, and stderr carries the same id with the detail
  - that test needs a stderr-capturing variant of `serve()` at `tests/answer_records.rs:237-244`, which uses `Stdio::inherit()` today — cited for the line, inferred for the need
  - `tests/ask_adversary.rs:536` and `:560` assert only `reason == "reader-failed"`, so they should survive a changed message — cited, unproven
- **Documents:** `website/docs/ask-service.md:106-107` (meanings of `model-failed` and `reader-failed`) and `:169-170` (refusal shape `{"error","message"}`, no `store-failed` row) — inferred, to be updated if the wire shape gains a request id
- **Confidence:** high for `serve.rs` and `final_event` (story cites them and the code confirms the leak path); medium for spec/generated (depends on whether `request_id` becomes a typed `AskError` field, which the story does not decide)
- **Would collide with:** any unit touching `src/ask/serve.rs` (record routes, `ask` handler, body limit) or `final_event` in `src/ask/mod.rs`. The `depends_on` story `story:ask-service-bounds` covers `serve.rs`, `mod.rs`, `mcp.rs`, `model.rs` and `spec/domains/ask.yaml`, so the two cannot run in parallel. The dependency already orders them.
- **Safety fact:** every client-visible internal text leaves through two functions: `serve.rs:78-81` `store_failed` for the record routes and `mod.rs:386-387` `final_event` for `/ask`. Filtering there leaves the log and the answer record unchanged. Proof level 2: `rg 'store_failed|final_event'` and `rg 'eprintln' src/ask` read, but the record-route JSON fields and SSE `message` consumers outside this repo were not checked. Unproven.

## Decisions

Taken 2026-10-30 by the wave coordinator.

- The request id is a `request_id` field of the record routes' JSON refusal and is appended to the SSE `error.message` as `(request <id>)`; no spec change.
- The detail goes to stderr with the same id.
