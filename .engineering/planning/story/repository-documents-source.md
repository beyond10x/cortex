---
format: aep.planning-md/3
id: story:repository-documents-source
kind: story
status: draft
title: A source that indexes the documents of a code host's repositories
summary: A repository source walks projects, their default-branch trees and files through Connectors, with globs, caps, a head-commit plus file-id delta and deleted files ended
relations:
- decomposes: epic:organisation-scale-instance
- serves: vision:self-updating-instances
revision: 1
---
## Outcome

An instance can index the documents of a code host's repositories (READMEs, docs, planning files,
ESS specifications) as one source, read through Connectors:

1. A new source kind, `repository`, walks three levels: a project list, then each project's file
   tree on its default branch, then each selected file. Each file is one document, its id the
   project and path, its text the file's raw content.
2. Path globs choose the files (`include`, with `exclude` winning over them). Caps bound the work:
   a file larger than `max_file_bytes` is skipped and named; a project contributes at most
   `max_files_per_project` files; a run reads at most `max_documents_per_run` files and continues
   in the next run where it stopped.
3. Delta: a project whose default-branch head commit is unchanged since the last run is not walked
   again; in a changed project, a file is read again only when its file id (blob id) changed. A
   file that disappears from a walked project's tree is ended in the store, as `dropped:
   Supersede` ends a structured record that is gone.
4. A child read per project (such as the project's languages) is mapped onto properties of the
   project's node, as a structured source maps a record's fields.
5. Every read goes through Connectors operations named in the spec (the list, tree, raw-file and
   languages reads of the GitLab adapter), never a code host API called by cortex itself.

## Why

A downstream deployment wants its ~280 repositories' documentation in its store: 1,800-2,300
documents per full run. Today a `connectors` source runs one operation per run and maps its
records; nothing walks a list, then a tree, then files, and nothing keys a delta on a head commit
plus a file id or ends a deleted file. The Connectors side (GitLab raw-file and languages reads)
is recorded in that repository.

## Domain relations

New nouns, spec first: `cortex.instance.RepositorySource` (a `SourceSettings` variant
`repository`, with `SourceKind` `Repository`), naming the Connectors adapter, connection or
profile, the list, tree, file and child operations with their inputs, the include and exclude
globs, the three caps, and the child mapping (a `cortex.instance.RecordMapping`). The checkpoint
per project (head commit, file ids) is a new state record beside the seen state of a source.

## Work

- Spec first: model the source, its operations and caps, its checkpoint, and the run's report of
  skipped and ended files; validate with the newest `ess`, the conformance suite, `task generate`.
  What the specification cannot express (glob matching, the three-level walk) goes in `UNMAPPED`
  comments naming the code and tests that hold it.
- `src/sources.rs` and a new `src/repository.rs`: the walk, globs, caps, delta and ending.
- `src/run.rs`: the documents go through the model extraction path (a file is text, not a record);
  the child read through the structured mapping path.
- Docs: the spec-file page, the operating page (checkpoint, caps), CHANGELOG.

## Acceptance

- `ess specify validate` passes and `task drift` is clean after `task generate`.
- Against a stand-in Connectors with 3 projects: the first run reads only files matching the
  include globs and not the excludes, skips and names a file over `max_file_bytes`, stops at
  `max_files_per_project` per project and at `max_documents_per_run`, and the next run continues.
- A second run with no head commit changed reads no file.
- A changed head commit with one file changed and one removed: only the changed file is read
  again, and the removed file's document is ended.
- The languages read of each project is written onto the project's node as mapped properties.
- No HTTP request leaves cortex except through Connectors (the stand-in records every call).

## Not in this story

- Code files indexed as code (symbols, call graphs).
- Branches other than the default branch, merge requests, issues.
- The Connectors operations themselves.
