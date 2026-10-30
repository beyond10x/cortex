---
format: aep.planning-md/3
id: story:ask-service-bounds
kind: story
status: active
title: The ask service bounds answers in flight, bodies, tool calls and reader start-ups
summary: A global semaphore on answers, smaller vote and review bodies, a cap on tool calls per step and on gateway body size, a reused reader, and the redaction policy compiled once
tags:
- review-20261030
relations:
- decomposes: epic:instance-answers-questions
- serves: vision:self-updating-instances
- depends_on: story:ask-outcome-truthful
scope:
- confidence: cited
  path: generated/cortex-model/src/ask.rs
- confidence: cited
  path: generated/docs/docs/domains/cortex-ask.md
- confidence: cited
  path: generated/schema/schema
- confidence: cited
  path: generated/spec-types
- confidence: cited
  path: spec/domains/ask.yaml
- confidence: cited
  path: spec/suite.json
- confidence: cited
  path: src/ask/limit.rs
- confidence: cited
  path: src/ask/mcp.rs
- confidence: cited
  path: src/ask/mod.rs
- confidence: cited
  path: src/ask/model.rs
- confidence: cited
  path: src/ask/serve.rs
- confidence: inferred
  path: src/model_map.rs
- confidence: inferred
  path: tests/answer_records.rs
- confidence: cited
  path: tests/ask_adversary.rs
- confidence: inferred
  path: tests/ask_service.rs
- confidence: inferred
  path: website/docs/ask-service.md
- confidence: inferred
  path: website/docs/reference/ess/cortex-ask.md
- confidence: inferred
  path: website/static/schemas/instance-spec.schema.json
revision: 24
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T10:11:02Z", actor: "human:timo", revision: 22}
- {from: "proposed", to: "active", at: "2026-10-30T10:11:02Z", actor: "human:timo", revision: 23}
---
## Outcome

- No global cap on answers in flight: each holds a blocking thread up to `max_seconds` and starts
  one `ekr mcp` (`src/ask/serve.rs:84-88`, `:198`; `src/ask/mod.rs:449`; R1-03);
- a new `connections launch … ekr mcp` per question, with a cold store open (`src/ask/mcp.rs:33-58`,
  `src/ask/mod.rs:165-180`, `:447-451`; P-08);
- votes and review closes accept 1 MiB bodies outside the limiter (`src/ask/serve.rs:135`,
  `:232-294`; R1-05);
- any number of tool calls per step and an unbounded gateway body (`src/ask/mod.rs:554`,
  `src/ask/model.rs:117`; R1-06);
- the redaction policy is compiled twice per question, the first time on the async worker
  (`src/ask/serve.rs:174-179`, `src/ask/mod.rs:211`, `:307`, `:435`; P-09, R1-13).

## Acceptance

1. With `ask.max_in_flight: 2`, a third concurrent question is answered `busy` (429) at once
   (new test).
2. A vote body over the new limit is refused with 413.
3. A model step with more tool calls than the cap is cut to the cap and the answer says so.
4. The redaction policy is compiled once per service start (a counter seam or a code read in
   review; say which).
5. Readers: either a pooled `ekr mcp` reader serves two consecutive questions with one spawn
   (spawn count in a stand-in), or the story records why per-question start stays.

## Findings it closes

R1-03, R1-05, R1-06, P-08, P-09, R1-13 (residual).

## Scope

Derived 2026-10-30 by `story-scoper`, on the tree at b61a56c8. That tree has the same `src/`, `spec/`, `generated/` and `tests/` as a870a628. Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `src/ask/` (the answer service: `serve.rs`, `mod.rs`, `mcp.rs`, `model.rs`) — cited
- **Files and symbols (`src/ask/serve.rs`):**
  - `Service` :29-33, which gains the in-flight semaphore — cited lines; the edit is inferred
  - `BODY_LIMIT` :135 — cited
  - `ask` :137-229, with the rate limiter at :179-194 and the answer's blocking thread at :198-226 — cited
  - `vote` :232-294 — cited
  - `close` :341-412: the story does not cite it, but the same 1 MiB `DefaultBodyLimit` layer at :462 covers it — cited from the tree
  - `serve` :420-469, where the `Service` is built at :438-442 and the router layer is at :457-463 — cited
- **Files and symbols (`src/ask/mod.rs`):**
  - `check` :86-137, which refuses a new limit of 0 or less — cited lines; the edit is inferred
  - `Settings` :196-205 and `Settings::of` :209-236, which compiles the policy and drops the result at :211 — cited
  - `question_refused` :306-313, compile at :307 — cited
  - `final_event` :355-389, which gains a `busy` reason — cited lines; the edit is inferred
  - `answer` :414-579: compile at :435-439, reader start at :447-454, tool-call loop at :553-576 — cited
- **Files and symbols (`src/ask/mcp.rs`):** `Reader` :16-23, `Drop for Reader` :25-30, which kills the child, and `Reader::start` :34-102, which spawns at :54-59 — cited
- **Files and symbols (`src/ask/model.rs`):** `Gateway::chat` :93-125; the body is read with no bound by `response.text()` at :117 — cited
- **Corrections to the body's citations:**
  - `serve.rs:84-88` is the `blocking` helper of the vote, review and close store calls, not the answer's thread. The answer thread is `serve.rs:198`, which the body also cites. — cited
  - `mod.rs:449` is the closing `}` of the status emit; `mcp::Reader::start` is at `:450` — cited
  - `mod.rs:165-180` ends inside the `launch` arm, which runs to `:183`; the `None` arm with plain `ekr mcp` is `:184` — cited
  - `mcp.rs:33-58` should be `:34-59` for the spawn, or `:34-102` for the start with its handshake — cited
  - The body cites `:135`, `:232-294`, `:554`, `model.rs:117`, `:174-179`, `:211`, `:307`, `:435` and `:447-451`; each was read and confirmed — cited
- **Also likely:** `src/ask/limit.rs` (`RateLimiter` :12-51). The story's store scope lists it, but the body cites no line in it. A new in-flight type may live there, or the semaphore may sit in `serve.rs` instead. — inferred
- **Also likely:** `src/model_map.rs` `ask_spec` :173-212. It maps every `AskLimits` and `AskSpec` field and holds the only struct literal of `ma::AskLimits` / `ma::AskSpec` in `src/`, `tests/` and `crates/`, so any new spec field must be mapped there. — cited lines; the edit is inferred
- **Spec (`spec/domains/ask.yaml`):**
  - `cortex.ask.AskLimits` :32-38 and its comment :18-31, for a tool-call cap and possibly the in-flight cap — cited lines; the edit is inferred
  - `cortex.ask.AskSpec` :79-90, if `max_in_flight` sits on the `ask` section as Acceptance 1 writes it — cited lines; the placement is inferred
  - `cortex.ask.AskError` reason list :226-228, which gains `busy` — cited lines; the edit is inferred
  - A new error beside `cortex.ask.RateLimited` :470-476 — inferred
  - A `busy` outcome in `cortex.ask.Ask` :554-560 — inferred
  - Possibly `cortex.ask.AskTool` :175-183 or `AskStatus`, for "the answer says so" when tool calls are cut — inferred
- **Generated (`task generate`, then `task drift` clean):**
  - `generated/cortex-model/src/ask.rs` (`AskLimits` :106, `AskSpec` :119) — cited
  - `generated/spec-types/types.rs`, `source.schema.json`, `types-report.json` — cited
  - `generated/schema/schema/types/cortex.ask.AskLimits.schema.json` and `cortex.ask.AskSpec.schema.json` — cited
  - `generated/schema/schema/types/cortex.instance.InstanceSpec.schema.json` — cited
  - `generated/schema/schema/commands/cortex.instance.{Create,Update,Adopt}Instance.schema.json` — cited
  - `generated/schema/schema/commands/cortex.ask.Ask.schema.json` and a new `errors/cortex.ask.<Busy>.schema.json` — inferred
  - `generated/docs/docs/domains/cortex-ask.md` and `spec/suite.json` — cited
- **Documents:**
  - `website/docs/ask-service.md`, a hand-written page: the limits example :32, the limits table from :43, and the reason/status table :102-104 — cited lines; the edit is inferred
  - `website/docs/reference/ess/cortex-ask.md` and `website/static/schemas/instance-spec.schema.json`, regenerated by `docs-generate` (`crates/cortex-docs`) — inferred
- **Tests:**
  - `tests/ask_service.rs`: new cases for the in-flight 429 `busy`, the vote 413 and the tool-call cap, built on its stand-in `ekr` :161-183 and `serve_with` :190 — inferred
  - `tests/ask_adversary.rs`: a pooled reader breaks or changes `every_reader_an_answer_started_is_gone_when_its_stream_ends` :569 and `a_closed_request_stops_the_loop_and_its_reader` :650. The 1 MiB body case is at :834. — cited
  - `tests/answer_records.rs`: the vote and close tests :417 and :720, if the record-route body limit changes — inferred
  - `src/ask/limit.rs` tests :53-70, if the semaphore lives there — inferred
- **Confidence:** high for the `src/ask/` surface, because the body cites lines and each was read against the tree. Medium for the spec, generated and test edits, because the field names, placement and new outcome are design choices the story leaves open.
- **Would collide with:**
  - any unit touching `src/ask/mod.rs` `answer` or `final_event`
  - any unit touching `src/ask/model.rs` `Gateway::chat` or `parse`
  - any unit touching the `cortex.ask.Ask` outcomes or the `AskError` reasons in `spec/domains/ask.yaml`, and with them every file under `generated/` and `spec/suite.json`
  - `tests/ask_adversary.rs`
  - the record routes in `src/ask/serve.rs`
- **Safety fact 1:** `ma::AskLimits` and `ma::AskSpec` are built only in `src/model_map.rs:175-178`, so a new `Optional<Integer>` field breaks no Rust literal. Existing specs (`spec/suite.json`, `examples/`, the website example) stay valid only if the field is Optional. Proven to level 2 (`git grep -n 'AskLimits {\|AskSpec {'`), unproven.
- **Safety fact 2:** `redact::Redactor` (`src/redact.rs:135`) derives neither `Clone` nor `Debug`, while `Settings` derives both (`src/ask/mod.rs:196`). Keeping the compiled policy in `Settings` therefore needs an `Arc` and a manual `Debug`, or must move it onto `Service`. `Batch<'a>` borrows the `Redactor` across the blocking thread. Proven to level 2, unproven.

## Decisions

Taken 2026-10-30 by the wave coordinator.

- `ask.max_in_flight: Optional<Integer>` on `AskSpec` (default 8) and `limits.max_tool_calls: Optional<Integer>` on `AskLimits` (default 16).
- Over the in-flight cap: 429 with reason `busy`, a new error beside `RateLimited`.
- Vote, review and close bodies are limited to 64 KiB per route.
- The gateway body is read with a 4 MiB cap.
- Readers stay per question; the story records why (the two adversary invariants in `tests/ask_adversary.rs:569`, `:650`).
- Correction (2026-10-30, wave coordinator): a reader that departs holds its `max_in_flight` slot until the answer's thread ends, not until it leaves (D14 reverses D4; `review-result:security-w30a-merged-pass-1`, finding 2).
