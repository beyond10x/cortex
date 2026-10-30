---
format: aep.planning-md/3
id: story:spec-integer-bounds
kind: story
status: active
title: Out-of-range numbers in a spec are refused at validation instead of overflowing at run time
summary: '`refresh_after_days`, hook `timeout_s` and `view_port` are bounded in the spec check'
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
scope:
- confidence: cited
  path: spec/domains/instance.yaml
- confidence: inferred
  path: src/main.rs
- confidence: cited
  path: src/spec.rs
- confidence: inferred
  path: src/status.rs
- confidence: cited
  path: tests/spec_forms.rs
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T10:11:03Z", actor: "human:timo", revision: 8}
- {from: "proposed", to: "active", at: "2026-10-30T10:11:03Z", actor: "human:timo", revision: 9}
---
## Outcome

The spec check refuses numbers that overflow later:
- `refresh_after_days * DAY_MS` overflows for large values (`src/state.rs:186`, `:200`,
  `src/sources.rs:94`; only `>= 0` is checked at `src/spec.rs:203-209`; R5-06, R3-12);
- a hook `timeout_s` near `i64::MAX` overflows `Instant` and panics after the run line
  (`src/status.rs:492-498`; `src/spec.rs:370` checks only `<= 0`; R5-05);
- `view_port` is cast `as u16`, so 70000 becomes 4464 and -1 becomes 65535 (`src/main.rs:1059`,
  `:1413`; R5-07).

## Acceptance

`tests/spec_forms.rs` cases: each out-of-range value is refused by `cortex create` with the field
named; the largest accepted value runs without overflow.

## Findings it closes

R3-12, R5-05, R5-06, R5-07.

## Scope

Derived 2026-10-30 by `story-scoper` from the tree of a870a628. Branch `wave/20261030a` at b61a56c8 has no diff under `src/`, `tests/`, `spec/` or `generated/`. Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `src/spec.rs`, `fn check` (lines 180–375, called from `parse` at :167) — cited
- **Story citations, all confirmed at a870a628:**
  - `src/state.rs:186`: `refresh_after_days * DAY_MS` in `SeenState::wants`, with `DAY_MS` at :78 — cited
  - `src/state.rs:200`: the same product in `holds` — cited
  - `src/sources.rs:94`: `start_ms - refresh_after_days * 86_400_000` in `Window::of_run` (fn at :86), a literal rather than `DAY_MS` — cited
  - `src/spec.rs:203-209`: the policy loop in `check` (tuple :203, `value < 0` test :207, refusal :208) — cited
  - `src/spec.rs:370`: `hook.timeout_s.is_some_and(|t| t <= 0)` — cited
  - `src/status.rs:492-498`: `Duration::from_secs(...)` at :492 and the panic site `started + timeout` at :498 in `call_hook`. A negative value goes to `HOOK_TIMEOUT_S` through `u64::try_from` (:494–495), and `HOOK_TIMEOUT_S` is declared at :27 — cited
  - `src/main.rs:1059`: `instance_created.view_port as u16`, in `fn create` (:922) — cited
  - `src/main.rs:1413`: `instance_adopted.view_port as u16`, in `fn adopt` (:1304) — cited
- **Edit range, refresh_after_days:** `src/spec.rs:202-209`. Add an upper bound so that `refresh_after_days * 86_400_000` fits in i64. The ceiling and its message (`sources.<name>.policy.refresh_after_days: <v>`) are the implementor's choice — inferred
- **Edit range, hook timeout_s:** `src/spec.rs:369-372`. Add an upper bound so that `Instant + Duration` cannot overflow — inferred
- **Edit range, view_port:** `check` has no `view_port` check today. Add one for `spec.serve.view_port` in the range 1..=65535, placed near :369. Whether 0 is accepted is open: `main.rs:1017` falls back to 0 when no port is free — inferred
- **Also likely:**
  - `src/main.rs:1059` and `:1413`: `as u16` replaced by `u16::try_from` as defence in depth. Both values come from the spec value pushed into the integers queue at `main.rs:1015-1019` and `:1365-1369` — inferred
  - `src/status.rs:492-498`: `checked_add` rather than `+` at :498 — inferred, optional once the spec bounds the value
  - The `src/state.rs` and `src/sources.rs` sites stay unchanged if the bound is enforced in the spec check — inferred
- **Placement fact:** `check` runs inside `parse`. `load` calls `parse`, and `load` reads an instance's frozen copy (`src/instance.rs:102`, `src/home.rs:92`). A bound in `check` therefore also refuses existing instances whose frozen spec holds an out-of-range value. The alternative is `load_given` (:56), the path `reserved_refusal`, `infinite_refusal` and `gateway_refusal` use, which keeps frozen copies loading. The implementor has to pick one — cited (code read), consequence inferred
- **Spec domain `spec/domains/instance.yaml`:**
  - Field types: `FetchPolicy.refresh_after_days: Integer` (:92), `RunHook.timeout_s: Optional<Integer>` (:672), `ServeSpec.view_port: Optional<Integer>` (:642) — cited
  - No type change: ESS `Integer` carries no range in this domain or its generated schema (`generated/schema/.../MonitoringSpec.schema.json:39` is a bare `"type": "integer"`) — cited
  - At most, the type comments gain the bound, for example the RunHook comment at :658–667 ("absent means 10") — inferred
  - The store records this file as `cited`, but the story body never names it. Downgrade it to `inferred`, or remove it — cited (from `aep plan artifact show`)
- **generated/:** no change expected. Types are unchanged, and the type-level yaml comments do not reach `website/docs/reference/ess/cortex-instance.md`: the RunHook comment text is absent from that page, while :158 renders the field as "`refresh_after_days` — `Integer`". `website/static/schemas/instance-spec.schema.json` comes from `ess generate --kind schema` and has bare integers at :361, :952 and :1078 — inferred
- **Tests:**
  - `tests/spec_forms.rs` (342 lines) gets new cases that reuse `spec_text` (:19, which already carries `refresh_after_days: 0` at :39 and `view_port: 18997` at :40, and no hook), `cortex_raw` (:52) and `write` (:69), following `a_refused_store_names_the_store_field` (:178). Exit code 2, stderr names the field — cited (story) / pattern inferred
  - "The largest accepted value runs without overflow" probably needs a run through `common::ekr`, or unit tests beside `wants`/`holds` (`src/state.rs` tests from about :441) and `call_hook` — inferred
- **Existing values stay inside likely bounds:** tests, docs and examples use `refresh_after_days` 0/1/7/30, `timeout_s` 1–900 and `view_port` 18986–18999 (`git grep` survey) — cited
- **Conformance is unaffected:**
  - `spec/suite.json` holds hook `timeout_s` 1048595 (:4773) and `view_port` 1048596 (:19291) — cited
  - `tests/conformance.rs` builds `m::InstanceSpec` directly, imports nothing from `cortex_cli::spec`, and `src/ports.rs` never reads these fields, so a bound in `check` does not reach the suite — cited (grep), unproven
- **Documents:** `website/docs/spec-file.md` (field tables, for example :82) and `website/docs/operating.md:50` could state the bounds, and `CHANGELOG.md` could get an entry — inferred
- **Would collide with:**
  - any unit editing `src/spec.rs` `check`/`load_given` or `tests/spec_forms.rs`
  - if the casts change, any unit touching `src/main.rs` `create` (:922–1140) or `adopt` (:1304–1430)
  — inferred from the edit ranges above
- **Safety fact:** every one of the three values enters only through the spec, and once the check bounds it no downstream site can overflow. This holds for the `src/state.rs`, `src/sources.rs`, `src/status.rs` and `src/main.rs` sites above, and `model_map.rs:161` is the only path in for the hook value (`int`, :22) — step 2, unproven
- **Not established:**
  - the intended upper bounds and whether `view_port: 0` stays valid (no ADR or comment states either)
  - whether `cortex update` (`main.rs:1490`) re-derives `view_port` and needs the same cast fix
  - whether `CrawlPolicy` or `SourceSpec` carries a second `refresh_after_days` path (`generated/schema/.../SourceSpec.schema.json:213` has the field; the nesting was not read)
  - whether the R3-12/R5-0x findings name other sites
  - what form "runs without overflow" acceptance takes (CLI run against EKR, or unit tests)
- **Confidence:** high for the defect sites and the primary surface (every story citation matches the tree). Medium for the edit placement (`check` vs `load_given`) and the test shape, which are the implementor's choices.

## Decisions

Taken 2026-10-30 by the wave coordinator.

- Bounds in `spec::check`: `refresh_after_days` 0..=36500, hook `timeout_s` 1..=86400, `view_port` 0..=65535 (0 keeps "pick a free port").
- `as u16` becomes `u16::try_from` at both sites.
