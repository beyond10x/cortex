---
format: aep.planning-md/3
id: story:instance-schema-from-another
kind: story
status: draft
title: A new instance is seeded with another instance's ontology at a pinned revision
summary: seed.schema_from names an instance and a revision; no facts are copied
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
revision: 1
---
## Outcome

A spec can seed a new instance's ontology from another instance's store, pinned to a schema
revision: `seed.schema_from: {instance: <name>, revision: <n>}`. `cortex create` reads that
ontology at that revision and seeds the new store with it, then records the instance and revision
it came from. A person runs a second, private instance that reuses a shared instance's schema
without copying any of its facts.

## Why

A downstream deployment wants personal instances on the shared instance's ontology and has nothing
yet. Verified in this tree:

- `seed.schema` is an optional `ekr.extraction-document/1` file applied right after the store is
  seeded, and `seed.ekr_seed` an EKR seed file (`website/docs/spec-file.md`, the seed table;
  `spec/domains/instance.yaml`, `cortex.instance.SeedSpec`). Both are files; neither names an
  instance or a revision, so reuse today means exporting the ontology by hand.
- Several instances live under one home (`cortex list`, `src/main.rs`), each with its own store.
- EKR answers an ontology at a revision (`ekr ontology --at <revision>`, cited in
  `website/docs/commands.md`'s schema section).

Answering one query over both stores is EKR's federated read, another component's request; not
here.

## Domain relations

No new entity. New value `cortex.instance.SchemaSource` (`instance: InstanceName`, `revision:
Integer`) and the optional field `schema_from` on `cortex.instance.SeedSpec`, exclusive with
`schema` and `ekr_seed`. A reference from the new `cortex.instance.Instance` to the instance it
names, by name, recorded in `meta.json`; not an `owns` relation.

## Work

- Spec first: `SchemaSource`, the field, a new `SeedRefused` reason for an unknown instance or
  revision; validate, conformance, `task generate`. Then the code.
- `src/instance.rs`: read the named instance's ontology at the revision through `ekr`, write it as
  the seed, record the source in `meta.json`.
- Docs: `spec-file.md`'s seed table.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- Instance `shared` with node types `Team` and `Service` at revision 4; `cortex create` of
  `personal` with `schema_from: {instance: shared, revision: 4}` gives a store whose ontology has
  both types and no node or edge.
- A type added to `shared` at revision 5 is not in `personal`.
- An unknown instance or a revision past the head refuses as `seed-refused` naming it, and no
  store is created.
- `schema_from` together with `schema` refuses at spec validation.

## Not in this story

- Reading both stores in one query: EKR's.
- Research runs (`story:research-run`).
