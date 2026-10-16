---
format: aep.planning-md/3
id: architecture-decision-record:derivation-rule-language
kind: architecture-decision-record
status: proposed
title: Derivation rules are typed conjunctive queries in the spec, joined on edges or equal property values
summary: 'Proposed: YAML match/join/then rules with Datalog-like semantics, no recursion or negation, evaluated by cortex over ekr snapshot'
relations:
- decides: story:derivation-run
revision: 1
---
## Context

`story:derivation-run` has cortex derive assertions from an instance's store by rule, with no model
call: each derived assertion is recorded pending with the rule and its premises, and applied only
after a person approved it, inside a gated stage. A downstream deployment needs rules that join
two nodes not only on an edge between them but on equal property values (an e-mail address, an
account key that two sources both carry). How rules are written, and what a join is, decides what
the spec models, what cortex evaluates, and what an operator can review. It is decided here once,
before any implementation.

What exists, verified in this tree:

- cortex reads the store through `ekr`: `ekr ontology` (`src/ekr.rs:484`), `ekr sample`, and
  `ekr snapshot`, the whole graph at the head with every assertion's lifecycle
  (`src/ekr.rs:489-495`). EKR's command line has no query verb (EKR `crates/ekr/src/cli/`), so
  there is no EKR query language to compile rules to.
- The spec models the instance in `spec/domains/instance.yaml`; types are ESS structs, unions and
  enums, and the instance's spec file is held to the JSON Schema `ess` generates for
  `cortex.instance.InstanceSpec` (`src/main.rs:171`, `cortex schema` with no name prints it).
- EKR has the evidence kind a derived assertion needs, another assertion (`GraphAssertion`, EKR
  `crates/ekr-graph/src/evidence.rs:25-26`); an extraction admits only `!HumanStatement`
  (`src/evidence.rs:24-25`), so derived assertions go in as transactions.
- Nothing in cortex evaluates a rule today.

## Options considered

| option | what it costs | why not (or why) |
|---|---|---|
| A. A small typed Datalog subset in its own text syntax (`works_on(A, C) :- works_on(A, B), part_of(B, C).`; a property join as a shared variable over two property atoms) | a parser and an evaluator in cortex; a second syntax inside the YAML spec, which the generated JSON Schema sees only as a string; type checking against the ontology done by hand | the semantics are the right ones (conjunctive, safe, stratifiable) and well known; the text form is opaque to ESS and to the spec's schema, and an operator reviewing a derivation reads a language cortex invented |
| B. A declarative YAML match/join form, typed in the spec, compiled to EKR queries | the same evaluator as A, plus a query target that does not exist: EKR has no query verb | as stated it cannot be built; its typed YAML half is kept in D |
| C. An embedded existing query language (Cypher or GQL, SPARQL, or an embedded Datalog engine's own syntax) evaluated over the snapshot | a dependency large next to cortex's (its `Cargo.toml` has no query engine); a mapping from EKR's nodes, edges, properties and assertion lifecycles onto that engine's model; rules as strings again | familiar to some readers, but the mapping of lifecycles and evidence is the hard part and the engine does not do it; recursion, negation and aggregation arrive whether wanted or not, and a premise list per derived fact is not what these engines return |
| D. (proposed) B's typed YAML form with A's semantics: each rule is a conjunctive query in YAML (`match` atoms over node types, edge types and property values, `join` pairs of equal property values, `then` one edge or property), non-recursive and without negation, evaluated by cortex over `ekr snapshot` | an evaluator in cortex (joins over the snapshot's maps); ESS types for atoms, joins and heads; checks ESS cannot express done when the spec is read | every rule is typed in the spec and its schema, checked against the ontology before any run, and returns its premises by construction; recursion and negation stay out until a need is shown |

## Decision (proposed)

1. **Rules are YAML in the instance's spec, `derive.rules`, each a conjunctive query**: `match`, a
   list of atoms (a node of a type bound to a variable; an edge of a type between two variables; a
   property of a variable equal to a constant); `join`, a list of pairs `(variable.property,
   variable.property)` whose values are equal; `then`, one head (an edge of a type between two
   bound variables, or a property of a bound variable set to a bound value or a constant).
2. **The join model is equality**: an edge atom joins its two variables; a `join` pair joins two
   nodes on equal values of a property of each, compared after the value kind the ontology declares
   for both (a pair whose two kinds differ is refused). No inequality, range, function or
   aggregation.
3. **No recursion and no negation** in this record: a rule's head edge type may not appear in its
   own body, and a set of rules is evaluated once, each over the snapshot, not to a fixpoint. A
   later record may admit stratified recursion.
4. **Evaluation is cortex's, over `ekr snapshot`, active assertions only.** Each derived assertion
   carries the ids of every assertion its match used (edges and property values) as premises; that
   list becomes its `GraphAssertion` evidence when applied.
5. **Checks when the spec is read**: every type, edge type and property an atom names exists in the
   ontology; every head variable is bound; a head edge connects types its edge type admits.

## Consequences

- cortex gains an evaluator; its cost is joins over the snapshot, which is read whole. A store too
  large to snapshot per pass needs a reader of EKR's own later; this record does not provide one.
- Rules are reviewable as data: the spec's JSON Schema shows them, and a pending derived assertion
  names its rule and premises.
- What the language cannot say (transitive closure, "no edge exists", counts) is not derivable
  until a later record admits it; a deployment that needs it says so first.
- A rule changed after a pass leaves that pass's pending assertions naming the old rule; whether
  they are invalidated is open (an `UNMAPPED` in the story).
- No implementation of `story:derivation-run` starts until the operator accepts this record.

## What ESS must model

In `spec/domains/instance.yaml`:

- struct `cortex.instance.DeriveSpec` (`rules: List<cortex.instance.DerivationRule>`) as an
  optional `derive` on `cortex.instance.InstanceSpec`.
- struct `cortex.instance.DerivationRule` (`name`, `match`, `join`, `then`).
- union `cortex.instance.RuleAtom` (`node`, `edge`, `property`), struct
  `cortex.instance.PropertyJoin`, union `cortex.instance.RuleHead` (`edge`, `property`), and a
  constant value as a union over the value kinds of `cortex.instance.SchemaValueKind`.
- entity `cortex.instance.DerivedAssertion` with `rule`, `premises` and the review lifecycle of
  `story:schema-proposals-on-approval`, owned by `cortex.instance.Instance`.
- What ESS cannot check (bound variables, types against a live ontology, kinds of a join) as
  `UNMAPPED` comments naming the code and tests that hold them.

## Status

Proposed, not accepted. It decides the rule language and join model of `story:derivation-run`.
