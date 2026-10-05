---
format: aep.planning-md/3
id: review-result:adversary-redaction-before-model-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: redaction-before-model (wave 20261005c)'
relations:
- reviews: story:redaction-before-model
revision: 1
---
unit: U3 story:redaction-before-model, uncommitted working tree on base 6352006 (cortex-w2-redact)
verdict: NEEDS-CHANGE
cases: executed 58→79, red 19
origin: introduced 18 / pre-existing 0 / undecided 0
wrote-outside-worktree: 2 paths (listed in part 6)
needs-coordinator: `src/main.rs` and `website/docs/limits.md` are not in this unit's scope. The implementor already has patches for both (`run-report-detail.patch`, `limits-doc.patch` in the scratch dir).

The most serious finding: an email address in a document's URL or record id goes to the model and into the store unredacted. The `Source:` line and the evidence identity are built from `doc.key`, and redaction never touches it. Record sources build the key from the operator's chosen `id` field, so contacts keyed by email would send that email with every document.

**1. Diff stat**
```
 AGENTS.md  | 11 ++++++++---
 src/lib.rs |  1 +
 src/run.rs | 25 +++++++++++++++++++++++--
```
Those three are the implementor's. `git status` adds the untracked files `src/redact.rs`, `tests/redaction.rs` (implementor's) and `tests/redaction_adversary.rs` (mine, the only file I wrote, a test file). I touched no implementation file.

**2. Cases** — `tests/redaction_adversary.rs`, headed "ADVERSARY CASES". I wrote all 21, then ran each alone (`cargo test --test redaction_adversary -- --exact <name>`). 19 are red, 2 green. Each red one fails on its own assertion, and the panic message shows the PII still in the output:

| case | red output (verbatim) |
|---|---|
| phone, NBSP separators | `Call +44 20 7946 0958 today.` (NBSPs kept) |
| phone, `00` prefix | `Call 0044 20 7946 0958 today.` |
| phone, label glued on | `Phone+44 20 7946 0958` |
| phone, `x` extension | `Reach me at 202-555-0143x12.` |
| opening hours (over-redaction) | `Open Monday to Friday [REDACTED:Phone].` |
| email, apostrophe | `Write to mary.o'[REDACTED:Email] today.` |
| email, non-ASCII local part | `Write to jörg.müller@example.de today.` |
| email, IDN domain | `Write to jane.doe@bücher.example today.` |
| email, quoted local part | `Write to "jane doe"@example.com today.` |
| email, cut by provider truncation | `Write to jane.doe@exam` |
| IPv4 after `host:` | `Connected host:192.168.10.24 at noon.` |
| IPv6 after `server:` | `Connected server:2001:db8:85a3::8a2e:370:7334 at noon.` |
| IPv4-mapped IPv6 | `Client [REDACTED:IpAddress].168.10.24 logged in.` |
| 19-digit card, 4-4-4-4-3 | `Card 6250 9410 1652 8599 122 paid.` |
| card, NBSP separators | `Card 4111 1111 1111 1111 paid.` |
| rule matches a built-in's replacement | `left: "Mail [[account]:Email] today."` |
| e2e: email in URL | prompt carries `Source: https://example.org/team/jane.doe@example.com` |
| e2e: 2nd run, nothing applied | `"documents_new":0,…"redacted":{"Email":1,…}` |
| e2e: `cortex run` report | `detail` has no `redacted` key: `left: Null` |

The two green cases: an email in a title is redacted, and `store.sqlite` holds the payload as plain text. So the existing e2e store check can fail; it just finds `[REDACTED:Email]` through `prompt.txt` rather than the store.

**3. Suite run (after the cases existed)**
- `task check` → `EXIT=201`. The fmt step failed first on my file; I ran `rustfmt` on that file only and reran.
- On the rerun: `test result: FAILED. 2 passed; 19 failed; …` for `redaction_adversary`.
- `cargo test --locked --workspace --no-fail-fast` → `EXIT=101`, 79 cases executed, 19 failed.
- The "before" figure of 58 comes from the implementor's `gate.log`.

**4. Judgement finding**
- `website/docs/limits.md:22` still says "No PII redaction". This unit makes that false. CONFIRMED, introduced.

**5. Attacked, could not break**
- Email plus-tags.
- Redaction in titles.
- Truncation by cortex itself: redaction runs at `run.rs:158`, before truncation at `:163`.
- Replacement text is literal (`$1` is not expanded).
- Plaintext evidence in the store.
- ReDoS: I only reasoned about this, nothing ran. The `regex` crate does not backtrack.

**6. Written outside the worktree**
- `~/.cache/cortex-wave-20261005c/redact/adversary-1/` — 26 logs.
- `~/.cache/b10x-target/cortex-w2-redact` — the assigned build dir, which now holds the `redaction_adversary` test binary.

**What reaches each finding**
- URL: any web result whose URL contains an address, and record sources (`sources.rs:277`).
- NBSP: HTML `&nbsp;` in phone and card numbers. I don't know whether the provider turns these into plain spaces.
- Other format cases: plain fetched text in those formats.
- Counts: any repeat run (`refresh_after_days: 0` in the test spec). Redactions are counted before `seen.select` at `run.rs:171`, so skipped documents and text cut off by truncation are counted too.

```findings
[
 {"file":"src/evidence.rs","line":36,"category":"acceptance","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"doc.key (URL, file path, record id) is never redacted, so an email in it reaches the prompt Source line (extract.rs:159) and the stored evidence identity"},
 {"file":"src/redact.rs","line":81,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"phone separators accept only space, dot and hyphen, so a number written with non-breaking spaces is not redacted"},
 {"file":"src/redact.rs","line":87,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a phone number written with the 00 international prefix (0044 20 7946 0958) is not redacted"},
 {"file":"src/redact.rs","line":134,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"phone_check rejects the whole match when a letter touches it, so Phone+44… and 202-555-0143x12 leak in full"},
 {"file":"src/redact.rs","line":58,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the email pattern is ASCII-only and has no apostrophe or quotes, so IDN and non-ASCII addresses leak whole and mary.o'neil leaks mary.o'"},
 {"file":"src/redact.rs","line":58,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"an address cut before its TLD by provider truncation (jane.doe@exam) is not redacted"},
 {"file":"src/redact.rs","line":124,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"ip_check rejects a match preceded by ':', and the IPv6 alternative then takes the colon, so host:192.168.10.24 and server:2001:db8::… leak"},
 {"file":"src/redact.rs","line":72,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"an IPv4-mapped IPv6 address is cut at ::ffff:192, leaving .168.10.24 in the text"},
 {"file":"src/redact.rs","line":63,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a Luhn-valid 19-digit card printed 4-4-4-4-3, or with non-breaking spaces, is not redacted because the rejected 16-digit candidate uses up the match"},
 {"file":"src/redact.rs","line":87,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"opening hours 0900-1700 are replaced as a Phone, which destroys a fact"},
 {"file":"src/redact.rs","line":219,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"rules run over the built-ins' output, so a rule matching REDACTED corrupts the marker and inflates its own count"},
 {"file":"src/run.rs","line":167,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"redactions are counted on every fetched document before selection and truncation, so a run that applied nothing logs Email 1"},
 {"file":"src/main.rs","line":685,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the run report printed by cortex run has no redacted counts; only cortex.log has them"},
 {"file":"website/docs/limits.md","line":22,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"the published limits page still says No PII redaction"}
]
```