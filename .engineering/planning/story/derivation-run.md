---
format: aep.planning-md/3
id: story:derivation-run
kind: story
status: draft
title: Rules derive assertions as a reviewable delta, applied after approval
summary: cortex derive --review over the spec's derivation rules; approve applies them citing the facts they rest on
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:schema-proposals-on-approval
revision: 1
---
## Outcome

`cortex derive <name> --review` evaluates the derivation rules an instance's spec declares over its
store and records the assertions they would derive, each with the rule and the facts it rests on,
as a pending delta; `cortex derive approve <name> <stamp>` applies the approved ones through EKR as
one extraction, citing the facts they rest on as evidence. A derived assertion reaches the store
only after approval, and says which rule made it.

## Why

A downstream deployment wants derivation as a reviewable run and has nothing yet. Verified in this
tree: cortex writes assertions only from a model's extraction or a structured import
(`src/run.rs`, `src/structured.rs`); nothing derives an assertion from other assertions, and the
spec has no rule type (`spec/domains/instance.yaml`). What a rule may express (a path of edges, a
property condition) and whether EKR accepts an assertion whose evidence is other assertions are not
checked here (inferred: EKR's evidence kinds may refuse it, as it refuses URL evidence,
`upstream-blocker:ekr-url-evidence`).

## Domain relations

- value `cortex.instance.DerivationRule` (`name`, `when` as a path of edge types with property
  conditions, `then` as the edge or property to assert), a list `derive.rules` on
  `cortex.instance.InstanceSpec`.
- entity `cortex.instance.DerivedAssertion` (identity: the run's stamp and index), `rule`,
  `rests_on: List<String>` (EKR assertion ids), lifecycle as `SchemaProposal`'s from
  `story:schema-proposals-on-approval`. Relation: `cortex.instance.Instance` owns many via
  `instance_name`; `UNMAPPED: DerivedAssertion to DerivationRule` (by name; cardinality to a rule
  that later changes is not settled).

## Work

- Spec first: the rule type, the entity, the commands `ProposeDerivations` and
  `ApproveDerivations`; validate, conformance, `task generate`. Then the code.
- Find out which evidence EKR admits for a derived assertion; if none, file an upstream blocker
  before any code.
- `src/derive.rs`: rule evaluation over the store, no model call; the review and approve verbs
  shared with the schema approval.
- Docs: `spec-file.md` (`derive`), `commands.md`.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- With a rule "`a works_on b`, `b part_of c` ⇒ `a works_on c`" and a store holding the two edges,
  `--review` records one pending derived assertion naming the rule and both edge ids, applies
  nothing, and asks no model.
- `approve` applies it: the store holds `a works_on c`, and its evidence cites the two edges.
- A second `--review` proposes nothing new for an assertion already in the store.

## Not in this story

- Schema changes (`story:schema-proposals-on-approval`).
- Model-proposed derivations.
