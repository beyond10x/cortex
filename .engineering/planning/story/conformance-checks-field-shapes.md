---
format: aep.planning-md/3
id: story:conformance-checks-field-shapes
kind: story
status: draft
title: The conformance runner checks each event field's shape, not only its key
relations:
- decomposes: epic:standalone-1-0
- serves: vision:self-updating-instances
scope:
- confidence: cited
  path: tests/conformance.rs
revision: 2
---
## Outcome

The in-process conformance runner checks an event field's value against the shape the suite states (`optional`, `kind: decimal`, …), so a scenario fails when an `Optional` field is absent where it must be present, or holds a value of the wrong kind.

## Why

Found by adversary pass 1 of `story:events-carry-measurements` (wave 20261007b), `pre-existing`: the shape check at `tests/conformance.rs:602` tests only that a key is present, and the runner renders a missing value as `null`, so the suite's `optional: true` and `kind: decimal` on `QualityMeasured.rate`, `QualityMeasured.cost_usd`, `SchemaChangesProposed.cost_usd` and `SourceRan.cost_usd` are never checked; the `measured`, `proposed`, `applied` and `ran` scenarios pass whatever those fields hold. The binary-level tests in `tests/quality.rs`, `tests/schema.rs` and `tests/e2e.rs` cover the values.

## Acceptance

- A mutant that puts a non-decimal string in `QualityMeasured.cost_usd` makes `cargo test --test conformance` fail on the `measured` scenario.
- The suite still passes on `main`.

## Not modelled

Test harness only: the specification already states the shapes; the runner does not read them.
