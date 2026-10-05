---
format: aep.planning-md/3
id: review-result:adversary-credential-mask-covers-titles-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: credential-mask-covers-titles (wave 20261005e)'
relations:
- reviews: story:credential-mask-covers-titles
revision: 1
---
```
unit: story:credential-mask-covers-titles (U2, wave 20261005e), uncommitted working tree on b920177 in ~/.local/state/worktree/trees/b10x/cortex/cortex-w4-mask
verdict: NEEDS-CHANGE
cases: executed 180→188, red 8
origin: introduced 5 / pre-existing 2 / undecided 0
wrote-outside-worktree: 15 paths, all under ~/.cache/cortex-wave-20261005e/mask/adversary-1/ (listed in part 6)
needs-coordinator: none (I took no worktree session lease; the coordinator holds the tree)
```

**1. `git --no-pager diff --stat`**
```
 AGENTS.md              |  7 ++++---
 src/run.rs             | 19 ++++++++++++++++---
 website/docs/limits.md |  7 ++++---
```
Those three are the unit's. Untracked: `tests/credential_mask.rs` (the unit's) and `tests/credential_mask_adversary.rs` (mine, the only file I wrote). I changed no non-test path. `cargo fmt -p cortex-cli` changed only my file; the stat matched before and after.

**2. Cases** (`tests/credential_mask_adversary.rs`, headed "ADVERSARY CASES"; each run alone first, all red now; logs in `red-<case>.log`)

| case | asserts | first red output |
|---|---|---|
| `two_urls_that_differ_after_the_token_stay_two_documents` :105 | `?access_token=T&page=1` and `&page=2` keep 2 keys; a second run finds nothing new | `two documents, one key: ["https://example.org/doc?access_token=[masked:secret-assignment]"] left: 1 right: 2` |
| `two_urls_that_differ_only_in_the_token_are_one_document` :128 | same page, two tokens → `documents_new` 1 | `"documents_applied":2,"documents_new":2 … left: Number(2) right: 1` |
| `a_percent_encoded_access_token_in_a_url_is_masked` :149 | `?next=%2Fapi%3Faccess_token%3DT` → T not in the prompt or any instance file | `Zr4p0p0p0p0p0p0p0p0 reached the model` |
| `a_userinfo_password_in_a_url_is_masked` :162 | `https://alice:T@example.org/a` | `… reached the model` |
| `a_client_secret_in_a_url_is_masked` :175 | `?client_secret=T` | `… reached the model` |
| `an_env_style_password_in_a_text_is_masked` :188 | text `DB_PASSWORD=T` | `… reached the model` |
| `a_token_in_a_record_key_stays_out_of_the_run_report` :264 | record id (a URL with a token) with its child call failing, 3 runs | runs 1–2 `"stopped":"deliveries.list: child call failed for tracker:hooks.list:https://hooks.example.org/in?access_token=Zr4p0…`; run 3 `"skipped":[… on 3 runs; skipped]` |
| `a_token_in_a_record_key_stays_out_of_the_log_and_the_state` :283 | same setup, no instance file holds T | `run 1..3: …/instances/r/state/tracker.json`, `…/instances/r/cortex.log` |

**3. Suite** (run after the cases existed)
- `task check` (`suite.log`): `test result: FAILED. 0 passed; 8 failed` for `credential_mask_adversary`, then `task: Failed to run task "check"`, `EXIT=201`. It is fail-fast, so later test binaries did not run.
- `cargo test --locked --workspace --no-fail-fast` (`suite-nofailfast.log`): 188 executed, 180 passed, 8 failed (all mine), `EXIT=101`.
- `<before>` = 180: the sum of the `test result:` lines in the implementor's `gate.log`.

**4. Findings**
- **The key collapse comes from this unit (blocker).**
  - The `secret-assignment` value class `[^\s"']{6,}` (`src/mask.rs:28`) runs past `&` and `#`. Masking now also runs on the key (`src/run.rs:233`), so everything after the token is swallowed and distinct URLs merge into one key and one evidence identity.
  - Dedupe runs before masking (`src/sources.rs:442`), on the unmasked keys, so it never sees the merge.
  - Effect: the state file keeps one hash per merged key. With `refresh_after_days` passed, the other document re-extracts on every run. The prompt shows two documents with the same `Source:`.
  - What reaches it: an `access_token=` followed by more parameters in a search or crawl URL, which is the case the story names.
  - Fix, named and not applied: end the value at `&` and `#` when masking a key, and dedupe again after masking in `process()`.
- **Same page, two tokens.** It is extracted and cited twice under one identity. Introduced, warning. The same post-mask dedupe fixes it.
- **URL shapes this unit's coverage misses.** Introduced, because the unit's docs claim URL coverage:
  - percent-encoded `access_token%3D` (redirect and `next` parameters): warning;
  - userinfo password: note, since I found no provider that returns one.
- **`\b` in `src/mask.rs:28` misses prefixed names.** `client_secret=`, `DB_PASSWORD=` and `x_api_key=` are not masked, in text as well as keys. Pre-existing, warning. Suggested fix: allow a `[A-Za-z0-9]+_` prefix before the name.
- **Record-key leak.** Confirmed at runtime: the record key reaches the run report, `cortex.log` and `state/<source>.json`.
  - It goes through `child_failures` (`src/sources.rs:549`), `skipped` (:554) and `unread`, which becomes `stopped` (:560).
  - Pre-existing. INFEASIBLE as found: the case sets `id: url` with a tokened URL, and I could not show anybody configures that.
  - `AGENTS.md:102` "never stored" (text added by this unit) is contradicted by this and by the two items above. Note.
- **Existing instances (inferred from code, not run).** A key masked for the first time is a new key: the document re-extracts once, and the old unmasked entry stays in the state file, since nothing prunes it (`src/state.rs` `record` and `wants`). Pre-existing data; note only.

**5. Attacked, not broken**
- `#access_token=` in a fragment and a token in a URL path are masked. Shown with a regex probe only (`probe_regex.py`), not a cortex run.
- A token split across title and description: each field is masked alone, and the prompt puts `\nDescription: ` between them, so no masked shape spans the two fields.
- Mask before redaction: all four fields are masked at `run.rs:233` before scrub and pseudonymise run.
- Seed run: it goes through the same `process()`. The frozen seed copies are operator files, by existing design.
- A masked key equal to a real key would need a provider URL that literally contains `[masked:…]`. INFEASIBLE, so I wrote no case.
- The unit's 3 tests assert literal values and would fail if the functions returned defaults.

**6. Paths written outside the worktree**
- In `~/.cache/cortex-wave-20261005e/mask/adversary-1/`: `build.log`, `probe_regex.py`, `stat-before-fmt.txt`, `suite.log`, `suite-nofailfast.log`, the 8 `red-*.log` and `tmp/` (now empty; the tests' temporary directories went here).
- I deleted my own first log, `red-a_token_in_a_record_key_stays_out_of_the_report_the_log_and_the_state.log`, after splitting that case in two. Its red output is quoted in part 2.
- Build output went to `~/.cache/b10x-target/cortex-w4-mask` (assigned).

**7.**
```findings
[
 {
  "file": "src/run.rs",
  "line": 233,
  "category": "acceptance",
  "severity": "blocker",
  "verdict": "NEEDS-CHANGE",
  "origin": "introduced",
  "message": "masking the key with secret-assignment swallows every query parameter after an access_token, so distinct URLs merge into one key and identity and re-extract each run (red case at tests/credential_mask_adversary.rs:105)"
 },
 {
  "file": "src/sources.rs",
  "line": 442,
  "category": "boundary",
  "severity": "warning",
  "verdict": "NEEDS-CHANGE",
  "origin": "introduced",
  "message": "dedupe runs on unmasked keys, so one page returned with two tokens is extracted and cited twice under one masked identity (case at :128)"
 },
 {
  "file": "src/mask.rs",
  "line": 28,
  "category": "acceptance",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "a percent-encoded access_token%3D in a document URL reaches the model and the store unmasked (case at :149)"
 },
 {
  "file": "src/mask.rs",
  "line": 28,
  "category": "acceptance",
  "severity": "note",
  "verdict": "CONFIRMED",
  "origin": "introduced",
  "message": "a URL userinfo password (https://user:pass@host) is not masked in the key (case at :162)"
 },
 {
  "file": "src/mask.rs",
  "line": 28,
  "category": "boundary",
  "severity": "warning",
  "verdict": "CONFIRMED",
  "origin": "pre-existing",
  "message": "the leading \\b skips prefixed names, so client_secret= in a URL and DB_PASSWORD= in text are not masked (cases at :175 and :188)"
 },
 {
  "file": "src/sources.rs",
  "line": 549,
  "category": "contract-drift",
  "severity": "note",
  "verdict": "INFEASIBLE",
  "origin": "pre-existing",
  "message": "an unmasked record key reaches the run report, cortex.log and state/<source>.json through child_failures, skipped and stopped, but only in a configuration I constructed (id pointing at a tokened URL) (cases at :264 and :283)"
 },
 {
  "file": "AGENTS.md",
  "line": 102,
  "category": "contract-drift",
  "severity": "note",
  "verdict": "NEEDS-CHANGE",
  "origin": "introduced",
  "message": "the new text says a credential in a key is never stored, which the percent-encoded, userinfo, client_secret and record-key cases contradict"
 }
]
```