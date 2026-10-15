---
format: aep.planning-md/3
id: story:adopt-seen-by-key
kind: story
status: draft
title: An adopted source's seen documents can be named by key, and the first run records their hash
relations:
- decomposes: epic:periodic-collection-reconcile
- serves: vision:self-updating-instances
revision: 1
---
## Outcome

`cortex adopt --seen <source>=<file>` accepts a `cortex.seen/1` document whose entries name a key
and `applied_at` but no `hash`. On the source's first run after the adoption, cortex computes the
hash of each such document as it renders and masks it, records it, and does not extract the
document; a later run extracts it again only when that hash changes. An adopter that knows which
records its store already holds (their keys) can adopt the store without re-extracting its history.

## Why

A downstream consumer adopts a store another pipeline built. The hash cortex remembers is SHA-256
of the text as cortex renders, scrubs and masks it (author lookup, mention mapping, thread context,
redaction), which only cortex can compute; the adopter can only give the keys. Today an entry
without a hash is refused and an entry whose hash differs is re-extracted, so the adoption
re-extracts the whole history at model cost and writes a second set of assertions and evidence.
Verified in this tree:

- `Seen` has a required `hash: String` (`src/state.rs:18-27`), and `cortex adopt --seen` reads the
  file with `SeenState::read_document` (`src/state.rs:112-126`), which deserialises it as a state
  file, so an entry with no `hash` fails to parse and `adopt` fails with
  `--seen <source>: <file>: missing field hash` (`src/main.rs:1084-1099`). The document's shape is
  `{"format": "cortex.seen/1", "documents": {"<key>": {"hash": ..., "applied_at": ...}}}`
  (`src/state.rs:28-31`), keyed by the masked key.
- A run extracts a document when its key is unseen or its hash differs past `refresh_after_days`;
  a record read from a file waits for no refresh window (`src/state.rs:79-81`, `:150-161`,
  `SeenState::wants`), so a files source re-extracts at once every adopted record whose hash
  differs.
- `cortex.instance.SeenDocument` declares `content_hash: String`
  (`spec/domains/instance.yaml:588-602`), and the `ran` outcome's `UNMAPPED:` comment states the
  extraction rule (`:1048-1055`). `AdoptInstance` says each source's seen documents start from the
  `--seen` file (`:767-772`); `adopted` counts `seen_documents` and `seen_without_evidence`
  (`src/main.rs:1052-1071`). The operating page describes `--seen` at
  `website/docs/operating.md:257-264`.

## Domain relations

No new noun. `cortex.instance.Source` owns many `cortex.instance.SeenDocument` through its relation
`seen` (`spec/domains/instance.yaml:567-571`, via `source_id`); this story makes the
`content_hash` of a `SeenDocument` optional until the source's next run records it.

## Work

- Spec first: in `spec/domains/instance.yaml`, make `SeenDocument.content_hash`
  `Optional<String>` with a comment saying when it is absent (adopted by key, not yet run), and
  extend the `UNMAPPED:` rule of `RunSource`'s `ran` outcome and the `AdoptInstance` comment: a
  seen document with no hash is recorded with the hash of the document the run fetched under its
  key and is not extracted. Run `ess specify validate` and the conformance suite green, then
  `task generate`. Only then change the code.
- `src/state.rs`: `Seen.hash` becomes optional on read; `read_document` accepts an entry with
  `applied_at` and no `hash`, and refuses one with neither. `wants` answers false for a key whose
  hash is absent, and `holds` likewise; the run records the computed hash, keeping the entry's
  `applied_at`.
- `src/run.rs` (`process`): before selection, every fetched document whose key is seen with no
  hash gets its hash recorded and is counted in the run's report (a new count, for example
  `hashed`), not in `documents_new`; the state is written by `finish` as today, also on a run
  with nothing else new. A key that is seen without a hash and not fetched keeps no hash.
- Choice made: the hashless entry in the existing `--seen` file, so one flag and one format.
  Option left open: `--seen-keys <source>=<file>`, one key per line, `applied_at` the adoption
  time; not built here.
- Keys in the file are the keys cortex gives documents (masked); the docs say so with an example
  for a files source with records (`<path>#<id>`).
- Tests: `src/state.rs` unit tests beside
  `a_seen_document_is_read_as_a_state_file_and_another_format_is_refused` (`:274`), and an
  end-to-end case in `tests/e2e.rs`. Update `website/docs/operating.md:257-264`.

## Acceptance

- `ess specify validate` passes with `content_hash` optional, and `task drift` is clean after
  `task generate`.
- An instance with one files source `chat` over `archive/*.jsonl` (records `id: ts`), adopted with
  `--seen chat=seen.json` where `seen.json` is
  `{"format":"cortex.seen/1","documents":{"archive/2026-09.jsonl#1.0":{"applied_at":1}}}`:
  `cortex adopt` exits 0 and reports `seen_documents: 1`.
- The first `cortex run x/chat` after that sends no prompt holding record `1.0` to the stand-in
  `claude`, applies nothing for `archive/2026-09.jsonl#1.0`, extracts the other records of the file,
  and leaves `state/chat.json` with a 64-hex `hash` and `applied_at: 1` for that key.
- A second run over the same file extracts nothing; after the text of record `1.0` changes, the next
  run extracts it.
- A `--seen` entry with neither `hash` nor `applied_at` is refused by `cortex adopt`, naming the
  key.

## Not in this story

The `--seen-keys` form; deriving keys from the store's evidence; adopting a store at `cortex create`;
any change to how a hash is computed.
