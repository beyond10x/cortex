---
format: aep.planning-md/3
id: story:schema-widens-edge-type
kind: story
status: draft
title: A schema proposal widens which node types an existing edge type connects
summary: SchemaChange gains widen_edge_type, written as EKR's WidenEdgeType with each end whole, reviewed and applied like any proposal
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:schema-proposals-on-approval
revision: 1
---
## Outcome

A schema proposal can widen which node types an existing edge type connects, and cortex writes it
as EKR's own widening:

1. `cortex.instance.SchemaChange` gains `widen_edge_type` (the edge type's name, the node types to
   add to its sources, the node types to add to its targets). The model is shown it beside
   `add_edge_type` and may propose it.
2. cortex plans it against the ontology at the head: the edge type must exist, every named node
   type must exist, and at least one type must be new to an end. It writes EKR's `!WidenEdgeType`
   with each end whole (what the end holds now plus the additions), never a narrower end.
3. It is reviewed and applied like any other proposal (`story:schema-proposals-on-approval`):
   recorded `pending`, applied only once approved, inside the gated stage.
4. After it is applied, a fact whose edge connects the newly admitted types is no longer refused
   with `edge-endpoint-type`.

## Why

An instance's edge types are declared once with fixed ends; when sources bring the same relation
between other node types, every such fact is refused. Verified:

- EKR can widen: `!WidenEdgeType` is a transaction operation (EKR `crates/ekr-kernel/src/
  transaction.rs:139-160` the `EdgeWidening` payload, `:550-551` the operation), applied by
  `widen` (EKR `crates/ekr-ontology/src/evolve.rs:166-199`), which refuses an end that loses a type
  (`edge-endpoint-removed`) or names an unknown one (`unknown-endpoint-type`). It shipped in EKR
  0.0.18 (`story:edge-type-endpoints-widen` there); this tree pins EKR 0.0.34
  (`.github/workflows/check.yml:21`).
- cortex cannot ask for it. `cortex.instance.SchemaChange` has `add_edge_type` and
  `remove_edge_type` and no widening (`spec/domains/instance.yaml:670-684`); cortex's EKR
  operations are `DefineNodeType`, `DefineEdgeType` and `ModifyProperty` for the schema
  (`src/ekr.rs:646-677`). A proposal that re-adds an existing edge type with more ends is `invalid`
  ("the ontology already declares a type", `src/schema.rs:766-770`, checked for `add_edge_type`
  at `src/schema.rs:799-801`).
- A fact whose edge joins types outside the edge type's ends is refused by EKR with
  `edge-endpoint-type` (EKR `crates/ekr-kernel/src/validate/types.rs:58`) and counted in the run's
  `facts_refused` (`src/run.rs:938`).

Order: after `story:schema-proposals-on-approval`, whose review and apply path it is applied
through (`depends_on`: the widening is a new proposal kind of that path, and both change
`src/schema.rs`). Queued after `story:repository-documents-source`; no dependency on it.

## Domain relations

Spec first, in `spec/domains/instance.yaml`:

- value `cortex.instance.EdgeTypeWidening` (`name: String`, `add_source_types: List<String>`,
  `add_target_types: List<String>`), a struct beside `cortex.instance.NewEdgeType` (`:630`).
- `cortex.instance.SchemaChange` (`:670`) gains the variant `widen_edge_type:
  cortex.instance.EdgeTypeWidening`; the comment above it (`:667-669`) names `WidenEdgeType` among
  the changes EKR applies.
- `cortex.instance.SchemaProposalStatus` (`:708`) is unchanged; an EKR refusal of the widening is
  `refused` with EKR's code in `codes`.

## Work

- Spec first: the struct and the variant; validate, conformance suite, `task generate`. Then the
  code.
- `src/ekr.rs`: `Operation::WidenEdgeType { edge_type, source_types, target_types }`, written as
  `!WidenEdgeType`.
- `src/schema.rs`: `change_of`/`change_json` for the variant; `plan` resolves names to ids, merges
  each end with what the ontology holds, and is `invalid` for an unknown edge type, an unknown node
  type, or a widening that adds nothing; `Names::add` widens its copy so later proposals of the
  same round plan against it; the prompt lists the variant.
- Docs: `website/docs/commands.md`'s schema section.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- A store with node types `Person`, `Team`, `Project` and an edge type `works_on` from `Person` to
  `Project`; a stand-in model proposing `widen_edge_type works_on` adding source `Team`: the
  proposal is recorded `pending`, and after approve and apply `ekr ontology` lists `works_on` with
  sources `Person` and `Team` and target `Project`.
- Before the widening a structured or extracted fact `Team works_on Project` is refused
  (`facts_refused` 1); after it, the same fact is applied.
- A widening naming an edge type the ontology lacks, or a node type it lacks, is `invalid` with a
  note naming it, and nothing is submitted.
- A widening that adds nothing new to either end is `invalid`.
- The written transaction carries each end whole: a test reads `schema-<n>.yaml` and finds
  `Person` still among the sources.

## Not in this story

- Narrowing an edge type's ends, or removing one: EKR has no such change.
- Widening a node type's parents or a property's kind (`redeclare_property` covers the kind).
- The review and apply path itself (`story:schema-proposals-on-approval`).
