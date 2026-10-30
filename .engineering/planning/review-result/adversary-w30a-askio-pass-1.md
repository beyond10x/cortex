---
format: aep.planning-md/3
id: review-result:adversary-w30a-askio-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: unit askio (wave 20261030a)'
relations:
- reviews: story:ask-records-deadline
- reviews: story:eval-proxy-bounds
revision: 1
---
unit: askio of wave 20261030a, branch unit/w30a-askio. Findings cover 6723ca08 plus my commit 4aca30d1.
verdict: NEEDS-CHANGE
cases: executed 9→11, red 2
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 paths
needs-coordinator: none

**Verdict: two findings, both warnings, one root cause.** Neither blocks the unit. The stall it was built for is handled correctly. The defect shows up under load: the deadline starts when a call enters, so time spent queueing for the records mutex counts against it. When a call's deadline runs out, it kills the records process even if that process is healthy. Before this unit, nothing ever killed it.

1. `git --no-pager diff --stat 6723ca08 HEAD`: `tests/askio_adv_w30a.rs | 188 +++`, 1 file. It is a test file only, and no implementation file was touched.

2. **Cases added** in `tests/askio_adv_w30a.rs`. Both are red now and were red on 3 of 3 runs alone.
   - `a_healthy_records_process_is_not_stopped_because_a_caller_queued_behind_others`, warning. Each statement takes 0.6 s against a 1 s deadline. Two calls go at once, then one more after them; the test expects one launch. Red output:
     `assertion left == right failed: a records process that answered each statement in 0.6 s (deadline 1s) was stopped and started again; ... left: 2 right: 1`
   - `a_stall_costs_one_restart_not_one_per_caller_queued_behind_it`, warning. One stalled call, with two callers arriving 0.3 s and 0.4 s into it. A new start takes 0.5 s, standing in for a Connectors launch plus migration. The test expects 2 launches. Red output:
     `assertion left == right failed: one stall, 4 launches; ... the queued callers: [... "did not start within 1s", ... "did not start within 1s"] left: 4 right: 2`

3. **Targeted run**, made after the cases existed:
   - Command: `cargo test --no-fail-fast --test answer_records_deadline --test ask_redaction_security --test askio_adv_w30a`
   - Result: 3 ok, 6 ok, then askio_adv_w30a `FAILED. 0 passed; 2 failed`, EXIT=101.
   - The 9 "before" cases are the unit's two files in this same run, with mine excluded.

4. **Findings**

| file:line | finding | verdict | origin | what reaches it |
|---|---|---|---|---|
| `src/ask/records/postgres.rs:609`, `:636` | The deadline is set when the call enters, so a queued caller can have most of it spent before its statement is even sent. A timeout then stops a healthy process, along with its statement in flight. If that statement was a committed write, it is reported as `store-failed`. | NEEDS-CHANGE | introduced | Concurrent record calls from `serve.rs` (`keeper.record`, vote, `open_reviews`, close, run through `blocking`) while the database is slow but not stalled. |
| `src/ask/records/postgres.rs:626` | A start begun with what is left of a queued caller's deadline gets killed half-way. Each caller queued behind one stall then kills the start the one before it began: one stall costs one Connectors launch plus migration per queued caller. | NEEDS-CHANGE | introduced | Traffic that arrives during a 30 s stall, then queues on the mutex. |

   Fix, which I did not apply: time the exchange from when the request is sent, and fail a call whose deadline ran out while it queued without touching the process. Let a start outlive the caller that began it, keeping the process for the next call. Origin is `introduced` because at base 3c7066a4 nothing kills the process; `Launch.deadline` does not exist there.

5. **Attacked and could not break**
   - **Late reply after a restart:** channels belong to one process, so a line from a killed child cannot reach a later call. No call read an answer from launch 1.
   - **Thread leak:** 10 stalled calls left the thread count unchanged (at most +2).
   - **Large messages:** a 300 KB reply and a 300 KB request both pass.
   - **Partial line, then stall:** the call fails within the deadline, and the next call is answered by a new process.
   - **Head cap counted with line ends:** a head of many short lines past 64 KiB gets a 431.
   - **Over-cap header sent whole:** 20 of 20 clients sending a 128 KiB header line read the 431, not a reset.
   - **Exact boundary:** a head of exactly 64 KiB followed by a body gets a 200; one byte more gets a 431.
   - **Slow client:** a client sending one byte per 100 ms holds its thread only until the case ends; `serve` returns within 3 s after that. Each read has a 30 s timeout, so within a case it holds the thread indefinitely, as it already did at base.
   - **431 before the bearer check:** the response has a fixed status, `content-length: 0`, and nothing else in it.
   - **Clients closing mid-response:** 200 of them leave `open_connections() == 0`.
   - **Orphaned grandchild:** Connectors runs the consumer with `PR_SET_PDEATHSIG` (SIGTERM), and `cortex ask records` installs no SIGTERM handler, so killing Connectors ends the records process too.

6. **Written outside the worktree** (all in the assigned scratch directory):
   - `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/askio/build.log`
   - `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/askio/suite.log`
   - `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/askio/commit-msg.nXl5R8`

   The build went into the worktree's own `target/`.

Commit 4aca30d1 is authored and committed by `b10x-bot[bot]` and was scanned clean. The cases are left failing, not ignored: they are in a new file and block no other lane. I released my own lease (`adversary-w30a-askio`).

```findings
- file: src/ask/records/postgres.rs
  line: 636
  category: concurrency
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a call whose deadline was spent waiting for the records mutex stops a healthy records process on timeout (test a_healthy_records_process_is_not_stopped_because_a_caller_queued_behind_others, 2 launches where 1 is expected)
- file: src/ask/records/postgres.rs
  line: 626
  category: concurrency
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: callers queued behind one stall each begin a start with the rest of their deadline and kill it half-way, so one stall costs one launch per queued caller (test a_stall_costs_one_restart_not_one_per_caller_queued_behind_it, 4 launches where 2 are expected)
```
