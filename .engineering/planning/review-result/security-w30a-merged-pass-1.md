---
format: aep.planning-md/3
id: review-result:security-w30a-merged-pass-1
kind: review-result
status: active
title: 'Security review pass 1: merged units conn, ask, redact, cli (wave 20261030a)'
relations:
- reviews: story:ask-service-bounds
- reviews: story:ask-outcome-truthful
- reviews: story:redaction-patterns-hold
- reviews: story:mask-quoted-credentials
- reviews: story:durable-home-writes
- reviews: story:connectors-invoke-0-60
revision: 1
---
```
unit: cortex wave 20261030a, merged units, at 3ac47068; my cases committed as 246a7515 on unit/w30a-sec
verdict: NEEDS-CHANGE
cases: executed 58→60, red 2
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 paths
needs-coordinator: whether finding 2 blocks the wave, given the coordinator chose "a reader who leaves frees its slot at once"
```

**1. Diff stat (3ac47068..246a7515)**
```
 tests/w30a_security.rs | 330 ++++++++
 1 file changed, 330 insertions(+)
```
It is a test file only. No implementation, `.engineering/` or spec file was touched.

**2. Cases added** (`tests/w30a_security.rs`; both red now; output from the first run with only this file)

- **`a_gateway_failure_reaches_neither_the_client_nor_the_log_with_a_credential_in_it`**
  - A stand-in gateway answers 401 with an `sk-…` API key (a shape the always-on mask covers) in its body.
  - The client's error event is clean. The service's log line is not:
    ```
    panicked at tests/w30a_security.rs:285:5:
    the service's log carries the gateway's key unmasked: cortex: ask: request 01a18ca2-…: the gateway answered 401 Unauthorized: {"error":{"message":"Incorrect API key provided: sk-Qx7Qx7Qx7Qx7Qx7Qx7Qx7Qx7Qx7"}}
    ```
- **`readers_of_departed_answers_stay_within_max_in_flight`**
  - Setup: `max_in_flight: 1` and a gateway that takes 6 s to answer.
  - Three identities each read the stream until "asking the model", then disconnect.
  - Result:
    ```
    panicked at tests/w30a_security.rs:324:5:
    max_in_flight 1, yet 3 readers run ([70972, 71014, 71035]) and the gateway was asked 3 times at once
    ```

**3. Suite run** (after the cases existed)

`cargo test --test ask_adversary --test ask_service --test answer_records --test credential_mask --test w30a_security`, exit 101:
```
answer_records   ok. 9 passed
ask_adversary    ok. 22 passed
ask_service      ok. 10 passed
credential_mask  ok. 17 passed
w30a_security    FAILED. 0 passed; 2 failed  (:285, :324)
```
The "before" count of 58 is the same run without `w30a_security`. I don't have the implementing state's own `cases:` line.

**4. Findings**

| # | file:line | severity | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|---|---|
| 1 | `src/ask/serve.rs:296` | warning | NEEDS-CHANGE | introduced | `eprintln!("cortex: ask: request {answer_id}: {detail}")` writes the gateway's error body unmasked; test fails at :285 | Any gateway error body that echoes a key. The text comes from `model.rs` `the gateway answered {status}: {shown}` |
| 2 | `src/ask/serve.rs:190` (`Events::left`) | blocker | NEEDS-CHANGE | introduced | The slot is freed while the answer's thread keeps its `ekr mcp` reader and gateway call until the call returns (up to `max_seconds`); test fails at :324 | Any client that disconnects during a model call |

- **Finding 1, origin.** I read the base without running it. At a870a628, `mod.rs:386` sent the same unmasked text to the HTTP client. This wave moved it from the client to a new log line, which is why I call it introduced.
  - Suggested fix: pass `detail` through `crate::mask::mask` before logging, as `records.rs:1163` already does.
- **Finding 2, why blocker.** The identity header is whatever the client sends, so the per-identity minute limit bounds nothing across identities. Disconnecting is enough to get past the cap that R1-03 was meant to add. It also contradicts `website/docs/ask-service.md:80`: "At most `max_in_flight` readers run at once".
  - Suggested fix (one of): kill the reader when `left()` fires and keep a separate bound on model calls still running; or hold the permit until the blocking thread ends, and drop the "frees the slot at once" decision.

**5. Reviewed and could not fault**
- The mask changes: separator, `KEY_VALUE`, cut-key rule, `already_masked`, `url-password`. No regression against base beyond the five gaps already filed.
- Redaction: `link`'s `s.get(..4)` fix, phone `/`, brackets in a URL body.
- `store_failed` and the fixed texts for `model-failed`/`reader-failed`. The record stores a fixed reason, and question, answer and arguments are masked.
- The 64 KiB route-level body limit overrides the outer 1 MiB layer. The 4 MiB gateway body cap holds (`take(MAX+1)`). The `max_tool_calls` and `max_in_flight` clamps hold.
- Identity is checked before the body on vote and close. The bearer token goes only in the gateway request header and appears in no error text.
- `write_atomic`: sync, rename, directory sync. File modes are the same 0644 as base; no file it writes is credential-bearing by design.
- `create`/`adopt` rollback deletes only a directory the same command just created. Adopt copies the store rather than moving it.
- `bounds_refusal` covers every path into `refresh_after_days`, hook `timeout_s` and `view_port`. A frozen copy is saturated or clamped downstream (`state.rs:195`, `sources.rs:113`, `status.rs:511`), and the `u16::try_from` casts are in place.
- Unit names: `is_name` admits only `[a-z0-9-]`, so a name cannot inject into a unit name. The collision check covers source and viewer units and runs in create, adopt and update. The cross-home case is already covered by an ignored test in `w30a_cli_adversary.rs`.
- Connectors: input goes on stdin through a writer thread, and argv carries only target, operation and connection. A refusal names only code and stage, never stdout/stderr text or the input.

**6. Paths written outside the worktree**
- `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/sec/stories.txt`
- `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/sec/run1.log`
- `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/sec/suite.log`

The commit-message temp file was deleted after the commit. Build output is in the worktree's own `target/`. My session lease (`w30a-sec-review`) is released.

**7. Findings block**
```findings
- file: src/ask/serve.rs
  line: 296
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the new request-id log line writes a model or reader failure's detail unmasked, so a credential echoed in a gateway's error body reaches the service log (tests/w30a_security.rs:285)
- file: src/ask/serve.rs
  line: 190
  category: contract-drift
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Events::left frees the max_in_flight slot while the departed answer keeps its ekr reader and gateway call up to max_seconds, so clients that disconnect exceed the documented cap on readers and answers in flight (tests/w30a_security.rs:324)
```
