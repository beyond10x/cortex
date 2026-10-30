---
format: aep.planning-md/3
id: story:mask-quoted-credentials
kind: story
status: implemented
title: The always-on mask catches quoted, `@`-bearing, percent-separated and truncated credentials
summary: '`"password": "…"`, `p@ss` userinfo, `%3A` separators, raw `%` in values and a PEM block cut before its END line are masked'
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
scope:
- confidence: cited
  path: src/mask.rs
- confidence: cited
  path: tests/credential_mask.rs
- confidence: inferred
  path: website/docs/limits.md
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T10:11:02Z", actor: "human:timo", revision: 7}
- {from: "proposed", to: "active", at: "2026-10-30T10:11:02Z", actor: "human:timo", revision: 8}
- {from: "active", to: "implemented", at: "2026-10-30T15:30:11Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
## Outcome

The always-on credential mask (`src/mask.rs`) catches five shapes it misses today:
- a JSON-quoted name, `"password": "…"`: the separator must follow the name directly
  (`src/mask.rs:28`, `:30`, `:37`; the opt-in class at `src/redact.rs:316` accepts the quote; R4-03);
- a URL password with a raw `@` (`postgres://app:p@ss-word@db/x`), class `[^\s/?#@]+` at
  `src/mask.rs:77-79` (R4-07);
- a value with a raw `%` that is not an escape (`src/mask.rs:34`; R4-12);
- a PEM private-key block without its END line, as left by the head cut (`src/mask.rs:40-44`;
  R4-13);
- `%3A` as separator (`src/mask.rs:30`; R4-14).

## Acceptance

One case per shape in `tests/credential_mask.rs`, each asserting the secret is absent from the
masked text; the existing cases stay green.

## Findings it closes

R4-03, R4-07, R4-12, R4-13, R4-14. Widening the always-on name list (R4-06) is a documented
limit (`website/docs/limits.md:73-77`) and not changed here.

## Scope

Derived 2026-10-30 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `src/mask.rs` (the always-on mask, one module, `pub mod mask` at `src/lib.rs:13`) — cited
- **Files:** `src/mask.rs:28-44` and `:76-80`, the constants `NAME`, `SEPARATOR`, `KEY_VALUE` and the `build` rule table — cited
- **Symbols:** `SEPARATOR` (`src/mask.rs:30`, today `(\s*(?:[:=]|%3d)\s*)`: no quote after the name and no `%3a`; this covers R4-03 and R4-14) — cited
- **Symbols:** `NAME` (`src/mask.rs:28`) and the `assignment` format in `build` (`src/mask.rs:37`, `{NAME}{SEPARATOR}["']?{value}`; a quote is allowed only before the value) — cited
- **Symbols:** `KEY_VALUE` (`src/mask.rs:34`; a raw `%` that is not an escape ends the value, R4-12). `TEXT_VALUE` (`:32`) already admits a raw `%`, so R4-12 affects only `mask_key` — cited
- **Symbols:** the `url-password` rule (`src/mask.rs:76-80`, pattern at `:79`, `([^\s/?#@]+)@`: the password class excludes `@`, R4-07) — cited
- **Symbols:** the `private-key` rule (`src/mask.rs:40-44`, pattern at `:42`, needs `-----END … PRIVATE KEY-----`, R4-13) — cited
- **Reference only, not edited:** `src/redact.rs:315` (`(["']?\s*(?:=>|[:=]|%3d)\s*)["']?{value}`), the opt-in `Credential` class's separator that accepts the quote. The story says `:316`; the line is 315 — cited
- **Tests:** `tests/credential_mask.rs` (489 lines, end-to-end through `common::World`); one new case per shape, existing cases at `:115-489` unchanged — cited
- **Also likely:** `src/mask.rs:134-267` (`mod tests`), unit cases next to the rules, following the existing userinfo and percent-encoding tests — inferred
- **spec/domains:** no type change. `spec/domains/ask.yaml:344-346` and `spec/domains/instance.yaml:1258`, `:1710-1711` and `:1820` describe masking in comments only and name no shape list — cited
- **generated/:** no impact; `git grep 'masked:' -- generated` finds nothing, and no shape name is added or renamed — cited
- **Documents:** `website/docs/limits.md:24-34` lists the always-on shapes. It may gain a clause for quoted names and `@` passwords, but the acceptance does not require it. `website/docs/limits.md:73-77` (R4-06) is not changed, as the story says — inferred
- **Confidence:** high — the story names every rule line, and each was confirmed in `src/mask.rs`. Only the location of the PEM "head cut" is unconfirmed (see below).
- **Would collide with:** any unit editing `src/mask.rs` or `tests/credential_mask.rs`. Less likely, a unit relying on stored document-key identity in `src/state.rs` or `src/run.rs` around the `mask_key` call — inferred
- **Safety fact:** `build` makes both rule sets (`text_rules` and `key_rules`, `src/mask.rs:88-96`), so a wider `SEPARATOR` or `url-password` also changes `mask_key`. `State::load` keeps only keys where `mask_key(k).1 == 0` (`src/state.rs:98-100`). So after this change, a stored key that newly matches is dropped once and that document is read again under its new masked key (`src/run.rs:674`). This is a one-time key change for the affected documents, not a repeated one — level 2, unproven

## Decisions

Taken 2026-10-30 by the wave coordinator.

- R4-12 is tested on `mask_key`, where the gap is.
- A stored key that newly matches is dropped once and the document read again; the CHANGELOG entry says so.
