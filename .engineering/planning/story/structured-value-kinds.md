---
format: aep.planning-md/3
id: story:structured-value-kinds
kind: story
status: implemented
title: A structured mapping writes each property in the value kind the store holds
summary: PropertyMapping gains an optional value_kind; without it cortex reuses the store's declared kind and refuses only a record whose value does not convert
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T13:33:20Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-16T13:33:21Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-16T15:58:58Z", actor: "human:timo", revision: 5, decided_on: {"asserted":{"test_result":723}}, correlation: "wave-20261016c"}
---
## Outcome

A structured source writes each mapped property in the value kind the store holds for it, so a
batch is never refused because cortex redeclared a property as `String`:

1. A `PropertyMapping` may name its property's value kind (`value_kind`, one of
   `cortex.instance.SchemaValueKind`: `String`, `Integer`, `Decimal`, `Boolean`, `Timestamp`).
   cortex declares the property in that kind and converts each record's value to it.
2. With no `value_kind`, cortex reuses the kind the store's ontology already declares for that
   property of that node type (`ekr ontology`), and `String` when the store declares none.
3. A record whose value does not convert to the kind (a `"12a"` for `Integer`, a `"maybe"` for
   `Boolean`) is refused alone: it is left out of the batch, named with its key and the property
   in the run's report and `cortex.log`, and the rest of the batch is applied. A mapping whose
   `value_kind` differs from the kind the store declares is refused when the spec is read
   (`cortex create`, `cortex update`, a run), naming the node type, the property and both kinds.

## Why

Seen on a downstream deployment (cortex 0.3.1, PostgreSQL store): a structured source mapped two
fields the store already declared `Integer` and `Boolean`. The whole batch was refused, 357 new
documents and 0 applied, because EKR redeclares a property under its existing id and schema
evolution refuses a change of its kind.

Verified on `main`:

- `src/structured.rs` `string_property` declares every mapped property
  `{"value_kind": "String"}`, and `Mapping::document` writes every mapped value as
  `{"value_kind": "String", "value": …}`.
- `cortex.instance.PropertyMapping` has `property` and `path` only; no kind can be given.
- `cortex.instance.SchemaValueKind` already lists the kinds `cortex schema` proposes.

## Domain relations

No new noun: an optional `value_kind: cortex.instance.SchemaValueKind` on
`cortex.instance.PropertyMapping`.

## Work

- Spec first: the optional field, with a comment on the default (the store's declared kind, else
  `String`) and the per-record refusal; validate, conformance suite, `task generate`. Then the
  code.
- `src/structured.rs`: the declared kind of each mapped property from the mapping or the store's
  ontology (`src/ekr.rs` reads `ekr ontology`); the value converted per kind (`Integer`: a JSON
  integer or a decimal string; `Decimal`: a number or a decimal string; `Boolean`: `true`/`false`
  in JSON or text; `Timestamp`: RFC 3339); a record that does not convert left out of the batch
  and reported.
- `src/model_map.rs`: the field mapped.
- Docs: the spec-file page's structured mapping, and the operating page's report fields.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- A store whose ontology declares `count` `Integer` and `open` `Boolean` on a node type, and a
  mapping of both with no `value_kind`: a batch of records with `"count": 3` / `"count": "4"` and
  `"open": true` / `"open": "false"` is applied, and `ekr` reads the values back as Integer and
  Boolean.
- The same with one record whose `count` is `"12a"`: that record alone is refused and named with
  its key and property; every other record of the batch is applied.
- `value_kind: Integer` on a property the store does not yet declare: it is declared `Integer`.
- `value_kind: String` on a property the store declares `Integer`: the spec is refused naming the
  node type, the property and both kinds, and nothing is applied.
- A mapping with no `value_kind` on a store that declares nothing for the property: `String`, as
  before (existing structured tests unchanged and green).

## Not in this story

- Changing the kind of a property the store already holds (a schema change, `cortex schema`).
- Kinds for the model-extracted path, which reads `ekr schema` itself.
