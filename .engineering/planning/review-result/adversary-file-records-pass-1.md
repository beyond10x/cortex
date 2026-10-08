---
format: aep.planning-md/3
id: review-result:adversary-file-records-pass-1
kind: review-result
status: archived
title: 'Adversary pass 1: file-records (wave 20261005h)'
relations:
- reviews: story:file-records
revision: 2
transitions:
- {from: "active", to: "archived", at: "2026-10-08T13:07:36Z", actor: "human:timo", revision: 2}
---
```
unit: story:file-records (U1, wave 20261005h), working tree on base dfe3fc1 plus the uncommitted change
verdict: NEEDS-CHANGE
cases: executed 254→265, red 9
origin: introduced 10 / pre-existing 0 / undecided 1
wrote-outside-worktree: 1 path (~/.cache/cortex-wave-20261005h/records/adversary-1/), plus builds into the assigned ~/.cache/b10x-target/cortex-w7-records
needs-coordinator: none
```

The worst finding is a privacy leak. With threads on, a name written once in a chat export reaches the model unredacted, because the message is repeated as `[context]` in its reply.

**1. What I touched.** `git --no-pager diff --stat` shows only the implementor's 4 tracked files. My one addition is untracked: `?? tests/file_records_adversary.rs` (392 lines, a test file). I changed no implementation file.

**2. Cases** in `~/.local/state/worktree/trees/b10x/cortex/cortex-w7-records/tests/file_records_adversary.rs`. Each was run alone before the suite ran; logs are `adversary-1/red-<name>.log`.

| case | now | red output, captured first |
|---|---|---|
| `a_record_without_a_thread_field_carries_no_other_records_text` | red | `b.jsonl#1` is shown as "Bravo writes a standalone note.\n\n[context] Alpha opens a thread…\n\n[context] Alpha replies in that thread." |
| `a_name_written_once_stays_pseudonymised_…_thread_context` | red | Control passes: with no thread, "Zorblat" is replaced. With a thread it is sent raw in both `#1` and `#2`, once as "[context] Ask Zorblat about…" |
| `a_fence_is_not_closed_by_the_other_fence_character` | red | keys `[#Not a heading, #Setup]`, expected `[#Setup, #Usage]`. The real `## Usage` section is lost. |
| `a_byte_order_mark_does_not_hide_the_first_section` | red | keys `[#Second]`. `#First` is gone and not listed in `skipped`. |
| `a_byte_order_mark_does_not_lose_the_first_json_line` | red | keys `[#2]`. `skipped: ["…: line 1 is not JSON; skipped"]` |
| `a_repeated_heading_loses_no_section_silently` | red | `documents_applied: 2`, no `skipped`. The second `## Notes` section is never read and nothing says so. |
| `a_markdown_file_without_a_level_two_heading_is_not_lost_silently` | red | `documents_new: 0`, no `skipped` |
| `a_setext_level_two_heading_opens_a_section` | red | keys `[]` |
| `one_invalid_byte_does_not_lose_a_whole_json_lines_file_silently` | red | `documents_new: 0`, no `skipped`. Records 1 and 3 are lost along with the bad line. |
| `guard_an_unchanged_threaded_export_is_not_extracted_again` | green | Attack held. Second run gives `documents_new: 0`, with CRLF, no trailing newline and redaction on. |
| `guard_a_credential_in_a_heading_is_masked_in_the_key` | green | Attack held. |

**3. Suite**, run after the cases existed.
- `task check ESS=$HOME/.cache/ess/toolchains/0.52.0/ess` → `EXIT=201`. The first run failed on `cargo fmt --check` for my file. I fixed it with `rustfmt` on that file alone.
- The second run ended: `error: test failed, to rerun pass -p cortex-cli --test file_records_adversary` / `task: Failed to run task "check"` / `EXIT=201`.
- Cargo stopped at the first failing binary, so I reran `cargo test --locked --workspace --no-fail-fast` to get a full count: `EXIT=101`, 256 passed, 9 failed. The only failing line is `test result: FAILED. 2 passed; 9 failed`. Every other binary was green.
- The 254 "before" comes from the implementor's `records/gate.log`.

**4. Fixes and where each defect comes from.** Line numbers are in `src/sources.rs`.

| finding | what reaches it | fix (not applied) |
|---|---|---|
| :798 `unwrap_or(id)`: a record with no thread field takes on the context of whatever thread has its id | Any two files with ids numbered per file, or a reply that comes before its parent. The doc at spec-file.md:228 says such a record *opens* a thread. | Append context only when the field is present |
| :798–803 context repeats text, so RareName counts it twice | Default chat setup with `thread` and `RareName` on, first run | Count rare words over a record's own text only |
| :865 `fenced = !fenced` flips on either fence character | Any markdown that shows a fence of the other style inside a fence | Track the opening fence's character and length |
| :867 / :845 a BOM blocks `## ` and JSON | Exports from editors that write a BOM | Strip a leading U+FEFF |
| :790 a repeated key is dropped without a word | Duplicate headings (`## Notes`, `## Example`) | List it in `skipped`, or suffix the key |
| :867 a file with no `##` is dropped without a word | `**/*.md` over a mixed folder | List the file in `skipped` |
| :704 a file that is not UTF-8 is dropped without a word | One bad byte in an export | List it in `skipped`, or read line by line |
| :798–803 context grows quadratically | `thread` set to a grouping field such as a channel. Arithmetic, not run: 5,000 × 200-character messages ≈ 2.5 GB held before the cut | Cap the context |
| :803 context is oldest first, after the record's own text | The cut drops the message directly replied to first | Put the nearest context first |

The UTF-8 skip at :704 is the same code as `dfe3fc1` :690, and the base docs mention it. I did not run it against the base, so its origin is `undecided`.

**5. Attacks that held**
- Re-delivering unchanged records every run: covered by the first guard.
- Keys colliding across files: keys carry the file path, so they don't collide. Only threads collide.
- Old state files: the format is unchanged.
- Filter on a missing field: behaves as documented.
- CRLF and a last line with no newline.
- A credential in a key.
- I did not test ids containing `#` or `/`. A key collision there needs a file name containing `#`.

**6. Paths written outside the worktree:**
- `~/.cache/cortex-wave-20261005h/records/adversary-1/`, holding:
  - `build.log`, `list.log`
  - 11 `red-*.log`
  - `gate.log`, `suite-nff.log`
  - `tmp/` (empty)
- Builds into the assigned `~/.cache/b10x-target/cortex-w7-records`.

```findings
[
{"file":"src/sources.rs","line":798,"category":"judgement","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"Thread context repeats a message's text in its reply, so RareName counts a name written once as seen twice and sends it to the model raw."},
{"file":"src/sources.rs","line":798,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"A record with no thread field takes the context of the thread named by its id, so with per-file ids it carries another file's records as its own text, against spec-file.md:228."},
{"file":"src/sources.rs","line":865,"category":"boundary","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"A fence of one style is closed by the other style, so a fenced ## line becomes a heading and the next real section is lost."},
{"file":"src/sources.rs","line":867,"category":"boundary","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"A leading byte-order mark hides the first ## heading and its section is dropped without being listed in skipped."},
{"file":"src/sources.rs","line":845,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"A leading byte-order mark makes the first JSON line unparseable, so that record is lost (it is listed in skipped)."},
{"file":"src/sources.rs","line":790,"category":"boundary","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"A repeated heading's second section is dropped by the key check, never reaches the model and is not listed in skipped."},
{"file":"src/sources.rs","line":867,"category":"boundary","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"A markdown file with no ## heading yields no document and is not listed in skipped."},
{"file":"src/sources.rs","line":867,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"Setext level-two headings do not open a section, so a file that uses only them yields nothing."},
{"file":"src/sources.rs","line":704,"category":"boundary","severity":"warning","verdict":"CONFIRMED","origin":"undecided","message":"One byte that is not UTF-8 drops every record of a JSON-lines file and nothing is listed in skipped."},
{"file":"src/sources.rs","line":803,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"Context is uncapped and every record holds all earlier records of its thread, so a thread of n records builds about n squared over 2 times the message length in memory before the cut (arithmetic, not run)."},
{"file":"src/sources.rs","line":803,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"Context comes oldest first after the record's own text, so the length cut drops the message directly replied to first."}
]
```