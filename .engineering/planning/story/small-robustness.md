---
format: aep.planning-md/3
id: story:small-robustness
kind: story
status: implemented
title: Five small panics and leaks found by the review are closed
summary: A `"!"` key in a model document, a non-object EKR host, inherited `EKR_*` on the direct path, unmasked EKR stderr, and two structured sources sharing identities
tags:
- review-20261030
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
scope:
- confidence: cited
  path: src/ekr.rs
- confidence: cited
  path: src/extract.rs
- confidence: inferred
  path: src/run.rs
- confidence: cited
  path: src/spec.rs
- confidence: inferred
  path: tests/credential_mask.rs
- confidence: inferred
  path: tests/structured_files.rs
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T10:11:05Z", actor: "human:timo", revision: 10}
- {from: "proposed", to: "active", at: "2026-10-30T10:11:05Z", actor: "human:timo", revision: 11}
- {from: "active", to: "implemented", at: "2026-10-30T15:30:13Z", actor: "human:timo", revision: 12, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
## Outcome

- an object key `"!"` in the model's document reaches `Tag::new("")`, which panics
  (`src/extract.rs:949-952`; R2-07, plausible);
- EKR's stderr from a failed apply goes into `report.stopped` and `cortex.log` unmasked
  (`src/ekr.rs:95-96`, `src/run.rs:1136`; R2-08, plausible);
- the direct store path strips only `EKR_STAGE`, while the launched path strips every inherited
  `EKR_*` (`src/ekr.rs:300-314` vs `:270-276`; R4-21);
- `host_json` indexes a non-object `host` or `authority` and panics (`src/ekr.rs:125-128`; R4-22);
- two structured sources with the same adapter and operation under `dropped: Supersede` share
  identities (`src/structured.rs:1141-1144`, `:1192-1196`; guard `src/spec.rs:220-243`; R3-05,
  plausible): the spec check refuses the pair.

## Acceptance

One test per item: the `"!"` key is refused as an extraction error; a credential-shaped string in
EKR's stderr is masked in the report; `EKR_FULL_REPLAY` set in the environment does not reach a
direct-path child; a non-object `host` is an error; the spec check refuses the pair.

## Findings it closes

R2-07, R2-08, R3-05, R4-21, R4-22.

## Scope

Derived 2026-10-30 by `story-scoper` against main a870a628 (no changes under `src/`, `spec/`, `tests/` or `generated/` up to wave/20261030a HEAD b61a56c8). Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** the cortex root crate, `src/ekr.rs` (3 of the 5 items) — cited
- **Files:** `src/extract.rs:949-952`, in `tagged` (939-962): `k.strip_prefix('!')` at 949, then `Tag::new(tag)` at 951 with `tag == ""` for a `"!"` key; confirmed — cited
- **Files:** `src/ekr.rs:95-96`, in `fn run` (88-98): stderr goes into `Err` unmasked; confirmed — cited
- **Files:** `src/ekr.rs:300-314`, `Store::store_cmd`: the direct (`None`) arm at 303-307 never calls `remove_inherited_ekr`, and `.env_remove("EKR_STAGE")` at 312 is the only stripping; the launched path strips every `EKR_*` through `remove_inherited_ekr` at 270-276, called at :231; confirmed — cited
- **Files:** `src/ekr.rs:125-128`, `Binary::host_json` (121-130): `host["tenant"] =` and `&mut host["authority"]["validation_profile"]` panic on a non-object; confirmed — cited
- **Files:** `src/spec.rs:220-243`, the guard inside `fn check` (180-…): `shared` at 231-234 matches only a `SourceSettings::Connectors` partner, not a second `Structured` source with `StructuredInput::Connectors` on the same adapter and operation; confirmed — cited
- **Files:** `src/run.rs:1136` (`report.stopped = Some(e)` after `store.apply`) is the sink; the same unmasked string also reaches `Failure::Apply(e)` at 1134 and `cortex.log` through `log_line` `"stopped"` at 2200; confirmed — cited
- **Read, not edited:** `src/structured.rs:1141-1144` (`owns`) and `:1192-1196` (`owner`) are the shared-identity site. Both sources get the prefix `<adapter>:<operation>` from `prefix_parts` (112-143). Per the Outcome ("the spec check refuses the pair"), the fix belongs in `spec.rs`, not here — cited
- **Symbols:** `extract::tagged`, `extract::merge` (1037-1071), `ekr::run`, `ekr::Binary::host_json`, `ekr::Store::store_cmd`, `ekr::remove_inherited_ekr`, `spec::check` — cited
- **Also likely:** `src/run.rs:1118` and `:1957`, the only two callers of `extract::merge`. Refusing `"!"` as an extraction error probably makes `tagged` and `merge` return `Result`, so both callers change. 1118 would map to `Failure::Extract`, matching 1099-1105 — inferred
- **Also likely:** `src/ekr.rs:463-466`, `Store::seeded`, a second unmasked-stderr site outside `run`. The story does not name it — inferred
- **Also likely:** the direct-path fix is a `remove_inherited_ekr(&mut cmd, std::env::vars_os()…)` call in the `None` arm of `store_cmd` (303-307) — inferred
- **Tests (changed):** `src/extract.rs` test module: `bang_keys_become_yaml_tags` (1338) and the `tagged(` calls at 1185 and 1330 change if `tagged` returns `Result`; the `"!"` refusal test goes beside them — inferred
- **Tests (added):** `src/ekr.rs` test module (830-…), beside `a_launch_removes_every_inherited_ekr_variable_and_keeps_the_rest` (1057) and `a_store_without_a_connection_starts_ekr_itself` (1113), for `EKR_FULL_REPLAY` on the direct path and a non-object `host` — inferred
- **Tests (added):** `tests/structured_files.rs`, next to `adv_a_a_supersede_beside_a_connectors_source_on_the_same_operation_is_refused` (609-690), for the structured/structured pair — inferred
- **Tests (added):** `tests/credential_mask.rs` (stand-in `ekr`/`claude` through `tests/common`) or the `src/ekr.rs` test module, for a credential in EKR's stderr being masked in the report — inferred
- **Spec / generated:** none. No `spec/domains/*.yaml` type changes: `extraction-failed` already exists in `instance.yaml:648`, and spec-check refusals are free-text exit-2 reasons that the domain lists only as `UNMAPPED` comments. So `generated/` (`cortex-model`, `schema`, `spec-types`, `docs`) does not change — cited
- **Documents:** none — inferred
- **Confidence:** high for the five defect sites (the story names each one and each matches the tree exactly); medium for the `run.rs` call-site edits and test-file placement (these depend on the implementor's choices)
- **Would collide with:** any unit touching `src/ekr.rs` (`run`, `Binary`, `Store::store_cmd`/`cmd`, its test module), `src/extract.rs` (`tagged`/`merge`), `spec::check` in `src/spec.rs`, or the extraction loop in `src/run.rs` around 1099-1140 and 1945-1970
- **Safety fact:** masking inside `ekr::run` must leave `"publication outcome is unknown"` (`OUTCOME_UNKNOWN`, `src/ekr.rs:103`) intact, because `run.rs:1133` and `:1958` decide `store-outcome-unknown` on that substring through `ekr::outcome_unknown`. `src/mask.rs` masks only credential shapes — step 2 (`src/ekr.rs:103-109`, `src/mask.rs:1-8`), unproven

## Decisions

Taken 2026-10-30 by the wave coordinator.

- A `"!"` key is refused by a check before `merge`; `tagged`/`merge` keep their signatures.
- EKR stderr is masked inside `ekr::run` and `Store::seeded`; a test proves the `OUTCOME_UNKNOWN` text survives masking.
- The direct `store_cmd` path calls `remove_inherited_ekr`; `Binary` commands are left as they are.
- `EKR_FULL_REPLAY` is tested with a stand-in `ekr` integration test, not `set_var`.
