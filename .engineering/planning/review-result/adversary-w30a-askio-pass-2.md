---
format: aep.planning-md/3
id: review-result:adversary-w30a-askio-pass-2
kind: review-result
status: active
title: 'Adversary pass 2: unit askio (wave 20261030a)'
relations:
- reviews: story:ask-records-deadline
- reviews: story:eval-proxy-bounds
revision: 1
---
unit: askio of wave 20261030a, branch unit/w30a-askio. Findings cover correction 83db1afb plus my commit 07aa2dd8.
verdict: NEEDS-CHANGE
cases: executed 11→12, red 1
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 paths
needs-coordinator: none

**The correction fixes both pass 1 findings but leaves one limit unenforced. A call's wait for the calls before it is never timed. It is only measured once the call holds the lock, so behind a start followed by a stall a call waits about twice the deadline.** The documents say it waits at most 30 s. The "about 3 × 30 s" worst case holds when the queue is first-come-first-served, but Rust's standard `Mutex` does not guarantee that order.

1. `git --no-pager diff --stat 83db1afb HEAD`: `tests/askio_adv_w30a_pass2.rs | 114 +++`, 1 file. Test file only.

2. **Case added** in `tests/askio_adv_w30a_pass2.rs`, red now.
   - `a_call_behind_a_start_and_a_stall_waits_at_most_one_deadline_for_it`: the deadline is 1 s. A stall first leaves no records process. Call A then starts one, which takes 0.9 s, and stalls on its statement. Call C arrives 0.1 s after A. The case expects C to end within the deadline plus 0.4 s. Red output when run alone:
     `a call that arrived 0.1 s after the one before it waited 1.803646051s for it, past the deadline of 1s for the calls before it; it answered Err("ask.records: the record call waited 1s behind earlier ones")`, EXIT=101.
   - I wrote a second case and dropped it before committing; it is in no commit. A caller making statements back to back, while C was already waiting, should not get ahead of C. It got ahead in 3 of 9 runs: C failed after 2.30 s, where a first-come-first-served queue answers C at about 1.6 s. It never went past 3 deadlines, and a case that fails a third of the time is not one I can commit.

3. **Targeted run**, made after the case existed:
   - Command: `cargo test --no-fail-fast --test answer_records_deadline --test ask_redaction_security --test askio_adv_w30a --test askio_adv_w30a_pass2`
   - Result: 3 ok, 6 ok, 2 ok (pass 1's cases are now green), then `askio_adv_w30a_pass2` 0 passed, 1 failed. EXIT=101.
   - The "before" count of 11 is the first three files in this same run, with my file excluded.

4. **Findings** (cover 07aa2dd8)

| file:line | finding | verdict | origin | what reaches it |
|---|---|---|---|---|
| `src/ask/records/postgres.rs:614-622` | `lock()` has no timeout, and the queue check runs only after the lock is held. A queued call therefore waits as long as the holder keeps the lock: up to a full start plus a full exchange, about 60 s. Two documents promise 30 s: `spec/domains/ask.yaml:113`, "waits at most 30 seconds for the calls before it", and `website/docs/ask-service.md:254`. | NEEDS-CHANGE | introduced | A database stall while `serve.rs` handles several requests. The stalled statement kills the process. The next caller's start then connects to the same stalled database, and callers queued behind it fail only after about 60 s. |
| `src/ask/records/postgres.rs:614` | The queue does not serve calls in arrival order. A thread that releases the lock and asks again at once can get ahead of a call that is already waiting. That call then fails with "waited behind earlier ones", although only one call was ahead of it when it arrived. Measured in 3 of 9 runs, not committed (flaky). | CONFIRMED | introduced | `Keeper::vote` makes several record calls back to back: `store.answer`, the vote write, then `open_review` at `src/ask/records.rs:1077`. A burst of concurrent requests does the same. |
| `src/ask/records.rs:1077` | A down vote whose write succeeded can still fail on `open_review` if that call times out in the queue. The client gets 500 `store-failed`, but the vote is kept and no `voted-down` review item exists. I found this by reading the code and did not run it. | CONFIRMED | introduced | A down vote while record calls are queueing behind a slow but healthy database. |

   Fixes, which I did not apply:
   - Bound the queue wait itself, for example with a timed and fair lock such as `parking_lot::Mutex::try_lock_for(queued_until - now)`. `parking_lot` is already in `Cargo.lock`, but I did not check whether it is a direct dependency.
   - Or change the two documents to say a call fails once the calls before it end, which can be a start and an exchange later.

   Origin is `introduced` because at base 3c7066a4 there is no deadline at all.

5. **Attacked and could not break**
   - **Waiting time counted twice:** it isn't. The start and the exchange each get a fresh `Instant::now() + deadline`.
   - **Several restarts in one call:** not possible; a call makes at most one `spawn`.
   - **A start that stalls forever:** the wait for the ready line has a deadline, and dropping the process kills and reaps it. The not-ready path waits in `ended_by` until the same deadline.
   - **A start that outlives its caller:** this cannot happen any more. The start runs inside the caller while it holds the lock, so no start is left without a caller and no process leaks. I found this by reading the code.
   - **A call that times out in the queue:** it returns before sending anything and leaves the process alone. It surfaces as `store-failed`, the route mapping at `src/ask/serve.rs:386`.
   - **Exceeding 3 deadlines:** with a first-come-first-served queue the arithmetic stays under 90 s, and I measured nothing over 3 deadlines.
   - **Pass 1's cases:** no lines changed since 4aca30d1 (`git diff 4aca30d1 HEAD -- tests/` is empty), and both are green now. One side effect: in `a_stall_costs_one_restart_not_one_per_caller_queued_behind_it`, the third caller now fails in the queue. The case does not assert on that caller, so it passes anyway.
   - **Unit tests:** `answer_records_deadline` passes unchanged, which matches the correction's claim.

6. **Written outside the worktree** (all in the assigned scratch directory). The build went into the worktree's own `target/`.
   - `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/askio/pass2-build.log`
   - `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/askio/pass2-suite.log`
   - `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/askio/commit-msg-pass2.aHym4g`

Commit 07aa2dd8 has `b10x-bot[bot]` as both author and committer and was scanned clean. I released my lease (`adversary2-w30a-askio`).

```findings
- file: src/ask/records/postgres.rs
  line: 614
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the wait for earlier calls is an untimed lock() checked only afterwards, so a call queued behind a start and a stall waits 1.8x the deadline the spec and ask-service page promise as its limit (test a_call_behind_a_start_and_a_stall_waits_at_most_one_deadline_for_it)
- file: src/ask/records/postgres.rs
  line: 614
  category: concurrency
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the records mutex does not serve calls in arrival order, so a caller issuing back-to-back statements gets ahead of a waiting call and makes it fail in the queue (3 of 9 runs, not committed as a case)
- file: src/ask/records.rs
  line: 1077
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a down vote whose write committed can fail on open_review in the queue, answering store-failed while the vote is kept without its voted-down review item (read, not run)
```
