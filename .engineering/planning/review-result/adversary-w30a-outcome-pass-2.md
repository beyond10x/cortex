---
format: aep.planning-md/3
id: review-result:adversary-w30a-outcome-pass-2
kind: review-result
status: active
title: 'Adversary pass 2: unit outcome (wave 20261030a)'
relations:
- reviews: story:update-refuses-rekey
- reviews: story:schema-apply-outcome-unknown
revision: 1
---
```
unit: outcome (story:update-refuses-rekey, story:schema-apply-outcome-unknown); findings cover e7537d3b on unit/w30a-outcome (420b8144 plus one test file)
verdict: NEEDS-CHANGE (no blocker; three warnings, one note)
cases: executed 19→23 (rekey_adversary 4 + schema_outcome_unknown 4 + seed_change 11, then my 4 with --include-ignored), red 4
origin: introduced 2 / pre-existing 0 / undecided 2
wrote-outside-worktree: 2 paths
needs-coordinator: should `cortex update` refuse a new redaction rule that rewrites structured ids? Refusing blocks a privacy change; accepting re-extracts every record.
```

**1. Diff (commit e7537d3b, by `b10x-bot[bot]` as author and committer).** Only a test file changed, and the working tree is clean.
```
 tests/rekey_adversary_pass2.rs | 322 +++++++++++++++++++++++++++++++++++++++++
```

**2. The four cases.** All are in `tests/rekey_adversary_pass2.rs` and carry `#[ignore = "adversary: …"]`. Each was run alone with `--exact` before the suite ran, and each was red:

- **`a_redaction_rule_that_rewrites_structured_ids_with_seen_records_is_refused`** (:147). The update is accepted and the next run re-extracts both records.
  - Red output: `redaction rule over the ids accepted: {"command":"update","detail":{"added_sources":[],"name":"r"},"outcome":"updated"}; rerun exit 0, outcome "ran", documents_new 2; seen keys before ["files:registry:*.json:p1", "files:registry:*.json:p2"], after ["files:registry:*.json:3946ca64ff78d93c", "files:registry:*.json:f64551fcd6f07823", "files:registry:*.json:p1", "files:registry:*.json:p2"]`
  - Cause: a structured record's stored key comes from `Source::identity_in` (`src/structured.rs:852`). It becomes `<prefix>:<digest>` whenever `redactor.scrub` would change the raw id, so the instance's `redaction.rules` feed the seen key. `keyed_by` only looks at source fields.
- **`a_files_records_format_change_with_seen_documents_is_refused`** (:210). Changing `records.format` from `WholeFile` to `JsonLines` is accepted.
  - Red output: `records.format WholeFile -> JsonLines accepted: …"outcome":"updated"}; rerun exit 0, outcome "ran", documents_new 2; seen keys before [".../logs/a.jsonl"], after [".../logs/a.jsonl", ".../logs/a.jsonl#1", ".../logs/a.jsonl#2"]` (I shortened the scratch path to `...`.)
  - Cause: `WholeFile` keys each file by its path, and `JsonLines` keys each record as `<path>#<id>`. `keyed_by` (`src/main.rs:1523`) compares only whether `records` is set and `records.id`.
  - The check is inconsistent: going from no `records` to `JsonLines` makes the same key change and is refused.
  - The reverse case, a false refusal, also follows from the code but I did not test it. Under `WholeFile`, `records.id` is not part of the key, yet changing it is refused. So is going from no `records` to `{format: WholeFile}`, which the docs (`website/docs/spec-file.md:452`) call the same thing.
- **`adding_a_root_to_a_files_source_with_seen_documents_is_not_a_rekey`** (:261). Changing `paths: [docs]` to `[docs, notes]` is refused.
  - Red output: `assertion left == right failed: {"command":"update","detail":{"field":"paths","name":"a","source":"docs"},"outcome":"rekey-refused"}` / `left: (1, Some("rekey-refused"))` / `right: (0, Some("updated"))`
  - Cause: a file's key is `<spec_dir>/<root>/<file>`, and `update` never rewrites `spec_dir`. Adding, removing or reordering a root re-keys no seen document, but `keyed_by` compares `format!("{:?}", f.paths)`, the whole list as text.
  - Effect: the refusal's documented way forward is to remove `state/<source>.json`, which re-extracts every document. That is the cost the story exists to prevent.
  - `docs/` against `docs` gives the same keys too, so it is also a false refusal. I did not test it separately.
- **`the_whole_record_id_and_a_member_named_dollar_are_two_paths`** (:307). Changing the id path from `$` to `$.$` is accepted.
  - Red output: `assertion left == right failed: {"command":"update","detail":{"added_sources":[],"name":"d"},"outcome":"updated"}` / `left: (0, Some("updated"))` / `right: (1, Some("rekey-refused"))`
  - Cause: `at()` reads `$` as the whole record and `$.$` as a member named `$`, but `record_path` (`src/main.rs:1491`) maps both to `$`. This is the only collision. For every other form `at()` accepts (`a.b`, `$.a.b`, `$.`, empty), `record_path` matches `at` exactly: it strips `$.` once and splits on `.`, with no bracket or index forms.

**3. Suite run, after the cases existed.**
```
cargo test -q --test rekey_adversary --test rekey_adversary_pass2 --test schema_outcome_unknown --test seed_change
test result: ok. 4 passed; 0 failed; 0 ignored   (rekey_adversary)
test result: ok. 0 passed; 0 failed; 4 ignored   (rekey_adversary_pass2)
test result: ok. 4 passed; 0 failed; 0 ignored   (schema_outcome_unknown)
test result: ok. 11 passed; 0 failed; 0 ignored  (seed_change)
… -- --include-ignored:
test result: ok. 4 passed (rekey_adversary)
test result: FAILED. 0 passed; 4 failed (rekey_adversary_pass2: the four above)
```
The `--include-ignored` run stopped at the first failing target, so cargo did not reach the remaining two.

**4. Findings.**

| file:line | finding | verdict | origin | what reaches it |
|---|---|---|---|---|
| src/main.rs:1509 | A redaction rule that matches structured ids re-keys every seen record, and the update is accepted | NEEDS-CHANGE | undecided | Adding `redaction.rules` through `cortex update`, a documented spec field. The fix is either to compare the identity each seen id would get, or to have the coordinator rule it out of scope. |
| src/main.rs:1523 | `records.format` is not compared, so `WholeFile` to `JsonLines` or `MarkdownSections` is accepted | NEEDS-CHANGE | undecided | A documented spec field (`spec-file.md:452`). The fix is to key on "whole files" vs `format` and compare `records.id` only when the format is not `WholeFile`. |
| src/main.rs:1524 | A `files` source's `paths` are compared as one list, so adding, removing or reordering a root is refused | NEEDS-CHANGE | introduced | Adding a folder to a files source is an ordinary edit. The fix is to drop the list compare, since each file's own path is its key and an existing file is not keyed another way. |
| src/main.rs:1491 | `record_path` maps `$` and `$.$` to one path | INFEASIBLE | introduced | Nothing found. It needs a record with a member literally named `$`, and the change makes records unreadable rather than re-extracted. |

The origin is undecided for the first two because I did not run them at base 97573d67, which has no re-key check at all.

**5. Attacked and could not break.**
- **`apply_run` with a mix of outcomes:** Applied plus Invalid or Refused says "changes are in the store". All non-Applied says "no change was committed".
- **`apply_run` edge arms:** an Applied proposal always carries `revision` (`src/schema.rs:1213`), so the `(true, None, None)` arm cannot be reached. Zero approved proposals is refused earlier as `NotApproved`.
- **Pass-1 tests:** commit 420b8144 only removed the five `#[ignore]` lines. No assertion was dropped or relaxed.
- **Web sources:** the key is the URL alone, so adapter or connection changes do not re-key.
- **`connectors` sources:** the key is `adapter:operation:id`. `connection`, `records`, `child` and paging are not part of it.
- **`spec_dir`:** `update` never rewrites it, so relative paths keep resolving the same way.
- **Structured files prefix:** paths in their order plus the glob are compared exactly as `prefix_parts` uses them.
- **Mixed field-list lengths in `rekeyed`:** a kind, input or records switch always differs before `zip` truncates the lists.

**6. Paths written outside the worktree.**
- `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/outcome/pass2-build.log`
- `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/outcome/msg.Ew8iGt` (the commit message)
- The test runs also created their own worlds under `~/.cache/claude-tmp/cortex-*`.

**7.**
```findings
- file: src/main.rs
  line: 1509
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: undecided
  message: a structured record's seen key depends on the instance's redaction rules, so adding a rule that matches the ids re-keys every seen record and the update is accepted
- file: src/main.rs
  line: 1523
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: undecided
  message: keyed_by ignores records.format, so WholeFile to JsonLines re-keys every file as path#id and is accepted while the same change from no records is refused
- file: src/main.rs
  line: 1524
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a files source's paths are compared as a whole list, so adding a root, which keys no seen document another way, is refused and the operator is sent to remove the state and re-extract everything
- file: src/main.rs
  line: 1491
  category: property
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: record_path maps $ (the whole record) and $.$ (a member named $) to one path although at() reads them differently, so that id change is accepted
```
