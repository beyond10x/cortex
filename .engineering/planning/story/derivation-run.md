---
format: aep.planning-md/3
id: story:derivation-run
kind: story
status: draft
title: A rule-based derivation run records pending assertions citing their premises, applied after approval in a gated stage
summary: cortex derive, no model call; rules join on edges or property values; approve, reject and apply through the schema review path
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- depends_on: story:schema-proposals-on-approval
- depends_on: architecture-decision-record:derivation-rule-language
revision: 3
---
## Outcome

`cortex derive <name>` is a rule-based pass over an instance's store with no model call:

1. It evaluates the derivation rules the instance's spec declares over the store at its head and
   records each assertion the rules derive as `pending` in `derive/<stamp>/assertions.jsonl`,
   naming the rule and citing its premises (the ids of the assertions the match rested on). It
   applies nothing.
2. A rule's body may join two matched nodes on an edge between them or on equal values of a
   property of each (a `Person`'s `email` equal to an `Account`'s `owner_email`), not only on an
   edge.
3. A person approves or rejects each pending assertion (`cortex derive approve|reject <name>
   <stamp> [--only <n>…]`), and `cortex derive apply <name> <stamp>` applies exactly the approved
   ones inside the gated stage `story:schema-proposals-on-approval` introduces, each derived
   assertion carrying its premises as evidence.
4. A second pass derives nothing already in the store, and nothing whose premises have since been
   retracted or superseded.

No implementing wave runs for this story until
`architecture-decision-record:derivation-rule-language` is accepted; the rule language and its join
model below follow that record's proposal and change with it.

## Why

A downstream deployment wants derivation from an existing store as a reviewable run, with joins
on identifiers its sources share as property values (an e-mail address, an account key) where no
edge links the two nodes. Verified in this tree:

- cortex writes assertions only from a model's extraction or a structured import: `store.apply`
  of an extraction document (`src/run.rs:950`, `src/run.rs:1530`) and `transact` of supersessions
  (`src/run.rs:1631`). Nothing in `src/` derives an assertion from other assertions (no match for
  `deriv` outside `#[derive]` and `src/home.rs:17`), and the spec has no rule type
  (`spec/domains/instance.yaml`, `cortex.instance.InstanceSpec` at `:570`).
- cortex can read the whole graph with its assertions' lifecycles (`ekr snapshot`,
  `src/ekr.rs:489-495`); it uses no EKR query verb, and EKR's command line has none (EKR
  `crates/ekr/src/cli/`: no query module).
- EKR has the evidence kind a derived assertion needs, another assertion of the graph
  (`GraphAssertion`, EKR `crates/ekr-graph/src/evidence.rs:25-26`, `:70-71`), but an extraction
  admits only `!HumanStatement` evidence (EKR `crates/ekr-sdk/src/document/extraction.rs:59`;
  `src/evidence.rs:24-25`). So a derived assertion goes in as a transaction, not an extraction.
  Not verified: that a cortex-proposed transaction asserting with `GraphAssertion` evidence is
  admitted; the first step of Work checks it.
- There is no review or gated apply for anything cortex writes outside a source run; that path is
  `story:schema-proposals-on-approval`'s (`depends_on`).

Order: queued after `story:repository-documents-source` (no dependency on it), after
`story:schema-proposals-on-approval` (`depends_on`, the review and apply path) and after
`story:schema-widens-edge-type` (no dependency; a rule may assert an edge only between the ends its
edge type admits, which that story lets an operator widen first).

## Domain relations

Spec first, in `spec/domains/instance.yaml`, as the record proposes:

- value `cortex.instance.DerivationRule` (`name`, `match: List<cortex.instance.RuleAtom>`,
  `join: List<cortex.instance.PropertyJoin>`, `then: cortex.instance.RuleHead`) and a list
  `derive: Optional<cortex.instance.DeriveSpec>` on `cortex.instance.InstanceSpec` (`:570`).
- union `cortex.instance.RuleAtom` (`node`: variable and node type; `edge`: edge type, source and
  target variable; `property`: variable, property, a value to equal) and struct
  `cortex.instance.PropertyJoin` (two variable-property pairs whose values are equal).
- union `cortex.instance.RuleHead` (`edge`: edge type between two bound variables; `property`: a
  property of a bound variable set to a bound or constant value).
- entity `cortex.instance.DerivedAssertion` (identity: the run's stamp and index): `rule`, the
  derived assertion, `premises: List<String>` (EKR assertion ids), `status`, lifecycle as
  `cortex.instance.SchemaProposal`'s from `story:schema-proposals-on-approval`. Relation:
  `cortex.instance.Instance` owns many via `instance_name`. UNMAPPED: `DerivedAssertion` to
  `DerivationRule` is by name; whether a rule changed after the run invalidates its pending
  assertions is open.
- commands `ProposeDerivations`, `ApproveDerivations`, `RejectDerivations`, `ApplyDerivations`
  for `cortex.instance.Operator` (`:817`).
- What ESS cannot check (an unbound head variable, a join of two kinds of value, an atom naming a
  type the ontology lacks) is refused by cortex when the spec is read, and named in `UNMAPPED`
  comments citing the code and tests that hold it.

## Work

- Wait for `architecture-decision-record:derivation-rule-language` to be accepted.
- Check that EKR admits a cortex transaction asserting an edge or property with `GraphAssertion`
  evidence; if not, file an upstream blocker against this story before code.
- Spec first: the rule types, the entity, the commands and their outcomes; validate, conformance
  suite, `task generate`. Then the code.
- `src/derive.rs`: rule checking against `ekr ontology`, evaluation over `ekr snapshot` (active
  assertions only), the pending record, and apply through the shared review and stage path.
- Docs: `website/docs/spec-file.md` (`derive`), `website/docs/commands.md`.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- Edge join: with a rule "`a works_on b`, `b part_of c` ⇒ `a works_on c`" and a store holding
  the two edges, `cortex derive` records one pending assertion naming the rule and both premise
  ids, applies nothing, and the stand-in model records no call.
- Property join: with nodes `Person{email: x@example.org}` and `Account{owner_email:
  x@example.org}` and no edge between them, a rule joining `email` to `owner_email` ⇒ `Person
  holds Account` records one pending assertion citing both nodes' property assertions; a third
  node with another e-mail derives nothing.
- `approve` then `apply`: the store holds `a works_on c`, its evidence names the two premises, and
  the apply ran in a stage on `postgres` (abandoned, with nothing in the store, when the gate fails).
- A second `cortex derive` proposes nothing for an assertion already in the store, and nothing
  after one premise was retracted.
- A rule naming a node type the ontology lacks, or a head variable no atom binds, is refused when
  the spec is read, naming the rule.

## Not in this story

- Model-proposed rules or derivations.
- Recursive rules and negation, unless the accepted record admits them.
- Schema changes (`story:schema-proposals-on-approval`, `story:schema-widens-edge-type`).
