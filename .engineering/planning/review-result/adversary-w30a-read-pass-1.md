---
format: aep.planning-md/3
id: review-result:adversary-w30a-read-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: unit read (wave 20261030a)'
relations:
- reviews: story:one-store-read-per-run
- reviews: story:idle-supersede-source-cheap
revision: 1
---
verdict: fail

unit: `read` (story:one-store-read-per-run, story:idle-supersede-source-cheap). The findings cover `f47fc2f1` on unit/w30a-read: the unit head `2600a88c` plus my test-only commit.
cases: executed 4→8, red 4
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 12 paths, all under `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/read/`
needs-coordinator: one fix decision for findings 1 and 2, below. I did not take a worktree lease for this pass.

```
 tests/idle_supersede_adv.rs | 241 +++
 tests/one_store_read_adv.rs | 236 +++
```
Both are test files; no implementation file changed. They are in commit `f47fc2f1` (author and committer are `b10x-bot[bot]`; scan-text passed on both files and the message). Each case carries `#[ignore = "adversary: …"]`, so the branch stays green.

## Findings

Every command ran in the worktree with `CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=4 nice -n 19`. Each case was red the first time it ran alone.

**1. Blocks merge.** A run that applies records but does not finish its ends leaves the old fingerprint in the state file. The next idle run with that listing skips the end, so a record that has left the source keeps its value active.
- Test: `a_record_applied_by_a_run_that_failed_before_its_ends_is_still_ended_once_dropped`
- Command: `cargo test --test idle_supersede_adv -- --ignored a_record_applied_by_a_run_that_failed`
- Result: `assertion `left == right` failed: P-3's team, which the store holds and the source no longer lists, is retracted: {... "outcome":"ran"}` / `left: Null` / `right: 1`
- The case checks that P-3's `Core` is in the store before the idle run.
- Where: `src/run.rs:897` runs `apply_records`, which saves `seen`, and that saved state still holds the old `ended_listing`. The only place it is reset is `:906`, after `end_dropped` returns.
- What reaches it: any failure or SIGTERM between the apply and the end. On PostgreSQL that end is the snapshot that takes minutes. Code reading says a connector fetch that leaves pages unread also reaches it, because that run applies records and ends nothing (`:905`); I did not run that path.
- Origin: introduced. At base `7a217da7`, `src/run.rs:841` sends every Supersede run to `end_dropped`.
- Suggested fix: set `seen.ended_listing = None` before the first write of any run that selected something, and in every run where `listed` is `None`.

**2. Blocks merge (same cause and same fix as 1).** Runs under `dropped: Keep` write values and never clear `ended_listing`. Back on `Supersede` with the remembered listing, nothing is ended.
- Test: `a_record_applied_while_the_source_kept_dropped_values_is_ended_back_on_supersede`
- Command: `cargo test --test idle_supersede_adv -- --ignored a_record_applied_while_the_source_kept`
- Result: `assertion `left == right` failed: P-3's team, which the source no longer lists, is retracted on the first Supersede run: {...}` / `left: Null` / `right: 1`
- Where: `src/run.rs:851`. What reaches it: an operator switching the source to `Keep` with `cortex update`, a record added and removed meanwhile, then switching back.

**3. Follow-up.** The grown index treats a store node's canonical name as an alias. EKR compares aliases only (`ekr-integrate` `resolve`: "`canonical_name` is never compared"). So when EKR creates a second node beside one with that canonical name, the index leaves it out.
- Test: `a_node_ekr_creates_beside_a_canonical_name_that_is_no_alias_is_in_the_grown_index`
- Command: `cargo test --test one_store_read_adv -- --ignored a_node_ekr_creates_beside`
- Result: `left: ["Organization \"Acme Corp\" [\"acme\"]"]` / `right: ["Organization \"Acme Corp\" [\"acme\"]", "Organization \"Acme Corp\" []"]`
- Where: `src/known.rs:265`. What reaches it: a node whose canonical name is not one of its aliases, as an `ekr_seed` can hold.
- Effect: the model still sees a node of that name, so the impact is low.

**4. Follow-up.** The index drops both ends of a rejected `facts[i]`. A fact the kernel rejects at commit (here `edge-cardinality`) had its nodes created before the commit, so the next batch is not shown a node that is in the store.
- Test: `the_object_of_a_fact_rejected_at_commit_is_in_the_grown_index`
- Command: `cargo test --test one_store_read_adv -- --ignored the_object_of_a_fact_rejected`
- Result: `left: ["Organization \"Beta Inc\" []"]` / `right: ["Organization \"Beta Inc\" []", "Organization \"Gamma Inc\" []"]`
- Where: `src/known.rs:227`. What reaches it: any edge type of cardinality `One` in a seed or schema. `entity_names` reads only `entities`, so the node is not shown as a run name either.
- Suggested fix: do not drop the named things of a rejected `facts[i]` that carries a `transaction_id`, since that marks a commit-time rejection.

## Suite after the cases existed

`cargo test --no-fail-fast --test idle_supersede --test one_store_read --test idle_supersede_adv --test one_store_read_adv -- --include-ignored` exited 101. The two existing targets passed (`ok. 2 passed` each); both new targets failed (`FAILED. 0 passed; 2 failed`). The before count of 4 comes from the same command without the new files and without `--include-ignored`, which exited 0.

## Probed without a failing case

- **Apply that stopped partway:** this can show a node as "in the store" that was never created. It changes only the label of a known-entity line: known names still go through `pseudonymise` and `refused` (`src/run.rs:2299`), so no redaction decision changes, and no fact is lost because EKR creates the node when a later batch names it. I could not build this without fault injection (INFEASIBLE).
- **Aliases EKR adds to an existing node:** `apply-extraction` 0.0.34 never adds an alias to a node it resolved, so this gap does not occur.
- **Batch-0 reuse under `refuse_if_left`:** reading the code, it is used only for the first pop, which is batch 0 unsplit, and the size check uses the pre-check's known entities. No test shows acceptance 3; nothing outside the run can see it.
- **PostgreSQL stage path:** not run, as there is no PostgreSQL here.
- **Could not break:**
  - the `ended` index and its early returns (the edges ended only come from retracted relations);
  - a held prefix released, or an unsettled record settling (both change the fingerprint);
  - a definition change hidden from `{:?}` (the model has no HashMap);
  - a failed end commit (`Ok(false)` clears the fingerprint);
  - group naming against EKR's `grouped` (it is the least first alias, as the index has it).

## Paths written outside the worktree

All 12 are in `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/read/`:
- logs: `case-a.log`, `case-b.log`, `case-c.log`, `case-d.log`, `suite-before.log`, `suite-after.log`, `suite-default.log`
- commit message: `commit-msg.J86jWM`
- EKR 0.0.34 source copies: `ekr-integrate-0.0.34.rs`, `extraction-0.0.34.rs`, `sdk-extraction-0.0.34.rs`, `cli-extraction-0.0.34.rs`

The tests' own temporary directories under `~/.cache/claude-tmp/cortex-e2e*` are removed by `tempfile`.

```findings
- file: src/run.rs
  line: 897
  category: concurrency
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: apply_records saves seen with the previous ended_listing, so a run that applies records and fails or stops before end_dropped leaves a fingerprint that makes the next idle run skip ending a record no longer listed
- file: src/run.rs
  line: 851
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: runs under dropped Keep write values but never clear ended_listing, so the first Supersede run with the remembered listing ends nothing
- file: src/known.rs
  line: 265
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: Index::applied treats a store node's canonical name as an alias, though EKR resolves by aliases only, so a node EKR creates beside it is missing from the next batch's store hits
- file: src/known.rs
  line: 227
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: Index::applied drops the named things of a facts[i] rejected at commit, whose nodes EKR created before the commit, so the next batch is not shown them
```
