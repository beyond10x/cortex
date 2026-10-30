---
format: aep.planning-md/3
id: review-result:adversary-w30a-cli-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: unit cli (wave 20261030a)'
relations:
- reviews: story:unit-names-unique
- reviews: story:spec-integer-bounds
- reviews: story:durable-home-writes
revision: 1
---
verdict: fail

```
unit: cli of wave 20261030a, unit/w30a-cli at 2c3fbc4e (implementation 5f302739 plus one adversary test commit)
verdict: NEEDS-CHANGE (2 block merge), CONFIRMED (3 follow-ups)
cases: executed 63→68, red 5
origin: introduced 4 / pre-existing 0 / undecided 1
wrote-outside-worktree: 3 paths (part 6)
needs-coordinator: findings 1–2 go against your Decision "bounds in spec::check"; the fix needs your call
```

**1. Diff stat** (`git --no-pager diff --stat 97573d67...HEAD -- tests/w30a_cli_adversary.rs`): `tests/w30a_cli_adversary.rs | 311 +++`. That is a test file only; I edited no implementation file. The commit is 2c3fbc4e, made by `b10x-bot[bot]` as author and committer, and scan-text passed. All five cases carry `#[ignore = "adversary: …"]`, so the default run stays green (`test result: ok. 0 passed; 0 failed; 5 ignored`).

**2. Cases.** I ran each one alone first: `CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=4 nice -n 19 cargo test --test w30a_cli_adversary -- --ignored --exact <name>`. Every run ended `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 4 filtered out`.

| # | severity | test | verbatim red line |
|---|---|---|---|
| 1 | blocks merge | `adv_w30a_an_instance_frozen_with_a_large_refresh_window_still_runs_after_the_upgrade` | `{"command":"run","detail":{"reason":".../instances/t/instance.yaml: sources.news.policy.refresh_after_days: 40000: use 0 to 36500 days"},"outcome":"fetch-failed"}` / `left: (Some(1), Some("fetch-failed"))` `right: (Some(0), Some("ran"))` |
| 2 | blocks merge | `adv_w30a_a_timer_run_failing_on_a_refused_frozen_copy_calls_the_failure_hook` | `no hook call; source state "Disabled"; runs: [ "Some(1) {...\"outcome\":\"counted\"} ", "Some(1) {...\"outcome\":\"disabled\"} " ]` |
| 3 | follow-up | `adv_w30a_the_changelog_hand_fix_puts_the_instance_back_on_its_schedule` | `after the hand fix the source stays off its timer` `left: String("Disabled")` `right: String("Enabled")` |
| 4 | follow-up | `adv_w30a_a_colliding_unit_of_a_live_instance_in_another_home_is_not_taken_over` | `"units":{"taken_over":["cortex-a-b-c.service (home .../home)","cortex-a-b-c.timer (home .../home)"]}` and `left: ... --home ".../home2" run a-b/c --record-failure` `right: ... --home ".../home" run a/b-c --record-failure` |
| 5 | follow-up | `adv_w30a_a_state_copy_that_fails_part_way_leaves_state_as_it_was` | `"state_failed":"...: zz-unwritable.json is not text","state_restored":false` and `state/ is not as it was`. `news.json` is at the snapshot's hash `63cd2778…`, not the live `a2d05cdc…` |

- **Findings 1 and 2: an upgrade breaks a running instance.**
  - **What I measured:** `src/spec.rs:218` is inside `check`, so it also reads the frozen copy. A `refresh_after_days` of 40000 never overflowed: 40000 × 86 400 000 is about 3.5e12, and i64 only overflows above about 1.07e8 days. Cortex 0.6.1 accepted and ran such a spec. After the upgrade every run ends `fetch-failed`. The failure hook reads `monitoring` from that same refused copy (`src/main.rs:2838`, `.ok()`), so it is never called. Two timer runs then disable the source with nobody alerted. `view_port` and `timeout_s` values from 0.6.1 hit the same path.
  - **What reaches it:** any 0.6.1 instance with `refresh_after_days` from 36501 to about 1e8 (for example 99999 meaning "never"), a hook `timeout_s` above 86400, or a wrapped `view_port`, running on its timer.
  - **The codebase's own rule says otherwise.** `load_given` and the test `a_frozen_spec_with_a_reserved_source_name_still_parses` keep frozen copies loading.
  - **Fix I'd suggest:** move the three bounds into `load_given`, beside `reserved_refusal`, `infinite_refusal` and `gateway_refusal`. Keep `u16::try_from` and `checked_add`, and use `saturating_mul` at `state.rs` and `sources.rs`. That turns findings 1–3 green.
- **Finding 3: the CHANGELOG hand fix is incomplete.**
  - **What I measured:** the entry (`CHANGELOG.md:125`) says the instance is refused "until the value in that file is brought into range". Doing that leaves the source disabled. The entry never mentions `cortex source enable`.
  - **What reaches it:** findings 1–2. It goes away if the bound moves; otherwise the entry needs that step added.
- **Finding 4: two homes can still take over each other's units.**
  - **What I measured:** `Systemd::foreign` (`src/schedule.rs:393`) asks the other home's registry for the requesting instance (`a-b`), finds none, and reads the live unit of `a/b-c` as orphaned.
  - **What reaches it:** two homes on one user manager, which the code supports.
  - **Origin is undecided:** `foreign` is unchanged by this diff, but I did not run the case on base. The story's acceptance says "in one home", so this is outside its scope.
- **Finding 5: a restore that fails part-way leaves `state/` half put back.**
  - **What I measured:** the unit's own `restored` summary (`spec/domains/instance.yaml:2254`, `operating.md`) says the state is left "as it was". `write_state` (`src/snapshot.rs:248`) writes one file at a time, so a failure on a later file leaves `state/` partly restored.
  - **What reaches it:** I built it with a non-text entry. The real trigger would be a disk that fills or an I/O error on the second file; I did not produce that.

**3. Suite run, after the cases existed:** `… cargo test --test w30a_cli_adversary --test spec_forms --test durable_home --test unit_homes_adversary --test snapshots --test adopt -- --include-ignored`
- `adopt` 21, `durable_home` 2, `snapshots` 14, `spec_forms` 10 and `unit_homes_adversary` 16 all passed: `test result: ok. … 0 failed`.
- `w30a_cli_adversary` ended `test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.71s`, and the run printed `EXIT=101`.

**4. Judgement findings** (they cover 2c3fbc4e; I did not run them):
- **J1** `src/main.rs:932`: the rollback removes the directory but leaves a PostgreSQL lineage that `seed_instance` already seeded. A retry then answers "tenant already holds a store", which contradicts "so it can be retried" (`operating.md:15`). INFEASIBLE here: no PostgreSQL run, and the save_meta failure is rare.
- **J2** `src/main.rs:628` and `src/status.rs:198`: one corrupt status file fails `cortex sources` for the whole home. Runs never rewrite it, so it stays stale for good. No document tells the operator to delete it.
- **J3** `src/snapshot.rs:192`: snapshot writes and renames are not synced. The CHANGELOG does not claim they are, but they are home files a restore depends on.

**5. What I attacked and could not break:**
- `cortex sources` and `list` still work when the frozen copy is refused. There is no `cortex status` command.
- The collision check refuses names with several dashes, viewers either way round, a source named `view`, and an update that adds a colliding source. Its code does not depend on `--no-units`.
- Names are limited to `[a-z0-9-]` (`spec.rs:177`), so no two names can collide by case.
- `create` and `adopt` refuse an instance directory that already exists before they write anything. So the rollback's `remove_dir_all` only ever deletes a directory the command itself created, never an adopted one.
- An empty status file and a truncated one take the same path as a corrupt one.
- `write_atomic` covers the registry, status, state, meta and stage files.
- The new message in `src/run.rs` `undo` is accurate. Note: it does not say that the run's documents stay marked as seen.

**6. Paths written outside the worktree:**
- `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/cli/build.log`
- `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/cli/suite.log`
- `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/cli/commitmsg.0VGbVl`

```findings
- file: src/spec.rs
  line: 218
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: bounds in spec::check refuse a frozen copy 0.6.1 accepted and ran without overflow, so every run of that instance fails after the upgrade
- file: src/main.rs
  line: 2838
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a timer run failing on a refused frozen copy calls no failure hook and two such runs disable the source silently
- file: CHANGELOG.md
  line: 125
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the hand fix (bring the frozen value into range) leaves the source the refused runs disabled; cortex source enable is not named
- file: src/schedule.rs
  line: 393
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: across two homes instance a-b/c takes over the live unit cortex-a-b-c of a/b-c because foreign asks the other home for the requesting instance
- file: spec/domains/instance.yaml
  line: 2254
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a state copy that fails part-way leaves state/ half restored while the restored summary says state is as it was
- file: src/main.rs
  line: 932
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: create rollback leaves a seeded PostgreSQL lineage, so the retry the docs promise answers tenant already holds a store
- file: src/main.rs
  line: 628
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: one corrupt status file fails cortex sources for the whole home and no run or document repairs it
- file: src/snapshot.rs
  line: 192
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: undecided
  message: snapshot state and store copies are written and renamed without fsync
```
