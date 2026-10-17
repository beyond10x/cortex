---
format: aep.planning-md/3
id: story:structured-lookup-across-types
kind: story
status: draft
title: A structured record resolves to a store node of another type by its name or alias
summary: structured mapping may match store nodes of any type before it creates one
relations:
- serves: vision:self-updating-instances
revision: 1
---
## Outcome

A structured source's record resolves to a node an adoption or another writer already stored under
another node type, when one of the record's aliases (its `alias_templates` included) or its name
equals that node's canonical name or one of its aliases. The run then writes the record's
properties and relations onto that node instead of creating a second node of the mapping's type
holding the same alias.

## Why

A downstream deployment adopted a store whose repositories are `Project` nodes with aliases such as
`repo:acme/api`, and reads its repository registry through a structured source whose mapping's
`node_type` is `Repository`. A model-extracted run looks the batch's terms up in the store across
every type (`story:store-entities-known-to-model`), but a structured run calls no model and makes
no lookup, and EKR resolves an alias within one node type only. Each record therefore becomes a
second node holding the same alias. Verified by
`tests/known_entities_adversary.rs::a_template_alias_reaches_a_store_node_of_another_type`
(ignored, out of scope of that story); the spec says so in
`spec/domains/instance.yaml` (the `RunSource` paragraph on store hits: "a template alias matches a
store node of the mapping's own type only").

## Domain relations

- `cortex.instance.RecordMapping` gains a choice of how a record meets the store: its own type
  only (today's behaviour, the default) or any type, by name or alias, lowercased.
- Which node a record resolved to is decided before the batch is applied; a record that matches
  nodes of more than one type is refused alone, naming the types, and the rest of the batch is
  applied.

## Work

- Spec first: the mapping setting and its UNMAPPED paragraph in `RunSource`; validate, conformance,
  `task generate`.
- Code: `src/structured.rs` reads the store once per batch (`ekr snapshot`, as `src/run.rs` does for
  store hits) and maps each matching record onto the found node's id and type.
- Docs: `website/docs/spec-file.md` (structured mappings), `limits.md`.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- The ignored case above runs and passes with the mapping set to any type: one node holds
  `repo:acme/api` after the run, and it carries the record's `path` property.
- With the default, the run behaves as today (the existing structured tests unchanged).
- A record whose alias matches a `Project` and a `Service` node is refused alone, naming both
  types; the other records of the batch are applied.

## Not in this story

- Merging two nodes that already hold the same alias.
- A lookup for model-extracted sources (`story:store-entities-known-to-model` has it).
