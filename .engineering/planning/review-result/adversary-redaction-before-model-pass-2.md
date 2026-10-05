---
format: aep.planning-md/3
id: review-result:adversary-redaction-before-model-pass-2
kind: review-result
status: active
title: 'Adversary pass 2: redaction-before-model (wave 20261005c)'
relations:
- reviews: story:redaction-before-model
revision: 1
---
```
unit: U3 story:redaction-before-model, uncommitted working tree on base 6352006 (cortex-w2-redact, with the coordinator's src/main.rs patch applied)
verdict: NEEDS-CHANGE
cases: executed 90→100, red 8
origin: introduced 9 / pre-existing 1 / undecided 0
wrote-outside-worktree: 2 paths (part 6)
needs-coordinator: the seed-report fix is in src/main.rs:488, which belongs to story:store-backend-per-instance. The draft story for the credential-mask gap (`redact/mask-gap-story.md`) covers title and description but not the document key (URL). It should cover all three.
```

**1. Diff stat**
```
 AGENTS.md              |  15 ++++++-
 src/lib.rs             |   1 +
 src/main.rs            |  27 +++++++-----
 src/run.rs             | 117 ++++++++++++++++++++++++++++++++++++++++++++++---
 website/docs/limits.md |  16 +++++--
```
Those changes were there when I started (the implementor's work plus the coordinator's main.rs patch). The only file I added is the untracked `tests/redaction_adversary.rs`, which is a test file. I changed no implementation file.

**2. Cases** are in `tests/redaction_adversary.rs`, headed "ADVERSARY CASES". I wrote all 10 before running anything, then ran each alone with `--exact`. Logs: `adversary-2/red-<name>.log`.

| case | now | red output (verbatim) |
|---|---|---|
| phone with the next label glued after it | red | `Tel +44 20 7946 0958Fax [Phone-1]` |
| card with the next label glued after it | red | `Card 4111 1111 1111 1111Exp 12/27` |
| `%40`-encoded email (unit) | red | `https://example.org/unsubscribe?email=jane.doe%40example.com` |
| e2e: `%40` email in a URL | red | prompt has `Source: https://example.org/unsubscribe?email=jane.doe%40example.com` |
| e2e: credential in a URL | red | prompt has `Source: https://example.org/a?access_token=s3cr3tvalue1234567` |
| e2e: seed report from `cortex create` | red | `left: Null` / `right: Object {"Email": Number(1), …}`; create's detail is only `"seed":{"cost_usd":0.01,"documents_applied":1}` |
| four-part version kept | red | `left: "Released version [IpAddress-1] today."` |
| ms timestamp kept | red | `left: "observed_at [Card-1] by the crawler"` |
| zero-padded IPv4 replaced | green | — |
| literal `[Email-1]` in doc B does not capture doc A's value | green | — |

**3. Suite run (after the cases existed)**
- `task check` → `EXIT=201`. It stops at the first failing binary: `test result: FAILED. 2 passed; 8 failed` for `redaction_adversary`.
- `cargo test --locked --workspace --no-fail-fast` → `EXIT=101`, 100 cases run, 8 failed. Every other binary is `ok`.
- The "before" figure of 90 comes from fix-1's `gate-with-detail-patch.log`.

**4. Findings, with what reaches each**
- **`%40` email** (`redact.rs:90`, the email pattern needs a literal `@`): reached by any web result whose URL carries an encoded address. It goes into the `Source:` line.
- **Credential in the document key** (`run.rs:158`, `mask` runs on `d.text` only): the base does the same (`git show 6352006:src/run.rs`, line 145), so this is pre-existing. It is reached by signed links and tokens in search-result URLs; the token goes to the model and into the store.
  - This unit's new text says the opposite: AGENTS.md:100 says "a secret is never stored" and limits.md:24 says "no secret is stored". That claim is introduced.
- **Glued phone or card** (`redact.rs:227`, `:193`): the match is rejected when a letter follows it. It is the mirror of pass 1's glued-label case, so it is reached the same way, by scraped text. I don't know whether the provider glues table cells.
- **Seed report** (`main.rs:488`): `cortex create` with seed documents and a policy. `cortex.log` counts `Email: 1`; the printed report has no `redacted` field. `cortex run` does match its log.
- **Over-redaction** (`redact.rs:203` and `:161`): four-part versions and Luhn-valid IDs or timestamps reach the model as `[IpAddress-1]` or `[Card-1]`. A placeholder the model copies is restored, so the store keeps the value; only what the model understands of it is damaged.
- **Judgement, the mapping on disk** (`run.rs:283-287`): each `runs/<ts>/batch-N/` directory holds `model.json`, with placeholders, next to `extraction.yaml`, with the restored values. Side by side they give the mapping. The test at `tests/redaction.rs:210` only checks one file at a time, so it cannot catch this. The store holds the originals anyway, so this is a note.
- **Judgement, `replacement` ignored** (`redact.rs:272`): a rule's `replacement` field, which the spec declares, is silently ignored. A rule an operator wrote to hide a credential is now restored into the store.

**5. Attacked, could not break**
- A document containing a literal `[Email-1]`, within one document and across documents in one batch (my green case and an existing test).
- Restore only rewrites JSON values, never keys.
- Labels are matched case-sensitively, so `[email-1]` is never restored.
- Unbracketed `Email-1` is counted as unrestored and not restored.
- No `Debug` on `Batch`, so the mapping cannot reach the log.
- Known entity names are pseudonymised again in the next batch.
- A value split by the cut is replaced and restored to the kept part.
- The `cortex run` report matches `cortex.log`.
- Zero-padded IPv4 is replaced.

**6. Written outside the worktree**
- `~/.cache/cortex-wave-20261005c/redact/adversary-2/`: build.log, list.txt, 10 `red-*.log`, gate.log, suite-no-fail-fast.log, and an empty `tmp/`.
- `~/.cache/b10x-target/cortex-w2-redact`: the assigned build dir, which now also holds the `redaction_adversary` test binary.
- I took no session lease.

```findings
[
 {"file":"src/redact.rs","line":90,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a percent-encoded address (jane.doe%40example.com) in a URL is not matched and reaches the prompt's Source line"},
 {"file":"src/redact.rs","line":227,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a phone number with a letter glued after it (0958Fax) is rejected whole and sent to the model"},
 {"file":"src/redact.rs","line":193,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a card number with a letter glued after it (1111Exp) is rejected whole and sent to the model"},
 {"file":"src/run.rs","line":158,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"credential masking covers d.text only, so an access_token in a document URL reaches the prompt and the store"},
 {"file":"AGENTS.md","line":100,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"AGENTS.md and limits.md:24 now state that no secret is stored, which a credential in a URL, title or description disproves"},
 {"file":"src/main.rs","line":488,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the seed report printed by cortex create has no redacted or unrestored counts, though the seed run's cortex.log line has them"},
 {"file":"src/redact.rs","line":203,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"a four-part version number (4.18.2.1) is shown to the model as [IpAddress-1]"},
 {"file":"src/redact.rs","line":161,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"a Luhn-valid 13-19 digit id or millisecond timestamp is shown to the model as [Card-1]"},
 {"file":"src/run.rs","line":283,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"each batch dir keeps model.json with placeholders beside extraction.yaml with restored values, so the mapping can be rebuilt from disk; tests/redaction.rs:210 checks single files only"},
 {"file":"src/redact.rs","line":272,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"a rule's spec field replacement is silently ignored, so a rule written to hide a credential is now restored into the store"}
]
```