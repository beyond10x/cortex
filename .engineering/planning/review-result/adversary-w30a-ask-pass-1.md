---
format: aep.planning-md/3
id: review-result:adversary-w30a-ask-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: unit ask (wave 20261030a)'
relations:
- reviews: story:ask-outcome-truthful
- reviews: story:ask-service-bounds
- reviews: story:ask-error-text
revision: 1
---
verdict: fail
unit: ask, wave 20261030a, on unit/w30a-ask at 4cf3f098 (my cases are commit 76100f38)
verdict: CONFIRMED / NEEDS-CHANGE (1 blocks merge, 2 follow-up)
cases: executed 19→22, red 3
origin: introduced 2 / pre-existing 0 / undecided 1
wrote-outside-worktree: 12 paths, all in the scratch directory (part 6)
needs-coordinator: decide finding 1: change the code, or change the decision and the three documents that repeat it

**1. Diff stat.** `git --no-pager diff --stat 4cf3f098..HEAD`:
```
 tests/ask_adversary.rs | 142 +++++++++++++++++++++++++++++++++++++++++++++++++
 1 file changed, 142 insertions(+)
```
Only a test file changed. I committed it through `b10x-gates bot`; author and committer are both `b10x-bot[bot]`. Both files passed scan-text first.

**2. Cases added.** All three are in `~/.local/state/worktree/trees/b10x/cortex/cortex-w30a-ask/tests/ask_adversary.rs` and marked `#[ignore = "adversary: …"]`. Each is red now. Each was run alone with `cargo test --test ask_adversary -- --ignored --exact <name>`:

- **`adversary_a_client_closed_answer_past_slow_after_ms_opens_no_review_item`** (blocks merge). The story's decision says "A `client-closed` answer … opens no review item". So do the `ClientClosed` summary in the spec, the site and the CHANGELOG. But `src/ask/records.rs:1013` applies the `slow` check to every outcome.
  `a client-closed answer opened review items ["slow"]` / `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 26 filtered out; finished in 5.38s`
  - What reaches it: any spec that sets `ask.slow_after_ms`, plus a reader who leaves after that time. That is the usual case: people leave slow answers.
  - Fix: skip the `slow` item for `ClientClosed`, or change the decision and the three documents.
- **`adversary_a_departed_reader_frees_its_in_flight_slot_at_once`** (follow-up). Setup: `max_in_flight: 1`, the stand-in gateway answers after 5 s, and ann leaves during the model call. Ben then gets `busy`.
  `assertion left != right failed: ann left; her abandoned answer still holds the only slot: [("error", … "reason": String("busy"))]  left: 429 right: 429` / `test result: FAILED. 0 passed; 1 failed; … finished in 0.92s`
  - The slot is held by the blocking gateway call (`src/ask/mod.rs:607`, permit at `src/ask/serve.rs:249`) until the gateway answers or `max_seconds` passes. The spec's `ClientClosed` summary and the site say the answer stops "at once".
  - This is not a permanent leak (see part 5). But with the default of 8, eight abandoned questions refuse everyone for up to `max_seconds`.
  - Fix: check whether the request is closed while waiting on the model call (for example, run the call off-thread and wait on `tx.closed()`), or soften the "at once" wording.
- **`adversary_a_reader_gone_during_a_model_call_makes_no_tool_call`** (follow-up). If the reply has a tool call and `content: null`, the loop runs `reader.call` (`src/ask/mod.rs:694`) before its first `emit` (`:702`). So the tool call runs for a reader who has already gone.
  `a tool call was made for a reader who had left: {"id":3,"jsonrpc":"2.0","method":"tools/call",…}` / `test result: FAILED. 0 passed; 1 failed; … finished in 4.42s`
  - The recorded steps and tokens (1, 10) are right.
  - Origin is undecided: I did not run the case against the base. `git show 7a217da7:src/ask/mod.rs:554-564` has the same order.

**3. Suite run, after the cases existed.** Command: `cargo test --test ask_adversary` with `CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=4 nice -n 19`.
- Plain run: `test result: ok. 19 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 8.66s`, EXIT=0.
- With `-- --include-ignored`: `test result: FAILED. 19 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.99s`, EXIT=101.
- `cargo clippy --test ask_adversary -- -D warnings` exits 0. The fmt run (`-p cortex-cli -p cortex-docs`) changed only my file.

**4. Findings** (they cover 4cf3f098):

| file:line | verdict | origin | what reaches it |
|---|---|---|---|
| src/ask/records.rs:1013 | NEEDS-CHANGE | introduced | `slow_after_ms` set and a reader leaving after it |
| src/ask/serve.rs:249 | CONFIRMED | introduced | `max_in_flight` (default 8) and any reader leaving during a model call |
| src/ask/mod.rs:694 | CONFIRMED | undecided | a reply with a tool call and no thinking text, and its reader gone |

Things I could not turn into a failing case:
- A failed tool's error text from `ekr mcp` still goes to the model, which can quote it into the answer. Not measured; it depends on what `ekr` puts in its errors.
- Other `finish_reason` values that also mean a cut-off answer (for example `model_length`) are recorded `answered`. The decision limits cut-off to `length` and `content_filter`, so this is a note only.

**5. Attacked and could not break.** I ran these as probes, each green alone. The patch with all of them is in `all-cases-with-probes.patch` in scratch:
- **Slot counting:** a slot is freed after `model-failed` and after `client-closed`. The permit is dropped exactly once: when the blocking closure ends, including on panic. Busy and rate-limit refusals do not hold one. No permanent leak.
- **Over-cap tool calls:** with 5 calls and a cap of 2, all 5 ids (call-1 to call-5) get exactly one tool message each.
- **Vote and close cap:** a chunked body of about 80 KiB to vote or close gets 413.
- **Gateway cap:** an endless gateway body, chunked or with no length, fails `model-failed` in about 3 s, and less than 32 MiB is taken from the socket.
- **Cut-off after a streamed step:** a cut-off on step 2, after step 1 streamed its thinking, records steps 2 and tokens (20, 2, context 10).
- **Leaks:** every `json_error`, `bad_request` and `final_event` text is fixed or comes from the caller. `store_failed` and the `blocking` join error carry only an id.
- **Older reader:** an older cortex's `answer_row` fails only that record's lookups (a vote on it, an eval case built from it), not whole listings. That matches the CHANGELOG sentence.
- **Redaction compiled once:** confirmed by reading the code. `Settings::of` compiles it (`src/ask/mod.rs:250`), `ask_serve` calls that once (`src/main.rs:2840`), and `question_refused` and `answer` only take batches from `settings.redactor`. Built-in patterns are `OnceLock`s. There is no counter seam, so no test proves it.

**6. Paths written outside the worktree**, all under `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/ask/`:
- `build.log`, `suite.log`, `suite-ignored.log`, `clippy.log`
- `commit-msg.K0wZWj`, `all-cases-with-probes.patch`
- the three red-case logs: `adversary_a_*.log`
- the five probe logs: `adversary_probe_*.log`

The session lease `w30a-ask-adversary` was taken and released.

```findings
- file: src/ask/records.rs
  line: 1013
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a client-closed answer past slow_after_ms opens a `slow` review item, while the story decision, spec ClientClosed summary, site and CHANGELOG say it opens none
- file: src/ask/serve.rs
  line: 249
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a reader who leaves during a model call holds its max_in_flight slot until the gateway answers, so the next question is refused busy although the spec says the answer stops at once
- file: src/ask/mod.rs
  line: 694
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: a tool call is made against the store for a reader who already left when the reply carries no thinking text, contrary to the site's "stops the answer at once"
```
