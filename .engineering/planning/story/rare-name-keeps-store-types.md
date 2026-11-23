---
format: aep.planning-md/3
id: story:rare-name-keeps-store-types
kind: story
status: active
title: RareName keeps the store names of chosen node types
summary: An optional redaction field names the node types whose store names RareName never takes, so a customer named once keeps its facts and a known person stays hidden (runbook 0.8.0 finding 4)
relations:
- serves: vision:self-updating-instances
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T20:06:38Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T20:06:39Z", actor: "human:timo", revision: 3}
---
## Outcome

An instance can tell `RareName` which names the store already holds are not rare: the facts about
a customer named once in a batch are extracted, and a person the store knows stays hidden from the
model unless the policy says otherwise.

## Why this is a story of its own

Finding 4 of the runbook consumer's run of 0.8.0: `RareName` hides a customer name said once, and
its facts are lost (recall 20 covered of 34 lines with `RareName` on, 26 with it off).
`story:placeholder-restore-and-retry-error` asked that a word that is a node name or alias in the
store at the run's head is not a rare name, or for this draft if that needs more than the change
can carry.

It does. `tests/redaction_gate.rs`
`a_person_the_store_already_knows_is_not_shown_to_the_model_by_name` guards the opposite rule,
which `website/docs/limits.md` states: a person the store already knows is hidden in the prompt's
known entities and in the text. Exempting every store name from `RareName` turns that test red
and shows every known person's name to the model on every instance with `RareName`. That is a
change to the privacy contract, not a fix. It was built on `fix/placeholder-restore` (keep the
words of every node name and alias of the run's first `ekr snapshot` through
`Batch::keep_names`) and taken out again before the pull request.

## Design

- Add an optional field to `cortex.instance.RedactionPolicy`: `keep_store_types:
  Optional<List<String>>`, the node types whose store names `RareName` never takes. Spec first,
  `task generate`, an additive change that `task compat` may report.
- A run reads the canonical names and aliases of the nodes of those types from the snapshot it
  already takes for the known entities (`known::Lookup`, at the first batch, before anything is
  applied), and passes their words to `Batch::keep_names`, as for the ontology names.
  `known_names` and the rules still replace them.
- A node of any other type, a `Person` among them, keeps today's rule: hidden when rare.
- The `refuse_if_left` check reads the same words as kept, as it does for the ontology names.

## Acceptance

With `keep_store_types: [Organization]`, a store holding `Organization` "Quillfeather" and
`Person` "Rowan Pell" at the run's head, and a page that names each of them once, the prompt shows
"Quillfeather" as written and "Rowan Pell" as placeholders. The extraction holds both restored,
and `a_person_the_store_already_knows_is_not_shown_to_the_model_by_name` stays green. Without the
field, the prompt is as in 0.8.0.
