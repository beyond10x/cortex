---
format: aep.planning-md/3
id: story:ask-outcome-truthful
kind: story
status: implemented
title: An answer's recorded outcome says what happened
summary: A reader closing the stream is recorded as `client-closed`, not `model-failed`; a `length` or filtered finish is not recorded as `answered`
tags:
- review-20261030
relations:
- decomposes: epic:instance-answers-questions
- serves: vision:self-updating-instances
scope:
- confidence: cited
  path: generated/cortex-model
- confidence: cited
  path: generated/docs/docs/domains/cortex-ask.md
- confidence: cited
  path: generated/schema
- confidence: cited
  path: spec/domains/ask.yaml
- confidence: cited
  path: spec/suite.json
- confidence: cited
  path: src/ask/mod.rs
- confidence: cited
  path: src/ask/model.rs
- confidence: cited
  path: src/ask/records.rs
- confidence: inferred
  path: src/ports.rs
- confidence: inferred
  path: tests/answer_records.rs
- confidence: cited
  path: tests/ask_adversary.rs
- confidence: cited
  path: tests/conformance.rs
- confidence: cited
  path: website/data/ess/cortex-ask.domain-graph.json
- confidence: cited
  path: website/docs/ask-service.md
- confidence: cited
  path: website/docs/reference/ess/cortex-ask.md
revision: 21
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T10:11:01Z", actor: "human:timo", revision: 19}
- {from: "proposed", to: "active", at: "2026-10-30T10:11:01Z", actor: "human:timo", revision: 20}
- {from: "active", to: "implemented", at: "2026-10-30T15:30:08Z", actor: "human:timo", revision: 21, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
## Outcome

- A reader who closes the answer stream is recorded as `model-failed` (`src/ask/mod.rs:426`,
  `:486-487`, `:573`; `src/ask/serve.rs:210`; `src/ask/records.rs:924-928`), which the docs define
  as the gateway not answering (R1-01);
- the answer is `message.content` with an empty default and `finish_reason` is never read
  (`src/ask/mod.rs:496-497`, `src/ask/model.rs:141-184`), so a cut-off or filtered answer is
  recorded `answered` (R1-02).

## Acceptance

1. A client that disconnects mid-answer yields a record with outcome `client-closed` and the
   tokens of the steps already taken (new case in `tests/ask_adversary.rs`).
2. A gateway stand-in answering `finish_reason: "length"` yields an outcome that is not `answered`
   and opens a review item.
3. `spec/domains/ask.yaml` declares both outcomes; `task drift` clean.

## Findings it closes

R1-01, R1-02.

## Scope

Derived 2026-10-30 by `story-scoper` against `wave/20261030a` (b61a56c8). Its `src/`, `spec/`, `tests/` and `generated/` match main a870a628, so the body's line numbers hold. Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** the ask answer loop and its records, `src/ask/{mod,model,records}.rs`, plus the `cortex.ask` domain in `spec/domains/ask.yaml` — cited
- **Body citations checked:**
  - `src/ask/mod.rs:426` (`gone` = `model_failed("the request was closed before the answer")`): confirmed — cited
  - `src/ask/mod.rs:486-487` and `:573`: confirmed — cited
  - Correction: `gone()` is also returned at `:447-448` (first status), `:519-520` (answer delta) and `:544-548` (thinking delta/discard). That makes five call sites, not three — cited
  - `src/ask/serve.rs:210` (`tx.send(event).is_ok()`, the false that ends the loop): confirmed — cited
  - `src/ask/records.rs:924-928` (`ModelFailed` gives `AnswerOutcome::Failed`, reason `"model-failed"`, `&zero` tokens): confirmed — cited
  - `src/ask/mod.rs:496-497` (`tool_calls.is_empty()`, then `content.unwrap_or_default()`): confirmed — cited
  - `src/ask/model.rs:141-184` (`parse`, never reads `/choices/0/finish_reason`): confirmed — cited
  - "the docs define [model-failed] as the gateway not answering": `spec/domains/ask.yaml:483-484` (`ModelFailed` summary) and `website/docs/ask-service.md:106`. The body names no file, so this mapping is mine — inferred
- **Files and symbols edited:**
  - `src/ask/mod.rs`:
    - `answer` 414-579: the `gone` closure at 426 has to carry `usage` (declared at 481) and the step, so it moves or takes arguments. Changes at the five call sites. The answered branch 496-540 gains a `finish_reason` check before `Answered` — cited (sites), inferred (shape)
    - `final_event` 355-389: new arms; the match on the generated `AskOutcome` is exhaustive — cited
    - a new constructor beside `model_failed` 405-409 — inferred
  - `src/ask/model.rs`: `Reply` 45-53 gains a finish-reason field, and `parse` 140-184 reads it — cited
  - `src/ask/records.rs`:
    - `Keeper::record` 904-975: new arms in the match at 917-938. The new outcomes' tokens and steps go in place of `&zero` (905-916) — cited
    - `outcome_name` / `outcome_of` 632-648 gain the new `AnswerOutcome` variant(s) — cited
  - `src/ask/serve.rs`: no edit expected. 210 is where the defect starts, not where the fix goes — inferred
  - `src/ask/records/postgres.rs`: no edit expected. It reuses `outcome_name`/`outcome_of` (26, 829, 936), and `outcome` is `TEXT NOT NULL` with no CHECK (46) — inferred
  - `src/ports.rs`: only if a new error payload needs a `generate_*` the ports lack. `generate_integer` (145/306) and `generate_cortex_ask_ask_done` (179/326) already exist — inferred
- **Spec (`spec/domains/ask.yaml`):**
  - `cortex.ask.AnswerOutcome` 137-143 (`variants: [answered, failed, limited]`) gains `client-closed` and an outcome for a cut-off or filtered finish — cited (Acceptance 1, 3)
  - new error type(s) beside `ModelFailed`/`ReaderFailed` 478-491; for client-closed it carries the token counts — inferred
  - `cortex.ask.Ask` outcomes 545-594 gain both as `external:` branches — inferred
  - comments to update: `AskError` reason list 226-228, `AnswerRecord` "token counts … 0 without one" 337-347, `RecordAnswer` 600-604 — cited
- **Generated (`task drift`, Taskfile.yml:20-35):**
  - `generated/cortex-model/src/{ask.rs,behaviour.rs,ports/cortex_ask.rs}`, `generated/cortex-model/{plan.json,PLAN.md}` — cited (they name `ModelFailed`/`AnswerOutcome` today)
  - `generated/schema/schema/types/cortex.ask.AnswerOutcome.schema.json`, `.../entities/cortex.ask.AnswerRecord.schema.json`, `.../commands/cortex.ask.RecordAnswer.schema.json`, `.../events/cortex.ask.AnswerRecorded.schema.json`, plus new `.../errors/cortex.ask.<New>.schema.json` — cited / inferred for the new files
  - `generated/docs/docs/domains/cortex-ask.md`, `spec/suite.json` — cited
  - `generated/spec-types/` is untouched unless `AskEvent`/`AskError` fields change: it holds none of the outcome names today — cited
- **Documents:**
  - `website/docs/reference/ess/cortex-ask.md` and `website/data/ess/cortex-ask.domain-graph.json`, regenerated by `task docs-generate` and checked by `docs-check` — cited
  - `website/docs/ask-service.md` (hand-written): error table 100-108, records paragraph 155-160, review-cause table 182-185 — cited
- **Tests:**
  - `tests/ask_adversary.rs`:
    - new client-disconnect case beside `a_closed_request_stops_the_loop_and_its_reader` 647-694 — cited (Acceptance 1)
    - new `finish_reason: "length"` case using `text_reply` 112 — inferred file. The body names no file for Acceptance 2
    - this file has no records readback today; it needs a helper that opens `ask.sqlite` or `Records` — cited (absence) / inferred (helper)
  - `tests/conformance.rs` 1042-1074 (exhaustive `AskOutcome` match) and 1107-1111 (`AnswerOutcome` list) — cited
  - `tests/answer_records.rs` 462-487: the existing `Failed`/`"model-failed"` assertions stay valid. It is the other place an `answers()` readback exists — inferred
- **Confidence:** high for the code sites, because the body cites them and I read each. Medium for the spec shape (new `AnswerOutcome` variants and new `Ask` error outcomes vs. a reason string only), which the acceptance leaves open
- **Would collide with:** any unit touching the `answer` loop or `final_event` in `src/ask/mod.rs`, `Keeper::record` or `outcome_name`/`outcome_of` in `src/ask/records.rs`, `model::parse`/`Reply`, the `cortex.ask` domain in `spec/domains/ask.yaml` and its whole `generated/` + `spec/suite.json` + `website/docs/reference/ess` projection, `tests/conformance.rs`'s ask driver, or `website/docs/ask-service.md` — cited
- **Safety fact:** outcomes are stored as free `TEXT` with no CHECK (`src/ask/records.rs:106`, `src/ask/records/postgres.rs:46`), so a new variant needs no migration. But `outcome_of` (records.rs:640-648, postgres.rs:829) makes an older cortex fail to read a row written with the new name. That makes rollback one-way unless `SCHEMA_VERSION` (records.rs:145) is bumped so the older binary refuses the file cleanly — level 2, unproven

## Decisions

Taken 2026-10-30 by the wave coordinator.

- New `AnswerOutcome` variants `client-closed` and `cut-off`; `cut-off` covers `finish_reason` `length` and `content_filter`.
- A `client-closed` answer records its tokens and steps and opens no review item; `cut-off` opens one.
- No `SCHEMA_VERSION` bump.
