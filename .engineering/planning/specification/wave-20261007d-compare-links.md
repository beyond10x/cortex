---
format: aep.planning-md/3
id: specification:wave-20261007d-compare-links
kind: specification
status: implemented
title: 'Wave 20261007d: structured compare links'
relations:
- specifies: story:structured-compare-links
revision: 5
transitions:
- {from: "draft", to: "in_review", at: "2026-10-07T09:14:19Z", actor: "human:timo", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-07T09:14:19Z", actor: "human:timo", revision: 3}
- {from: "approved", to: "implemented", at: "2026-10-07T22:59:55Z", actor: "human:timo", revision: 4}
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

## Outcome

Closed 2026-10-07. U1 `story:structured-compare-links`: `035e597`, adversary cases `da7c46f`,
fix `262f0e8`, merged `9a40c5a`; review, changelog, status and docs `307261b`.

Package-scoped steps on `307261b`, each exit 0: `ess specify validate`, `task drift`,
`task docs-check`, `cargo fmt --check`, `cargo clippy -p cortex-cli -p cortex-docs --all-targets
-D warnings`, `cargo test -p cortex-cli`, `cargo test -p cortex-docs`, `--list`: 42 suites, 536
passed, 0 failed, 0 ignored. The pull request's CI runs the full gate.

| agent | tokens | tool uses | wall time |
|---|---|---|---|
| implementor U1, round 1 (stopped by a usage limit; its uncommitted work was kept) | not recorded | not recorded | not recorded |
| implementor U1, round 2 (fresh; checked and finished round 1's work) | 118,664 | 54 | 622 s |
| adversary U1 pass 1 | 228,535 | 63 | 998 s |
| implementor U1, round 3 (the pass 1 fixes) | 218,322 | 78 | 1,117 s |

U1 pass 1: 3 findings, all introduced. (1) A change applied before its tag waited for the refresh
window before gaining its edge: `SeenDocument.unlinked_hash` was added to the spec, and a change
whose text differs only in its links no longer waits. (2) A link failure held every value of the
parent's changes: it now holds only the link relation's assertions. (3) Four mutants the original
suite missed: the adversary's green cases catch them. Fixed in `262f0e8`; the coordinator read the
correction (no assertion dropped, the adversary file unchanged). The wave ran one adversary pass,
as its rules say.

Round 1 was cut off after its gate and a mutation run, before its report; the build directory was
removed with `cargo clean` while it was stopped, and `ekr` 0.0.32 was rebuilt into the unit tree for
the end-to-end tests. `b10x-gates scan-text` without `--policy` exits 1 with `protected file
unavailable`; `AGENTS.md` now says to pass the policy.
