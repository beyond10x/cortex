---
format: aep.planning-md/3
id: review-result:adversary-w30a-redact-pass-2
kind: review-result
status: active
title: 'Adversary pass 2: unit redact (wave 20261030a)'
relations:
- reviews: story:redaction-patterns-hold
- reviews: story:mask-quoted-credentials
revision: 1
---
unit: redact (story:redaction-patterns-hold, story:mask-quoted-credentials), correction 7193dd0e, findings cover the integration head d63686d1; the cases are in cb5c03fd on unit/w30a-p2
verdict: CONFIRMED
cases: executed 25→27, red 2
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 paths shared by the whole pass (part 6)
needs-coordinator: none

**1. Diff stat** (`git --no-pager diff --stat d63686d1 HEAD`): five test files, `tests/w30a_p2_{cli,cost,fail,read,redact}.rs`, 538 insertions, no other path. Commit cb5c03fd, author and committer `b10x-bot[bot]`; scan-text passed.

**2. Cases** (`tests/w30a_p2_redact.rs`, through the public `cortex_cli::redact::compile` with the `Url` class). Run alone first with `cargo test --test w30a_p2_redact`: `test result: FAILED. 0 passed; 2 failed`. Neither is ignored.
- `a_link_with_an_unclosed_parenthesis_is_replaced_whole`
  - Red: `left: "See [Url-1](Doe-4711 for the record."` / `right: "See [Url-1] for the record."`
- `a_link_with_a_stray_closing_parenthesis_inside_is_replaced_whole`
  - Red: `left: "Open [Url-1])Jane_Doe_contract.pdf today."` / `right: "Open [Url-1] today."`

**3. Suite run** (after the cases existed; log `w30a/p2/suite.log`):
- `redact_mask_adv_w30a`: `test result: ok. 8 passed; 0 failed; 5 ignored`. The 5 ignored are the known follow-ups.
- `credential_mask`: `test result: ok. 17 passed; 0 failed; 0 ignored`
- `w30a_p2_redact`: `test result: FAILED. 0 passed; 2 failed`
- The command ended `EXIT=101`.

**Pass-1 assertions:** the correction removed seven `#[ignore]` lines and changed no assertion (`git show 7193dd0e -- tests/`).

**4. Findings**

| # | file:line | verdict | origin | severity |
|---|---|---|---|---|
| 1 | src/redact.rs:170 (`url`: `plain`, `paren`, `bracket`) | CONFIRMED | introduced | warning |

1. **The rest of a link after an unbalanced parenthesis is left in clear.**
   - **The cause:** the correction's `Url` pattern admits a `(` or `)` only as part of a pair it opens and closes. A match now stops at an unclosed `(` or a stray `)` inside a link, and the rest of the link (a name, a record number, a file name) reaches the model unredacted.
   - **Release 0.6.1** (`git show a870a628:src/redact.rs`, `url` at :162 and `link` at :583) admitted every parenthesis and trimmed only an unbalanced `)` at the end, so both links were replaced whole. Two links joined by `),[` are correctly two links now; that was the reason for the change.
   - **What was measured:** `tests/w30a_p2_redact.rs:38` and `:45`, exit 101.
   - **What reaches it:** a link holding an unmatched parenthesis, under a policy with the `Url` class: a query typed by a person (`?q=Jane(Doe`), or a path with a stray `)`. Uncommon, but the leaked part is exactly what the class is meant to hide.
   - **Fix to name** (not applied): in the URL body, accept a `(` or `)` that has no partner while the next character is still a URL character (not whitespace or a quote). Then cut it in `link` as before: a trailing unbalanced `)` and the `](`/`),` boundaries.

**5. Attacked and could not break**
- **Linear time:**
  - The new `(?:plain|paren|bracket)*` cannot nest a group in a group of its own kind, so a match's lookahead dies at the next `(`, `[`, `)` or `]`. The next link's match covers at least the same run, so scans stay linear.
  - The private-key rules split in two: the `BEGIN`-only rule has no `[\s\S]*?` to read to the end.
- **`already_masked`:** the value it reads is the captured group, which stops at the value's own stop characters. So after one pass a token is never followed by more value characters, and `mask`/`mask_key` stay idempotent on realistic keys.
  - Tokens from another rule followed by more (`api_key=sk-…aaaa.v2`) are masked once more by `secret-assignment` and are then stable. The count goes up by one, but the text is not re-masked a second time.
- **`url-password` without `"<>{}|\^`:** it ends at the end of a compact JSON string, and still masks a raw `@` in the password up to the last `@` before the path.
- **The dated phone rule:** it needs a `/` in the candidate and a `/` right after it, so `+49 30/1234567` stays a number.
- **The known follow-ups were not re-reported:** escaped JSON-quoted password, twice-encoded separator, URL password after a user with `@`, `%%26` key merge, and `%3A` re-mask.

**6. Paths written outside the worktree** (shared by the pass, under `~/.cache/scratch/125ab618-63d4-4250-8265-9ad3e24d879c/w30a/`): `p2/build0.log`, `p2/suite.log`, `p2/commit-msg.x1If5v`, and this report. I took no worktree session lease.

**7. Findings block**
```findings
- file: src/redact.rs
  line: 170
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the Url pattern stops a link at an unclosed or stray parenthesis inside it, so the rest of the link reaches the model in clear where 0.6.1 replaced the link whole
```
