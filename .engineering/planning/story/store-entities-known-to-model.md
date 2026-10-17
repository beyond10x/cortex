---
format: aep.planning-md/3
id: story:store-entities-known-to-model
kind: story
status: draft
title: The model sees the store's existing entities for a batch's keys, and mappings can add templated aliases
summary: Per-batch store lookup of keys and aliases across types as known entities; alias templates on structured mappings
relations:
- serves: vision:self-updating-instances
revision: 1
---
## Outcome

Before each batch is sent to the model, cortex looks the batch's record keys and aliases up in
the store, across every node type, and shows the model each hit (type, name, aliases) as a known
entity, so the model reuses the existing node instead of creating it again under another type.
A structured mapping may add a fixed-format alias from a template (for example
`repo:{namespace}/{path}`), so a source's key scheme can match the aliases of an adopted store.

## Why

A downstream deployment adopted an existing store and suspended two ingestion sources because
entities written by the adoption or by another writer were created again under a different type.
Verified in this tree: the model sees only names from cortex's own earlier runs, at most
`KNOWN_PER_TYPE` (50) per type, read from the instance's entities file (`src/run.rs`,
`KNOWN_PER_TYPE` and `load_entities`); and a structured mapping's aliases come only from record
fields (`src/structured.rs`, `aliases`: `<name> (<identity>)`, the identity, then
`mapping.aliases` paths).

## Domain relations

- `cortex.instance.RecordMapping` gains `alias_templates: List<String>`: each a fixed text with
  `{field.path}` placeholders, rendered per record; a template whose field is missing yields no
  alias.
- A known entity shown to the model carries `type`, `name`, `aliases` and its origin (`store` or
  `run`). No new persisted noun: the lookup is per batch, through `ekr`'s read surface.

## Work

- Spec first: `alias_templates` on `RecordMapping`, its validation (unknown placeholder, empty
  template); the lookup as an UNMAPPED note on `RunSource` where ESS cannot state it; validate,
  conformance, `task generate`. Then the code.
- The lookup: one read per batch of the store's nodes whose name or alias equals one of the
  batch's keys, aliases or template aliases (exact match after case folding), all types; hits
  merged with the run's own known names, store hits first, deduplicated by node; the per-type cap
  applies to run names only.
- Redaction: store hits pass through the instance's redaction like every known name.
- Docs: `sources.md` (alias templates), `operating.md` (known entities).

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- On a store holding node `acme/api` of type `Repository` written by another writer, a batch
  whose record key is `acme/api` shows the model that node as a known entity of type
  `Repository`; a stand-in model that reuses known names yields no new node, and the store's
  node count is unchanged.
- A mapping with `alias_templates: ["repo:{namespace}/{path}"]` gives a record
  `{namespace: acme, path: api}` the alias `repo:acme/api`, and a store node with that alias is
  found by the lookup.
- A template naming a field the record lacks yields no alias and no error; a template with an
  unknown placeholder syntax is refused at spec load.
- With more than 50 run-known names of a type, every store hit for the batch is still shown.

## Not in this story

- Merging nodes already duplicated in a store.
- Fuzzy matching of names.
