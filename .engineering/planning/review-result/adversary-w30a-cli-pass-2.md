---
format: aep.planning-md/3
id: review-result:adversary-w30a-cli-pass-2
kind: review-result
status: active
title: 'Adversary pass 2: unit cli (wave 20261030a)'
relations:
- reviews: story:unit-names-unique
- reviews: story:spec-integer-bounds
- reviews: story:durable-home-writes
revision: 1
---
unit: cli (story:unit-names-unique, story:spec-integer-bounds, story:durable-home-writes), correction f37f52e8, findings cover the integration head d63686d1; the cases are in cb5c03fd on unit/w30a-p2
verdict: NEEDS-CHANGE
cases: executed 15→17, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 paths shared by the whole pass (part 6)
needs-coordinator: whether finding 1 holds the unit (pass-1 finding 1, the same class on `cortex run`, was rated blocker)

**1. Diff stat** (`git --no-pager diff --stat d63686d1 HEAD`): five test files, `tests/w30a_p2_{cli,cost,fail,read,redact}.rs`, 538 insertions, no other path. Commit cb5c03fd, author and committer `b10x-bot[bot]`; scan-text passed.

**2. Cases** (`tests/w30a_p2_cli.rs`). Each case starts `cortex ask serve --spec`, reads its first stdout line and kills it.

Run alone first with `cargo test --test w30a_p2_cli`: `test result: FAILED. 1 passed; 1 failed`. The first two runs were not findings: one used the wrong limits field, the other a token variable cortex refuses. The control case caught both. The run quoted here is the third, after those fixes.

- **`ask_serve_starts_over_a_spec_with_a_short_refresh_window`** (the control, `refresh_after_days: 7`): green.
- **`ask_serve_starts_over_a_spec_0_6_1_served_with_a_long_refresh_window`** (`refresh_after_days: 40000`): red, not ignored.
  - Red: `no ready line; exit Some(2); stderr: cortex: ~/.cache/claude-tmp/cortex-p2-cli…/asker.yaml: …: sources.chat.policy.refresh_after_days: 40000: use 0 to 36500 days`

**3. Suite run** (after the cases existed; log `w30a/p2/suite.log`):
- `w30a_cli_adversary`: `test result: ok. 4 passed; 0 failed; 1 ignored`. The ignored case is the known cross-home follow-up.
- `spec_forms`: `test result: ok. 11 passed; 0 failed; 0 ignored`
- `w30a_p2_cli`: `test result: FAILED. 1 passed; 1 failed`
- The command ended `EXIT=101`.

**Pass-1 assertions** (`git show f37f52e8 -- tests/w30a_cli_adversary.rs`)
- **Cases 1 and 5:** unignored and unchanged.
- **Case 2 (hook):** before the correction it failed because the run never started. Now it creates a fetch failure (`fail` file) and asserts the exact hook calls `["t/news fetch-failed", "t/news disabled"]`. That is stronger than pass-1's "not empty".
- **Case 3:** rewritten. The disabled-source scenario no longer arises, so it now asserts that the timer runs succeed, that an out-of-range `update` is refused and leaves the frozen copy unchanged, and that an in-range update is made. Nothing was relaxed for a scenario that still exists.
- **Case 5:** passes through the new check-before-write pass, because a non-text entry is refused before the first write. The rollback path itself is covered by the unit test `a_state_copy_that_fails_part_way_puts_back_what_it_wrote`.

**4. Findings**

| # | file:line | verdict | origin | severity |
|---|---|---|---|---|
| 1 | src/spec.rs:67 (`bounds_refusal` in `load_given`), called by src/main.rs:2934 (`ask_serve`) | NEEDS-CHANGE | introduced | warning |

1. **`cortex ask serve` refuses a spec that 0.6.1 served.**
   - **The cause:** the correction moved the bounds into `load_given` so that frozen copies keep running. But `load_given` is also how `cortex ask serve --spec <file>` reads its spec, in 0.6.1 as now (`git show a870a628:src/main.rs`, `ask_serve` at :2825). The 0.6.1 `load_given` had no bounds.
   - **The result:** after the upgrade, an answer service over a spec with `refresh_after_days` above 36500, a hook `timeout_s` above 86400, or a `view_port` above 65535 exits 2 and does not start. The answer service reads none of those fields.
   - **What was measured:** `tests/w30a_p2_cli.rs:101`, exit 101. The control at `:90` passes.
   - **What reaches it:** the population of pass-1 finding 1, an instance 0.6.1 created with such a value, whose operator runs `cortex ask serve` over its spec. The site says `--spec <instance spec>`.
   - **Fix to name** (not applied): hold `bounds_refusal` in `create`, `update` and `adopt` only (the three `load_given` callers at `main.rs:998`, `:1384` and `:1578`), not in `load_given` itself. Or have `ask_serve` load the spec without the bounds.

**5. Attacked and could not break**
- **Saturating arithmetic:**
  - `wants`/`holds` and `Window::of_run`, after the merge with fail's `pending_since`, saturate only on the first-run branch.
  - `rfc3339(i64::MIN)` does not overflow.
  - `evidence::document_time_ms` multiplies a parsed date's days, which are bounded by the date format.
- **`call_hook`:** caps a frozen `timeout_s` at 86400. A non-positive value is still refused by `check`, which still reads frozen copies.
- **`write_state`:** checks names and text, and reads every touched path (written and removed) before the first write. On failure it puts back, in reverse order, what was written or removed, and names any put-back that also failed. A directory in place of a state file is refused before any write.
- **`update`:** refuses an out-of-range value and leaves the frozen copy unchanged (the pass-1 case 3 rewrite runs this).

**6. Paths written outside the worktree** (shared by the pass, under `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/`): `p2/build0.log`, `p2/suite.log`, `p2/commit-msg.x1If5v`, and this report. The cases' tempdirs under `~/.cache/claude-tmp/` are removed by `tempfile`. I took no worktree session lease.

**7. Findings block**
```findings
- file: src/spec.rs
  line: 67
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: bounds_refusal sits in load_given, which cortex ask serve also reads, so an answer service over a spec 0.6.1 served (refresh_after_days 40000) no longer starts
```
