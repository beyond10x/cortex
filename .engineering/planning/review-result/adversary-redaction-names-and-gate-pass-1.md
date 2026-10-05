---
format: aep.planning-md/3
id: review-result:adversary-redaction-names-and-gate-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: redaction-names-and-gate (wave 20261005f)'
relations:
- reviews: story:redaction-names-and-gate
revision: 1
---
unit: story:redaction-names-and-gate (U1, wave 20261005f), uncommitted working tree on base fc7a332 in `~/.local/state/worktree/trees/b10x/cortex/cortex-w5-names`
verdict: NEEDS-CHANGE
cases: executed 235→244, red 9
origin: introduced 10 / pre-existing 0 / undecided 0
wrote-outside-worktree: 14 paths, all under `~/.cache/cortex-wave-20261005f/names/adversary-1/`, plus build output in `~/.cache/b10x-target/cortex-w5-names`
needs-coordinator: none

**1. What I touched.** `git --no-pager diff --stat` lists only the implementor's five modified files: AGENTS.md, src/redact.rs, src/run.rs, website/docs/limits.md and website/docs/spec-file.md. I edited none of them. The only path I added is `tests/adversary_names_gate.rs`, which is untracked, so the stat does not show it. I formatted it with `rustfmt` on that file alone.

**2. Cases added** in `~/.local/state/worktree/trees/b10x/cortex/cortex-w5-names/tests/adversary_names_gate.rs`, under the "ADVERSARY CASES" header. Each was run alone first and was red (exit 101); each log is `red-<test>.log`.

| line | case | red output (verbatim) |
|---|---|---|
| :55 | URL with a password in it, `Url` on | `git.example.org reached the model (gate found {}): Clone [Url-1]:[masked:url-password]@git.example.org/people/jane-doe/notes.git today.` |
| :77 | token in a URL's query, `Url`+`Credential` on | `the link's tail reached the model (gate found {}): Reset at [Url-1][masked:secret-assignment]&user=jane.doe today.` |
| :96 | link written without a scheme | `a link reached the model (gate found {}): Her profile is at intranet.example.org/people/jane-doe for reference.` |
| :117 | name also inside a masked email address | `a person's name reached the model (gate found {}): [Name-1], Rowan Pell <[Email-1]>` |
| :134 | known name in capitals or lower case | `a known name reached the model: [Name-1]: JANE DOE escalation. jane doe replied.` |
| :145 | known name in decomposed Unicode (NFD) | `a known name reached the model: Zoë Faber wrote the release notes.` |
| :160 | `pwd:` value with `;#&` in it | `the password is stored: Database login: pwd: Xk9;mP2#vL7&qR4w` |
| :174 | non-secret `auth:` setting | `a non-secret fact was masked irreversibly: The gateway is configured with auth: [masked:secret-assignment] for all clients.` |
| :227 | end to end over two runs, a person the store already knows | run 2's prompt contains `- Person: Rowan Pell` and `Rowan Pell called again.`, under `refuse_if_left: [RareName]`, outcome `ran` |

**3. Suite run, after the cases existed.**
- `task check > adversary-1/gate.log`: ess validate, drift, fmt and clippy passed. The test step stopped at the first failing binary: `test result: FAILED. 0 passed; 9 failed`, `task: Failed to run task "test": exit status 101`, `EXIT=201`.
- `cargo test --locked --workspace --no-fail-fast > adversary-1/suite.log`: 235 passed, 9 failed (all nine are mine), `EXIT=101`.
- The 235 before my cases comes from the implementor's own `gate.log` (sum of its `test result:` lines).

**4. Findings**

| # | file:line | verdict | severity | measured | what reaches it |
|---|---|---|---|---|---|
| 1 | src/run.rs:801 and src/redact.rs:954 | NEEDS-CHANGE | blocker | case :227 | Every run with `RareName` on. Run 1 restores the name and saves it in `state/entities.json`. In each later run, the "Known entities" entry counts as a second sighting, so any person already in the store and mentioned once is no longer rare. The name is then sent in the known list and in the text, and `refuse_if_left: [RareName]` passes it because it uses the same counts. limits.md says it counts "the batch's texts". Fix: do not count known-entity names or masked spans toward rarity. |
| 2 | src/redact.rs:954 | NEEDS-CHANGE | warning | case :117 | Same root cause. `reserve` counts capitalised words inside text an earlier class hides (email, URL, the Source key), so a name seen once in prose plus once in a hidden address goes to the model. Email signatures produce this. |
| 3 | src/redact.rs:143 | NEEDS-CHANGE | warning | cases :55, :77 | The `Url` pattern stops at `[`. `mask::mask` runs on every run and puts `[masked:url-password]` inside any URL with a password; the `Credential` class does the same for `token=` in a query. Only the start of the URL gets `[Url-n]`; the host, path or rest of the query goes to the model, and the `Url` gate does not see it. Fix: let a URL run through a `[masked:<shape>]` token, or detect URLs before masking. |
| 4 | src/redact.rs:143 | CONFIRMED | warning | case :96 | Links without `scheme://` or `www.` (as chat and tickets write them) are not caught, yet limits.md says the class "replaces links". |
| 5 | src/redact.rs:259 | NEEDS-CHANGE | warning | case :160 | The value stops at `"'&#,;<>`; a value with any of these in its first 6 characters is not matched at all. `mask.rs` does not know `pwd`, `token`, `passphrase`, `credentials` or `auth`, so the whole password is sent and stored. This contradicts limits.md ("assignments to … `pwd` … whose value mixes letters and digits"). |
| 6 | src/redact.rs:258 | CONFIRMED | note | case :174 | The plain name `auth` makes ordinary config facts (`auth: oauth2-pkce`) be masked irreversibly in the store. This matches the documented rule, but the fact is lost for good. |
| 7 | src/redact.rs:634 | CONFIRMED | note | case :134 | Known names match case-sensitively, and `RareName` needs a lower-case letter, so `JANE DOE` and `jane doe` reach the model. limits.md documents this. |
| 8 | src/redact.rs:634 | CONFIRMED | note | case :145 | No Unicode normalisation: an NFD spelling of a listed name is not matched. |
| 9 | src/run.rs:451 | INFEASIBLE | note | read only, no case | A refusal found in a later batch stops the run after earlier batches were sent and stored. limits.md and AGENTS.md say "nothing is sent or stored". To reach it, a refuse-only class must first appear in entity names from an earlier batch; I could not show any normal configuration does that. |

Side effect seen in cases :117 and :134: the words "Thanks" and "Subject" became `[Name-1]`. They are restored afterwards, so no fact is lost.

**5. Attacked and could not break**
- All 23 credential shapes, plus `ASIA`, `github_pat_`, `sk-ant-`, `gsk_`, `hf_`, `xai-`, `pplx-`, `r8_`, JSON `client_secret`, `private_key` and `X-Api-Key`: masked when stored and when shown, with all classes on and nothing left for the gate. This was a green probe; I removed it from the file. Log: `probes.log`.
- Restore: a literal `[Name-1]` or `Name-2` in one document never picked up another document's name (green probe, removed).
- A gate refusal in the first batch makes no model call and leaves `ekr head` unchanged. Nothing is written before the gate: `seen`, `entities` and the run directory are all written after it.
- The gate sees the key, title, description, text and known names as they are shown. The `file:` / `record:` prefix of the Source line adds nothing the gate would detect.
- With a policy, the prompt, raw answer and report are not written to disk. Only the restored `extraction.yaml` is, and the log records counts only.
- Possessives (`Jane's`), names split across lines, names inside other words (`Rowanberry`, `iPhone`), markdown links, and `Url` versus `Email`/`IpAddress` ordering all held.

**6. Paths written outside the worktree**
- `~/.cache/cortex-wave-20261005f/names/adversary-1/` holding `build.log`, `gate.log`, `suite.log`, `probes.log`, nine `red-adversary_*.log` and `tmp/`. I also created and then moved away a transient `trimmed.rs` there.
- Build output in `~/.cache/b10x-target/cortex-w5-names` (the assigned build directory).

```findings
[
  {"file": "src/run.rs", "line": 801, "category": "acceptance", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "Known-entity names count toward RareName frequency, so any person already in the store and mentioned once in a later run is sent to the model by name, and the RareName gate passes it."},
  {"file": "src/redact.rs", "line": 954, "category": "property", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "Batch::reserve counts capitalised words inside spans an earlier class hides (email, URL, key), so a name seen once in prose and once in a hidden address reaches the model."},
  {"file": "src/redact.rs", "line": 143, "category": "boundary", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The Url pattern stops at the [masked:...] token that mask.rs or Credential puts inside a URL, so the host, path or query tail after it reaches the model unseen by the Url gate."},
  {"file": "src/redact.rs", "line": 143, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "A link without scheme:// or www. is neither replaced nor detected by the gate, although limits.md says the Url class replaces links."},
  {"file": "src/redact.rs", "line": 259, "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "A secret-assignment value with ; # & , in its first six characters is not matched at all, so a mixed letter-and-digit pwd/token/passphrase value is sent and stored, contrary to limits.md."},
  {"file": "src/redact.rs", "line": 258, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The bare name auth makes non-secret configuration such as auth: oauth2-pkce be masked irreversibly, destroying the fact in the store."},
  {"file": "src/redact.rs", "line": 634, "category": "boundary", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Known names match case-sensitively and RareName needs a lower-case letter, so JANE DOE and jane doe reach the model (documented limit)."},
  {"file": "src/redact.rs", "line": 634, "category": "boundary", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Known names are not Unicode-normalised, so an NFD spelling of a listed name reaches the model."},
  {"file": "src/run.rs", "line": 451, "category": "contract-drift", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "A refusal found in a later batch stops the run with earlier batches already sent and stored, while limits.md and AGENTS.md say a refusal sends and stores nothing; no common configuration was shown to reach it."}
]
```