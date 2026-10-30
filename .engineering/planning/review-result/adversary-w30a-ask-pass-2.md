---
format: aep.planning-md/3
id: review-result:adversary-w30a-ask-pass-2
kind: review-result
status: active
title: 'Adversary pass 2: unit ask (wave 20261030a)'
relations:
- reviews: story:ask-outcome-truthful
- reviews: story:ask-service-bounds
- reviews: story:ask-error-text
revision: 1
---
unit: ask (story:ask-outcome-truthful, story:ask-service-bounds, story:ask-error-text), corrections 7c952aa4 and 4f236df6, findings cover the integration head d63686d1 (no ask case in cb5c03fd)
verdict: CONFIRMED (note only)
cases: executed 22→22, red 0
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 paths shared by the whole pass (part 6)
needs-coordinator: confirm that inverting pass-1's slot assertion (4f236df6) is the decision of record, and whether the ClientClosed summary should say so

**1. Diff stat** (`git --no-pager diff --stat d63686d1 HEAD`): five test files, `tests/w30a_p2_{cli,cost,fail,read,redact}.rs`, 538 insertions, no other path. None of them is for ask. Commit cb5c03fd, author and committer `b10x-bot[bot]`.

**2. Cases added for ask:** none.

**3. Suite run** (log `w30a/p2/suite.log`): `ask_adversary`: `test result: ok. 22 passed; 0 failed; 0 ignored`.

**Pass-1 assertions**
- 7c952aa4 only removed the three `#[ignore]` markers.
- 4f236df6 inverted one of them. It renamed `adversary_a_departed_reader_frees_its_in_flight_slot_at_once` to `adversary_a_departed_reader_holds_its_in_flight_slot_until_its_answer_ends` and changed `assert_ne!(next.status, 429)` to `assert_eq!`. Its commit body gives the reason, a security review: a slot freed on leaving let disconnecting clients run unbounded readers and model calls.
- The new case is stronger in what it observes: the busy refusal, no model call, the tokens recorded, and the slot free afterwards. It is the opposite of the pass-1 assertion by decision, not a relaxation.

**4. Findings**

| # | file:line | verdict | origin | severity |
|---|---|---|---|---|
| 1 | spec/domains/ask.yaml:521 | CONFIRMED | introduced | note |

1. **The spec still says the answer "stops at once".** The `cortex.ask.ClientClosed` summary reads "the answer stops at once and opens no review item". Since 4f236df6 the model call runs on to its reply or `max_seconds`, and keeps the `max_in_flight` slot. The site (`website/docs/ask-service.md:126-129`) says the call is not cut; the spec summary does not.
   - What was measured: read only; no case. What reaches it: a reader of the generated reference.
   - Fix to name: one clause in the summary, such as "a model call in flight runs to its reply or the deadline and keeps the answer's slot".

**5. Attacked and could not break**
- A `client-closed` answer past `slow_after_ms` returns before both review checks (`records.rs`). A `failed` answer whose reader left during the model call is recorded `client-closed`, so it opens no review either.
- The guard against a tool call after the reader left (`stream.gone()` before each call) is in the merged loop. The reader leaving during a slow `reader.call` ends the loop at the next check; no further tool call is made.
- The permit is moved into the blocking closure (`let _permit = permit`), so no path drops it before the answer ends. Busy and rate-limit refusals take no slot.
- `log()` masks every stderr line written under `src/ask/`. The other stderr writers I found carry no credential:
  - `main.rs` `ask_serve` prints `Keeper::open` errors, which name a SQLite path or a URL that `url_refusal` keeps free of a password.
  - The PostgreSQL records process inherits the child's stderr (`records/postgres.rs:467`). Its URL is password-free by `url_refusal`.

**6. Paths written outside the worktree** (shared by the pass, under `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/`): `p2/build0.log`, `p2/suite.log`, `p2/commit-msg.x1If5v`, and this report. I took no worktree session lease.

**7. Findings block**
```findings
- file: spec/domains/ask.yaml
  line: 521
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the ClientClosed summary says the answer stops at once, while since 4f236df6 the model call runs to its reply or max_seconds and keeps the answer's max_in_flight slot
```
