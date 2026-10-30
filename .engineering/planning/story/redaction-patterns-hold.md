---
format: aep.planning-md/3
id: story:redaction-patterns-hold
kind: story
status: active
title: Redaction neither panics nor leaks on non-ASCII hosts, slashed phone numbers or bracketed query keys
summary: '`link()` slices on char boundaries; the international phone separator admits `/`; a URL body admits `[` and `]`'
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
scope:
- confidence: cited
  path: src/redact.rs
- confidence: inferred
  path: website/docs/limits.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T10:11:02Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-10-30T10:11:02Z", actor: "human:timo", revision: 7}
---
## Outcome

The redaction policy handles three inputs it mishandles today:
- a host-only link whose fourth byte is inside a multi-byte character (`café.fr/menu`) panics the
  run under the `Url` class: `s[..4]` at `src/redact.rs:597`, reached from `:162-170` and `:694`
  (R4-01);
- `+49 30/1234567` matches only `+49 30`, which the refine rejects, so the number leaks: the
  international separator lacks `/` (`src/redact.rs:205`, `:210`, `:552-568`; R4-05);
- a URL body stops at `[`, so `?filter[owner]=x` keeps the rest unredacted (`src/redact.rs:167`;
  R4-17).

## Acceptance

Unit tests in `src/redact.rs`: `café.fr/menu` and `中国.cn/x` are redacted without a panic;
`+49 30/1234567` is replaced whole; `https://crm/x?filter[owner]=jane_doe` is replaced whole.

## Findings it closes

R4-01, R4-05, R4-17.

## Scope

Derived 2026-10-30 by `story-scoper` against main `a870a628` (the body's citations were read again on
`wave/20261030a`, which has no change under `src/`, `spec/`, `generated/` or `tests/` since then).
Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `src/redact.rs`, the built-in `Url` and `Phone` patterns and their refines. Every edit is in this one file — cited
- **Files:** `src/redact.rs:597`, `s[..4].eq_ignore_ascii_case("www.")` inside `link` (lines 583-605). This is the byte slice that panics when byte 4 falls inside a multi-byte character. It is the only `s[..N]` literal slice in the file — cited (confirmed)
- **Files:** `src/redact.rs:162-170`, `fn url()`. The body pattern at `:167`, `(?:[^\s<>"'`{}|\\^\[\]]|\[masked:[a-z0-9\-]+\])*`, excludes `[` and `]`, so a match stops at `?filter[` — cited (confirmed)
- **Files:** `src/redact.rs:694`, where `built_in` maps `RedactionClass::Url` to `(url(), Some(link))` — cited (confirmed)
- **Files:** `src/redact.rs:202-224`, `fn phone()`. Line `:205` is `let sep = r"[\p{Zs}.\-]"`, which has no `/`. Line `:210` is the international branch `(?:\+|\b00)[1-9][0-9]{0,2}(?:{sep}?\(?[0-9]{1,4}\)?){2,5}` — cited (confirmed)
- **Files:** `src/redact.rs:552-568`, `fn phone_number`. It needs 7 to 15 digits, so `+49 30` (4 digits) is rejected and nothing is replaced. The refine is read, not edited — cited (confirmed)
- **Symbols:** `link`, `url`, `phone`, `phone_number`, `built_in`, `m::RedactionClass::{Url, Phone}` — cited
- **Also likely:** `src/redact.rs:698` (`Phone => (phone(), Some(phone_number))`) is read, not edited — cited
- **Also likely:** the edit will be narrower than changing the shared `sep`. The fix will add `/` to the international branch only, for example through a separate separator. `sep` is also used by the parenthesised (`:212`), North American (`:214`) and national (`:216`) branches, and the national branch already allows one `/` explicitly — inferred
- **Also likely:** `link` (`:583-605`) needs `]` added to its trailing trim, the way it trims an unbalanced `)`. Otherwise, once the body admits `]`, a link written in brackets would take the closing `]` with it. The `\[masked:…\]` alternative at `:167` is then redundant — inferred
- **Tests:** new unit tests in `src/redact.rs` `mod tests` (`:1440-2592`), next to `a_link_without_a_scheme_does_not_reach_the_model` (`:2472`) and `phone_with_00_international_prefix_is_replaced` (`:1582`). Cases: `café.fr/menu`, `中国.cn/x`, `+49 30/1234567`, `https://crm/x?filter[owner]=jane_doe` — cited (Acceptance names `src/redact.rs`, the module was read)
- **Tests:** none added under `tests/`. `tests/redaction.rs` and `tests/redaction_beside_marks_adv.rs` drive `cortex_cli::redact` and are not edited — inferred
- **spec/domains / generated:** none. `cortex.instance.RedactionClass` (`spec/domains/instance.yaml:571-573`) and `generated/cortex-model/src/instance.rs` keep their variants. No type, field or schema changes — cited
- **Documents:** none required. `website/docs/limits.md:48-51` ("Links are found by their shape") does not list the excluded characters and can stay as it is. An optional line saying brackets in a query stay inside the link would land there — inferred
- **Confidence:** high. The story names every defect site, and all seven line numbers check out against the tree
- **Would collide with:** any unit that edits `src/redact.rs`, especially the built-in patterns (`:155-260`), the refines (`:470-610`) or its `mod tests` (`:1440-`). That includes any other `review-20261030` redaction story
- **Safety fact:** `url()` and `phone()` are each read only by `built_in` (`:694`, `:698`). Widening them changes only the `Url` and `Phone` classes, and `phone_number` still filters each match to 7-15 digits. Level 2 (`grep -n 'phone()\|url()' src/redact.rs`), unproven. Still to prove: a `/`-separated date or fraction after `+`/`00` is not taken as a phone number, and a `[Class-N]` placeholder from an earlier class inside a link is not swallowed now that `[` is admitted

## Decisions

Taken 2026-10-30 by the wave coordinator.

- `/` joins the international phone branch only.
- `link` trims an unbalanced trailing `]` as it trims `)`.
- Tests cover a `[Class-N]` placeholder inside a link and a `+1 2/3/2024`-shaped date.
