---
format: aep.planning-md/3
id: decision-blocker:parent-identity-prefix-encoding
kind: decision-blocker
status: cleared
title: 'Parent identity prefixes masking reads: migrate, refuse or encode only affected parts'
relations:
- blocks: story:parent-identity-prefix-unmasked
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-08T13:06:24Z", actor: "human:timo", revision: 3}
---
## What stops the story

`story:parent-identity-prefix-unmasked` names two ways and picks neither. The parent prefix is `<adapter>:<operation>` or `files:<paths>:<glob>` (`src/structured.rs` `prefix`). When the operation, a path or the glob ends in a credential's name, masking reads it and the `:` after it as an assignment, `SeenState::load` drops the key and the record is applied again on every run.

## Options

| option | does | costs |
|---|---|---|
| A | write every prefix part in hex, as `child_prefix` does for children | every structured identity of every existing store changes: a migration of seen state and of evidence keys |
| B | refuse such a spec in `spec::load_given` | an existing instance with such a source stops loading; its operator must rename a path or an operation they may not own |
| C | write a prefix part in hex only when masking would change `<part>:<digest>`, the test `child_prefix` already applies to operations | identities of unaffected sources do not change, so no migration; an affected source's records get new evidence identities once (they are re-applied on every run today) |

Recommended: C. It reuses the rule children already follow, changes no identity that works today, and loads every existing spec.

## What would clear it

A choice of A, B or C, recorded in the story's `## Open` section as decided.

## Decided (2026-10-08)

Option C: a parent prefix part is written in hex only when masking would change `<part>:<digest>`, the test `child_prefix` already applies to operations. A changes every working identity; B stops existing instances loading. It lands in the next wave; it does not reach a running instance without its own decision.
