---
format: aep.planning-md/3
id: specification:wave-20261007d-compare-links
kind: specification
status: approved
title: 'Wave 20261007d: structured compare links'
relations:
- specifies: story:structured-compare-links
revision: 3
transitions:
- {from: "draft", to: "in_review", at: "2026-10-07T09:14:19Z", actor: "human:timo", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-07T09:14:19Z", actor: "human:timo", revision: 3}
---
## Wave 20261007d: structured compare links

Opened 2026-10-07 by the coordinating session, `aep:implementing` 0.20.1 wave mode. N is one:
`story:docs-for-1-0` shares `website/docs/spec-file.md` with this story and documents it, so it
follows; `story:parent-identity-prefix-unmasked` has an open decision (migration or refusal);
`story:postgres-run-undo` is blocked upstream; `story:codex-model-backend` is held by its decision
blocker.

**Approval:** the operator approved every wave up front on 2026-10-05 ("I approve all waves upfront
and now. do not ask for permission, you orchestrate this").

## Units

| unit | story | scope | branch | worktree | build dir | scratch |
|---|---|---|---|---|---|---|
| U1 | `story:structured-compare-links` | cited (2 lines inferred) | `unit/structured-compare-links` | `cortex-w20-a` | `cortex-w20-a/target` | `cortex-w20-a/target/wave-scratch` |

Integration branch `wave/20261007d`, tree `cortex-w20-int`.

## Decisions taken for U1

The story leaves these open; the coordinator decided them before dispatch:

- **Construct:** a new `CompareLink` on `StructuredConnectors` (`links: Optional<List<CompareLink>>`):
  the compare `operation`, its `input`, `records` and `paging` as a child call has them; `tags` and
  `changes`, the names of two child operations of the same source; `order`, a field of the tag
  record compared as a time (the tags' order, ascending); `change_id`, the field of a compare record
  that holds the change's `id`; `relation`, the edge name (`shipped_in`).
- **Ranges:** per parent, the tags sorted by `order`, ties by identity; one compare call per tag,
  from the tag before it to it. The first tag's call renders `{from.…}` empty (the existing
  `render` behaviour), so the first range is "everything up to the first tag"; the docs say so.
- **First wins:** a change in more than one range is linked to the earliest tag only.
- **Matching:** a compare record's `change_id` value is matched to the change records of the same
  parent through the identity `prepare` gives them; a change with no match gets no edge.
- **Evidence:** the `shipped_in` link is part of the change's own document (its hashed, cleaned
  content), so a change merged before its tag gets the edge in the run that first sees the tag.
- **Failures:** a compare call that fails, or a tag with no usable `order` value, is named in
  `skipped`; changes of that range get no new edge in that run, and edges already stored are not
  ended for it.
- **Out of scope:** tags on other branches; changes reverted after a tag.

## Rules

- One implementor (`aep:implementor`), one adversary pass (`aep:adversary`).
- Every command runs with `TMPDIR` under the unit's scratch directory.
- The full `task check` runs once, on the integration branch, before the push; under 30G free it
  waits for the build slot.

## Commits approval authorises

The unit commit through `b10x-gates bot`, the adversary's test commit and a fix commit; the merges
into `wave/20261007d`; coordinator commits for the store and the changelog; the closing
planning-store commit; the pull request into `main` and its merge.
