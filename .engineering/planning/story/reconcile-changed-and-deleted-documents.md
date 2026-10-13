---
format: aep.planning-md/3
id: story:reconcile-changed-and-deleted-documents
kind: story
status: draft
title: A changed or no longer listed document ends the facts only it supported
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
revision: 1
---
## Outcome

For a model-extracted source, a document whose text changed retracts or supersedes what cortex
applied from its earlier text and the new text no longer supports, and, where the source opts in, a
document no longer listed at the source has every value it alone supports ended.

## Why

Verified in this tree:

- "Change detection is a content hash. [...] A document whose text changed is extracted again only
  after `refresh_after_days`; cortex retracts nothing it applied from the earlier text"
  (`website/docs/limits.md:99-102`). The window is `SeenState::wants` and `holds`
  (`src/state.rs:152-158`, `:167-170`).
- Ending what a source no longer lists exists for structured sources only: the policy
  `cortex.instance.DropPolicy` (`Keep`, `Supersede`) is a field of `cortex.instance.StructuredSource`
  alone (`spec/domains/instance.yaml:241-251`); a run applies it only to structured sources with
  `dropped: Supersede` (`src/run.rs:466`, `end_dropped` at `:989-1000`); ownership of an assertion is
  decided by its evidence identity `record:<identity>` (`src/structured.rs:868-873`), and the ending
  rules are at `src/structured.rs:876-889`. The `Held` set (`:382-390`) keeps values a run did not
  read.
- A model-extracted document is cited by its key (a URL), `file:<key>` or `record:<key>`
  (`src/evidence.rs:26-31`), so the assertions a document supports can be found the same way.
- EKR operations used to end values exist in cortex: `!SupersedeAssertion` and
  `!RetractAssertion` (`src/ekr.rs:532-538`).

## Domain relations

- `Source -> SeenDocument`, one-to-many, the source owns it: `spec/domains/instance.yaml`,
  `cortex.instance.Source` relation `seen` (`:568-572`).
- `SeenDocument -> assertion`, one-to-many, through the evidence identity the assertion cites;
  an assertion belongs to the document only when every evidence item it cites is that document. Inferred
  from `src/evidence.rs:26-31` and `src/structured.rs:868-873`; no ess/1 document declares it.

## Work

- Spec first: extend `cortex.instance.DropPolicy` (or a sibling field) to the source kinds whose
  listing is complete (connectors walks and files), add the counts a run reports for ended values
  if they are new for these kinds, and validate with `ess specify validate` and the conformance
  suite before code.
- On re-extraction of a changed document, compare what the store holds from that document with the
  new extraction and end what the new text no longer supports, as `end_dropped` does for structured
  records; a value another document also supports stays.
- With the opt-in, a document no longer listed ends what it alone supports; a run that listed
  nothing, or failed part-way, ends nothing (as for structured sources).
- Update `website/docs/limits.md:99-102` and the spec-file page.

## Acceptance

- After a document is re-extracted from changed text that drops a property value, that value moves
  from active to retracted or superseded in the store, and a value still stated stays active.
- With the opt-in on, a run that no longer lists a document moves each value only that document
  supports from active to ended, and the run report counts it.

## Not in this story

Web search sources (a result missing from one search is not a deletion); the default of
`refresh_after_days`, which stays the operator knob for how soon a changed document is extracted
again; undoing a run (`story:postgres-run-undo`).
