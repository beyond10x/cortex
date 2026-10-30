---
format: aep.planning-md/3
id: review-result:adversary-w30a-outcome-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: unit outcome (wave 20261030a)'
relations:
- reviews: story:update-refuses-rekey
- reviews: story:schema-apply-outcome-unknown
revision: 1
---
verdict: fail

```
unit: outcome (story:update-refuses-rekey, story:schema-apply-outcome-unknown); findings cover 0efa277b on unit/w30a-outcome (a492f083 plus test files only)
verdict: NEEDS-CHANGE (one blocks merge, four follow-up)
cases: executed 18→23 in rekey_adversary + schema_outcome_unknown + seed_change (the 18 are 3+11 run plus the 4 new ignored ones as 0); red 5 with --ignored
origin: introduced 2 / pre-existing 0 / undecided 3
wrote-outside-worktree: 1 path
needs-coordinator: whether a structured source's mapping.id belongs on the re-key list. The Decisions list leaves it out; the story's Outcome covers it.
```

**1. Diff of what I touched (commit 0efa277b, by `b10x-bot[bot]`).** Only test files changed. Working tree clean.
```
 tests/rekey_adversary.rs        | 313 ++++
 tests/schema_outcome_unknown.rs |  54 ++++
```

**2. Findings.** Each case is `#[ignore = "adversary: …"]`. Each red line below comes from running that case alone with `cargo test -q --test <file> -- --ignored --exact <name>`, before the suite ran.

**Blocks merge**
- **`a_structured_mapping_id_change_with_seen_records_is_refused`** (`tests/rekey_adversary.rs`, `--test rekey_adversary`)
  - Result: `mapping.id change accepted: {"command":"update","detail":{"added_sources":[],"name":"s"},"outcome":"updated"}; documents_new after the update: 2; seen keys before ["files:registry:*.json:p1", "files:registry:*.json:p2"], after ["files:registry:*.json:A-1", "files:registry:*.json:A-2", "files:registry:*.json:p1", "files:registry:*.json:p2"]`
  - A structured record's key is `<prefix>:<raw id at mapping.id>`. That is the same defect as C-04 (a changed `id` path re-ingests everything), on structured sources. `keyed_by` (`src/main.rs:1488`) does not compare `mapping.id`.
  - The fix is one more entry in `keyed_by`: `("id", s.mapping.id)`.
  - Origin undecided: not run at base. Base has no re-key check at all.

**Follow-up**
- **`a_structured_files_paths_change_with_seen_records_is_refused`** (`--test rekey_adversary`)
  - Result: `paths change accepted: …"outcome":"updated"}; documents_new after the update: 2; seen keys before ["files:registry:*.json:p1", "files:registry:*.json:p2"], after ["files:people:*.json:p1", "files:people:*.json:p2", "files:registry:*.json:p1", "files:registry:*.json:p2"]`
  - A files input's `paths` and `glob` are part of the prefix (`src/structured.rs` `prefix_parts`), but `keyed_by` compares only `("input","files")`.
  - `commands.md` and CHANGELOG say "a `structured` source's input" counts as a re-key, so a reader will assume paths are covered.
  - Origin undecided.
- **`a_files_source_paths_change_with_seen_documents_is_refused`** (`--test rekey_adversary`)
  - Result: `paths change accepted: …"outcome":"updated"}; documents_new after the update: 1; seen keys before [".../docs/a.md"], after ["..././docs/a.md", ".../docs/a.md"]`. I shortened the absolute scratch path to `...`.
  - A `files` source keys each document by its path, so `paths: [docs]` → `[./docs]` re-extracts every file. The story's scoper wrote "files has none"; that is wrong.
  - Origin undecided.
- **`an_equivalent_id_path_spelling_is_not_a_rekey`** (`--test rekey_adversary`): a harmless change is refused.
  - Result: `assertion `left == right` failed: {"command":"update","detail":{"field":"id","name":"t","source":"issues"},"outcome":"rekey-refused"}` / `left: (1, Some("rekey-refused"))` / `right: (0, Some("updated"))`
  - `id: key` and `id: "$.key"` produce the same keys because `at()` strips `$.`, but `keyed_by` compares the text. Few users will hit this.
  - Origin introduced.
- **`a_postgres_apply_that_committed_nothing_does_not_say_changes_are_in_the_store`** (`tests/schema_outcome_unknown.rs`, `--test schema_outcome_unknown`)
  - Result: `nothing was committed, yet: the store's head could not be read: ekr head failed: ekr: the store is unavailable: injected; the changes are in the store: its stage was published at head 4; every decision is written to proposals.jsonl`
  - In `src/schema.rs` `apply_run`, the `(Some(head), _)` arm says "the changes are in the store" whenever a stage exists, even when no proposal was applied (here the proposal was `invalid` and the head did not move). This is the reverse of the false claim the story fixed.
  - Origin introduced.

**3. Suite run, after the cases existed.**
```
cargo test --test rekey_adversary --test schema_outcome_unknown --test seed_change
test result: ok. 0 passed; 0 failed; 4 ignored; …   (rekey_adversary)
test result: ok. 3 passed; 0 failed; 1 ignored; …   (schema_outcome_unknown)
test result: ok. 11 passed; 0 failed; 0 ignored; …  (seed_change)
EXIT=0
… -- --include-ignored:
test result: FAILED. 0 passed; 4 failed; 0 ignored; … (rekey_adversary)
test result: FAILED. 3 passed; 1 failed; 0 ignored; … (schema_outcome_unknown)
```

**4. Judgement findings (no test written).**
- **Spec text narrower than the code** (`spec/domains/instance.yaml`, `rekey-refused` external). The spec names only a structured connectors input's `adapter` and `operation`. The code also refuses switching the input between connectors and files (field `input`), and `commands.md` lists that. Small doc drift. Introduced, CONFIRMED.
- **A refusal with no seen documents.** When the state file exists but cannot be parsed (a zero-byte file, say), it counts as holding a seen document. The update is refused naming the changed field, though the state file is the real problem. Removing the file, as the refusal says, fixes it. INFEASIBLE: nothing found that writes such a file.

**5. What I attacked and could not break.**
- `cortex schema` stops at an unknown commit: no later proposal reaches `ekr commit`, earlier lines stay written, exit 4.
- The unknown write is caught both from `plan()`'s mint and from `apply()`.
- The `propose-failed` and `apply-failed` reasons never claim nothing was applied when something was.
- `stage_head` returns the head the stage began at, which is the head `publish` uses.
- An unknown commit inside `cortex schema apply` comes out as `apply-failed`, exit 1. The hold has already undone it (snapshot restored, or stage abandoned), and the spec's `store-outcome-unknown` wording for apply covers only the stage.
- Exit 4 matches the spec, `commands.md` and the test.
- Schedule, policy, `inputs`/query, ekr-pin changes and reordering sources are not refused.
- A refused update writes no frozen spec, state, units or registry row.
- A missing state file, or one with no documents, is accepted.
- A renamed source is accepted and re-extracts everything. The story scopes renames out.

**6. Paths written outside the worktree.**
- `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/outcome/msg.70KGxV` (commit message)
- The test runs created their own worlds under `~/.cache/claude-tmp/cortex-*`.

**7.**
```findings
- file: src/main.rs
  line: 1488
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: undecided
  message: keyed_by omits a structured source's mapping.id, so changing it with seen records is accepted and every record is re-extracted under new keys, the C-04 defect on structured sources
- file: src/main.rs
  line: 1508
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: a structured files input's paths and glob key every record but only the input kind is compared, while commands.md says a structured source's input is a re-key field
- file: src/main.rs
  line: 1510
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: a files source keys documents by path, and a paths change such as docs to ./docs is accepted and re-extracts every file
- file: src/main.rs
  line: 1500
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the connectors id path is compared as text, so the equivalent spelling $.key for key is refused as a re-key
- file: src/schema.rs
  line: 1484
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: after a postgres publish that committed nothing, a head-read failure answers apply-failed saying the changes are in the store
- file: spec/domains/instance.yaml
  line: 1622
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the rekey-refused external omits the structured input-kind switch that the code refuses and commands.md lists
- file: src/main.rs
  line: 1532
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: an unreadable state file counts as seen and the refusal names the changed field rather than the unreadable state
```
