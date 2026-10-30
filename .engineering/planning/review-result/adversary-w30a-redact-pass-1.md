---
format: aep.planning-md/3
id: review-result:adversary-w30a-redact-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: unit redact (wave 20261030a)'
relations:
- reviews: story:redaction-patterns-hold
- reviews: story:mask-quoted-credentials
revision: 1
---
verdict: fail
unit: redact (story:redaction-patterns-hold, story:mask-quoted-credentials); findings cover unit/w30a-redact at 3772a0d0, and my cases are commit 492ef533
cases: executed 99→100, red 12 (13 cases added: 12 ignored, 1 passing sweep)
origin: introduced 9 / pre-existing 3 / undecided 0
wrote-outside-worktree: 2 paths (part 6)
needs-coordinator: none. I took no worktree session lease (I never moved the tree or ran a store verb).

**1. Diff stat** (`git --no-pager diff --stat 3772a0d0 HEAD`)
```
 tests/redact_mask_adv_w30a.rs | 333 ++++++++++++++++++++++++++++++++++++++++++
 1 file changed, 333 insertions(+)
```
Only a test file changed. Committed as `492ef533 test: adversary cases for the redaction and credential-mask unit` through `b10x-gates bot`, after `scan-text` printed "1 file(s) carry no private rule". The author and committer are both `b10x-bot[bot]`.

**2. Cases and their red output**

Each case was run alone with `CARGO_PROFILE_DEV_DEBUG=line-tables-only CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=4 nice -n 19 cargo test -q --test redact_mask_adv_w30a -- --include-ignored --exact <name>`. All 12 ignored cases are red now. For the base comparison I ran copies of base and head `src/mask.rs`, plus the `url`, `phone`, `link` and `phone_number` code, in a scratch release probe.

Blocks merge:
- **`begin_lines_without_an_end_line_are_masked_in_linear_time`**: the always-on mask takes quadratic time on `BEGIN … PRIVATE KEY` lines that have no `END` line (`src/mask.rs:51`). For every `BEGIN`, the `BEGIN…END` alternative scans to the end of the text.
  - Red: `4x the text took 15.5x the time (62.684796ms -> 970.667568ms)`
  - Probe on 1 MiB, release build: `begin-only base: mask 6.473931ms` against `head: mask 24.410693012s`.
  - What reaches it: every fetched title, description, text and key goes through `mask`/`mask_key` (`src/run.rs:726` and the loop after it).
  - Suggested fix: two rules, a complete `BEGIN…END` block first and then `BEGIN{KEY_LINES}` alone.
- **`bracketed_links_back_to_back_are_found_in_linear_time`**: with the `Url` class, the URL body now admits `[` and `]` (`src/redact.rs:169`) and `link` cuts at `]`. So each link in a run with no spaces rescans the rest of the run.
  - Red: `4x the text took 16.0x the time (154.822534ms -> 2.471554664s)`
  - Probe on 1 MiB: `base: 5.962988ms` against `head: 84.409434653s (links=49933)`.
- **`a_stored_key_holding_the_rest_of_a_credential_is_dropped_on_load`**: a key stored before this change kept part of the password, and it is not dropped on load. The CHANGELOG (`CHANGELOG.md:84`) says such an old key "is dropped from the source's state". But the masked value starts with `[masked:`, so `apply` skips it (`src/mask.rs:118`), `mask_key` counts 0 and `SeenState::load` (`src/state.rs:98`) keeps it for good.
  - Red: `kept after load: ["https://example.org/a?access_token=[masked:secret-assignment]%zzKd9xQ2xQ2xQ2xQ2&page=2", "postgres://app:[masked:url-password]@ss-Kd9xQ2xQ2xQ2xQ2@db.internal:5432/x", …]`
  - What reaches it: any state file written by 0.6.x for a key with a raw `@` in the password or a raw `%` in the value. The probe shows base produced exactly these keys.
  - Suggested fix: skip a value only when it is exactly `[masked:<shape>]`.

Follow-up, all introduced:
- **`a_url_password_does_not_run_past_its_host_to_a_later_at_sign`** (`src/mask.rs:89`). In compact JSON, the "last `@`" rule masks the host and the next key.
  - Red: `the host is masked: {"dsn":"postgres://app:[masked:url-password]@corp.example"}`
  - Base masked only the password.
- **`an_iso_date_after_a_plus_one_is_not_a_phone_number`** (`src/redact.rs:213`).
  - Red: `left: "Votes: [Phone-1]/30 so far."`
  - Base found no phone number. This contradicts the CHANGELOG line "a date after `+`, with two, is still not a number".
- **`a_citation_after_a_parenthesised_link_stays_outside_it`** (`src/redact.rs:587-601`).
  - Red: `left: "Read it (see [Url-1] today."` against `right: "Read it (see [Url-1])[1] today."`
  - `https://example.org/a.[2]` is also swallowed whole.
- **`two_markdown_links_joined_without_a_space_are_two_links`**.
  - Red: `left: "Sources: [a]([Url-1])."` against `right: "Sources: [a]([Url-1]),[b]([Url-2])."`

Note, introduced, but nothing I found reaches these shapes (INFEASIBLE):
- **`a_raw_percent_before_an_encoded_ampersand_keeps_two_keys_apart`** (`src/mask.rs:36`): `%%26` is read as a raw `%`, so two different keys merge into one.
  - Red: `assertion `left != right` failed`
- **`a_masked_key_is_not_masked_again`**: this answers your seen-key question. Re-extraction forever and a child-failure count restarting every run happen only for a key whose masked form `mask_key` masks again.
  - Red: `left: ("x://password%3a[masked:[masked:url-password]@h", 1)`
  - The cause is the `:` inside `[masked:secret-assignment]`, and it also exists at base (`s://password=abcdefgh&x@h`).
  - A sweep over 1,000,000 generated keys found 10 such keys that only head has. All are scheme-plus-name-plus-`@` shapes, and I found no realistic record key of that form.
  - For realistic keys the re-read happens once, as the story says.

Follow-up, pre-existing (the base copy also leaks these):
- **`an_escaped_json_quoted_password_is_masked`**. Red: `Log: {\"password\": \"Kd9xQ2xQ2xQ2xQ2\"}`
- **`a_twice_encoded_separator_is_masked`**. Red: `https://example.org/r?next=%252Fapi%253Faccess_token%253DKd9xQ2xQ2xQ2xQ2`
- **`a_url_password_after_a_user_with_an_at_sign_is_masked`**. Red: `ftp://jane@corp.example:Kd9xQ2xQ2xQ2xQ2@files.example/x`

Green: **`no_generated_text_panics_the_built_in_classes`**, a 20,000-string sweep with multi-byte characters at slice boundaries.

**3. Suite, run after the cases existed** (same environment variables)
- `cargo test --lib -- mask:: redact::`: `test result: ok. 82 passed; 0 failed; 0 ignored; 0 measured; 192 filtered out; finished in 0.07s`
- `cargo test --test credential_mask`: `test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.58s`
- `cargo test --test redact_mask_adv_w30a`: `test result: ok. 1 passed; 0 failed; 12 ignored; 0 measured; 0 filtered out; finished in 2.50s`
- `… -- --ignored`: `test result: FAILED. 0 passed; 12 failed; 0 ignored; 0 measured; 1 filtered out; finished in 7.76s` (exit 101)
- `cargo clippy --test redact_mask_adv_w30a -- -D warnings`: exit 0

**4. Judgement findings:** none beyond the cases above.

**5. Attacked and not broken**
- No panic in `link`, `url`, `phone`, `mask` or `mask_key`: 20,000 redaction strings and 1.3 million mask strings.
- `café.fr/menu` and `中国.cn/x` are redacted without a panic.
- `+49 30/12`, `+1 2/3/2024`, `+12 03/04/2024` and `+1 10/30/2024` are not read as phone numbers.
- Quoted, YAML and `'…' =>` separators and `%3a` are masked.
- `mask`, and `mask_key` on realistic keys, are idempotent.
- `url-password` with no `@`, and the `%`-value pattern, run in linear time.

**6. Paths written outside the worktree**
- `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/redact/probe/`: the scratch Rust probe with its own `target/`, logs and `each.sh`/`suite.sh`.
- `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/redact/adv-msg.k01EuK`: the commit message.

**7. Findings**
```findings
- file: src/mask.rs
  line: 51
  category: property
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: private-key masking is quadratic in BEGIN lines without an END line, 24.4 s against 6.5 ms on 1 MiB in the always-on path
- file: src/redact.rs
  line: 169
  category: property
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: bracketed links back to back are scanned in quadratic time, 84.4 s against 6.0 ms on 1 MiB under the Url class
- file: CHANGELOG.md
  line: 84
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: a key stored before the change keeps the rest of the password after load, though the CHANGELOG says the old key is dropped
- file: src/mask.rs
  line: 89
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: url-password runs past a pathless host to a later @, masking the host and the next JSON key
- file: src/redact.rs
  line: 213
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: an ISO date after +1 (+1 2024/10/30) is replaced as a phone number
- file: src/redact.rs
  line: 587
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a [1] citation after a link, and the closing parenthesis before it, are swallowed into the link
- file: src/redact.rs
  line: 594
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: two Markdown links joined by a comma without a space become one link that hides the second label
- file: src/mask.rs
  line: 36
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a raw % before %26 swallows the rest of the query, so two keys merge; no caller found that produces it
- file: src/mask.rs
  line: 118
  category: property
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: mask_key masks a %3A-separated key after a scheme again, so it would be re-read every run, but no realistic key has that shape
- file: src/mask.rs
  line: 31
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: an escaped JSON-quoted password is not masked
- file: src/mask.rs
  line: 31
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: a twice percent-encoded separator is not masked
- file: src/mask.rs
  line: 89
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: a URL password after a user name with a raw @ is not masked
```
