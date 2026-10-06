---
format: aep.planning-md/3
id: review-result:adversary-quality-judge-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: quality-judge'
relations:
- reviews: story:quality-judge
revision: 1
---
unit: story:quality-judge (wave 20261006g, unit a). Findings cover the uncommitted working tree at ~/.local/state/worktree/trees/b10x/cortex/cortex-w14-a, base 750e084
verdict: NEEDS-CHANGE
cases: executed 7→10 (`--test quality`), red 2; lib 152→152
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 9 paths under ~/.cache/cortex-wave-20261006g/a/ (part 6)
needs-coordinator: `AGENTS.md` changed at 11:34 during this pass (scenario count 46→50). I did not make that change; it matches the implementor's `agents-md-scenario-count.patch`.

**1. Diff stat**

`git --no-pager diff --stat` shows the unit's 129 tracked files, the same set as at the start plus `AGENTS.md` (see above). The only path I touched is `tests/quality.rs`. It is untracked, so it does not show in `--stat`; it grew from 270 to 431 lines. I changed no non-test path. The mutation probe ran on a copy in scratch.

**2. Cases added (all in `tests/quality.rs`)**

| Case | Asserts | Now |
|---|---|---|
| `a_rare_name_a_fact_and_its_evidence_both_hold_reaches_the_judge_masked_as_it_reached_the_run` (:376) | With `RareName` and `rare_limit: 1`, the run's model sees the name as `[Name-…]`, and the judge's prompt must not hold the name either | **red** |
| `a_judge_failing_mid_measurement_states_everything_spent_in_all` (:342) | 25 facts; the 2nd call fails at 0.02 USD; the reason must say `cost 0.0400 USD in all` | **red** |
| `unclear_and_missing_verdicts_fail_and_an_injected_or_repeated_one_changes_nothing` (:277) | An `unclear`, a missing verdict, a repeated verdict and an injected one give passed 3 of 6, unclear 2, rate 0.5, and no line for the injected id | green; red on the mutant |

Red output of each case run alone. The first is `~/.cache/cortex-wave-20261006g/a/adv1-red-rare.log` (the run's own assertion passed first, so the run did mask the name):
```
panicked at tests/quality.rs:329:5:
a name the run's model was shown as a placeholder reaches the judge in the clear:
...
The engine is kept by Kowalczyk today.

=== Fact 01a11093-82d6-7737-8bbd-373270d1876e ===
Property (Node): Kowalczyk — role — keeper
```
The second is `adv1-red-cost.log`:
```
panicked at tests/quality.rs:349:5:
two calls of 0.02 USD were made: claude answered an error (error_max_budget_usd), cost 0.0200 USD; 20 of 25 facts judged before it, cost 0.0200 USD in all
```
Mutant probe: in a scratch copy I changed `src/quality.rs:502` from `== Yes` to `!= No`, so `unclear` counts as `Pass`. On that copy the 7 original `--test quality` cases and the 5 lib `quality::` cases all stayed green. The new case went red: `left: (6, 5, 0.8333) right: (6, 3, 0.5)`.

**3. Suite run (after the cases existed)**

- `cargo test -p cortex-cli --test quality` → `test result: FAILED. 8 passed; 2 failed`, `EXIT=101`. The two failures are the red cases above.
- `cargo test -p cortex-cli --lib` → `test result: ok. 152 passed`, `EXIT=0`.
- `--test quality -- --skip a_rare_name --skip unclear_and_missing --skip a_judge_failing_mid` → `7 passed; 3 filtered out`, `EXIT=0`. This is where the "before" count of 7 comes from.

**4. Findings**

| # | file:line | Severity | Verdict / origin | What was measured | What reaches it |
|---|---|---|---|---|---|
| F1 | src/quality.rs:460 (also :283) | blocker | NEEDS-CHANGE / introduced | Fact statements go through `p.reserve`, so the entity names in them count toward `RareName` rarity. `src/redact.rs`'s module doc says known entity names are not counted, and the run's `pseudonymise` uses `reserve_name` for them. A name that appears once in the evidence and once in its fact counts 2, which is above `rare_limit` 1, so it reaches the judge unmasked. The same miscount hides it from `refuse_if_left: [RareName]`. | Any instance with `RareName` whose facts are about a person named in their evidence, which is the normal case for that class. The fix would be `reserve_name` for statements (and for file-record context, as the run does). |
| F2 | src/quality.rs:444 | warning | NEEDS-CHANGE / introduced | `judged_before()` reads `cost_usd` before the failed answer is added. "cost X USD in all" leaves out the failed call, which contradicts the docs' "the reason states the cost so far". | Any judge call that fails after spending money, for example `error_max_budget_usd`. |
| F3 | src/quality.rs:502 | warning | CONFIRMED / introduced | No existing case produces `unclear` end to end, and `detail.unclear` is never asserted, so the `unclear`→`Pass` mutant survives. The code itself is correct; the gap is in the tests. | Covered by the new green case :277. |
| F4 | src/main.rs:247 | note | CONFIRMED / introduced | `quality` takes the blocking home-wide lock for the whole measurement: up to 50 batches × `timeout_s`. That is read only, but every instance's scheduled runs wait behind it. | systemd timer runs during a measurement. This is a judgement; I wrote no test for it. |

On the question about placeholders in `verdicts.jsonl`: yes, restored reasons hold values the policy had masked. This matches the run's restored `extraction.yaml`, and the store already holds those originals. Irreversible matches (Credential, and rules with a `replacement`) never get placeholders, so they are never restored. I read this as intended.

**5. Attacked and could not break**

- An injected verdict for a fact outside the batch is dropped, and a repeated one keeps its first answer (covered by :277).
- `refuse_if_left: [Email]` with the email still in the evidence gives `judge-failed`, with no call, no directory, and the class named but not the value. I probed this and deleted the green probe.
- A store with 0 facts gives measured, rate null, interval [0, 1], no call. A removed instance is measured. Both probes were green and deleted.
- A sample larger than the store works (existing case `isolated`).
- A credential split across facts: I could not build a case that anybody reaches. Each evidence text is masked on its own, the same as in the run.
- Stamp collisions: `create_dir` is atomic and `AlreadyExists` moves on to the `-N` suffix. Checked by reading only.
- The locator is masked with text-credential shapes, which mask more than the key-credential shapes, not less.

**6. Paths written outside the worktree** (all under ~/.cache/cortex-wave-20261006g/a/)

- adv1-red-rare.log
- adv1-red-cost.log
- adv1-probe2.log
- adv1-probe3.log
- adv1-probe4.log
- adv1-suite-quality.log
- adv1-suite-lib.log
- adv1-suite-quality-deselected.log
- adv1-mutant/ (249M: tree copy, target/, mutant.diff, mutant-quality.log, mutant-lib.log)

I also built into the assigned ~/.cache/b10x-target/cortex-w14-a and used the existing tmp/ as TMPDIR. The session lease `adversary-w20261006g-a-p1` is released.

**7. Findings block**

```findings
[
  {"file": "src/quality.rs", "line": 460, "category": "acceptance", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "fact statements are reserved with reserve() instead of reserve_name(), so an entity name counts toward RareName rarity and a name the run masks reaches the judge in the clear (and escapes refuse_if_left: [RareName])"},
  {"file": "src/quality.rs", "line": 444, "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "a failed judge call's reason states 'cost X USD in all' without the failed call's own cost, so the total spent is under-reported"},
  {"file": "src/quality.rs", "line": 502, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "mapping unclear to Pass survives the unit's suite; no case produced an unclear verdict end to end or asserted detail.unclear"},
  {"file": "src/main.rs", "line": 247, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the read-only quality command holds the blocking home-wide lock for the whole measurement, so every instance's scheduled runs wait behind it"}
]
```