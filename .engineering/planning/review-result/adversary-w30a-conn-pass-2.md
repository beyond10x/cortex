---
format: aep.planning-md/3
id: review-result:adversary-w30a-conn-pass-2
kind: review-result
status: active
title: 'Adversary pass 2: unit conn (wave 20261030a)'
relations:
- reviews: story:connectors-invoke-0-60
revision: 1
---
unit: conn (story:connectors-invoke-0-60), correction f611828e, findings cover the integration head d63686d1 on unit/w30a-p2 (adversary commit cb5c03fd adds no conn case)
verdict: nothing found
cases: executed 5→5, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 paths shared by the whole pass (part 6)
needs-coordinator: none

**1. Diff stat** (`git --no-pager diff --stat d63686d1 HEAD`; the commit is shared by all seven units)
```
 tests/w30a_p2_cli.rs    | 105 ++++++++++++++++++++++++++++++
 tests/w30a_p2_cost.rs   | 170 ++++++++++++++++++++++++++++++++++++++++++++++++
 tests/w30a_p2_fail.rs   |  88 +++++++++++++++++++++++++
 tests/w30a_p2_read.rs   | 129 ++++++++++++++++++++++++++++++++++++
 tests/w30a_p2_redact.rs |  46 +++++++++++++
 5 files changed, 538 insertions(+)
```
All test files. Commit cb5c03fd, author and committer `b10x-bot[bot]`; scan-text passed on each file and the message.

**2. Cases added for conn:** none. I found no defect to write a case for.

**3. Suite run** (after the pass's cases existed): `cargo test --no-fail-fast --test connectors_invoke_0_60 --test connectors_invoke_0_60_adv …` (one run with the other units' targets; log `w30a/p2/suite.log`):
- `connectors_invoke_0_60`: `test result: ok. 4 passed; 0 failed; 0 ignored`
- `connectors_invoke_0_60_adv`: `test result: ok. 1 passed; 0 failed; 0 ignored`

**Pass-1 assertions:** the correction only removed the `#[ignore]` from `adv_a_connectors_older_than_0_60_that_lists_connector_and_instance_is_named_in_the_reason`. Its assertion is unchanged, and it now passes.

**4. Findings:** none.

**5. Attacked and could not break**
- `failure_code` falls back to `error.code` for every command (`json`, `revalidate`, `status`), not only `invoke`. A fallback code cannot become `Lapsed` or `Missing`, because both also need `data.stage == "admission"`. `retryable` still reads only `data.code`, so a `cli_parse` refusal is never retried.
- `invoked` adds the 0.60.0 wording only to a `Failed` whose code is `cli_parse`. `Lapsed`, `Missing` and the admission-timeout retry (`ends_with(TIMEOUT_AT_ADMISSION)`) keep their message shape, so the retry still matches.
- A `cli_parse` from a 0.60+ Connectors would also be named "needs 0.60.0". The only cause I found is an operation id starting with `-`, which is pass-1 finding 3 (INFEASIBLE). I am not reporting it again.
- An envelope whose `error` is a string, or which has no `error`, still reads "unknown at unknown". No panic.

**6. Paths written outside the worktree** (shared by the pass, under `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/`): `p2/build0.log`, `p2/suite.log`, `p2/commit-msg.x1If5v`, and this report. I took no worktree session lease. I only added test files and built into the worktree's own `target/`.

**7. Findings block**
```findings
[]
```
