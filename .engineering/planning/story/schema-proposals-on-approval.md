---
format: aep.planning-md/3
id: story:schema-proposals-on-approval
kind: story
status: draft
title: Schema proposals are applied only after an operator approves them
summary: cortex schema --review records pending proposals; approve and reject apply or close them without asking the model again
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- informed_by: story:schema-emergence
revision: 1
---
## Outcome

Schema evolution is a reviewable run: `cortex schema <name> --review` records the model's proposals
as pending, applies none, and names the run's directory; `cortex schema approve <name> <stamp>
[--only <n>…]` applies the approved proposals, each as its own schema transaction as today, and
`cortex schema reject <name> <stamp> [--only <n>…] --reason <text>` closes the rest. A proposal
reaches the store only after somebody approved it.

## Why

A downstream deployment wants schema evolution and derivation as runs that produce a reviewable
delta, applied only after approval. Verified in this tree:

- `cortex schema <name>` has the model propose ontology changes and applies each change EKR accepts
  at once, as a schema transaction of its own; `--dry-run` records the proposals without applying
  any (`src/main.rs`, `Command::Schema`; `src/schema.rs`, `record` and `apply`;
  `website/docs/commands.md`). Delivered by `story:schema-emergence`.
- A dry-run's proposals are written to `schema/<stamp>/proposals.jsonl` with status `dry-run`
  (`spec/domains/instance.yaml`, `cortex.instance.SchemaProposalStatus`), but nothing can apply a
  recorded proposal later: approval today means running `cortex schema` again without `--dry-run`,
  which asks the model again and may propose something else.

## Domain relations

`cortex.instance.SchemaProposal` becomes an entity in `spec/domains/instance.yaml` (identity: the
run's stamp and the proposal's index), with a lifecycle `Pending → Approved → Applied | Refused`,
`Pending → Rejected`. Relation: `cortex.instance.Instance` owns many `SchemaProposal` via
`instance_name`. `cortex.instance.SchemaProposalStatus` gains `pending` and `rejected`. New commands
`ApproveSchemaProposals` and `RejectSchemaProposals` for the `Operator` actor.

## Work

- Spec first: the entity, its lifecycle, the status values and the two commands; validate,
  conformance, `task generate`. Then the code.
- `src/schema.rs`: `--review` writes `pending`; `approve` reads `proposals.jsonl`, re-checks each
  approved change against the store's current ontology (the `plan` checks), applies it, and
  rewrites its line with the result; `reject` records the reason.
- `cortex schema <name>` without `--review` behaves as today.
- Docs: `commands.md`'s schema section.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- `--review` with a stand-in proposing two node types applies none (the store's schema version is
  unchanged) and records both `pending`.
- `approve --only 1` applies the first: the schema version moves, its line says `applied` with the
  revision; the second stays `pending`; no model is asked.
- An approved proposal whose type was added meanwhile is recorded `invalid` and nothing is applied.
- `reject` on a pending proposal records `rejected` with the reason; `approve` on it refuses.
- Without `--review`, `tests/schema.rs` is unchanged and green.

## Not in this story

- Derived assertions (`story:derivation-run`), which reuses this approval.
- Proposing changes (already there).
