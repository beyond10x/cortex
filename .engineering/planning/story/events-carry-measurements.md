---
format: aep.planning-md/3
id: story:events-carry-measurements
kind: story
status: active
title: QualityMeasured and SchemaChangesProposed carry rate and cost_usd
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:quality-judge
- depends_on: story:schema-emergence
- depends_on: story:ess-pin-newest
scope:
- confidence: cited
  path: generated
- confidence: cited
  path: spec/domains/instance.yaml
- confidence: cited
  path: spec/suite.json
- confidence: cited
  path: src/main.rs
- confidence: cited
  path: src/ports.rs
- confidence: inferred
  path: src/quality.rs
- confidence: inferred
  path: src/schema.rs
- confidence: cited
  path: tests/conformance.rs
- confidence: cited
  path: tests/quality.rs
- confidence: cited
  path: tests/schema.rs
- confidence: cited
  path: website/docs/reference/ess/cortex-instance.md
revision: 23
transitions:
- {from: "draft", to: "proposed", at: "2026-10-07T06:23:04Z", actor: "human:timo", revision: 22}
- {from: "proposed", to: "active", at: "2026-10-07T06:23:04Z", actor: "human:timo", revision: 23}
---
## Outcome

`QualityMeasured` carries the measurement's `rate` and `cost_usd`, and `SchemaChangesProposed`
carries `cost_usd`, as the specification states them, so the event a reader receives holds every
number the command prints.

## Why

Two `UNMAPPED:` notes in `spec/domains/instance.yaml` (on the `measured` outcome of
`MeasureQuality` and the `proposed` outcome of `ProposeSchemaChanges`) leave these fields off the
events because ESS 0.52.0 synthesis filled an `Optional` `{generated: true}` payload field with a
constant `None` (https://github.com/beyond10x/ess/issues/467). Since ESS 0.55.0, which cortex pins
since wave 20261007a, generated behaviours read such a field from the context port
(`generate_optional_<t>`, `try_generate_optional_<t>`); a scratch probe in that wave adding
`rate: Optional<Decimal>` `{generated: true}` to `QualityMeasured` generated
`rate: self.ports.try_generate_optional_decimal()?`.

Today the command line prints both numbers beside the event's fields, `fact-quality.json` holds
`rate`, and `tests/quality.rs` and `tests/schema.rs` hold them.

## Work

1. Spec first: `QualityMeasured` gains `rate: Optional<Decimal>` and `cost_usd: Optional<Decimal>`,
   `SchemaChangesProposed` gains `cost_usd: Optional<Decimal>`, each `{generated: true}` in every
   outcome that emits the event; the two `UNMAPPED:` notes go. Validate with `ess` 0.55.0,
   `task generate`.
2. The context that implements the port answers the new `generate_optional_<t>` from the
   measurement: `rate` null when nothing was judged, `cost_usd` null as soon as one answer carried
   no cost and never 0, as the notes state today.
3. The command line prints the event's own fields; it no longer adds them beside the event.

## Acceptance

- `ess specify validate --path spec --strict-requires` exits 0 and the spec carries no `UNMAPPED:`
  note about `rate` or `cost_usd`.
- The conformance suite's scenarios for `measured`, `proposed` and `applied` run and pass.
- `tests/quality.rs`: the printed `QualityMeasured` holds `rate` and `cost_usd`, with `rate` null
  when no fact was judged and `cost_usd` null when one answer carried no cost.
- `tests/schema.rs`: the printed `SchemaChangesProposed` holds `cost_usd`, null under the same rule.

## Scope

Derived 2026-10-07 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `spec/domains/instance.yaml` (notes 1264-1268 and 1337-1340; outcomes 1269-1284 and 1341-1377; events 1442-1467) — cited
- **Files:** `generated/` (`cortex-model/src/behaviour.rs`, `instance.rs`, `PLAN.md`, `plan.json`, the two event schemas under `generated/schema/schema/events/`, `generated/docs/docs/*`), all regenerated — cited
- **Files:** `spec/suite.json` (the measured, applied and proposed scenarios) — cited
- **Files:** `website/docs/reference/ess/cortex-instance.md`, regenerated (it names both events) — cited
- **Files:** `src/ports.rs:22-33` (`Shared` gains an optional-decimal queue) and `:117-146` (`impl Context for Ports` gains the optional-decimal method) — cited
- **Files:** `src/main.rs:1495-1578` (`quality`) and `:1580-1695` (`propose_schema`) — cited
- **Files:** `tests/conformance.rs:113-139` (`event()` renders `rate`/`cost_usd`; the shape check at `:602-605` requires every shape key) — cited
- **Files:** `tests/quality.rs`, `tests/schema.rs` (the null cases the acceptance names are not asserted today) — cited
- **Also likely:** `src/schema.rs:1013-1021` (`extra` drops `cost_usd`), `src/quality.rs:61-68` (`Measurement.rate`) — inferred
- **Types:** `rate` is a `serde_json::Value` today and prints as a JSON number; `cost_usd` is `Option<f64>` and prints as a 4-place string or null. `Optional<Decimal>` fits both, as `SourceRan.cost_usd` already is (`spec/domains/instance.yaml:1421`); the printed forms stay — cited
- **Not touched:** `src/extract.rs`, `src/ekr.rs`, `src/run.rs`, `src/snapshot.rs`, `tests/store_backend.rs` — inferred
- **Confidence:** high; medium for `src/schema.rs`
- **Not established:** the exact signature of the generated optional-decimal method (required or defaulted), and the order the generated code reads `rate` and `cost_usd` in (one queue serves both only if the order is fixed)
