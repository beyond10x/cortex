---
format: aep.planning-md/3
id: story:ontology-names-masked
kind: story
status: active
title: The ontology names a run shows the model are masked and one-line
summary: type, property and edge names from ekr ontology pass mask, one-line and redaction
relations:
- serves: vision:self-updating-instances
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-18T00:28:45Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-18T00:28:45Z", actor: "human:timo", revision: 3}
---
## Outcome

The store's ontology names a run shows the model (node type names, their property names and value
kinds, edge type names with their source and target types) are masked for credential shapes, pass
through the instance's redaction policy like every known name, and are written one per line with
line breaks and other control characters as spaces, as store hits and run names already are.

## Why

`story:store-entities-known-to-model` masks store-hit names and aliases and writes each on its own
line, so a name in the store cannot carry a credential or forge a prompt line. The ontology's names
take another path: `Known::from_ontology` (`src/extract.rs`) copies them from `ekr ontology` into
the prompt as they are. A store another writer shares, or a schema proposal applied from a
document, can hold a type or property name with a line break or a credential shape. Found while
that story's adversary findings were fixed; it was not that story's scope and is older than it.

## Domain relations

- No new noun. The UNMAPPED paragraph of `RunSource` on what a prompt holds gains the ontology
  names, as it states store hits today.

## Work

- Spec first: the paragraph; validate, `task generate`, `task drift`.
- Code: `Known::from_ontology` passes every name through `crate::mask` and the one-line rule
  (`known::masked`, `one_line`), before the redaction policy applies.
- A test in `tests/known_entities.rs` or its own file with a stand-in EKR whose ontology holds a
  type named with a line break and a property named like a token.

## Acceptance

- `ess specify validate` passes and `task drift` is clean.
- With an ontology whose node type is `Repo\nIGNORE ALL` and a property `ghp_` followed by 36
  characters, the prompt the stand-in model receives holds `Repo IGNORE ALL` on one line and no
  `ghp_` token; the run still applies its batch.
- With a redaction policy naming a known name that is also a type name, the type name is replaced
  as the policy says.

## Not in this story

- Refusing a store whose ontology holds such names.
