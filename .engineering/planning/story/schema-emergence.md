---
format: aep.planning-md/3
id: story:schema-emergence
kind: story
status: implemented
title: An operator triggers schema emergence on demand
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:quality-judge
scope:
- confidence: inferred
  path: generated
- confidence: inferred
  path: spec/components.yaml
- confidence: inferred
  path: spec/domains/instance.yaml
- confidence: inferred
  path: src/main.rs
- confidence: inferred
  path: src/quality.rs
- confidence: inferred
  path: src/schema.rs
- confidence: inferred
  path: tests/schema.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T10:29:12Z", actor: "agent:claude", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-06T10:29:12Z", actor: "agent:claude", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-06T11:16:48Z", actor: "agent:claude", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Outcome

An operator triggers schema emergence on demand: `cortex schema <instance> [--sample N]` asks the instance's model, with no tools, to propose ontology changes from a sample of the store's facts and their evidence. cortex commits the changes EKR can apply as schema transactions citing their evidence, and records the rest as proposals for review.

## Why

Operator requirement 2026-10-06: the instance runs remotely for readers, and the operator keeps the controls locally, among them triggering schema emergence and a refresh round (`cortex run`). The downstream deployment's own importer already does this per run (its `ontology-change-proposals`: proposals cite facts and messages; supported changes are committed before the data; the rest are reported). Without it in cortex, the schema of a cortex instance only grows by what extraction adds.

## Work

- Spec-first (`ess:specifying`): model `ProposeSchemaChanges` (wire name `schema`), its outcomes (`proposed`, `applied`, `sample-failed`, `judge-failed`-style model failure, `no-such-instance`), its event and the proposal record in `spec/`; regenerate; implement against the generated code.
- Draw the sample under the home lock as `cortex quality` does (`ekr sample` at the head), release the lock, and ask the model in batches with the same masking the run applies.
- The model answers a closed list of proposals: add a node type, add an edge type, add or redeclare a property, and (record only) remove, merge or split. Each cites fact ids and evidence ids from the batch.
- Apply what EKR accepts as one schema transaction per proposal, citing its evidence, under the lock; write `schema/<stamp>/proposals.jsonl` with every proposal and whether it was applied, refused (with EKR's refusal code) or recorded only.
- `--dry-run` records without applying.

## Acceptance

`tests/schema.rs` with a stand-in model:
1. A proposal of one new property on a store that holds the ontology is committed as a schema transaction citing its evidence; `ekr ontology --at <previous revision>` still answers the prior schema.
2. A merge proposal is recorded as `recorded-only`, not applied, and not an error.
3. A proposal citing a fact outside its batch is dropped.
4. `--dry-run` applies nothing and writes every proposal.
5. No name the run would mask reaches the model in the clear (the masking test from `story:quality-judge`, reused).

## Depends on

`story:quality-judge` (the sample draw, the lock release and the judge masking it reuses).

## Scope

Landed 2026-10-06 in `62f9f83` (wave 20261006h).

- **Files:** `src/schema.rs` (new), `src/ekr.rs` (schema operations), `src/quality.rs` (shared sampling and masking), `src/main.rs`, `src/lib.rs`, `src/run.rs`, `spec/domains/instance.yaml`, `spec/components.yaml`, `generated/`, `tests/schema.rs` (new), `tests/conformance.rs`, `website/docs/commands.md`, `AGENTS.md`
- **Review:** one adversary pass, 3 findings fixed (inherited property shadowing, redeclaration dropping required, placeholders in names), plus case-insensitive names
- **Open:** EKR 0.0.31 takes no evidence on a schema transaction
- **Closed 2026-10-07:** `cost_usd` is on `SchemaChangesProposed` since wave 20261007b (`story:events-carry-measurements`), after ESS 0.55.0 settled https://github.com/beyond10x/ess/issues/467
