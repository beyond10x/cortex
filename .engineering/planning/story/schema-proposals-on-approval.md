---
format: aep.planning-md/3
id: story:schema-proposals-on-approval
kind: story
status: active
title: Schema proposals are recorded, decided one by one, and only the approved ones applied in a gated stage
summary: cortex schema --review records pending proposals; approve and reject decide each; apply runs exactly the approved ones in an EKR stage or after a snapshot, with no second model call
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
- informed_by: story:schema-emergence
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-17T02:21:55Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-17T02:21:56Z", actor: "human:timo", revision: 5}
---
## Outcome

Schema evolution is a reviewable run, applied only after a person decides each proposal:

1. `cortex schema <name> --review` has the model propose ontology changes, records every proposal
   `pending` in `schema/<stamp>/proposals.jsonl`, applies none, and names the run's stamp.
2. `cortex schema approve <name> <stamp> [--only <n>…]` and `cortex schema reject <name> <stamp>
   [--only <n>…] --reason <text>` record a decision per proposal. A decided proposal is not
   decided again.
3. `cortex schema apply <name> <stamp>` applies exactly the approved proposals, each re-checked
   against the ontology at the head, inside a gated stage: on a `postgres` store an EKR stage
   (`ekr stage begin`, published only when every approved change committed in it and the spec's
   `gate` passed, abandoned otherwise); on a `sqlite` store after a snapshot `cortex restore` can
   put back. No model is asked: the changes are the recorded ones, byte for byte.
4. `cortex schema <name>` without `--review` keeps today's behaviour until a later release says
   otherwise (`--dry-run` included).

## Why

A downstream deployment wants schema evolution as a run that produces a reviewable delta, applied
only after approval. Verified in this tree:

- A dry run's proposals cannot be applied later. `--dry-run` (`src/main.rs:183-185`) records each
  proposal with status `DryRun` (`src/schema.rs:934`) and, for a change EKR would apply, only adds
  it to the names it plans against (`src/schema.rs:959`); no verb reads `proposals.jsonl` back.
  Approving today means running `cortex schema` again without `--dry-run`, which draws a new sample
  and asks the model again (`src/main.rs:1673-1675`, `schema::draw` then `schema::ask` before
  `schema::record`), and may propose something else.
- A non-dry run applies at once: `record` submits each change EKR accepts as one schema
  transaction of its own, straight to the store's head (`src/schema.rs:960`, `apply` at
  `src/schema.rs:993-1005`, `t.store.submit` at `src/schema.rs:1004`), against the store handle of
  `target_of` (`src/schema.rs:905-911`), under the lock only (`src/main.rs:1679-1693`).
- There is no stage, no gate and no undo for it. `src/schema.rs` names no stage, gate or snapshot
  anywhere (its only match for `gate` is the model `gateway`, `src/schema.rs:452`). A source run on a
  `postgres` store held to a gate runs in an EKR stage (`src/run.rs:751-754`, `gated` at
  `src/run.rs:1126-1128`, the stage published or abandoned from `src/run.rs:1187-1193`), and a run
  on a `sqlite` store takes a snapshot before its first apply (`src/snapshot.rs:1-8`); a schema
  change gets neither. On `postgres` the only way back is the operator's database backup
  (`src/run.rs:751`, the comment).
- `cortex.instance.SchemaProposalStatus` is `[applied, refused, recorded-only, invalid, dry-run]`
  (`spec/domains/instance.yaml:708-710`); `cortex.instance.SchemaProposal` is a struct, not an
  entity, so nothing can be decided about it (`spec/domains/instance.yaml:718-730`).

Order: after `story:repository-documents-source` (the deployment's first need), and before
`story:schema-widens-edge-type`, `story:derivation-run` and `story:research-run`, which reuse this
review and apply path. No `depends_on` on the documents source: nothing here needs it.

Not verified here: whether an EKR stage admits a schema transaction (`ekr stage` is documented for
a run's extractions, EKR `docs/cli.md`, `ekr stage`). Inferred: it does, as a stage is a tenant
replayed through the kernel's authority; the first step of Work checks it.

## Domain relations

Spec first, in `spec/domains/instance.yaml`:

- `cortex.instance.SchemaProposal` (`:718`) becomes an entity, identity the run's stamp and the
  proposal's index, with a lifecycle `Pending → Approved → Applied | Refused | Invalid`,
  `Pending → Rejected`. Relation: `cortex.instance.Instance` owns many `SchemaProposal` via
  `instance_name`. A field `decision_reason: Optional<String>` and `decided_by: Optional<String>`.
- `cortex.instance.SchemaProposalStatus` (`:708`) gains `pending`, `approved` and `rejected`.
- New commands `ApproveSchemaProposals`, `RejectSchemaProposals` and `ApplySchemaProposals` for
  `cortex.instance.Operator` (`:817`), beside `ProposeSchemaChanges` (`:1720`); the apply's
  outcomes say `applied`, `gate-failed` (nothing published) and `store-outcome-unknown`.
- The gate is the existing `cortex.instance.RunGate` (`:509`), read from `InstanceSpec.gate`
  (`:584`). UNMAPPED: whether the schema apply needs measures of its own (the run report's
  measures, `src/gate.rs` `REPORT_MEASURES`, are a source run's) is open.

## Work

- Spec first: the entity, its lifecycle, the status values, the three commands and their outcomes;
  `ess specify validate`, the conformance suite, `task generate`. Then the code.
- Check that `ekr stage` admits a `DefineNodeType`, `DefineEdgeType` and `ModifyProperty`
  transaction and publishes it; if not, file an upstream blocker against this story before code.
- `src/schema.rs`: `--review` writes `pending`; `approve`, `reject` rewrite a line's decision;
  `apply` reads `proposals.jsonl`, plans each approved change against the head (the `plan`
  checks), and applies inside the stage or after the snapshot, sharing the stage verbs of
  `src/run.rs` (`Staged`) rather than copying them.
- `src/main.rs`: the three subcommands.
- Docs: `website/docs/commands.md`'s schema section, the operating page (stage, restore).

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- `--review` with a stand-in model proposing two node types applies none (the store's schema
  version is unchanged) and records both `pending`.
- `approve --only 1`, then `apply`: the first is applied (the schema version moves, its line says
  `applied` with the revision), the second stays `pending`, and the stand-in model records no
  call during `approve` or `apply`.
- On a `postgres` stand-in with a gate that fails: `apply` publishes nothing, abandons the stage,
  and every approved proposal stays `approved`; the store's head is unchanged.
- On `sqlite`: `apply` leaves a snapshot that `cortex restore` puts back to the ontology before it.
- An approved proposal whose type was added meanwhile is recorded `invalid`; nothing is applied
  for it.
- `reject` records `rejected` with the reason; `approve` and `apply` on it refuse.
- Without `--review`, `tests/schema.rs` is unchanged and green.

## Not in this story

- Widening an edge type's ends (`story:schema-widens-edge-type`).
- Derived assertions (`story:derivation-run`), which reuses this review and apply path.
- Proposing changes (already there, `story:schema-emergence`).
- Removing, merging or splitting types: EKR applies none (`spec/domains/instance.yaml:667-669`).
